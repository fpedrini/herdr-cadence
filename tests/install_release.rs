#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn executable(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn path_with_fake_tools(fake_bin: &Path) -> std::ffi::OsString {
    std::env::join_paths(
        [fake_bin.to_path_buf()]
            .into_iter()
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap()
}

fn fixture(tar_body: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/install-release.sh"),
        root.join("scripts/install-release.sh"),
    )
    .unwrap();
    fs::write(root.join("herdr-plugin.toml"), "version = \"0.9.0\"\n").unwrap();
    fs::create_dir(root.join("fake-bin")).unwrap();
    executable(
        root.join("fake-bin/uname").as_path(),
        "if [ \"$1\" = \"-s\" ]; then echo Linux; else echo x86_64; fi",
    );
    executable(
        root.join("fake-bin/curl").as_path(),
        r#"out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output" ]; then out="$2"; shift 2; else shift; fi
done
: > "$out"
"#,
    );
    executable(root.join("fake-bin/sha256sum").as_path(), "exit 0");
    executable(root.join("fake-bin/tar").as_path(), tar_body);
    temp
}

fn run_fixture(fixture: &tempfile::TempDir) -> std::process::Output {
    Command::new("sh")
        .arg(fixture.path().join("scripts/install-release.sh"))
        .current_dir(fixture.path())
        .env(
            "PATH",
            path_with_fake_tools(&fixture.path().join("fake-bin")),
        )
        .output()
        .unwrap()
}

#[test]
fn failed_extraction_preserves_existing_binary() {
    let fixture = fixture(
        r#"stage=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-C" ]; then stage="$2"; shift 2; else shift; fi
done
mkdir -p "$stage"
printf '%s\n' 'partial binary' > "$stage/herdr-cadence"
exit 1
"#,
    );
    fs::create_dir(fixture.path().join("bin")).unwrap();
    fs::write(fixture.path().join("bin/herdr-cadence"), "old binary\n").unwrap();

    let result = run_fixture(&fixture);

    assert!(!result.status.success());
    assert_eq!(
        fs::read_to_string(fixture.path().join("bin/herdr-cadence")).unwrap(),
        "old binary\n"
    );
}

#[test]
fn checksum_failure_does_not_extract_or_replace_binary() {
    let fixture = fixture("printf '%s\\n' invoked > tar-invoked\nexit 0");
    fs::write(
        fixture.path().join("fake-bin/sha256sum"),
        "#!/bin/sh\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(
        fixture.path().join("fake-bin/sha256sum"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::create_dir(fixture.path().join("bin")).unwrap();
    fs::write(fixture.path().join("bin/herdr-cadence"), "old binary\n").unwrap();

    let result = run_fixture(&fixture);

    assert!(!result.status.success());
    assert!(!fixture.path().join("tar-invoked").exists());
    assert_eq!(
        fs::read_to_string(fixture.path().join("bin/herdr-cadence")).unwrap(),
        "old binary\n"
    );
}

#[test]
fn successful_extraction_replaces_binary_only_after_staging() {
    let fixture = fixture(
        r#"stage=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-C" ]; then stage="$2"; shift 2; else shift; fi
done
mkdir -p "$stage"
printf '%s\n' 'new binary' > "$stage/herdr-cadence"
"#,
    );
    fs::create_dir(fixture.path().join("bin")).unwrap();
    fs::write(fixture.path().join("bin/herdr-cadence"), "old binary\n").unwrap();

    let result = run_fixture(&fixture);

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(fixture.path().join("bin/herdr-cadence")).unwrap(),
        "new binary\n"
    );
    assert_eq!(
        fs::metadata(fixture.path().join("bin/herdr-cadence"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
}

#[test]
fn missing_manifest_version_fails_before_download() {
    let fixture = fixture("exit 0");
    fs::write(
        fixture.path().join("herdr-plugin.toml"),
        "name = \"Cadence\"\n",
    )
    .unwrap();

    let result = run_fixture(&fixture);

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("determine Cadence version"));
    assert!(!fixture.path().join("bin").exists());
}
