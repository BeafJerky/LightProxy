use crate::error::{CoreError, CoreResult};
use crate::model::{now_ms, PersistedState, SCHEMA_VERSION};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static CORRUPT_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct LoadResult {
    pub state: PersistedState,
    pub warning: Option<String>,
    pub readonly: bool,
}

pub struct StateStore {
    dir: PathBuf,
    path: PathBuf,
    backup: PathBuf,
    readonly: AtomicBool,
}

impl StateStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            path: dir.join("state.json"),
            backup: dir.join("state.json.bak"),
            dir,
            readonly: AtomicBool::new(false),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly.load(Ordering::SeqCst)
    }

    pub fn load(&self) -> LoadResult {
        self.readonly.store(false, Ordering::SeqCst);
        let main_exists = self.path.exists();
        let main_error = match self.parse_file(&self.path) {
            Ok(state) => return self.finalize(state, None),
            Err(err) => err,
        };

        if main_exists {
            self.archive_corrupt();
        }

        match self.parse_file(&self.backup) {
            Ok(state) => self.finalize(
                state,
                Some(format!("主配置损坏（{main_error}），已加载上次备份")),
            ),
            Err(_) => {
                let warning = if main_exists {
                    Some("配置文件与备份均损坏，已重建默认配置，损坏文件已保留".to_string())
                } else {
                    None
                };
                let state = PersistedState::initial();
                let _ = self.save(&state);
                self.finalize(state, warning)
            }
        }
    }

    pub fn save(&self, state: &PersistedState) -> CoreResult<()> {
        if self.is_readonly() {
            return Err(CoreError::new(
                "state_read_only",
                "storage",
                "配置由更高版本写入，当前版本处于只读模式",
            )
            .with_hint("升级 LightProxy 后再修改设置"));
        }
        fs::create_dir_all(&self.dir)?;
        let text = serde_json::to_vec_pretty(state)?;
        let tmp = self.path.with_extension("json.tmp");
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(&text)?;
            file.sync_all()?;
        }
        if self.path.exists() {
            let _ = fs::copy(&self.path, &self.backup);
        }
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn parse_file(&self, path: &Path) -> Result<PersistedState, String> {
        let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
        serde_json::from_str::<PersistedState>(&text).map_err(|err| err.to_string())
    }

    fn finalize(&self, mut state: PersistedState, warning: Option<String>) -> LoadResult {
        let mut readonly = false;
        let mut warning = warning;
        if state.schema_version > SCHEMA_VERSION {
            readonly = true;
            self.readonly.store(true, Ordering::SeqCst);
            warning = Some("配置来自更高版本，已进入只读模式".to_string());
        } else {
            state.schema_version = SCHEMA_VERSION;
        }
        if state.profiles.is_empty() {
            state.profiles = PersistedState::initial().profiles;
        }
        if state
            .default_profile_id
            .as_deref()
            .and_then(|id| state.profiles.iter().find(|p| p.id == id))
            .is_none()
        {
            state.default_profile_id = state.profiles.first().map(|p| p.id.clone());
        }
        if let Some(active) = state.runtime.active_profile_id.clone() {
            if !state.profiles.iter().any(|p| p.id == active) {
                state.runtime.active_profile_id = None;
            }
        }
        state.settings.normalize();
        LoadResult {
            state,
            warning,
            readonly,
        }
    }

    fn archive_corrupt(&self) {
        let seq = CORRUPT_SEQ.fetch_add(1, Ordering::Relaxed);
        let target = self
            .dir
            .join(format!("state.corrupt-{}-{seq}.json", now_ms()));
        let _ = fs::rename(&self.path, target);
    }
}
