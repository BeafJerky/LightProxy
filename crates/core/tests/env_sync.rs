use lightproxy_core::backend::{MemoryBackend, ProxyBackend};
use lightproxy_core::model::{
    EnvVar, PersistedState, Profile, Protocol, ProxySnapshot, ProxyState,
};
use lightproxy_core::service::ProxyService;
use lightproxy_core::store::StateStore;
use std::sync::Arc;

fn baseline() -> ProxySnapshot {
    ProxySnapshot {
        proxy_enable: false,
        proxy_server: "".into(),
        proxy_override: "<local>".into(),
        auto_config_url: None,
    }
}

fn persisted(sync: bool) -> PersistedState {
    let mut state = PersistedState::initial();
    state.settings.preflight_enabled = false;
    state.settings.sync_env_proxy = sync;
    state.profiles.push(Profile {
        id: "p_socks".into(),
        name: "SOCKS".into(),
        protocol: Protocol::Socks5,
        host: "192.0.2.4".into(),
        port: 1080,
    });
    state
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

fn env_value(backend: &MemoryBackend, name: &str) -> Option<String> {
    backend
        .env_vars()
        .iter()
        .find(|v| v.name.eq_ignore_ascii_case(name))
        .and_then(|v| v.value.clone())
}

#[test]
fn enable_writes_env_and_disable_deletes_when_absent_before() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(true));
    svc.enable("p_default", true).unwrap();
    assert_eq!(
        env_value(&backend, "http_proxy").as_deref(),
        Some("http://127.0.0.1:7890")
    );
    assert!(svc.state().settings.sync_env_proxy);
    svc.disable().unwrap();
    assert!(env_value(&backend, "http_proxy").is_none());
    assert!(env_value(&backend, "no_proxy").is_none());
}

#[test]
fn sync_disabled_does_not_touch_env() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(false));
    svc.enable("p_default", true).unwrap();
    assert_eq!(backend.env_writes(), 0);
    assert!(env_value(&backend, "http_proxy").is_none());
}

#[test]
fn disable_restores_preexisting_value() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    backend.set_env(vec![EnvVar {
        name: "http_proxy".into(),
        value: Some("http://orig:1".into()),
        expand: false,
    }]);
    let mut svc = service(dir.path(), backend.clone(), persisted(true));
    svc.enable("p_default", true).unwrap();
    svc.disable().unwrap();
    assert_eq!(
        env_value(&backend, "http_proxy").as_deref(),
        Some("http://orig:1")
    );
}

#[test]
fn idempotent_enable_does_not_rewrite_env() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(true));
    svc.enable("p_default", true).unwrap();
    let writes = backend.env_writes();
    svc.enable("p_default", true).unwrap();
    assert_eq!(backend.env_writes(), writes);
}

#[test]
fn switch_http_to_socks_adds_all_proxy_and_back_restores() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(true));
    svc.enable("p_default", true).unwrap();
    svc.switch_profile("p_socks", false).unwrap();
    assert_eq!(
        env_value(&backend, "all_proxy").as_deref(),
        Some("socks5h://192.0.2.4:1080")
    );
    svc.switch_profile("p_default", false).unwrap();
    assert!(env_value(&backend, "all_proxy").is_none());
}

#[test]
fn toggling_setting_off_while_on_restores_env() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(true));
    svc.enable("p_default", true).unwrap();
    svc.update_settings(serde_json::json!({ "syncEnvProxy": false }))
        .unwrap();
    assert!(env_value(&backend, "http_proxy").is_none());
}

#[test]
fn toggling_setting_on_while_on_writes_env() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(false));
    svc.enable("p_default", true).unwrap();
    svc.update_settings(serde_json::json!({ "syncEnvProxy": true }))
        .unwrap();
    assert_eq!(
        env_value(&backend, "http_proxy").as_deref(),
        Some("http://127.0.0.1:7890")
    );
}

#[test]
fn external_tamper_is_preserved_on_disable() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(true));
    svc.enable("p_default", true).unwrap();
    backend
        .write_env(&[EnvVar {
            name: "https_proxy".into(),
            value: Some("http://mine:9".into()),
            expand: false,
        }])
        .unwrap();
    svc.disable().unwrap();
    assert_eq!(
        env_value(&backend, "https_proxy").as_deref(),
        Some("http://mine:9")
    );
    assert!(env_value(&backend, "http_proxy").is_none());
}

#[test]
fn keep_proxy_on_exit_keeps_env() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut state = persisted(true);
    state.settings.keep_proxy_on_exit = true;
    let mut svc = service(dir.path(), backend.clone(), state);
    svc.enable("p_default", true).unwrap();
    svc.shutdown();
    assert_eq!(
        env_value(&backend, "http_proxy").as_deref(),
        Some("http://127.0.0.1:7890")
    );
}

#[test]
fn env_write_failure_keeps_proxy_on_with_message() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(true));
    backend.fail_next_env_write();
    let state = svc.enable("p_default", true).unwrap();
    assert_eq!(state.state, ProxyState::On);
    assert!(state.message.unwrap().contains("命令行代理同步失败"));
    assert!(backend.snapshot().proxy_enable);
}

#[test]
fn env_restore_failure_keeps_snapshot_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted(true));
    svc.enable("p_default", true).unwrap();
    backend.fail_next_env_write();
    let err = svc.disable().unwrap_err();
    assert_eq!(err.code, "env_write_failed");
    assert_eq!(svc.state().state, ProxyState::Error);
    // 重试成功
    svc.disable().unwrap();
    assert!(env_value(&backend, "http_proxy").is_none());
}
