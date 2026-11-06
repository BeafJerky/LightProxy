use crate::backend::ProxyBackend;
use crate::error::{CoreError, CoreResult};
use crate::model::{AppState, HealthResult, Profile, ProfileInput, RepairRecord, Settings};
use crate::repair::{Diagnostic, RepairResult};
use crate::service::{Command, CommandReply, ProxyService};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

struct Envelope {
    command: Command,
    reply: Sender<CoreResult<CommandReply>>,
}

/// Handle to the single-writer actor thread. `Sender` is not `Sync`, so the
/// handle wraps it in a mutex; Tauri state requires `Send + Sync`.
#[derive(Clone)]
pub struct ActorHandle {
    tx: Arc<Mutex<Sender<Envelope>>>,
}

impl ActorHandle {
    pub fn spawn<B: ProxyBackend + 'static>(service: ProxyService<B>) -> CoreResult<Self> {
        let (tx, rx) = mpsc::channel::<Envelope>();
        std::thread::Builder::new()
            .name("lightproxy-actor".into())
            .spawn(move || {
                let mut service = service;
                while let Ok(envelope) = rx.recv() {
                    let shutdown = matches!(envelope.command, Command::Shutdown);
                    let result = service.handle(envelope.command);
                    let _ = envelope.reply.send(result);
                    if shutdown {
                        break;
                    }
                }
            })
            .map_err(|err| CoreError::internal(format!("无法启动 Actor 线程：{err}")))?;
        Ok(Self {
            tx: Arc::new(Mutex::new(tx)),
        })
    }

    fn call(&self, command: Command) -> CoreResult<CommandReply> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .lock()
            .map_err(|_| CoreError::internal("Actor 通道已损坏"))?
            .send(Envelope {
                command,
                reply: reply_tx,
            })
            .map_err(|_| CoreError::internal("Actor 线程已退出"))?;
        reply_rx
            .recv()
            .map_err(|_| CoreError::internal("Actor 未返回结果"))?
    }

    fn state_reply(&self, command: Command) -> CoreResult<AppState> {
        match self.call(command)? {
            CommandReply::State(state) => Ok(state),
            _ => Err(CoreError::internal("Actor 返回了非状态结果")),
        }
    }

    pub fn get_state(&self) -> CoreResult<AppState> {
        self.state_reply(Command::GetState)
    }

    pub fn enable(&self, profile_id: impl Into<String>, force: bool) -> CoreResult<AppState> {
        self.state_reply(Command::Enable {
            profile_id: profile_id.into(),
            force,
        })
    }

    pub fn disable(&self) -> CoreResult<AppState> {
        self.state_reply(Command::Disable)
    }

    pub fn toggle(&self, force: bool) -> CoreResult<AppState> {
        self.state_reply(Command::Toggle { force })
    }

    pub fn switch_profile(
        &self,
        profile_id: impl Into<String>,
        force: bool,
    ) -> CoreResult<AppState> {
        self.state_reply(Command::SwitchProfile {
            profile_id: profile_id.into(),
            force,
        })
    }

    pub fn check_health(&self, force: bool) -> CoreResult<HealthResult> {
        match self.call(Command::CheckHealth { force })? {
            CommandReply::Health(result) => Ok(result),
            _ => Err(CoreError::internal("Actor 返回了非检查结果")),
        }
    }

    pub fn save_profile(&self, input: ProfileInput) -> CoreResult<AppState> {
        self.state_reply(Command::SaveProfile(input))
    }

    pub fn delete_profile(&self, id: impl Into<String>) -> CoreResult<AppState> {
        self.state_reply(Command::DeleteProfile { id: id.into() })
    }

    pub fn set_default_profile(&self, id: impl Into<String>) -> CoreResult<AppState> {
        self.state_reply(Command::SetDefaultProfile { id: id.into() })
    }

    pub fn update_settings(&self, patch: serde_json::Value) -> CoreResult<Settings> {
        match self.state_reply(Command::UpdateSettings(patch)) {
            Ok(state) => Ok(state.settings),
            Err(err) => Err(err),
        }
    }

    pub fn diagnostics(&self) -> CoreResult<Vec<Diagnostic>> {
        match self.call(Command::Diagnostics)? {
            CommandReply::Diagnostics(items) => Ok(items),
            _ => Err(CoreError::internal("Actor 返回了非诊断结果")),
        }
    }

    pub fn run_repair(&self, ids: Vec<String>) -> CoreResult<Vec<RepairResult>> {
        match self.call(Command::RunRepair { ids })? {
            CommandReply::Repairs(items) => Ok(items),
            _ => Err(CoreError::internal("Actor 返回了非修复结果")),
        }
    }

    pub fn recent_repairs(&self) -> CoreResult<Vec<RepairRecord>> {
        match self.call(Command::RecentRepairs)? {
            CommandReply::RepairLogs(items) => Ok(items),
            _ => Err(CoreError::internal("Actor 返回了非修复记录")),
        }
    }

    pub fn poll(&self) -> CoreResult<Option<AppState>> {
        match self.call(Command::Poll)? {
            CommandReply::OptionalState(state) => Ok(state),
            _ => Err(CoreError::internal("Actor 返回了非轮询结果")),
        }
    }

    pub fn recover(&self) -> CoreResult<AppState> {
        self.state_reply(Command::Recover)
    }

    pub fn set_hotkey_failures(&self, failures: Vec<String>) -> CoreResult<()> {
        self.call(Command::SetHotkeyFailures(failures))?;
        Ok(())
    }

    pub fn shutdown(&self) -> CoreResult<()> {
        self.call(Command::Shutdown)?;
        Ok(())
    }

    pub fn list_profiles(&self) -> CoreResult<Vec<Profile>> {
        Ok(self.get_state()?.profiles)
    }
}
