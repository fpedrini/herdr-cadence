#[test]
fn plugin_version_matches_cargo_package() {
    let plugin: toml::Value = toml::from_str(include_str!("../herdr-plugin.toml")).unwrap();
    assert_eq!(plugin["version"].as_str(), Some(env!("CARGO_PKG_VERSION")));
}

#[test]
fn plugin_manifest_points_at_existing_runtime_commands() {
    let plugin: toml::Value = toml::from_str(include_str!("../herdr-plugin.toml")).unwrap();
    assert_eq!(plugin["id"].as_str(), Some("herdr-cadence"));
    assert!(plugin["platforms"].as_array().is_some_and(|platforms| {
        platforms
            .iter()
            .any(|platform| platform.as_str() == Some("linux"))
            && platforms
                .iter()
                .any(|platform| platform.as_str() == Some("macos"))
    }));
    assert_eq!(
        plugin["build"][0]["command"],
        toml::Value::Array(
            ["sh", "scripts/install-release.sh"]
                .into_iter()
                .map(|value| toml::Value::String(value.into()))
                .collect()
        )
    );
    assert_eq!(
        plugin["startup"][0]["command"][0].as_str(),
        Some("bin/herdr-cadence")
    );
    assert!(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/install-release.sh")
            .is_file()
    );
}
