use lightproxy_core::backend::MemoryBackend;
use lightproxy_core::model::{PersistedState, Profile, Protocol, ProxySnapshot, ProxyState};
use lightproxy_core::recovery::{decide_recovery, RecoveryDecision};
use lightproxy_core::service::ProxyService;
use lightproxy_core::store::StateStore;
use lightproxy_core::ProxyBackend;
use std::sync::Arc;

fn saved_proxy() -> ProxySnapshot {
    ProxySnapshot {
        proxy_enable: true,
        proxy_server: "127.0.0.1:7890".into(),
        proxy_override: "<local>".into(),
        auto_config_url: None,
    }
}

#[test]
fn crashed_with_reachable_known_proxy_restores_on() {
    let persisted = PersistedState::initial();
    let decision = decide_recovery(false, &saved_proxy(), &persisted, Some(true));
    assert_eq!(
        decision,
        RecoveryDecision::On {
            profile_id: "p_default".into(),
            fill_snapshot: true
        }
    );
}

#[test]
fn crashed_with_socks5_proxy_matches_socks_profile() {
    let mut persisted = PersistedState::initial();
    persisted.profiles.push(Profile {
        id: "p_socks".into(),
        name: "SOCKS 代理".into(),
        protocol: Protocol::Socks5,
        host: "192.168.5.4".into(),
        port: 1080,
    });
    let mut registry = saved_proxy();
    registry.proxy_server = "socks=192.168.5.4:1080".into();
    let decision = decide_recovery(false, &registry, &persisted, Some(true));
    assert_eq!(
        decision,
        RecoveryDecision::On {
            profile_id: "p_socks".into(),
            fill_snapshot: true
        }
    );
}

#[test]
fn crashed_with_unreachable_proxy_degrades() {
    let persisted = PersistedState::initial();
    let decision = decide_recovery(false, &saved_proxy(), &persisted, Some(false));
    assert!(matches!(decision, RecoveryDecision::Degraded { .. }));
}

#[test]
fn crashed_with_unknown_proxy_does_not_overwrite() {
    let persisted = PersistedState::initial();
    let mut registry = saved_proxy();
    registry.proxy_server = "172.16.1.1:3128".into();
    let decision = decide_recovery(false, &registry, &persisted, None);
    match decision {
        RecoveryDecision::Degraded { message, .. } => assert!(message.contains("172.16.1.1")),
        other => panic!("unexpected decision: {other:?}"),
    }
}

#[test]
fn crashed_with_proxy_off_stays_off() {
    let persisted = PersistedState::initial();
    let registry = ProxySnapshot::default();
    let decision = decide_recovery(false, &registry, &persisted, None);
    assert!(matches!(
        decision,
        RecoveryDecision::Off { message: Some(_) }
    ));
}

#[test]
fn pac_is_reported_not_taken_over() {
    let persisted = PersistedState::initial();
    let mut registry = saved_proxy();
    registry.auto_config_url = Some("http://pac/proxy.pac".into());
    let decision = decide_recovery(true, &registry, &persisted, Some(true));
    match decision {
        RecoveryDecision::Degraded { message, .. } => assert!(message.contains("PAC")),
        other => panic!("unexpected decision: {other:?}"),
    }
}

#[test]
fn service_recovery_fills_snapshot_and_marks_dirty() {
    // 本机监听保证探活成功，避免依赖真实网络
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let dir = tempfile::tempdir().unwrap();
    let mut persisted = PersistedState::initial();
    persisted.clean_shutdown = false;
    persisted.settings.preflight_enabled = false;
    persisted.profiles[0].host = "127.0.0.1".into();
    persisted.profiles[0].port = port;
    let registry = ProxySnapshot {
        proxy_server: format!("127.0.0.1:{port}"),
        ..saved_proxy()
    };
    let backend = Arc::new(MemoryBackend::new(registry));
    let mut service = ProxyService::new(
        StateStore::new(dir.path()),
        backend,
        persisted,
        None,
        false,
        Arc::new(|_| {}),
    );

    service.recover();
    let state = service.state();
    assert_eq!(state.state, ProxyState::On);
    assert_eq!(state.active_profile_id.as_deref(), Some("p_default"));
    assert_eq!(
        state.snapshot.unwrap().proxy_server,
        format!("127.0.0.1:{port}")
    );

    let saved = StateStore::new(dir.path()).load();
    assert!(!saved.state.clean_shutdown);
}

#[test]
fn drift_detection_reports_external_change() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(ProxySnapshot::default()));
    let mut persisted = PersistedState::initial();
    persisted.settings.preflight_enabled = false;
    let mut service = ProxyService::new(
        StateStore::new(dir.path()),
        backend.clone(),
        persisted,
        None,
        false,
        Arc::new(|_| {}),
    );
    service.enable("p_default", true).unwrap();

    let mut external = backend.snapshot();
    external.proxy_enable = false;
    backend.write(&external).unwrap();

    let drifted = service.poll_registry().unwrap();
    assert!(drifted.is_some());
    let state = service.state();
    assert_eq!(state.state, ProxyState::Degraded);
    assert!(state.message.unwrap().contains("其他程序"));
}
