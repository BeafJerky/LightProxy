use lightproxy_core::{ActorHandle, Watcher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri_plugin_global_shortcut::Shortcut;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    Toggle,
    Panel,
    NextProfile,
}

pub struct AppCtx {
    pub actor: ActorHandle,
    pub data_dir: PathBuf,
    pub view: Mutex<String>,
    pub autohide_generation: AtomicU64,
    pub hotkey_map: Mutex<Vec<(Shortcut, HotkeyAction)>>,
    pub last_hotkey: Mutex<Option<Instant>>,
    pub watcher: Mutex<Option<Watcher>>,
}

impl AppCtx {
    pub fn new(actor: ActorHandle, data_dir: PathBuf) -> Self {
        Self {
            actor,
            data_dir,
            view: Mutex::new("panel".into()),
            autohide_generation: AtomicU64::new(0),
            hotkey_map: Mutex::new(Vec::new()),
            last_hotkey: Mutex::new(None),
            watcher: Mutex::new(None),
        }
    }

    pub fn current_view(&self) -> String {
        self.view
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| "panel".into())
    }

    pub fn set_current_view(&self, view: &str) {
        if let Ok(mut current) = self.view.lock() {
            *current = view.to_string();
        }
    }

    pub fn cancel_autohide(&self) -> u64 {
        self.autohide_generation.fetch_add(1, Ordering::SeqCst) + 1
    }
}
