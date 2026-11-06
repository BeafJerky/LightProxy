use crate::backend::ProxyBackend;
use crate::error::CoreError;
use crate::model::ProxySnapshot;

#[cfg(windows)]
mod platform {
    use super::*;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    const DEFAULT_ROOT: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings";
    const INTERNET_OPTION_SETTINGS_CHANGED: u32 = 39;
    const INTERNET_OPTION_REFRESH: u32 = 37;

    #[derive(Debug, Clone)]
    pub struct WinInetBackend {
        root: String,
    }

    impl Default for WinInetBackend {
        fn default() -> Self {
            let root = std::env::var("LIGHTPROXY_TEST_REGISTRY_ROOT")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_ROOT.to_string());
            Self { root }
        }
    }

    impl WinInetBackend {
        pub fn with_root(root: impl Into<String>) -> Self {
            Self { root: root.into() }
        }

        fn key_read(&self) -> Result<RegKey, CoreError> {
            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey_with_flags(&self.root, KEY_READ)
                .map_err(|err| {
                    CoreError::new(
                        "registry_open_failed",
                        "read",
                        format!("无法打开代理注册表：{err}"),
                    )
                    .with_hint("检查当前用户是否可访问 Internet Settings")
                })
        }
    }

    impl ProxyBackend for WinInetBackend {
        fn read(&self) -> Result<ProxySnapshot, CoreError> {
            let key = match self.key_read() {
                Ok(key) => key,
                // Missing key means "never configured", which is a valid baseline.
                Err(_) => return Ok(ProxySnapshot::default()),
            };
            let enable: u32 = key.get_value("ProxyEnable").unwrap_or(0);
            let server: String = key.get_value("ProxyServer").unwrap_or_default();
            let override_value: String = key.get_value("ProxyOverride").unwrap_or_default();
            let auto: String = key.get_value("AutoConfigURL").unwrap_or_default();
            Ok(ProxySnapshot {
                proxy_enable: enable != 0,
                proxy_server: server,
                proxy_override: override_value,
                auto_config_url: if auto.trim().is_empty() {
                    None
                } else {
                    Some(auto)
                },
            })
        }

        fn write(&self, snapshot: &ProxySnapshot) -> Result<(), CoreError> {
            let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey(&self.root)
                .map_err(|err| {
                    CoreError::new(
                        "registry_write_failed",
                        "write",
                        format!("无法写入注册表：{err}"),
                    )
                    .with_hint("检查组策略或安全软件是否锁定 Internet Settings")
                })?;
            let _ = key.set_value("ProxyEnable", &u32::from(snapshot.proxy_enable));
            key.set_value("ProxyServer", &snapshot.proxy_server)
                .map_err(|err| CoreError::new("registry_write_failed", "write", err.to_string()))?;
            key.set_value("ProxyOverride", &snapshot.proxy_override)
                .map_err(|err| CoreError::new("registry_write_failed", "write", err.to_string()))?;
            match &snapshot.auto_config_url {
                Some(url) => {
                    key.set_value("AutoConfigURL", url).map_err(|err| {
                        CoreError::new("registry_write_failed", "write", err.to_string())
                    })?;
                }
                None => {
                    let _ = key.delete_value("AutoConfigURL");
                }
            }
            Ok(())
        }

        fn notify(&self) -> Result<(), CoreError> {
            let settings_changed = unsafe {
                windows_sys::Win32::Networking::WinInet::InternetSetOptionW(
                    std::ptr::null_mut(),
                    INTERNET_OPTION_SETTINGS_CHANGED,
                    std::ptr::null_mut(),
                    0,
                )
            };
            let refreshed = unsafe {
                windows_sys::Win32::Networking::WinInet::InternetSetOptionW(
                    std::ptr::null_mut(),
                    INTERNET_OPTION_REFRESH,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if settings_changed == 0 && refreshed == 0 {
                return Err(CoreError::new(
                    "notify_failed",
                    "notify",
                    "已写入注册表，但系统通知失败",
                )
                .with_hint("重启浏览器后生效"));
            }
            Ok(())
        }
    }
}

#[cfg(windows)]
pub use platform::WinInetBackend;

#[cfg(not(windows))]
#[derive(Debug, Default, Clone)]
pub struct WinInetBackend;

#[cfg(not(windows))]
impl ProxyBackend for WinInetBackend {
    fn read(&self) -> Result<ProxySnapshot, CoreError> {
        Err(CoreError::new(
            "unsupported_platform",
            "read",
            "仅支持 Windows",
        ))
    }

    fn write(&self, _snapshot: &ProxySnapshot) -> Result<(), CoreError> {
        Err(CoreError::new(
            "unsupported_platform",
            "write",
            "仅支持 Windows",
        ))
    }

    fn notify(&self) -> Result<(), CoreError> {
        Err(CoreError::new(
            "unsupported_platform",
            "notify",
            "仅支持 Windows",
        ))
    }
}
