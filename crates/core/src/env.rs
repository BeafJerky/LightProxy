//! 命令行环境变量同步的纯逻辑：映射、快照合并、所有权判定与收敛目标。
//! 不做任何 IO；注册表读写在 backend/wininet，编排在 service。

use crate::model::{EnvSnapshot, EnvVar, Profile, Protocol};

pub const ENV_VARS: [&str; 4] = ["http_proxy", "https_proxy", "all_proxy", "no_proxy"];
pub const NO_PROXY_BASE: &str = "localhost,127.0.0.1,::1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16";

pub fn canonical(name: &str) -> String {
    name.to_ascii_lowercase()
}

pub fn find<'a>(vars: &'a [EnvVar], name: &str) -> Option<&'a EnvVar> {
    let key = canonical(name);
    vars.iter().find(|v| canonical(&v.name) == key)
}

pub fn no_proxy_value(profile: &Profile) -> String {
    format!("{NO_PROXY_BASE},{}", profile.host)
}

fn proxy_url(profile: &Profile) -> String {
    match profile.protocol {
        Protocol::Socks5 => format!("socks5h://{}", profile.endpoint()),
        _ => format!("http://{}", profile.endpoint()),
    }
}

pub fn expected_vars(profile: &Profile) -> Vec<EnvVar> {
    let url = proxy_url(profile);
    let mut vars = vec![
        EnvVar {
            name: "http_proxy".into(),
            value: Some(url.clone()),
            expand: false,
        },
        EnvVar {
            name: "https_proxy".into(),
            value: Some(url.clone()),
            expand: false,
        },
    ];
    if profile.protocol == Protocol::Socks5 {
        vars.push(EnvVar {
            name: "all_proxy".into(),
            value: Some(url),
            expand: false,
        });
    }
    vars.push(EnvVar {
        name: "no_proxy".into(),
        value: Some(no_proxy_value(profile)),
        expand: false,
    });
    vars
}

/// 首次捕获优先：只补录 names 中尚未出现在快照的规范名。
pub fn merge_capture(snapshot: &mut EnvSnapshot, current: &[EnvVar], names: &[&str]) {
    for name in names {
        let key = canonical(name);
        if snapshot.vars.iter().any(|v| canonical(&v.name) == key) {
            continue;
        }
        let captured = match find(current, name) {
            Some(v) => v.clone(),
            None => EnvVar {
                name: key,
                value: None,
                expand: false,
            },
        };
        snapshot.vars.push(captured);
    }
}

/// 当前值仍等于期望值的规范名（仅考察 expected 中出现的变量）。
pub fn owned_names(current: &[EnvVar], expected: &[EnvVar]) -> Vec<String> {
    expected
        .iter()
        .filter_map(|e| {
            let key = canonical(&e.name);
            let same = match find(current, &key) {
                Some(c) => c.value == e.value,
                None => e.value.is_none(),
            };
            if same {
                Some(key)
            } else {
                None
            }
        })
        .collect()
}

/// 还原目标：owned 变量按快照还原（快照 value=None 表示删除），非 owned 不动。
pub fn restore_target(
    snapshot: &EnvSnapshot,
    current: &[EnvVar],
    expected: &[EnvVar],
) -> Vec<EnvVar> {
    owned_names(current, expected)
        .iter()
        .filter_map(|name| find(&snapshot.vars, name).cloned())
        .collect()
}

/// 开启方向写入目标：设置 expected 全部变量；快照中已记录但 expected 不需要的变量还原为快照值。
pub fn apply_target(snapshot: &EnvSnapshot, expected: &[EnvVar]) -> Vec<EnvVar> {
    let mut target = expected.to_vec();
    for s in &snapshot.vars {
        let key = canonical(&s.name);
        if !expected.iter().any(|e| canonical(&e.name) == key) {
            target.push(s.clone());
        }
    }
    target
}

/// 当前环境是否已等于写入目标（按规范名比较值）。
pub fn already_applied(current: &[EnvVar], target: &[EnvVar]) -> bool {
    target.iter().all(|t| match find(current, &t.name) {
        Some(c) => c.value == t.value,
        None => t.value.is_none(),
    })
}
