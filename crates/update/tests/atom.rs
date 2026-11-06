use lightproxy_update::{latest, parse_releases};

const FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.2"/></entry>
  <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.4"/></entry>
  <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.3"/></entry>
  <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/nightly"/></entry>
  <entry><link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.5-beta"/></entry>
  <entry><link rel="alternate" href="https://github.com/other/repo/releases/tag/v9.9.9"/></entry>
</feed>"#;

#[test]
fn extracts_only_strict_version_tags() {
    let entries = parse_releases(FEED);
    let versions: Vec<&str> = entries.iter().map(|entry| entry.version.as_str()).collect();
    assert_eq!(versions, ["0.1.2", "0.1.4", "0.1.3"]);
}

#[test]
fn picks_the_highest_version() {
    let entries = parse_releases(FEED);
    let newest = latest(&entries).expect("应找到版本");
    assert_eq!(newest.version, "0.1.4");
    assert_eq!(
        newest.url,
        "https://github.com/BeafJerky/LightProxy/releases/tag/v0.1.4"
    );
}

#[test]
fn ignores_duplicates_and_unknown_markup() {
    let feed = r#"
        <link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.2.0"/>
        <link rel="alternate" href="https://github.com/BeafJerky/LightProxy/releases/tag/v0.2.0"/>
        <link href="https://example.com/no-version"/>
    "#;
    assert_eq!(parse_releases(feed).len(), 1);
    assert!(latest(&[]).is_none());
}
