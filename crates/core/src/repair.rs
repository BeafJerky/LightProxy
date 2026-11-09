use crate::env;
use crate::model::{
    now_ms, Desired, EnvVar, HealthResult, ProxySnapshot, ProxyState, RepairRecord,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub severity: String,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairResult {
    pub id: String,
    pub title: String,
    pub ok: bool,
    pub message: String,
}

pub struct DiagnosticInput<'a> {
    pub desired: Desired,
    pub state: ProxyState,
    pub registry: &'a ProxySnapshot,
    pub last_health: Option<&'a HealthResult>,
    pub store_warning: Option<&'a str>,
    pub last_save_error: Option<&'a str>,
    pub hotkey_failures: &'a [String],
    pub sync_env_proxy: bool,
    pub env_current: &'a [EnvVar],
    pub env_expected: Option<&'a [EnvVar]>,
    pub env_snapshot_present: bool,
}

pub fn build_diagnostics(input: DiagnosticInput<'_>) -> Vec<Diagnostic> {
    let mut items = Vec::new();

    if input.desired == Desired::On && !input.registry.proxy_enable {
        items.push(Diagnostic {
            id: "R01".into(),
            title: "代理未保持开启".into(),
            detail: "记录显示期望开启，但系统代理当前为关闭，可能是异常退出后再被外部关闭。".into(),
            severity: "warn".into(),
            auto_fixable: true,
        });
    }
    if let Some(pac) = &input.registry.auto_config_url {
        items.push(Diagnostic {
            id: "R03".into(),
            title: "PAC 脚本优先".into(),
            detail: format!("检测到 AutoConfigURL：{pac}。清空后固定代理才会生效。"),
            severity: "warn".into(),
            auto_fixable: true,
        });
    }
    if let Some(health) = input.last_health {
        if !health.ok {
            items.push(Diagnostic {
                id: "R04".into(),
                title: "代理不可达".into(),
                detail: health
                    .error
                    .clone()
                    .unwrap_or_else(|| "最近一次 TCP 预检失败".into()),
                severity: "error".into(),
                auto_fixable: false,
            });
        }
        if health.checked_at_ms > now_ms().saturating_add(60_000) {
            items.push(Diagnostic {
                id: "R14".into(),
                title: "检查时间异常".into(),
                detail: "最近检查时间晚于系统时间，建议重新检查。".into(),
                severity: "warn".into(),
                auto_fixable: false,
            });
        }
    }
    if let Some(err) = input.last_save_error {
        items.push(Diagnostic {
            id: "R05".into(),
            title: "状态写入失败".into(),
            detail: err.to_string(),
            severity: "warn".into(),
            auto_fixable: true,
        });
    }
    if let Some(warning) = input.store_warning {
        items.push(Diagnostic {
            id: "R06".into(),
            title: "配置文件已恢复".into(),
            detail: warning.to_string(),
            severity: "warn".into(),
            auto_fixable: false,
        });
    }
    if !input.hotkey_failures.is_empty() {
        items.push(Diagnostic {
            id: "R07".into(),
            title: "快捷键未生效".into(),
            detail: input.hotkey_failures.join("；"),
            severity: "warn".into(),
            auto_fixable: false,
        });
    }
    if input.state == ProxyState::Error {
        items.push(Diagnostic {
            id: "R05E".into(),
            title: "状态机处于错误态".into(),
            detail: "请查看最近错误信息，重试或关闭代理后重新启用。".into(),
            severity: "error".into(),
            auto_fixable: false,
        });
    }
    if input.sync_env_proxy {
        let problem = if input.desired == Desired::On {
            match input.env_expected {
                Some(expected) => !env::already_applied(input.env_current, expected),
                None => true,
            }
        } else {
            input.env_snapshot_present
        };
        if problem {
            items.push(Diagnostic {
                id: "R15".into(),
                title: "命令行代理未同步".into(),
                detail: if input.desired == Desired::On {
                    "环境变量与当前配置不一致，新启动的命令行工具可能不走代理。".into()
                } else {
                    "代理已关闭，但命令行环境变量尚未恢复。".into()
                },
                severity: "warn".into(),
                auto_fixable: true,
            });
        }
    }
    items
}

pub fn push_repair_log(log: &mut Vec<RepairRecord>, record: RepairRecord) {
    log.push(record);
    let overflow = log.len().saturating_sub(20);
    if overflow > 0 {
        log.drain(0..overflow);
    }
}
