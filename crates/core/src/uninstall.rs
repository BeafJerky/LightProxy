use crate::backend::ProxyBackend;
use crate::error::CoreResult;
use crate::model::{Desired, PersistedState};
use crate::store::StateStore;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreOutcome {
    Restored,
    AlreadyRestored,
    NoSnapshot,
    NotOwned,
}

/// Restores the proxy snapshot captured before takeover, but only while the
/// registry still matches the profile LightProxy was supposed to be serving.
/// Anything else is treated as an external change and left untouched.
pub fn restore_snapshot(
    backend: &dyn ProxyBackend,
    persisted: &PersistedState,
) -> CoreResult<RestoreOutcome> {
    let Some(snapshot) = persisted.snapshot.clone() else {
        return Ok(RestoreOutcome::NoSnapshot);
    };
    let current = backend.read()?;
    if current == snapshot {
        return Ok(RestoreOutcome::AlreadyRestored);
    }
    let Some(profile) = persisted
        .runtime
        .active_profile_id
        .as_deref()
        .and_then(|id| persisted.profile(id))
    else {
        return Ok(RestoreOutcome::NotOwned);
    };
    let owned = current.proxy_enable
        && current.proxy_server == profile.server()
        && current.auto_config_url.is_none();
    if !owned {
        return Ok(RestoreOutcome::NotOwned);
    }
    backend.write(&snapshot)?;
    let _ = backend.notify();
    Ok(RestoreOutcome::Restored)
}

/// Entry point used by `LightProxy.exe --restore-snapshot` before uninstall.
pub fn restore_from_dir(
    backend: &dyn ProxyBackend,
    dir: impl AsRef<Path>,
) -> CoreResult<RestoreOutcome> {
    let store = StateStore::new(dir.as_ref().to_path_buf());
    if !store.path().exists() && !store.backup_path().exists() {
        return Ok(RestoreOutcome::NoSnapshot);
    }
    let loaded = store.load();
    let outcome = restore_snapshot(backend, &loaded.state)?;
    if matches!(
        outcome,
        RestoreOutcome::Restored | RestoreOutcome::AlreadyRestored
    ) {
        let mut state = loaded.state;
        state.snapshot = None;
        state.runtime.active_profile_id = None;
        state.runtime.desired = Desired::Off;
        state.clean_shutdown = true;
        // The uninstaller deletes this directory anyway; a save failure must
        // not turn a successful registry restore into an uninstall failure.
        let _ = store.save(&state);
    }
    Ok(outcome)
}
