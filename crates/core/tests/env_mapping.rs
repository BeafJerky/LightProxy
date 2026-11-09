use lightproxy_core::env;
use lightproxy_core::model::{EnvSnapshot, EnvVar, PersistedState, Profile, Protocol, Settings};

fn http_profile() -> Profile {
    Profile {
        id: "p1".into(),
        name: "HTTP".into(),
        protocol: Protocol::Http,
        host: "192.0.2.4".into(),
        port: 7890,
    }
}

fn socks_profile() -> Profile {
    Profile {
        id: "p2".into(),
        name: "SOCKS".into(),
        protocol: Protocol::Socks5,
        host: "192.0.2.4".into(),
        port: 1080,
    }
}

fn value_of(vars: &[EnvVar], name: &str) -> Option<String> {
    env::find(vars, name).and_then(|v| v.value.clone())
}

#[test]
fn http_expected_maps_two_vars_and_no_proxy() {
    let vars = env::expected_vars(&http_profile());
    assert_eq!(
        value_of(&vars, "http_proxy").as_deref(),
        Some("http://192.0.2.4:7890")
    );
    assert_eq!(
        value_of(&vars, "https_proxy").as_deref(),
        Some("http://192.0.2.4:7890")
    );
    assert!(env::find(&vars, "all_proxy").is_none());
    assert_eq!(
        value_of(&vars, "no_proxy").as_deref(),
        Some("localhost,127.0.0.1,::1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,192.0.2.4")
    );
}

#[test]
fn socks_expected_uses_socks5h_and_all_proxy() {
    let vars = env::expected_vars(&socks_profile());
    assert_eq!(
        value_of(&vars, "http_proxy").as_deref(),
        Some("socks5h://192.0.2.4:1080")
    );
    assert_eq!(
        value_of(&vars, "all_proxy").as_deref(),
        Some("socks5h://192.0.2.4:1080")
    );
}

#[test]
fn ipv6_endpoint_keeps_brackets() {
    let mut p = http_profile();
    p.host = "::1".into();
    let vars = env::expected_vars(&p);
    assert_eq!(
        value_of(&vars, "http_proxy").as_deref(),
        Some("http://[::1]:7890")
    );
}

#[test]
fn merge_capture_records_absence_and_preserves_first_capture() {
    let current = vec![EnvVar {
        name: "HTTP_PROXY".into(),
        value: Some("http://old:1".into()),
        expand: true,
    }];
    let mut snap = EnvSnapshot::default();
    env::merge_capture(&mut snap, &current, &env::ENV_VARS);
    assert_eq!(snap.vars.len(), 4);
    let http = env::find(&snap.vars, "http_proxy").unwrap();
    assert_eq!(http.name, "HTTP_PROXY");
    assert_eq!(http.value.as_deref(), Some("http://old:1"));
    assert!(http.expand);
    assert!(env::find(&snap.vars, "https_proxy")
        .unwrap()
        .value
        .is_none());

    // 再次捕获不覆盖首次记录
    let drifted = vec![EnvVar {
        name: "http_proxy".into(),
        value: Some("http://new:2".into()),
        expand: false,
    }];
    env::merge_capture(&mut snap, &drifted, &env::ENV_VARS);
    assert_eq!(
        env::find(&snap.vars, "http_proxy")
            .unwrap()
            .value
            .as_deref(),
        Some("http://old:1")
    );
}

#[test]
fn owned_names_matches_case_insensitively() {
    let expected = env::expected_vars(&http_profile());
    let current = vec![
        EnvVar {
            name: "HTTP_PROXY".into(),
            value: Some("http://192.0.2.4:7890".into()),
            expand: false,
        },
        EnvVar {
            name: "https_proxy".into(),
            value: Some("http://elsewhere:1".into()),
            expand: false,
        },
    ];
    let owned = env::owned_names(&current, &expected);
    assert!(owned.contains(&"http_proxy".to_string()));
    assert!(!owned.contains(&"https_proxy".to_string()));
}

#[test]
fn restore_target_restores_owned_and_skips_drifted() {
    let expected = env::expected_vars(&http_profile());
    let snapshot = EnvSnapshot {
        vars: vec![
            EnvVar {
                name: "http_proxy".into(),
                value: None,
                expand: false,
            },
            EnvVar {
                name: "https_proxy".into(),
                value: Some("http://orig:3".into()),
                expand: false,
            },
        ],
    };
    let current = vec![
        EnvVar {
            name: "http_proxy".into(),
            value: Some("http://192.0.2.4:7890".into()),
            expand: false,
        },
        EnvVar {
            name: "https_proxy".into(),
            value: Some("http://tampered:9".into()),
            expand: false,
        },
    ];
    let target = env::restore_target(&snapshot, &current, &expected);
    assert_eq!(target.len(), 1);
    assert_eq!(target[0].name, "http_proxy");
    assert!(target[0].value.is_none()); // 删除而非空串
}

#[test]
fn apply_target_restores_snapshot_vars_not_in_expected() {
    let expected = env::expected_vars(&http_profile()); // 无 all_proxy
    let snapshot = EnvSnapshot {
        vars: vec![EnvVar {
            name: "all_proxy".into(),
            value: Some("socks5h://orig:5".into()),
            expand: false,
        }],
    };
    let target = env::apply_target(&snapshot, &expected);
    assert_eq!(
        value_of(&target, "all_proxy").as_deref(),
        Some("socks5h://orig:5")
    );
    assert_eq!(
        value_of(&target, "http_proxy").as_deref(),
        Some("http://192.0.2.4:7890")
    );
}

#[test]
fn already_applied_compares_values_by_canonical_name() {
    let target = env::expected_vars(&http_profile());
    let current = vec![
        EnvVar {
            name: "HTTP_PROXY".into(),
            value: Some("http://192.0.2.4:7890".into()),
            expand: false,
        },
        EnvVar {
            name: "https_proxy".into(),
            value: Some("http://192.0.2.4:7890".into()),
            expand: false,
        },
        EnvVar {
            name: "no_proxy".into(),
            value: env::no_proxy_value(&http_profile()).into(),
            expand: false,
        },
    ];
    assert!(env::already_applied(&current, &target));
}

#[test]
fn settings_without_sync_field_defaults_false() {
    let json = serde_json::json!({
        "autostart": true, "closeAction": "hide", "preflightEnabled": true,
        "preflightTimeoutMs": 800, "theme": "system",
        "hotkeys": { "toggle": "Ctrl+Alt+P", "panel": "Ctrl+Alt+O", "nextProfile": "Ctrl+Alt+N" },
        "trayClickAction": "panel", "proxyOverride": "<local>", "keepProxyOnExit": false
    });
    let settings: Settings = serde_json::from_value(json).unwrap();
    assert!(!settings.sync_env_proxy);
}

#[test]
fn persisted_state_without_env_snapshot_defaults_none() {
    let mut state = PersistedState::initial();
    state.env_snapshot = None;
    let json = serde_json::to_value(&state).unwrap();
    let mut obj = json.as_object().unwrap().clone();
    obj.remove("envSnapshot");
    let loaded: PersistedState = serde_json::from_value(serde_json::Value::Object(obj)).unwrap();
    assert!(loaded.env_snapshot.is_none());
    assert_eq!(loaded.schema_version, 1);
}
