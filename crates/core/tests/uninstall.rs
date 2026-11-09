use lightproxy_core::backend::MemoryBackend;
use lightproxy_core::model::{Desired, EnvSnapshot, EnvVar, PersistedState, ProxySnapshot};
use lightproxy_core::store::StateStore;
use lightproxy_core::uninstall::{restore_from_dir, restore_snapshot, RestoreOutcome};

fn baseline() -> ProxySnapshot {
    ProxySnapshot {
        proxy_enable: false,
        proxy_server: "".into(),
        proxy_override: "<local>".into(),
        auto_config_url: None,
    }
}

fn active_target() -> ProxySnapshot {
    ProxySnapshot {
        proxy_enable: true,
        proxy_server: "127.0.0.1:7890".into(),
        proxy_override: lightproxy_core::model::DEFAULT_PROXY_OVERRIDE.into(),
        auto_config_url: None,
    }
}

fn owned_state() -> PersistedState {
    let mut state = PersistedState::initial();
    state.settings.preflight_enabled = false;
    state.runtime.desired = Desired::On;
    state.runtime.active_profile_id = Some("p_default".into());
    state.snapshot = Some(baseline());
    state
}

#[test]
fn restores_when_registry_still_points_at_active_profile() {
    let backend = MemoryBackend::new(active_target());

    let outcome = restore_snapshot(&backend, &owned_state()).unwrap();

    assert_eq!(outcome, RestoreOutcome::Restored);
    assert_eq!(backend.snapshot(), baseline());
    assert_eq!(backend.writes(), 1);
    assert_eq!(backend.notifies(), 1);
}

#[test]
fn restores_previous_proxy_even_when_it_was_enabled() {
    let previous = ProxySnapshot {
        proxy_enable: true,
        proxy_server: "10.0.0.2:8080".into(),
        proxy_override: "<local>".into(),
        auto_config_url: Some("http://pac/proxy.pac".into()),
    };
    let mut state = owned_state();
    state.snapshot = Some(previous.clone());
    let backend = MemoryBackend::new(active_target());

    assert_eq!(
        restore_snapshot(&backend, &state).unwrap(),
        RestoreOutcome::Restored
    );
    assert_eq!(backend.snapshot(), previous);
}

#[test]
fn already_restored_is_a_noop() {
    let backend = MemoryBackend::new(baseline());

    let outcome = restore_snapshot(&backend, &owned_state()).unwrap();

    assert_eq!(outcome, RestoreOutcome::AlreadyRestored);
    assert_eq!(backend.writes(), 0);
    assert_eq!(backend.notifies(), 0);
}

#[test]
fn missing_snapshot_is_ignored() {
    let mut state = owned_state();
    state.snapshot = None;
    let backend = MemoryBackend::new(active_target());

    assert_eq!(
        restore_snapshot(&backend, &state).unwrap(),
        RestoreOutcome::NoSnapshot
    );
    assert_eq!(backend.writes(), 0);
}

#[test]
fn externally_changed_registry_is_not_overwritten() {
    let mut current = active_target();
    current.proxy_server = "10.9.9.9:1".into();
    let backend = MemoryBackend::new(current.clone());

    assert_eq!(
        restore_snapshot(&backend, &owned_state()).unwrap(),
        RestoreOutcome::NotOwned
    );
    assert_eq!(backend.snapshot(), current);
    assert_eq!(backend.writes(), 0);
}

#[test]
fn external_pac_counts_as_not_owned() {
    let mut current = active_target();
    current.auto_config_url = Some("http://pac/other.pac".into());
    let backend = MemoryBackend::new(current.clone());

    assert_eq!(
        restore_snapshot(&backend, &owned_state()).unwrap(),
        RestoreOutcome::NotOwned
    );
    assert_eq!(backend.snapshot(), current);
    assert_eq!(backend.writes(), 0);
}

#[test]
fn missing_active_profile_is_not_owned() {
    let mut state = owned_state();
    state.runtime.active_profile_id = None;
    let backend = MemoryBackend::new(active_target());

    assert_eq!(
        restore_snapshot(&backend, &state).unwrap(),
        RestoreOutcome::NotOwned
    );
    assert_eq!(backend.writes(), 0);
}

#[test]
fn restore_from_dir_clears_snapshot_and_marks_clean_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    StateStore::new(dir.path()).save(&owned_state()).unwrap();
    let backend = MemoryBackend::new(active_target());

    assert_eq!(
        restore_from_dir(&backend, dir.path()).unwrap(),
        RestoreOutcome::Restored
    );
    assert_eq!(backend.snapshot(), baseline());

    let reloaded = StateStore::new(dir.path()).load().state;
    assert!(reloaded.clean_shutdown);
    assert!(reloaded.snapshot.is_none());
    assert_eq!(reloaded.runtime.desired, Desired::Off);
    assert_eq!(reloaded.runtime.active_profile_id, None);
}

#[test]
fn restore_from_dir_without_state_files_does_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let backend = MemoryBackend::new(active_target());

    assert_eq!(
        restore_from_dir(&backend, dir.path()).unwrap(),
        RestoreOutcome::NoSnapshot
    );
    assert_eq!(backend.writes(), 0);
    assert!(!dir.path().join("state.json").exists());
}

#[test]
fn not_owned_clears_persisted_snapshots_without_touching_registry() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = owned_state();
    state.env_snapshot = Some(EnvSnapshot {
        vars: vec![EnvVar {
            name: "http_proxy".into(),
            value: None,
            expand: false,
        }],
    });
    StateStore::new(dir.path()).save(&state).unwrap();
    let mut current = active_target();
    current.proxy_server = "10.9.9.9:1".into();
    let backend = MemoryBackend::new(current.clone());

    assert_eq!(
        restore_from_dir(&backend, dir.path()).unwrap(),
        RestoreOutcome::NotOwned
    );
    assert_eq!(backend.snapshot(), current);
    assert_eq!(backend.writes(), 0);

    let reloaded = StateStore::new(dir.path()).load().state;
    assert!(reloaded.snapshot.is_none());
    assert!(reloaded.env_snapshot.is_none());
    assert_eq!(reloaded.runtime.desired, Desired::Off);
    assert_eq!(reloaded.runtime.active_profile_id, None);
    assert!(reloaded.clean_shutdown);
}
