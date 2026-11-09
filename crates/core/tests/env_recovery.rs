use lightproxy_core::backend::MemoryBackend;
use lightproxy_core::model::{
    Desired, EnvSnapshot, EnvVar, PersistedState, ProxySnapshot, ProxyState,
};
use lightproxy_core::service::ProxyService;
use lightproxy_core::store::StateStore;
use lightproxy_core::uninstall::restore_from_dir;
use std::sync::Arc;

fn active_registry() -> ProxySnapshot {
    ProxySnapshot {
        proxy_enable: true,
        proxy_server: "127.0.0.1:7890".into(),
        proxy_override: lightproxy_core::model::DEFAULT_PROXY_OVERRIDE.into(),
        auto_config_url: None,
    }
}

fn env_value(backend: &MemoryBackend, name: &str) -> Option<String> {
    backend
        .env_vars()
        .iter()
        .find(|v| v.name.eq_ignore_ascii_case(name))
        .and_then(|v| v.value.clone())
}

fn service(
    dir: &std::path::Path,
    backend: Arc<MemoryBackend>,
    state: PersistedState,
) -> ProxyService<Arc<MemoryBackend>> {
    ProxyService::new(
        StateStore::new(dir),
        backend,
        state,
        None,
        false,
        Arc::new(|_| {}),
    )
}

#[test]
fn recover_rewrites_drifted_env() {
    // 用本机监听端口保证探活成功，避免依赖真实网络
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let dir = tempfile::tempdir().unwrap();
    let registry = ProxySnapshot {
        proxy_enable: true,
        proxy_server: format!("127.0.0.1:{port}"),
        proxy_override: lightproxy_core::model::DEFAULT_PROXY_OVERRIDE.into(),
        auto_config_url: None,
    };
    let backend = Arc::new(MemoryBackend::new(registry));
    backend.set_env(vec![EnvVar {
        name: "http_proxy".into(),
        value: Some("http://wrong:1".into()),
        expand: false,
    }]);
    let mut state = PersistedState::initial();
    state.settings.preflight_enabled = false;
    state.settings.sync_env_proxy = true;
    state.clean_shutdown = false;
    state.runtime.desired = Desired::On;
    state.profiles[0].host = "127.0.0.1".into();
    state.profiles[0].port = port;
    state.runtime.active_profile_id = Some("p_default".into());
    let mut svc = service(dir.path(), backend.clone(), state);
    svc.recover();
    assert_eq!(svc.state().state, ProxyState::On);
    let expected = format!("http://127.0.0.1:{port}");
    assert_eq!(
        env_value(&backend, "http_proxy").as_deref(),
        Some(expected.as_str())
    );
}

#[test]
fn recover_matching_env_without_snapshot_does_not_capture() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(active_registry()));
    let expected = lightproxy_core::env::expected_vars(&PersistedState::initial().profiles[0]);
    backend.set_env(expected);
    let mut state = PersistedState::initial();
    state.settings.preflight_enabled = false;
    state.settings.sync_env_proxy = true;
    state.clean_shutdown = false;
    state.runtime.desired = Desired::On;
    state.runtime.active_profile_id = Some("p_default".into());
    let mut svc = service(dir.path(), backend.clone(), state);
    svc.recover();
    assert_eq!(backend.env_writes(), 0);
    let reloaded = StateStore::new(dir.path()).load().state;
    assert!(reloaded.env_snapshot.is_none());
}

#[test]
fn recover_off_restores_leftover_env_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let registry = ProxySnapshot {
        proxy_enable: false,
        ..active_registry()
    };
    let backend = Arc::new(MemoryBackend::new(registry));
    backend.set_env(vec![EnvVar {
        name: "http_proxy".into(),
        value: Some("http://127.0.0.1:7890".into()),
        expand: false,
    }]);
    let mut state = PersistedState::initial();
    state.settings.preflight_enabled = false;
    state.settings.sync_env_proxy = true;
    state.clean_shutdown = false;
    state.runtime.desired = Desired::Off;
    state.runtime.active_profile_id = Some("p_default".into());
    state.env_snapshot = Some(EnvSnapshot {
        vars: vec![EnvVar {
            name: "http_proxy".into(),
            value: None,
            expand: false,
        }],
    });
    let mut svc = service(dir.path(), backend.clone(), state);
    svc.recover();
    assert!(env_value(&backend, "http_proxy").is_none());
}

#[test]
fn uninstall_restores_env_and_clears_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = PersistedState::initial();
    state.settings.preflight_enabled = false;
    state.settings.sync_env_proxy = true;
    state.runtime.desired = Desired::On;
    state.runtime.active_profile_id = Some("p_default".into());
    state.snapshot = Some(ProxySnapshot {
        proxy_enable: false,
        proxy_server: "".into(),
        proxy_override: "<local>".into(),
        auto_config_url: None,
    });
    state.env_snapshot = Some(EnvSnapshot {
        vars: vec![EnvVar {
            name: "http_proxy".into(),
            value: None,
            expand: false,
        }],
    });
    StateStore::new(dir.path()).save(&state).unwrap();

    let backend = MemoryBackend::new(active_registry());
    backend.set_env(vec![EnvVar {
        name: "http_proxy".into(),
        value: Some("http://127.0.0.1:7890".into()),
        expand: false,
    }]);
    restore_from_dir(&backend, dir.path()).unwrap();
    assert!(env_value(&backend, "http_proxy").is_none());
    let reloaded = StateStore::new(dir.path()).load().state;
    assert!(reloaded.env_snapshot.is_none());
}
