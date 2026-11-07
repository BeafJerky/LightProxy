use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

pub const THEMES: [&str; 8] = [
    "light",
    "dark",
    "system",
    "win11",
    "win11-dark",
    "terminal",
    "paper",
    "contrast",
];

pub const DEFAULT_PROXY_OVERRIDE: &str = "<local>;localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    Https,
    Socks5,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5 => "socks5",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "http" => Some(Self::Http),
            "https" => Some(Self::Https),
            "socks5" | "socks" => Some(Self::Socks5),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
}

impl Profile {
    /// The value written to `ProxyServer`; IPv6 literals keep their brackets.
    /// SOCKS5 uses the per-protocol `socks=` entry so HTTP clients are not
    /// pointed at a SOCKS port.
    pub fn server(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        match self.protocol {
            Protocol::Socks5 => format!("socks={}:{}", host, self.port),
            _ => format!("{}:{}", host, self.port),
        }
    }

    pub fn display_url(&self) -> String {
        format!("{}://{}", self.protocol.as_str(), self.server())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub protocol: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotkeys {
    pub toggle: String,
    pub panel: String,
    pub next_profile: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            toggle: "Ctrl+Alt+P".into(),
            panel: "Ctrl+Alt+O".into(),
            next_profile: "Ctrl+Alt+N".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub autostart: bool,
    pub close_action: String,
    pub preflight_enabled: bool,
    pub preflight_timeout_ms: u64,
    pub theme: String,
    pub hotkeys: Hotkeys,
    pub tray_click_action: String,
    pub proxy_override: String,
    pub keep_proxy_on_exit: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            autostart: true,
            close_action: "hide".into(),
            preflight_enabled: true,
            preflight_timeout_ms: 800,
            theme: "system".into(),
            hotkeys: Hotkeys::default(),
            tray_click_action: "panel".into(),
            proxy_override: DEFAULT_PROXY_OVERRIDE.into(),
            keep_proxy_on_exit: false,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        if !THEMES.contains(&self.theme.as_str()) {
            self.theme = "system".into();
        }
        if self.close_action != "exit" {
            self.close_action = "hide".into();
        }
        if self.tray_click_action != "toggle" {
            self.tray_click_action = "panel".into();
        }
        self.preflight_timeout_ms = self.preflight_timeout_ms.clamp(300, 3000);
        if self.proxy_override.trim().is_empty() {
            self.proxy_override = DEFAULT_PROXY_OVERRIDE.into();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyState {
    Off,
    Enabling,
    On,
    Degraded,
    Repairing,
    Disabling,
    Error,
}

impl ProxyState {
    pub fn is_active(self) -> bool {
        matches!(self, Self::On | Self::Degraded)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Desired {
    #[default]
    Off,
    On,
}

/// The four WinINET values LightProxy owns. `auto_config_url: None` means the
/// value must be absent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySnapshot {
    pub proxy_enable: bool,
    pub proxy_server: String,
    pub proxy_override: String,
    pub auto_config_url: Option<String>,
}

impl ProxySnapshot {
    pub fn same_proxy(&self, other: &Self) -> bool {
        self.proxy_enable == other.proxy_enable && self.proxy_server == other.proxy_server
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResult {
    pub ok: bool,
    pub stage: String,
    pub latency_ms: u64,
    pub checked_at_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeState {
    pub desired: Desired,
    pub active_profile_id: Option<String>,
    pub last_health: Option<HealthResult>,
    #[serde(default)]
    pub last_pac_url: Option<String>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            desired: Desired::Off,
            active_profile_id: None,
            last_health: None,
            last_pac_url: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairRecord {
    pub id: String,
    pub title: String,
    pub ok: bool,
    pub message: String,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PersistedState {
    pub schema_version: u32,
    pub clean_shutdown: bool,
    pub profiles: Vec<Profile>,
    pub default_profile_id: Option<String>,
    pub settings: Settings,
    pub runtime: RuntimeState,
    pub snapshot: Option<ProxySnapshot>,
    pub repairs: Vec<RepairRecord>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self::initial()
    }
}

impl PersistedState {
    pub fn initial() -> Self {
        let profiles = vec![
            Profile {
                id: "p_default".into(),
                name: "公司代理".into(),
                protocol: Protocol::Http,
                host: "192.168.5.4".into(),
                port: 7890,
            },
            Profile {
                id: "p_backup".into(),
                name: "备用代理".into(),
                protocol: Protocol::Http,
                host: "192.168.5.9".into(),
                port: 7890,
            },
        ];
        Self {
            schema_version: SCHEMA_VERSION,
            clean_shutdown: true,
            profiles,
            default_profile_id: Some("p_default".into()),
            settings: Settings::default(),
            runtime: RuntimeState::default(),
            snapshot: None,
            repairs: Vec::new(),
        }
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn default_profile(&self) -> Option<&Profile> {
        self.default_profile_id
            .as_deref()
            .and_then(|id| self.profile(id))
            .or_else(|| self.profiles.first())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub state: ProxyState,
    pub desired: Desired,
    pub active_profile_id: Option<String>,
    pub last_health: Option<HealthResult>,
    pub message: Option<String>,
    pub profiles: Vec<Profile>,
    pub default_profile_id: Option<String>,
    pub settings: Settings,
    pub snapshot: Option<ProxySnapshot>,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
