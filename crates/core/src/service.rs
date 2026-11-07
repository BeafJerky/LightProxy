use crate::backend::ProxyBackend;
use crate::error::{CoreError, CoreResult};
use crate::health;
use crate::model::{
    now_ms, AppState, Desired, HealthResult, PersistedState, Profile, ProfileInput, ProxySnapshot,
    ProxyState, RepairRecord, Settings, Transition,
};
use crate::profile::{new_profile_id, profile_from_input, validate_profile};
use crate::recovery::{self, RecoveryDecision};
use crate::repair::{
    build_diagnostics, push_repair_log, Diagnostic, DiagnosticInput, RepairResult,
};
use crate::store::StateStore;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

const HEALTH_CACHE: Duration = Duration::from_secs(30);

pub type StateListener = Arc<dyn Fn(&AppState) + Send + Sync>;

pub enum Command {
    GetState,
    Enable { profile_id: String, force: bool },
    Disable,
    Toggle { force: bool },
    SwitchProfile { profile_id: String, force: bool },
    CheckHealth { force: bool },
    SaveProfile(ProfileInput),
    DeleteProfile { id: String },
    SetDefaultProfile { id: String },
    UpdateSettings(serde_json::Value),
    Diagnostics,
    RunRepair { ids: Vec<String> },
    RecentRepairs,
    Poll,
    Recover,
    SetHotkeyFailures(Vec<String>),
    Shutdown,
}

pub enum CommandReply {
    State(AppState),
    Health(HealthResult),
    Diagnostics(Vec<Diagnostic>),
    Repairs(Vec<RepairResult>),
    RepairLogs(Vec<RepairRecord>),
    OptionalState(Option<AppState>),
    Unit,
}

pub struct ProxyService<B: ProxyBackend> {
    store: StateStore,
    backend: B,
    persisted: PersistedState,
    state: ProxyState,
    message: Option<String>,
    health: Option<(HealthResult, Instant)>,
    on_state: StateListener,
    transitions: VecDeque<Transition>,
    store_warning: Option<String>,
    last_save_error: Option<String>,
    hotkey_failures: Vec<String>,
    read_only: bool,
}

impl<B: ProxyBackend> ProxyService<B> {
    pub fn new(
        store: StateStore,
        backend: B,
        persisted: PersistedState,
        store_warning: Option<String>,
        read_only: bool,
        on_state: StateListener,
    ) -> Self {
        let stale = Instant::now()
            .checked_sub(HEALTH_CACHE)
            .unwrap_or_else(Instant::now);
        let health = persisted
            .runtime
            .last_health
            .clone()
            .map(|value| (value, stale));
        Self {
            store,
            backend,
            persisted,
            state: ProxyState::Off,
            message: None,
            health,
            on_state,
            transitions: VecDeque::new(),
            store_warning,
            last_save_error: None,
            hotkey_failures: Vec::new(),
            read_only,
        }
    }

    pub fn state(&self) -> AppState {
        self.app_state()
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn last_pac_url(&self) -> Option<&str> {
        self.persisted.runtime.last_pac_url.as_deref()
    }

    pub fn handle(&mut self, command: Command) -> CoreResult<CommandReply> {
        match command {
            Command::GetState => Ok(CommandReply::State(self.app_state())),
            Command::Enable { profile_id, force } => {
                self.enable(&profile_id, force).map(CommandReply::State)
            }
            Command::Disable => self.disable().map(CommandReply::State),
            Command::Toggle { force } => self.toggle(force).map(CommandReply::State),
            Command::SwitchProfile { profile_id, force } => self
                .switch_profile(&profile_id, force)
                .map(CommandReply::State),
            Command::CheckHealth { force } => self.check_health(force).map(CommandReply::Health),
            Command::SaveProfile(input) => self.save_profile(input).map(CommandReply::State),
            Command::DeleteProfile { id } => self.delete_profile(&id).map(CommandReply::State),
            Command::SetDefaultProfile { id } => {
                self.set_default_profile(&id).map(CommandReply::State)
            }
            Command::UpdateSettings(patch) => self.update_settings(patch).map(CommandReply::State),
            Command::Diagnostics => Ok(CommandReply::Diagnostics(self.diagnostics())),
            Command::RunRepair { ids } => Ok(CommandReply::Repairs(self.run_repair(&ids))),
            Command::RecentRepairs => Ok(CommandReply::RepairLogs(self.recent_repairs())),
            Command::Poll => self.poll_registry().map(CommandReply::OptionalState),
            Command::Recover => {
                self.recover();
                Ok(CommandReply::State(self.app_state()))
            }
            Command::SetHotkeyFailures(items) => {
                self.hotkey_failures = items;
                Ok(CommandReply::Unit)
            }
            Command::Shutdown => {
                self.shutdown();
                Ok(CommandReply::Unit)
            }
        }
    }

    /// Applies the crash/startup recovery rules. Call after the tray exists.
    pub fn recover(&mut self) {
        let clean = self.persisted.clean_shutdown;
        let registry = match self.backend.read() {
            Ok(value) => value,
            Err(err) => {
                self.set_state(ProxyState::Error, Some(err.message.clone()));
                self.persisted.clean_shutdown = false;
                self.persist_and_emit();
                return;
            }
        };
        let candidate = recovery::matching_profile(&self.persisted.profiles, &registry).cloned();
        let mut reachable = None;
        if registry.proxy_enable {
            if let Some(profile) = &candidate {
                let result = health::probe(
                    &profile.host,
                    profile.port,
                    Duration::from_millis(self.persisted.settings.preflight_timeout_ms),
                );
                reachable = Some(result.ok);
                self.health = Some((result.clone(), Instant::now()));
                self.persisted.runtime.last_health = Some(result);
            }
        }
        match recovery::decide_recovery(clean, &registry, &self.persisted, reachable) {
            RecoveryDecision::On {
                profile_id,
                fill_snapshot,
            } => {
                if fill_snapshot {
                    self.persisted.snapshot = Some(registry.clone());
                }
                self.persisted.runtime.desired = Desired::On;
                self.persisted.runtime.active_profile_id = Some(profile_id);
                self.set_state(ProxyState::On, None);
            }
            RecoveryDecision::Degraded {
                profile_id,
                message,
            } => {
                if let Some(profile_id) = profile_id {
                    self.persisted.runtime.active_profile_id = Some(profile_id);
                    self.persisted.runtime.desired = Desired::On;
                }
                self.set_state(ProxyState::Degraded, Some(message));
            }
            RecoveryDecision::Off { message } => {
                self.persisted.runtime.desired = Desired::Off;
                self.persisted.runtime.active_profile_id = None;
                self.persisted.snapshot = None;
                self.set_state(ProxyState::Off, message);
            }
        }
        self.persisted.clean_shutdown = false;
        self.persist_and_emit();
    }

    pub fn enable(&mut self, profile_id: &str, force: bool) -> CoreResult<AppState> {
        let profile = self.find_profile(profile_id)?.clone();
        let current = self.backend.read()?;
        let target = self.target_for(&profile);

        if current.proxy_enable
            && current.proxy_server == target.proxy_server
            && current.auto_config_url.is_none()
        {
            if self.persisted.snapshot.is_none() {
                self.persisted.snapshot = Some(current);
            }
            self.persisted.runtime.desired = Desired::On;
            self.persisted.runtime.active_profile_id = Some(profile.id.clone());
            self.set_state(ProxyState::On, None);
            self.persist_and_emit();
            return Ok(self.app_state());
        }

        if current.auto_config_url.is_some() && !force {
            return Err(CoreError::new(
                "pac_conflict",
                "preflight",
                "检测到 PAC 自动化脚本，PAC 会优先于固定代理",
            )
            .with_hint("确认清除 PAC 后可选择强制启用"));
        }
        self.remember_pac(&current);

        if self.persisted.settings.preflight_enabled && !force {
            let result = health::probe(
                &profile.host,
                profile.port,
                Duration::from_millis(self.persisted.settings.preflight_timeout_ms),
            );
            self.health = Some((result.clone(), Instant::now()));
            self.persisted.runtime.last_health = Some(result.clone());
            if !result.ok {
                let message = result
                    .error
                    .clone()
                    .unwrap_or_else(|| "代理端口不可达".into());
                self.message = Some(format!("预检失败：{message}"));
                self.persist_and_emit();
                return Err(CoreError::new("proxy_unreachable", "preflight", message)
                    .with_hint("检查代理地址，或选择强制启用"));
            }
        }

        let had_snapshot = self.persisted.snapshot.is_some();
        if !had_snapshot {
            self.persisted.snapshot = Some(current.clone());
        }
        self.set_state(ProxyState::Enabling, None);
        match self.write_verified(&target) {
            Ok(()) => {
                self.persisted.runtime.desired = Desired::On;
                self.persisted.runtime.active_profile_id = Some(profile.id.clone());
                self.set_state(ProxyState::On, None);
                self.persist_and_emit();
                Ok(self.app_state())
            }
            Err(err) => {
                let _ = self.backend.write(&current);
                let _ = self.backend.notify();
                if !had_snapshot {
                    self.persisted.snapshot = None;
                }
                self.persisted.runtime.desired = Desired::Off;
                self.persisted.runtime.active_profile_id = None;
                self.set_state(ProxyState::Error, Some(err.message.clone()));
                self.persist_and_emit();
                Err(err)
            }
        }
    }

    pub fn disable(&mut self) -> CoreResult<AppState> {
        if self.state == ProxyState::Off {
            self.persisted.snapshot = None;
            self.persisted.runtime.active_profile_id = None;
            self.persisted.runtime.desired = Desired::Off;
            self.set_state(ProxyState::Off, None);
            self.persist_and_emit();
            return Ok(self.app_state());
        }
        let current = self.backend.read()?;
        self.set_state(ProxyState::Disabling, None);
        let target = match self.persisted.snapshot.clone() {
            Some(snapshot) if snapshot != current => snapshot,
            Some(snapshot) => ProxySnapshot {
                proxy_enable: false,
                ..snapshot
            },
            None => ProxySnapshot {
                proxy_enable: false,
                ..current
            },
        };
        match self.write_verified(&target) {
            Ok(()) => {
                self.persisted.snapshot = None;
                self.persisted.runtime.active_profile_id = None;
                self.persisted.runtime.desired = Desired::Off;
                self.set_state(ProxyState::Off, None);
                self.persist_and_emit();
                Ok(self.app_state())
            }
            Err(err) => {
                self.persisted.runtime.desired = Desired::Off;
                self.set_state(ProxyState::Error, Some(err.message.clone()));
                self.persist_and_emit();
                Err(err)
            }
        }
    }

    pub fn toggle(&mut self, force: bool) -> CoreResult<AppState> {
        if self.state.is_active() {
            return self.disable();
        }
        let profile_id = self
            .persisted
            .runtime
            .active_profile_id
            .clone()
            .or_else(|| self.persisted.default_profile_id.clone())
            .or_else(|| self.persisted.profiles.first().map(|p| p.id.clone()))
            .ok_or_else(|| {
                CoreError::new("no_profile", "validate", "还没有可用的代理配置")
                    .with_hint("先在设置中添加一套配置")
            })?;
        self.enable(&profile_id, force)
    }

    pub fn switch_profile(&mut self, profile_id: &str, force: bool) -> CoreResult<AppState> {
        let profile = self.find_profile(profile_id)?.clone();
        if !self.state.is_active() {
            return self.enable(profile_id, force);
        }
        let previous = self.backend.read()?;
        if previous.auto_config_url.is_some() && !force {
            if self.remember_pac(&previous) {
                self.persist();
            }
            return Err(CoreError::new(
                "pac_conflict",
                "preflight",
                "检测到 PAC 自动化脚本，PAC 会优先于固定代理",
            )
            .with_hint("确认清除 PAC 后可选择强制切换"));
        }
        self.remember_pac(&previous);
        if self.persisted.settings.preflight_enabled {
            let result = health::probe(
                &profile.host,
                profile.port,
                Duration::from_millis(self.persisted.settings.preflight_timeout_ms),
            );
            self.health = Some((result.clone(), Instant::now()));
            self.persisted.runtime.last_health = Some(result.clone());
            if !result.ok {
                let message = result.error.clone().unwrap_or_else(|| "端口不可达".into());
                self.message = Some(format!("切换失败：{message}"));
                self.persist_and_emit();
                return Err(CoreError::new("proxy_unreachable", "preflight", message)
                    .with_hint("保持原配置不变"));
            }
        }
        let target = self.target_for(&profile);
        match self.write_verified(&target) {
            Ok(()) => {
                self.persisted.runtime.active_profile_id = Some(profile.id.clone());
                self.persisted.runtime.desired = Desired::On;
                self.set_state(ProxyState::On, None);
                self.persist_and_emit();
                Ok(self.app_state())
            }
            Err(err) => {
                let _ = self.backend.write(&previous);
                let _ = self.backend.notify();
                self.set_state(ProxyState::Degraded, Some(err.message.clone()));
                self.persist_and_emit();
                Err(err)
            }
        }
    }

    pub fn check_health(&mut self, force: bool) -> CoreResult<HealthResult> {
        if !force {
            if let Some((cached, at)) = &self.health {
                if at.elapsed() < HEALTH_CACHE {
                    return Ok(cached.clone());
                }
            }
        }
        let profile_id = self
            .persisted
            .runtime
            .active_profile_id
            .clone()
            .or_else(|| self.persisted.default_profile_id.clone());
        let profile = profile_id
            .as_deref()
            .and_then(|id| self.persisted.profile(id))
            .cloned()
            .ok_or_else(|| {
                CoreError::new("no_profile", "health", "没有可检查的配置")
                    .with_hint("先添加一套代理配置")
            })?;
        let result = health::probe(
            &profile.host,
            profile.port,
            Duration::from_millis(self.persisted.settings.preflight_timeout_ms),
        );
        self.health = Some((result.clone(), Instant::now()));
        self.persisted.runtime.last_health = Some(result.clone());
        if !result.ok && self.state == ProxyState::On {
            let message = result.error.clone().unwrap_or_else(|| "代理不可达".into());
            self.set_state(ProxyState::Degraded, Some(message));
        } else if result.ok && self.state == ProxyState::Degraded {
            if let Ok(current) = self.backend.read() {
                if current.same_proxy(&self.target_for(&profile)) {
                    self.set_state(ProxyState::On, None);
                }
            }
        }
        self.persist_and_emit();
        Ok(result)
    }

    /// Compares the registry with our intent; returns Some(state) on drift.
    pub fn poll_registry(&mut self) -> CoreResult<Option<AppState>> {
        if !self.state.is_active() {
            return Ok(None);
        }
        let current = self.backend.read()?;
        let profile = self
            .persisted
            .runtime
            .active_profile_id
            .as_deref()
            .and_then(|id| self.persisted.profile(id))
            .cloned();
        let expected = match &profile {
            Some(profile) => self.target_for(profile),
            None => {
                self.set_state(ProxyState::Degraded, Some("当前配置已被删除".into()));
                self.persist_and_emit();
                return Ok(Some(self.app_state()));
            }
        };
        if current.same_proxy(&expected) && current.auto_config_url == expected.auto_config_url {
            return Ok(None);
        }
        let message = if !current.proxy_enable {
            "代理已被其他程序关闭".to_string()
        } else if current.auto_config_url.is_some() {
            "其他程序设置了 PAC，当前可能绕过本应用配置".to_string()
        } else {
            format!("代理已被其他程序改为 {}", current.proxy_server)
        };
        self.set_state(ProxyState::Degraded, Some(message));
        self.persist_and_emit();
        Ok(Some(self.app_state()))
    }

    pub fn save_profile(&mut self, input: ProfileInput) -> CoreResult<AppState> {
        let id = input
            .id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(new_profile_id);
        let profile = profile_from_input(
            id.clone(),
            input.name,
            input.protocol,
            input.host,
            input.port,
        )?;
        validate_profile(&profile, &self.persisted.profiles)?;
        let editing_active =
            self.persisted.runtime.active_profile_id.as_deref() == Some(id.as_str());
        match self
            .persisted
            .profiles
            .iter_mut()
            .find(|item| item.id == id)
        {
            Some(existing) => *existing = profile,
            None => self.persisted.profiles.push(profile),
        }
        if self.persisted.default_profile_id.is_none() {
            self.persisted.default_profile_id = Some(id.clone());
        }
        self.persist();
        if editing_active && self.state.is_active() {
            return self.switch_profile(&id, false);
        }
        self.emit();
        Ok(self.app_state())
    }

    pub fn delete_profile(&mut self, id: &str) -> CoreResult<AppState> {
        if self.persisted.default_profile_id.as_deref() == Some(id) {
            return Err(
                CoreError::new("default_profile_locked", "validate", "默认配置不能删除")
                    .with_hint("先把其他配置设为默认"),
            );
        }
        if !self.persisted.profiles.iter().any(|p| p.id == id) {
            return Err(CoreError::profile_not_found(id));
        }
        if self.persisted.runtime.active_profile_id.as_deref() == Some(id) {
            let fallback = self.persisted.default_profile_id.clone();
            if let Some(fallback) = fallback {
                if self.state.is_active() {
                    self.switch_profile(&fallback, false)?;
                }
            }
        }
        self.persisted.profiles.retain(|profile| profile.id != id);
        if self.persisted.runtime.active_profile_id.as_deref() == Some(id) {
            self.persisted.runtime.active_profile_id = None;
        }
        self.persisted.runtime.desired = if self.state.is_active() {
            Desired::On
        } else {
            Desired::Off
        };
        self.persist_and_emit();
        Ok(self.app_state())
    }

    pub fn set_default_profile(&mut self, id: &str) -> CoreResult<AppState> {
        self.find_profile(id)?;
        self.persisted.default_profile_id = Some(id.to_string());
        self.persist_and_emit();
        Ok(self.app_state())
    }

    pub fn update_settings(&mut self, patch: serde_json::Value) -> CoreResult<AppState> {
        let mut current = serde_json::to_value(&self.persisted.settings)?;
        merge_json(&mut current, patch);
        let mut settings: Settings = serde_json::from_value(current).map_err(|err| {
            CoreError::new("invalid_settings", "settings", err.to_string())
                .with_hint("刷新设置页后重试")
        })?;
        settings.normalize();
        self.persisted.settings = settings;
        self.persist_and_emit();
        Ok(self.app_state())
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let registry = self.backend.read().unwrap_or_default();
        build_diagnostics(DiagnosticInput {
            desired: self.persisted.runtime.desired,
            state: self.state,
            registry: &registry,
            last_health: self.persisted.runtime.last_health.as_ref(),
            store_warning: self.store_warning.as_deref(),
            last_save_error: self.last_save_error.as_deref(),
            hotkey_failures: &self.hotkey_failures,
        })
    }

    pub fn recent_repairs(&self) -> Vec<RepairRecord> {
        self.persisted.repairs.clone()
    }

    pub fn run_repair(&mut self, ids: &[String]) -> Vec<RepairResult> {
        let mut results = Vec::new();
        for id in ids {
            let result = match id.as_str() {
                "R01" | "R05" => self.repair_reapply(),
                "R03" => self.repair_clear_pac(),
                _ => RepairResult {
                    id: id.clone(),
                    title: "需要手动处理".into(),
                    ok: false,
                    message: "该项没有自动修复动作，请按提示检查".into(),
                },
            };
            push_repair_log(
                &mut self.persisted.repairs,
                RepairRecord {
                    id: result.id.clone(),
                    title: result.title.clone(),
                    ok: result.ok,
                    message: result.message.clone(),
                    at_ms: now_ms(),
                },
            );
            results.push(result);
        }
        self.persist_and_emit();
        results
    }

    pub fn shutdown(&mut self) {
        if self.persisted.runtime.desired == Desired::On
            && !self.persisted.settings.keep_proxy_on_exit
        {
            let _ = self.disable();
        }
        self.persisted.clean_shutdown = true;
        self.persist();
    }

    pub fn set_hotkey_failures(&mut self, failures: Vec<String>) {
        self.hotkey_failures = failures;
    }

    fn repair_reapply(&mut self) -> RepairResult {
        let profile = self
            .persisted
            .runtime
            .active_profile_id
            .as_deref()
            .and_then(|id| self.persisted.profile(id))
            .or_else(|| self.persisted.default_profile())
            .cloned();
        let Some(profile) = profile else {
            return RepairResult {
                id: "R01".into(),
                title: "重写代理设置".into(),
                ok: false,
                message: "没有可用配置".into(),
            };
        };
        let target = self.target_for(&profile);
        match self.write_verified(&target) {
            Ok(()) => {
                self.persisted.runtime.desired = Desired::On;
                self.persisted.runtime.active_profile_id = Some(profile.id);
                self.set_state(ProxyState::On, None);
                RepairResult {
                    id: "R01".into(),
                    title: "重写代理设置".into(),
                    ok: true,
                    message: "已重新写入并校验注册表".into(),
                }
            }
            Err(err) => RepairResult {
                id: "R01".into(),
                title: "重写代理设置".into(),
                ok: false,
                message: err.message,
            },
        }
    }

    fn repair_clear_pac(&mut self) -> RepairResult {
        let current = match self.backend.read() {
            Ok(value) => value,
            Err(err) => {
                return RepairResult {
                    id: "R03".into(),
                    title: "清除 PAC".into(),
                    ok: false,
                    message: err.message,
                }
            }
        };
        if current.auto_config_url.is_none() {
            return RepairResult {
                id: "R03".into(),
                title: "清除 PAC".into(),
                ok: true,
                message: "没有检测到 PAC，无需处理".into(),
            };
        }
        self.remember_pac(&current);
        let target = ProxySnapshot {
            auto_config_url: None,
            ..current
        };
        match self.write_verified(&target) {
            Ok(()) => RepairResult {
                id: "R03".into(),
                title: "清除 PAC".into(),
                ok: true,
                message: "已清空 AutoConfigURL 并刷新系统代理".into(),
            },
            Err(err) => RepairResult {
                id: "R03".into(),
                title: "清除 PAC".into(),
                ok: false,
                message: err.message,
            },
        }
    }

    fn remember_pac(&mut self, current: &ProxySnapshot) -> bool {
        match &current.auto_config_url {
            Some(url) if self.persisted.runtime.last_pac_url.as_deref() != Some(url.as_str()) => {
                self.persisted.runtime.last_pac_url = Some(url.clone());
                true
            }
            _ => false,
        }
    }

    fn write_verified(&mut self, target: &ProxySnapshot) -> CoreResult<()> {
        self.backend.write(target)?;
        if let Err(err) = self.backend.notify() {
            self.message = Some(err.message);
        }
        let mut last = ProxySnapshot::default();
        for attempt in 0..2 {
            let readback = self.backend.read()?;
            if readback.same_proxy(target) && readback.auto_config_url == target.auto_config_url {
                return Ok(());
            }
            last = readback;
            if attempt == 0 {
                self.backend.write(target)?;
            }
        }
        Err(CoreError::new(
            "readback_mismatch",
            "verify",
            format!(
                "写入后读回不一致：期望 {}，实际 {}",
                target.proxy_server, last.proxy_server
            ),
        )
        .with_hint("检查组策略或安全软件是否锁定 Internet Settings"))
    }

    fn target_for(&self, profile: &Profile) -> ProxySnapshot {
        ProxySnapshot {
            proxy_enable: true,
            proxy_server: profile.server(),
            proxy_override: self.persisted.settings.proxy_override.clone(),
            auto_config_url: None,
        }
    }

    fn find_profile(&self, id: &str) -> CoreResult<&Profile> {
        self.persisted
            .profile(id)
            .ok_or_else(|| CoreError::profile_not_found(id))
    }

    fn app_state(&self) -> AppState {
        AppState {
            state: self.state,
            desired: self.persisted.runtime.desired,
            active_profile_id: self.persisted.runtime.active_profile_id.clone(),
            last_health: self.persisted.runtime.last_health.clone(),
            message: self.message.clone(),
            profiles: self.persisted.profiles.clone(),
            default_profile_id: self.persisted.default_profile_id.clone(),
            settings: self.persisted.settings.clone(),
            snapshot: self.persisted.snapshot.clone(),
        }
    }

    fn set_state(&mut self, to: ProxyState, message: Option<String>) {
        if self.state != to {
            let from = state_name(self.state).to_string();
            self.transitions.push_back(Transition {
                from,
                to: state_name(to).to_string(),
                reason: message.clone().unwrap_or_default(),
                at_ms: now_ms(),
            });
            while self.transitions.len() > 20 {
                self.transitions.pop_front();
            }
        }
        self.state = to;
        self.message = message;
    }

    fn persist(&mut self) {
        self.persisted.schema_version = crate::model::SCHEMA_VERSION;
        match self.store.save(&self.persisted) {
            Ok(()) => self.last_save_error = None,
            Err(err) => self.last_save_error = Some(err.message),
        }
    }

    fn persist_and_emit(&mut self) {
        self.persist();
        self.emit();
    }

    fn emit(&self) {
        (self.on_state)(&self.app_state());
    }
}

fn state_name(state: ProxyState) -> &'static str {
    match state {
        ProxyState::Off => "off",
        ProxyState::Enabling => "enabling",
        ProxyState::On => "on",
        ProxyState::Degraded => "degraded",
        ProxyState::Repairing => "repairing",
        ProxyState::Disabling => "disabling",
        ProxyState::Error => "error",
    }
}

fn merge_json(base: &mut serde_json::Value, patch: serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(base), serde_json::Value::Object(patch)) => {
            for (key, value) in patch {
                merge_json(base.entry(key).or_insert(serde_json::Value::Null), value);
            }
        }
        (base, patch) => *base = patch,
    }
}
