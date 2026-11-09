use crate::backend::ProxyBackend;
use crate::error::CoreError;
use crate::model::{EnvVar, ProxySnapshot};

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::env::ENV_VARS;
    use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    const DEFAULT_ROOT: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings";
    const INTERNET_OPTION_SETTINGS_CHANGED: u32 = 39;
    const INTERNET_OPTION_REFRESH: u32 = 37;

    fn decode_sz(bytes: &[u8]) -> String {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
        String::from_utf16_lossy(&units[..end])
    }

    fn encode_sz(value: &str) -> Vec<u8> {
        let mut units: Vec<u16> = value.encode_utf16().collect();
        units.push(0);
        units.iter().flat_map(|u| u.to_le_bytes()).collect()
    }

    fn absent_env() -> Vec<EnvVar> {
        ENV_VARS
            .iter()
            .map(|n| EnvVar {
                name: (*n).into(),
                value: None,
                expand: false,
            })
            .collect()
    }

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

        fn env_key_read(&self) -> Result<RegKey, CoreError> {
            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey_with_flags("Environment", KEY_READ)
                .map_err(|err| {
                    CoreError::new(
                        "env_read_failed",
                        "env_read",
                        format!("无法打开环境变量注册表：{err}"),
                    )
                    .with_hint("检查当前用户是否可访问 HKCU\\Environment")
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

        fn read_env(&self) -> Result<Vec<EnvVar>, CoreError> {
            let key = match self.env_key_read() {
                Ok(key) => key,
                Err(_) => return Ok(absent_env()),
            };
            let mut found: Vec<EnvVar> = Vec::new();
            for item in key.enum_values() {
                let (name, value) = item.map_err(|err| {
                    CoreError::new("env_read_failed", "env_read", err.to_string())
                })?;
                let lower = name.to_ascii_lowercase();
                if !ENV_VARS.contains(&lower.as_str()) {
                    continue;
                }
                // 环境变量正常均为字符串；非字符串类型罕见，按不存在处理。
                if matches!(value.vtype, RegType::REG_SZ | RegType::REG_EXPAND_SZ) {
                    found.push(EnvVar {
                        name,
                        value: Some(decode_sz(&value.bytes)),
                        expand: value.vtype == RegType::REG_EXPAND_SZ,
                    });
                }
            }
            Ok(ENV_VARS
                .iter()
                .map(|n| {
                    found
                        .iter()
                        .find(|v| v.name.eq_ignore_ascii_case(n))
                        .cloned()
                        .unwrap_or(EnvVar {
                            name: (*n).into(),
                            value: None,
                            expand: false,
                        })
                })
                .collect())
        }

        fn write_env(&self, vars: &[EnvVar]) -> Result<(), CoreError> {
            let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey("Environment")
                .map_err(|err| {
                    CoreError::new(
                        "env_write_failed",
                        "env_write",
                        format!("无法写入环境变量：{err}"),
                    )
                    .with_hint("检查组策略或安全软件是否锁定 HKCU\\Environment")
                })?;
            for v in vars {
                match &v.value {
                    Some(value) => {
                        let vtype = if v.expand {
                            RegType::REG_EXPAND_SZ
                        } else {
                            RegType::REG_SZ
                        };
                        key.set_raw_value(
                            &v.name,
                            &winreg::RegValue {
                                bytes: encode_sz(value),
                                vtype,
                            },
                        )
                        .map_err(|err| {
                            CoreError::new("env_write_failed", "env_write", err.to_string())
                        })?;
                    }
                    None => {
                        let _ = key.delete_value(&v.name);
                    }
                }
            }
            Ok(())
        }

        fn notify_env(&self) -> Result<(), CoreError> {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
            };
            let mut env: Vec<u16> = "Environment".encode_utf16().collect();
            env.push(0);
            let result = unsafe {
                SendMessageTimeoutW(
                    HWND_BROADCAST,
                    WM_SETTINGCHANGE,
                    0,
                    env.as_ptr() as isize,
                    SMTO_ABORTIFHUNG,
                    5000,
                    std::ptr::null_mut(),
                )
            };
            if result == 0 {
                return Err(CoreError::new(
                    "env_notify_failed",
                    "env_notify",
                    "已写入环境变量，但系统广播失败",
                )
                .with_hint("重新登录或重启资源管理器后生效"));
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

    fn read_env(&self) -> Result<Vec<EnvVar>, CoreError> {
        Err(CoreError::new(
            "unsupported_platform",
            "env_read",
            "仅支持 Windows",
        ))
    }

    fn write_env(&self, _vars: &[EnvVar]) -> Result<(), CoreError> {
        Err(CoreError::new(
            "unsupported_platform",
            "env_write",
            "仅支持 Windows",
        ))
    }

    fn notify_env(&self) -> Result<(), CoreError> {
        Err(CoreError::new(
            "unsupported_platform",
            "env_notify",
            "仅支持 Windows",
        ))
    }
}
