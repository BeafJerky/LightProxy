use crate::error::CoreError;
use crate::model::ProxySnapshot;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Every mutation of the Windows proxy goes through this trait, which keeps the
/// state machine testable without touching the real registry.
pub trait ProxyBackend: Send + Sync {
    fn read(&self) -> Result<ProxySnapshot, CoreError>;
    fn write(&self, snapshot: &ProxySnapshot) -> Result<(), CoreError>;
    fn notify(&self) -> Result<(), CoreError>;
}

#[derive(Debug)]
pub struct MemoryBackend {
    value: Mutex<ProxySnapshot>,
    writes: AtomicUsize,
    notifies: AtomicUsize,
    fail_next_write: AtomicBool,
    fail_reads: AtomicBool,
}

impl MemoryBackend {
    pub fn new(initial: ProxySnapshot) -> Self {
        Self {
            value: Mutex::new(initial),
            writes: AtomicUsize::new(0),
            notifies: AtomicUsize::new(0),
            fail_next_write: AtomicBool::new(false),
            fail_reads: AtomicBool::new(false),
        }
    }

    pub fn snapshot(&self) -> ProxySnapshot {
        self.value.lock().expect("memory backend poisoned").clone()
    }

    pub fn writes(&self) -> usize {
        self.writes.load(Ordering::SeqCst)
    }

    pub fn notifies(&self) -> usize {
        self.notifies.load(Ordering::SeqCst)
    }

    /// Makes exactly the next write fail.
    pub fn fail_next_write(&self) {
        self.fail_next_write.store(true, Ordering::SeqCst);
    }

    pub fn fail_reads(&self, fail: bool) {
        self.fail_reads.store(fail, Ordering::SeqCst);
    }
}

impl ProxyBackend for MemoryBackend {
    fn read(&self) -> Result<ProxySnapshot, CoreError> {
        if self.fail_reads.load(Ordering::SeqCst) {
            return Err(CoreError::new(
                "registry_read_failed",
                "read",
                "模拟读取失败",
            ));
        }
        Ok(self.snapshot())
    }

    fn write(&self, snapshot: &ProxySnapshot) -> Result<(), CoreError> {
        if self.fail_next_write.swap(false, Ordering::SeqCst) {
            return Err(CoreError::new(
                "registry_write_failed",
                "write",
                "模拟写入失败",
            ));
        }
        *self.value.lock().expect("memory backend poisoned") = snapshot.clone();
        self.writes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn notify(&self) -> Result<(), CoreError> {
        self.notifies.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

impl<T: ProxyBackend + ?Sized> ProxyBackend for std::sync::Arc<T> {
    fn read(&self) -> Result<ProxySnapshot, CoreError> {
        (**self).read()
    }

    fn write(&self, snapshot: &ProxySnapshot) -> Result<(), CoreError> {
        (**self).write(snapshot)
    }

    fn notify(&self) -> Result<(), CoreError> {
        (**self).notify()
    }
}

#[derive(Default)]
pub struct NullBackend;

impl ProxyBackend for NullBackend {
    fn read(&self) -> Result<ProxySnapshot, CoreError> {
        Ok(ProxySnapshot::default())
    }

    fn write(&self, _snapshot: &ProxySnapshot) -> Result<(), CoreError> {
        Ok(())
    }

    fn notify(&self) -> Result<(), CoreError> {
        Ok(())
    }
}
