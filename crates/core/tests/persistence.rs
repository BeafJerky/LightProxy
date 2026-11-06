use lightproxy_core::model::{PersistedState, ProxySnapshot};
use lightproxy_core::store::StateStore;
use std::fs;

#[test]
fn save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let store = StateStore::new(dir.path());
    let mut state = PersistedState::initial();
    state.snapshot = Some(ProxySnapshot {
        proxy_enable: true,
        proxy_server: "10.0.0.2:8080".into(),
        proxy_override: "<local>".into(),
        auto_config_url: None,
    });
    store.save(&state).unwrap();

    let loaded = store.load();
    assert!(loaded.warning.is_none());
    assert!(!loaded.readonly);
    assert_eq!(loaded.state.profiles.len(), 2);
    assert_eq!(loaded.state.snapshot.unwrap().proxy_server, "10.0.0.2:8080");
    assert!(dir.path().join("state.json").exists());
}

#[test]
fn corrupt_main_falls_back_to_backup() {
    let dir = tempfile::tempdir().unwrap();
    let store = StateStore::new(dir.path());
    let mut state = PersistedState::initial();
    state.profiles[0].name = "备份里的名字".into();
    store.save(&state).unwrap();
    // Second save creates the .bak copy.
    store.save(&state).unwrap();
    fs::write(dir.path().join("state.json"), b"{not-json").unwrap();

    let loaded = store.load();
    assert!(loaded.warning.is_some());
    assert_eq!(loaded.state.profiles[0].name, "备份里的名字");
    let corrupt: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("state.corrupt-")
        })
        .collect();
    assert_eq!(corrupt.len(), 1);
}

#[test]
fn both_corrupt_restores_defaults_and_keeps_copies() {
    let dir = tempfile::tempdir().unwrap();
    let store = StateStore::new(dir.path());
    fs::write(dir.path().join("state.json"), b"garbage").unwrap();
    fs::write(dir.path().join("state.json.bak"), b"garbage").unwrap();

    let loaded = store.load();
    assert!(loaded.warning.is_some());
    assert_eq!(loaded.state.profiles.len(), 2);
    let count = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.starts_with("state.corrupt-")
        })
        .count();
    assert!(count >= 1);
}

#[test]
fn newer_schema_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let store = StateStore::new(dir.path());
    let mut state = PersistedState::initial();
    state.schema_version = 99;
    store.save(&state).unwrap();

    let loaded = store.load();
    assert!(loaded.readonly);
    assert!(store.is_readonly());
    assert!(store.save(&loaded.state).is_err());
}
