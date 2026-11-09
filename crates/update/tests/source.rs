use lightproxy_core::Protocol;
use lightproxy_update::{asset_name, production_sources, ProxySource, Source};

const REPO: &str = "https://github.com/BeafJerky/LightProxy";

fn labels(sources: &[Source]) -> Vec<&str> {
    sources.iter().map(|source| source.label.as_str()).collect()
}

fn assert_all_https(sources: &[Source]) {
    for source in sources {
        assert!(
            source.download_base.starts_with("https://"),
            "{} 的下载地址不是 HTTPS：{}",
            source.label,
            source.download_base
        );
        if let Some(atom) = &source.atom_url {
            assert!(
                atom.starts_with("https://"),
                "{} 的 Atom 地址不是 HTTPS：{atom}",
                source.label
            );
        }
    }
}

#[test]
fn direct_source_targets_upstream_repository() {
    let direct = Source::direct().expect("直连来源应可构建");
    assert_eq!(direct.label, "直连");
    assert!(!direct.via_proxy);
    assert_eq!(
        direct.atom_url.as_deref(),
        Some("https://github.com/BeafJerky/LightProxy/releases.atom")
    );
    assert_eq!(
        direct.download_base,
        "https://github.com/BeafJerky/LightProxy/releases/download"
    );
    assert_eq!(
        direct.asset_url("0.1.4", &asset_name("0.1.4")),
        "https://github.com/BeafJerky/LightProxy/releases/download/v0.1.4/LightProxy_0.1.4_x64-setup.exe"
    );
}

#[test]
fn production_sources_keep_fixed_mirror_order_and_atom_support() {
    let sources = production_sources(None).expect("生产来源应可构建");
    assert_eq!(
        labels(&sources),
        [
            "直连",
            "镜像 gh-proxy.com",
            "镜像 gh-proxy.org",
            "镜像 wget.la"
        ]
    );
    assert_all_https(&sources);
    // 直连与两个 gh-proxy 镜像共享同一 Atom 路径，只差镜像前缀。
    assert_eq!(
        sources[0].atom_url.as_deref(),
        Some("https://github.com/BeafJerky/LightProxy/releases.atom")
    );
    assert_eq!(
        sources[1].atom_url.as_deref(),
        Some("https://gh-proxy.com/https://github.com/BeafJerky/LightProxy/releases.atom")
    );
    assert_eq!(
        sources[2].atom_url.as_deref(),
        Some("https://gh-proxy.org/https://github.com/BeafJerky/LightProxy/releases.atom")
    );
    assert_eq!(
        sources[3].atom_url, None,
        "wget.la 不支持 Atom，必须回退到无 Atom 分支"
    );
    assert_eq!(
        sources[3].download_base,
        format!("https://wget.la/{REPO}/releases/download")
    );
    assert!(sources.iter().all(|source| !source.via_proxy));
}

#[test]
fn proxy_source_keeps_upstream_urls_but_sets_via_proxy() {
    let proxy = ProxySource {
        url: "http://127.0.0.1:7890".to_string(),
        display: "127.0.0.1:7890".to_string(),
    };
    let sources = production_sources(Some(proxy)).expect("生产来源应可构建");
    assert_eq!(sources.len(), 5);
    assert_eq!(labels(&sources)[..2], ["直连", "代理 127.0.0.1:7890"]);
    assert_all_https(&sources);

    // 代理来源只标记 via_proxy（由 UpdateClient 换用代理客户端），URL 仍是上游地址。
    let proxied = &sources[1];
    assert!(proxied.via_proxy);
    assert_eq!(proxied.download_base, sources[0].download_base);
    assert_eq!(proxied.atom_url, sources[0].atom_url);
    assert_eq!(
        proxied.asset_url("0.1.4", &asset_name("0.1.4")),
        "https://github.com/BeafJerky/LightProxy/releases/download/v0.1.4/LightProxy_0.1.4_x64-setup.exe"
    );
}

#[test]
fn proxy_source_maps_http_and_socks5_urls() {
    let http = ProxySource::for_profile(Protocol::Http, "192.0.2.4:7890".into());
    assert_eq!(http.url, "http://192.0.2.4:7890");
    assert_eq!(http.display, "192.0.2.4:7890");

    let https = ProxySource::for_profile(Protocol::Https, "192.0.2.4:7890".into());
    assert_eq!(https.url, "http://192.0.2.4:7890");

    let socks = ProxySource::for_profile(Protocol::Socks5, "192.0.2.4:1080".into());
    assert_eq!(socks.url, "socks5h://192.0.2.4:1080");
    assert_eq!(socks.display, "192.0.2.4:1080");
}
