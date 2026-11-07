use crate::update::UpdateManager;
use lightproxy_core::{ActorHandle, Watcher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri_plugin_global_shortcut::Shortcut;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    Toggle,
    Panel,
    NextProfile,
}

/// 面板刚显示时的失焦可能只是系统把前台还给托盘弹出层或上一个窗口。
/// 只有显示稳定一段时间后的失焦才当作用户主动切换窗口。
pub struct RevealTracker {
    revealed_at: Mutex<Option<Instant>>,
}

impl RevealTracker {
    pub fn new() -> Self {
        Self {
            revealed_at: Mutex::new(None),
        }
    }

    pub fn mark_revealed(&self) {
        if let Ok(mut at) = self.revealed_at.lock() {
            *at = Some(Instant::now());
        }
    }

    pub fn reset(&self) {
        if let Ok(mut at) = self.revealed_at.lock() {
            *at = None;
        }
    }

    pub fn should_hide_on_blur(&self, settle: Duration) -> bool {
        match self.revealed_at.lock() {
            Ok(at) => at
                .map(|revealed_at| revealed_at.elapsed() >= settle)
                .unwrap_or(false),
            Err(_) => false,
        }
    }
}

pub struct AppCtx {
    pub actor: ActorHandle,
    pub data_dir: PathBuf,
    pub update: UpdateManager,
    pub view: Mutex<String>,
    pub reveal: RevealTracker,
    pub autohide_generation: AtomicU64,
    pub hotkey_map: Mutex<Vec<(Shortcut, HotkeyAction)>>,
    pub last_hotkey: Mutex<Option<Instant>>,
    pub watcher: Mutex<Option<Watcher>>,
}

impl AppCtx {
    pub fn new(actor: ActorHandle, data_dir: PathBuf, update_dir: PathBuf) -> Self {
        Self {
            actor,
            data_dir,
            update: UpdateManager::new(update_dir),
            view: Mutex::new("panel".into()),
            reveal: RevealTracker::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn blur_before_reveal_is_ignored() {
        let tracker = RevealTracker::new();
        assert!(!tracker.should_hide_on_blur(Duration::from_millis(300)));
    }

    #[test]
    fn blur_right_after_reveal_is_ignored() {
        let tracker = RevealTracker::new();
        tracker.mark_revealed();
        assert!(!tracker.should_hide_on_blur(Duration::from_millis(300)));
    }

    #[test]
    fn blur_after_panel_settles_hides() {
        let tracker = RevealTracker::new();
        tracker.mark_revealed();
        thread::sleep(Duration::from_millis(20));
        assert!(tracker.should_hide_on_blur(Duration::from_millis(10)));
    }

    #[test]
    fn reset_ignores_blur_until_next_reveal() {
        let tracker = RevealTracker::new();
        tracker.mark_revealed();
        thread::sleep(Duration::from_millis(20));
        tracker.reset();
        assert!(!tracker.should_hide_on_blur(Duration::ZERO));
    }
}
