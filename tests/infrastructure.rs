#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use herdr_cadence::git::{changed_paths_for_commit, head};
use herdr_cadence::model::path_within_scope;

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

#[test]
fn build_local_resolves_project_from_script_and_preserves_binary_on_install_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project with spaces");
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/build-local.sh"),
        root.join("scripts/build-local.sh"),
    )
    .unwrap();
    fs::create_dir(root.join("bin")).unwrap();
    fs::write(root.join("bin/herdr-cadence"), "old binary\n").unwrap();
    fs::create_dir(root.join("fake-bin")).unwrap();
    executable(
        root.join("fake-bin/cargo").as_path(),
        r#"mkdir -p target/release
printf '%s\n' 'new binary' > target/release/herdr-cadence
exit 0
"#,
    );
    executable(root.join("fake-bin/install").as_path(), "exit 1");

    let result = Command::new("sh")
        .arg(root.join("scripts/build-local.sh"))
        .current_dir(temp.path())
        .env("PATH", path_with_fake_tools(&root.join("fake-bin")))
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert_eq!(
        fs::read_to_string(root.join("bin/herdr-cadence")).unwrap(),
        "old binary\n"
    );
}

#[test]
fn build_local_success_uses_fake_herdr_and_replaces_binary() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project with spaces");
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/build-local.sh"),
        root.join("scripts/build-local.sh"),
    )
    .unwrap();
    fs::create_dir(root.join("fake-bin")).unwrap();
    executable(
        root.join("fake-bin/cargo").as_path(),
        r#"mkdir -p target/release
printf '%s\n' 'new binary' > target/release/herdr-cadence
exit 0
"#,
    );
    executable(
        root.join("fake-bin/install").as_path(),
        r#"cp "$3" "$4"
"#,
    );
    executable(
        root.join("fake-bin/herdr").as_path(),
        r#"printf '%s\n' "$*" > herdr-call
"#,
    );

    let result = Command::new("sh")
        .arg(root.join("scripts/build-local.sh"))
        .current_dir(temp.path())
        .env("PATH", path_with_fake_tools(&root.join("fake-bin")))
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(root.join("bin/herdr-cadence")).unwrap(),
        "new binary\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("herdr-call")).unwrap(),
        format!("plugin link {}\n", root.display())
    );
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn scoped_acceptance_rejects_rename_from_outside_scope() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repo");
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-b", "main"]);
    git(&root, &["config", "user.email", "cadence@example.test"]);
    git(&root, &["config", "user.name", "Cadence Test"]);
    fs::write(root.join("outside.txt"), "source\n").unwrap();
    git(&root, &["add", "outside.txt"]);
    git(&root, &["commit", "-m", "base"]);
    fs::create_dir(root.join("src")).unwrap();
    fs::rename(root.join("outside.txt"), root.join("src/inside.txt")).unwrap();
    git(&root, &["config", "diff.renames", "true"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-m", "rename"]);

    let commit = head(&root).unwrap();
    let changed = changed_paths_for_commit(&root, &commit).unwrap();
    assert_eq!(changed, ["outside.txt", "src/inside.txt"]);
    assert!(path_within_scope("src/inside.txt", &["src".into()]));
    assert!(!path_within_scope("outside.txt", &["src".into()]));
    assert!(
        changed
            .iter()
            .any(|path| !path_within_scope(path, &["src".into()]))
    );
}
