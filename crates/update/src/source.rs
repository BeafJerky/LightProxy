use lightproxy_core::{CoreError, Protocol};

pub const REPO_ROOT: &str = "https://github.com/BeafJerky/LightProxy";
pub const MIRRORS: [(&str, bool); 3] = [
    ("gh-proxy.com", true),
    ("gh-proxy.org", true),
    ("wget.la", false),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxySource {
    pub url: String,
    pub display: String,
}

impl ProxySource {
    pub fn for_profile(protocol: Protocol, endpoint: String) -> Self {
        match protocol {
            Protocol::Socks5 => Self {
                url: format!("socks5h://{endpoint}"),
                display: endpoint,
            },
            Protocol::Http | Protocol::Https => Self {
                url: format!("http://{endpoint}"),
                display: endpoint,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    pub label: String,
    pub atom_url: Option<String>,
    pub download_base: String,
    pub via_proxy: bool,
}

impl Source {
    pub fn direct() -> Result<Self, CoreError> {
        Self::trusted("直连", "", true, false)
    }

    pub fn proxy(proxy: &ProxySource) -> Result<Self, CoreError> {
        Self::trusted(&format!("代理 {}", proxy.display), "", true, true)
    }

    pub fn mirror(host: &str, supports_atom: bool) -> Result<Self, CoreError> {
        let valid = !host.is_empty()
            && host
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-');
        if !valid {
            return Err(CoreError::new(
                "update_check_failed",
                "update",
                format!("镜像地址无效：{host}"),
            ));
        }
        Self::trusted(
            &format!("镜像 {host}"),
            &format!("https://{host}/"),
            supports_atom,
            false,
        )
    }

    /// 所有生产来源都经这里装配：URL 只在构造期生成，并在构造期按 HTTPS 校验。
    fn trusted(
        label: &str,
        prefix: &str,
        supports_atom: bool,
        via_proxy: bool,
    ) -> Result<Self, CoreError> {
        let root = format!("{prefix}{REPO_ROOT}");
        let atom_url = supports_atom.then(|| format!("{root}/releases.atom"));
        let download_base = format!("{root}/releases/download");
        if let Some(atom) = atom_url.as_deref() {
            validate_https(atom)?;
        }
        validate_https(&download_base)?;
        Ok(Self {
            label: label.to_string(),
            atom_url,
            download_base,
            via_proxy,
        })
    }

    pub fn asset_url(&self, version: &str, asset: &str) -> String {
        format!("{}/v{version}/{asset}", self.download_base)
    }

    /// Test-only constructor that permits a local HTTP fixture.
    #[doc(hidden)]
    pub fn local(label: &str, base: &str) -> Self {
        Self {
            label: label.to_string(),
            atom_url: Some(format!("{base}/releases.atom")),
            download_base: format!("{base}/download"),
            via_proxy: false,
        }
    }
}

pub fn production_sources(proxy: Option<ProxySource>) -> Result<Vec<Source>, CoreError> {
    let mut sources = vec![Source::direct()?];
    if let Some(proxy) = proxy {
        sources.push(Source::proxy(&proxy)?);
    }
    for (host, supports_atom) in MIRRORS {
        sources.push(Source::mirror(host, supports_atom)?);
    }
    Ok(sources)
}

/// 更新链路的硬约束：所有请求都必须是 `https://`。由 [`Source`] 在构造期调用。
pub fn validate_https(url: &str) -> Result<(), CoreError> {
    if url.len() > "https://".len() && url.starts_with("https://") {
        Ok(())
    } else {
        Err(CoreError::new(
            "update_check_failed",
            "update",
            "拒绝非 HTTPS 更新地址",
        ))
    }
}
