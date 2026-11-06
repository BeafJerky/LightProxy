use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Polls the registry on a background thread. It is the fallback for
/// `RegNotifyChangeKeyValue`; the polling cost is one read every 2 seconds.
pub struct Watcher {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Watcher {
    pub fn spawn<F>(interval: Duration, mut tick: F) -> Self
    where
        F: FnMut() + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let handle = thread::Builder::new()
            .name("lightproxy-watcher".into())
            .spawn(move || {
                while !stop_flag.load(Ordering::SeqCst) {
                    thread::sleep(interval);
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    tick();
                }
            })
            .ok();
        Self { stop, handle }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
