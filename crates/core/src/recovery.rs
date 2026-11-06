use crate::model::{PersistedState, Profile, ProxySnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryDecision {
    On {
        profile_id: String,
        fill_snapshot: bool,
    },
    Degraded {
        profile_id: Option<String>,
        message: String,
    },
    Off {
        message: Option<String>,
    },
}

/// Decides what the registry means after a start. `reachable` is the TCP probe
/// result for the matched profile; `None` means "not probed".
pub fn decide_recovery(
    clean: bool,
    registry: &ProxySnapshot,
    persisted: &PersistedState,
    reachable: Option<bool>,
) -> RecoveryDecision {
    if let Some(pac) = &registry.auto_config_url {
        return RecoveryDecision::Degraded {
            profile_id: None,
            message: format!("检测到 PAC 配置（{pac}），PAC 优先生效，未自动接管"),
        };
    }

    if !registry.proxy_enable {
        return RecoveryDecision::Off {
            message: if clean {
                None
            } else {
                Some("上次未正常退出，代理当前为关闭状态".into())
            },
        };
    }

    let matched = persisted
        .profiles
        .iter()
        .find(|profile| profile.server() == registry.proxy_server);

    match matched {
        Some(profile) => {
            if reachable == Some(false) {
                RecoveryDecision::Degraded {
                    profile_id: Some(profile.id.clone()),
                    message: "代理已开启，但端口不可达".into(),
                }
            } else {
                RecoveryDecision::On {
                    profile_id: profile.id.clone(),
                    fill_snapshot: persisted.snapshot.is_none(),
                }
            }
        }
        None => RecoveryDecision::Degraded {
            profile_id: None,
            message: format!("检测到未知代理：{}", proxy_label(registry)),
        },
    }
}

pub fn matching_profile<'a>(
    profiles: &'a [Profile],
    registry: &ProxySnapshot,
) -> Option<&'a Profile> {
    profiles
        .iter()
        .find(|profile| profile.server() == registry.proxy_server)
}

fn proxy_label(registry: &ProxySnapshot) -> String {
    if registry.proxy_server.trim().is_empty() {
        "未填写地址".into()
    } else {
        registry.proxy_server.clone()
    }
}
