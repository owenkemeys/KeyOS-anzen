use std::{fs, path::PathBuf};

fn app_file(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../app")
        .join(name);
    fs::read_to_string(path).expect("read Prime app metadata")
}

fn quoted_value<'a>(contents: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key} = \"");
    contents
        .lines()
        .find_map(|line| line.strip_prefix(&prefix)?.strip_suffix('"'))
        .unwrap_or_else(|| panic!("missing quoted {key}"))
}

#[test]
fn keyos_app_id_is_16_bytes_and_consistent() {
    let app_config = app_file("app-config.toml");
    let manifest = app_file("manifest.toml");
    let config_id = quoted_value(&app_config, "app-id");
    let manifest_id = quoted_value(&manifest, "appId");

    assert_eq!(config_id, manifest_id, "KeyOS app IDs must match");
    let hex = config_id.strip_prefix("0x").expect("app ID starts with 0x");
    assert_eq!(hex.len(), 32, "KeyOS app ID must contain 16 bytes");
    assert!(hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
}
