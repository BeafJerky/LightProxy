use crate::env::ENV_VARS;
use crate::error::CoreError;
use crate::model::{EnvVar, ProxySnapshot};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Every mutation of the Windows proxy goes through this trait, which keeps the
/// state machine testable without touching the real registry.
pub trait ProxyBackend: Send + Sync {
    fn read(&self) -> Result<ProxySnapshot, CoreError>;
    fn write(&self, snapshot: &ProxySnapshot) -> Result<(), CoreError>;
    fn notify(&self) -> Result<(), CoreError>;
    fn read_env(&self) -> Result<Vec<EnvVar>, CoreError>;
    fn write_env(&self, vars: &[EnvVar]) -> Result<(), CoreError>;
    fn notify_env(&self) -> Result<(), CoreError>;
}

#[derive(Debug)]
pub struct MemoryBackend {
    value: Mutex<ProxySnapshot>,
    writes: AtomicUsize,
    notifies: AtomicUsize,
    fail_next_write: AtomicBool,
    fail_reads: AtomicBool,
    env: Mutex<Vec<EnvVar>>,
    env_writes: AtomicUsize,
    env_notifies: AtomicUsize,
    fail_next_env_write: AtomicBool,
    fail_env_reads: AtomicBool,
}

impl MemoryBackend {
    pub fn new(initial: ProxySnapshot) -> Self {
        Self {
            value: Mutex::new(initial),
            writes: AtomicUsize::new(0),
            notifies: AtomicUsize::new(0),
            fail_next_write: AtomicBool::new(false),
            fail_reads: AtomicBool::new(false),
            env: Mutex::new(Vec::new()),
            env_writes: AtomicUsize::new(0),
            env_notifies: AtomicUsize::new(0),
            fail_next_env_write: AtomicBool::new(false),
            fail_env_reads: AtomicBool::new(false),
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

    pub fn env_vars(&self) -> Vec<EnvVar> {
        self.env.lock().expect("memory env poisoned").clone()
    }

    pub fn env_writes(&self) -> usize {
        self.env_writes.load(Ordering::SeqCst)
    }

    pub fn env_notifies(&self) -> usize {
        self.env_notifies.load(Ordering::SeqCst)
    }

    /// 测试 seeding：直接设置环境值，不计入写入计数。
    pub fn set_env(&self, vars: Vec<EnvVar>) {
        *self.env.lock().expect("memory env poisoned") = vars;
    }

    pub fn fail_next_env_write(&self) {
        self.fail_next_env_write.store(true, Ordering::SeqCst);
    }

    pub fn fail_env_reads(&self, fail: bool) {
        self.fail_env_reads.store(fail, Ordering::SeqCst);
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

    fn read_env(&self) -> Result<Vec<EnvVar>, CoreError> {
        if self.fail_env_reads.load(Ordering::SeqCst) {
            return Err(CoreError::new(
                "env_read_failed",
                "env_read",
                "模拟环境读取失败",
            ));
        }
        let stored = self.env_vars();
        Ok(ENV_VARS
            .iter()
            .map(|name| {
                stored
                    .iter()
                    .find(|v| v.name.eq_ignore_ascii_case(name))
                    .cloned()
                    .unwrap_or(EnvVar {
                        name: (*name).into(),
                        value: None,
                        expand: false,
                    })
            })
            .collect())
    }

    fn write_env(&self, vars: &[EnvVar]) -> Result<(), CoreError> {
        if self.fail_next_env_write.swap(false, Ordering::SeqCst) {
            return Err(CoreError::new(
                "env_write_failed",
                "env_write",
                "模拟环境写入失败",
            ));
        }
        let mut stored = self.env.lock().expect("memory env poisoned");
        for v in vars {
            stored.retain(|s| !s.name.eq_ignore_ascii_case(&v.name));
            if v.value.is_some() {
                stored.push(v.clone());
            }
        }
        self.env_writes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn notify_env(&self) -> Result<(), CoreError> {
        self.env_notifies.fetch_add(1, Ordering::SeqCst);
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

    fn read_env(&self) -> Result<Vec<EnvVar>, CoreError> {
        (**self).read_env()
    }

    fn write_env(&self, vars: &[EnvVar]) -> Result<(), CoreError> {
        (**self).write_env(vars)
    }

    fn notify_env(&self) -> Result<(), CoreError> {
        (**self).notify_env()
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

    fn read_env(&self) -> Result<Vec<EnvVar>, CoreError> {
        Ok(Vec::new())
    }

    fn write_env(&self, _vars: &[EnvVar]) -> Result<(), CoreError> {
        Ok(())
    }

    fn notify_env(&self) -> Result<(), CoreError> {
        Ok(())
    }
}
