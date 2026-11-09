use lightproxy_core::backend::{MemoryBackend, ProxyBackend};
use lightproxy_core::model::{EnvVar, PersistedState, ProxySnapshot};
use lightproxy_core::service::ProxyService;
use lightproxy_core::store::StateStore;
use std::sync::Arc;

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

fn persisted() -> PersistedState {
    let mut state = PersistedState::initial();
    state.settings.preflight_enabled = false;
    state.settings.sync_env_proxy = true;
    state
}

#[test]
fn diagnostics_flags_env_drift_and_repair_fixes() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(ProxySnapshot::default()));
    let mut svc = service(dir.path(), backend.clone(), persisted());
    svc.enable("p_default", true).unwrap();
    backend
        .write_env(&[EnvVar {
            name: "http_proxy".into(),
            value: Some("http://wrong:1".into()),
            expand: false,
        }])
        .unwrap();

    let diag = svc.diagnostics();
    let r15 = diag.iter().find(|d| d.id == "R15").expect("R15 present");
    assert!(r15.auto_fixable);

    let results = svc.run_repair(&["R15".to_string()]);
    assert!(results[0].ok);
    assert!(!svc.diagnostics().iter().any(|d| d.id == "R15"));
}

#[test]
fn diagnostics_no_r15_when_synced() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(ProxySnapshot::default()));
    let mut svc = service(dir.path(), backend.clone(), persisted());
    svc.enable("p_default", true).unwrap();
    assert!(!svc.diagnostics().iter().any(|d| d.id == "R15"));
}

#[test]
fn diagnostics_no_r15_when_setting_off() {
    let dir = tempfile::tempdir().unwrap();
    let backend = Arc::new(MemoryBackend::new(ProxySnapshot::default()));
    let mut state = persisted();
    state.settings.sync_env_proxy = false;
    let svc = service(dir.path(), backend.clone(), state);
    assert!(!svc.diagnostics().iter().any(|d| d.id == "R15"));
}
