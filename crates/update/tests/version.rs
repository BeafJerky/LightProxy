use lightproxy_update::{
    asset_name, compare_versions, is_newer, is_version, parse_version, release_url, sha_asset_name,
    validate_https,
};
use std::cmp::Ordering;

#[test]
fn accepts_only_three_numeric_parts() {
    assert!(is_version("0.1.3"));
    assert!(is_version("10.20.30"));
    for value in ["0.1", "v0.1.3", "0.1.3-beta", "nightly", "0.1.3.4", "0..3"] {
        assert!(!is_version(value), "{value} 不应通过");
    }
}

#[test]
fn parses_and_compares_versions() {
    assert_eq!(parse_version("0.1.3").unwrap(), (0, 1, 3));
    assert!(parse_version("0.1").is_err());
    assert_eq!(compare_versions("0.1.10", "0.1.9"), Ordering::Greater);
    assert_eq!(compare_versions("0.2.0", "0.2.0"), Ordering::Equal);
    assert_eq!(compare_versions("0.1.2", "0.1.3"), Ordering::Less);
    assert!(is_newer("0.1.4", "0.1.3"));
    assert!(!is_newer("0.1.3", "0.1.3"));
    assert!(!is_newer("0.1.2", "0.1.3"));
    assert!(!is_newer("nightly", "0.1.3"));
}

#[test]
fn builds_fixed_asset_names_and_urls() {
    assert_eq!(asset_name("0.1.4"), "LightProxy_0.1.4_x64-setup.exe");
    assert_eq!(
        sha_asset_name("0.1.4"),
        "LightProxy_0.1.4_x64-setup.exe.sha256"
    );
    assert_eq!(
        release_url("0.1.4"),
        "https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.4"
    );
}

#[test]
fn rejects_non_https_urls() {
    assert!(validate_https("https://gh-proxy.com/").is_ok());
    assert!(validate_https("http://gh-proxy.com/").is_err());
    assert!(validate_https("https://").is_err());
}
