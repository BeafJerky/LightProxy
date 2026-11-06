use lightproxy_core::actor::ActorHandle;
use lightproxy_core::backend::{MemoryBackend, ProxyBackend};
use lightproxy_core::model::{PersistedState, ProxySnapshot, ProxyState};
use lightproxy_core::service::ProxyService;
use lightproxy_core::store::StateStore;
use std::sync::Arc;

fn baseline() -> ProxySnapshot {
    ProxySnapshot {
        proxy_enable: true,
        proxy_server: "10.0.0.2:8080".into(),
        proxy_override: "<local>".into(),
        auto_config_url: None,
    }
}

fn persisted_without_preflight() -> PersistedState {
    let mut state = PersistedState::initial();
    state.settings.preflight_enabled = false;
    state
}

fn service(
    dir: &std::path::Path,
    backend: Arc<MemoryBackend>,
    persisted: PersistedState,
) -> ProxyService<Arc<MemoryBackend>> {
    ProxyService::new(
        StateStore::new(dir),
        backend,
        persisted,
        None,
        false,
        Arc::new(|_| {}),
    )
}

#[test]
fn enable_then_disable_restores_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted_without_preflight());

    let state = svc.enable("p_default", true).unwrap();
    assert_eq!(state.state, ProxyState::On);
    assert_eq!(state.active_profile_id.as_deref(), Some("p_default"));
    assert_eq!(backend.snapshot().proxy_server, "192.168.5.4:7890");
    assert!(backend.snapshot().proxy_enable);
    assert_eq!(svc.state().snapshot.unwrap().proxy_server, "10.0.0.2:8080");

    let off = svc.disable().unwrap();
    assert_eq!(off.state, ProxyState::Off);
    assert_eq!(backend.snapshot(), baseline());
    assert!(off.snapshot.is_none());
    assert_eq!(off.active_profile_id, None);
}

#[test]
fn preflight_failure_never_writes_registry() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(ProxySnapshot::default()));
    let mut persisted = PersistedState::initial();
    persisted.settings.preflight_enabled = true;
    persisted.settings.preflight_timeout_ms = 300;
    persisted.profiles[0].host = "127.0.0.1".into();
    persisted.profiles[0].port = 1;
    let mut svc = service(dir.path(), backend.clone(), persisted);

    let err = svc.enable("p_default", false).unwrap_err();
    assert_eq!(err.code, "proxy_unreachable");
    assert_eq!(backend.writes(), 0);
    assert_eq!(svc.state().state, ProxyState::Off);
}

#[test]
fn switch_profile_stays_on_without_extra_disable_write() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted_without_preflight());

    svc.enable("p_default", true).unwrap();
    assert_eq!(backend.writes(), 1);
    let state = svc.switch_profile("p_backup").unwrap();
    assert_eq!(state.state, ProxyState::On);
    assert_eq!(state.active_profile_id.as_deref(), Some("p_backup"));
    assert_eq!(backend.writes(), 2);
    assert_eq!(backend.snapshot().proxy_server, "192.168.5.9:7890");
}

#[test]
fn write_failure_enters_error_and_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted_without_preflight());

    backend.fail_next_write();
    let err = svc.enable("p_default", false).unwrap_err();
    assert_eq!(err.code, "registry_write_failed");
    assert_eq!(backend.snapshot(), baseline());
    assert_eq!(svc.state().state, ProxyState::Error);
    assert!(svc.state().snapshot.is_none());
}

struct LyingBackend {
    inner: MemoryBackend,
}

impl ProxyBackend for LyingBackend {
    fn read(&self) -> Result<ProxySnapshot, lightproxy_core::CoreError> {
        let mut value = self.inner.snapshot();
        if self.inner.writes() > 0 {
            value.proxy_server = "wrong:1".into();
        }
        Ok(value)
    }

    fn write(&self, snapshot: &ProxySnapshot) -> Result<(), lightproxy_core::CoreError> {
        self.inner.write(snapshot)
    }

    fn notify(&self) -> Result<(), lightproxy_core::CoreError> {
        self.inner.notify()
    }
}

#[test]
fn readback_mismatch_retries_once_then_errors() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(LyingBackend {
        inner: MemoryBackend::new(baseline()),
    });
    let mut svc = ProxyService::new(
        StateStore::new(dir.path()),
        backend.clone(),
        persisted_without_preflight(),
        None,
        false,
        Arc::new(|_| {}),
    );

    let err = svc.enable("p_default", false).unwrap_err();
    assert_eq!(err.code, "readback_mismatch");
    assert_eq!(backend.inner.writes(), 3);
    assert_eq!(svc.state().state, ProxyState::Error);
}

#[test]
fn concurrent_enables_are_serialized_and_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let service = service(dir.path(), backend.clone(), persisted_without_preflight());
    let actor = ActorHandle::spawn(service).unwrap();

    let first = actor.clone();
    let second = actor.clone();
    let one = std::thread::spawn(move || first.enable("p_default", true).unwrap());
    let two = std::thread::spawn(move || second.enable("p_default", true).unwrap());
    let one = one.join().unwrap();
    let two = two.join().unwrap();

    assert_eq!(one.state, ProxyState::On);
    assert_eq!(two.state, ProxyState::On);
    assert_eq!(backend.writes(), 1);
}

#[test]
fn disable_while_off_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(baseline()));
    let mut svc = service(dir.path(), backend.clone(), persisted_without_preflight());

    let state = svc.disable().unwrap();
    assert_eq!(state.state, ProxyState::Off);
    assert_eq!(backend.writes(), 0);
    assert_eq!(backend.snapshot(), baseline());
}

#[test]
fn pac_requires_force() {
    let dir = tempfile::tempdir().unwrap();
    let mut initial = baseline();
    initial.auto_config_url = Some("http://pac/proxy.pac".into());
    let backend = Arc::new(MemoryBackend::new(initial));
    let mut svc = service(dir.path(), backend.clone(), persisted_without_preflight());

    let err = svc.enable("p_default", false).unwrap_err();
    assert_eq!(err.code, "pac_conflict");
    assert_eq!(backend.writes(), 0);
    assert_eq!(svc.state().state, ProxyState::Off);

    let state = svc.enable("p_default", true).unwrap();
    assert_eq!(state.state, ProxyState::On);
    assert_eq!(backend.snapshot().auto_config_url, None);
}

#[test]
fn deleting_default_profile_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(ProxySnapshot::default()));
    let mut svc = service(dir.path(), backend, persisted_without_preflight());

    let err = svc.delete_profile("p_default").unwrap_err();
    assert_eq!(err.code, "default_profile_locked");
    assert_eq!(svc.state().profiles.len(), 2);
}

#[test]
fn settings_partial_update_keeps_other_fields() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(ProxySnapshot::default()));
    let mut svc = service(dir.path(), backend, persisted_without_preflight());

    let state = svc
        .update_settings(serde_json::json!({
            "hotkeys": { "toggle": "Ctrl+Alt+K" },
            "theme": "dark"
        }))
        .unwrap();
    assert_eq!(state.settings.hotkeys.toggle, "Ctrl+Alt+K");
    assert_eq!(state.settings.hotkeys.panel, "Ctrl+Alt+O");
    assert_eq!(state.settings.theme, "dark");
    assert_eq!(state.settings.preflight_timeout_ms, 800);
}

#[test]
fn invalid_theme_and_timeouts_are_normalized() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(ProxySnapshot::default()));
    let mut svc = service(dir.path(), backend, persisted_without_preflight());

    let state = svc
        .update_settings(serde_json::json!({
            "theme": "rainbow",
            "preflightTimeoutMs": 99,
            "periodicIntervalSec": 7
        }))
        .unwrap();
    assert_eq!(state.settings.theme, "system");
    assert_eq!(state.settings.preflight_timeout_ms, 300);
    assert_eq!(state.settings.periodic_interval_sec, 60);
}
