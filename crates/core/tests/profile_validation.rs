use lightproxy_core::model::{Profile, Protocol};
use lightproxy_core::profile::{parse_endpoint, validate_profile};

fn profile(id: &str, name: &str, host: &str, port: u16) -> Profile {
    Profile {
        id: id.into(),
        name: name.into(),
        protocol: Protocol::Http,
        host: host.into(),
        port,
    }
}

#[test]
fn parses_supported_endpoints() {
    let (protocol, host, port) = parse_endpoint("http://192.168.5.4:7890").unwrap();
    assert_eq!(protocol, Protocol::Http);
    assert_eq!(host, "192.168.5.4");
    assert_eq!(port, 7890);

    assert_eq!(
        parse_endpoint("192.168.5.4:7890").unwrap(),
        (Protocol::Http, "192.168.5.4".into(), 7890)
    );
    assert_eq!(
        parse_endpoint("https://proxy.example.com:443").unwrap().0,
        Protocol::Https
    );
    assert_eq!(parse_endpoint("[::1]:7890").unwrap().1, "::1".to_string());
}

#[test]
fn rejects_bad_endpoints() {
    assert!(parse_endpoint("http://host:0").is_err());
    assert!(parse_endpoint("http://host:99999").is_err());
    assert!(parse_endpoint("ftp://host:21").is_err());
    assert!(parse_endpoint("host").is_err());
    assert!(parse_endpoint("http://:7890").is_err());
    assert!(parse_endpoint("http://ho st:7890").is_err());
    assert!(parse_endpoint("::1:7890").is_err());
}

#[test]
fn validates_profile_names_and_duplicates() {
    let existing = vec![profile("p1", "公司代理", "192.168.5.4", 7890)];
    assert!(validate_profile(&profile("p1", "公司代理", "192.168.5.4", 7890), &existing).is_ok());
    assert!(validate_profile(&profile("p2", "公司代理", "192.168.5.4", 7890), &existing).is_err());
    assert!(validate_profile(&profile("p2", "  ", "192.168.5.4", 7890), &existing).is_err());
    assert!(validate_profile(&profile("p2", "备用", "", 7890), &existing).is_err());
    assert!(validate_profile(&profile("p2", "备用", "192.168.5.4", 0), &existing).is_err());
}

#[test]
fn formats_ipv6_server_with_brackets() {
    let item = profile("p6", "IPv6", "::1", 7890);
    assert_eq!(item.server(), "[::1]:7890");
    assert_eq!(
        profile("p4", "v4", "10.0.0.2", 8080).server(),
        "10.0.0.2:8080"
    );
}
