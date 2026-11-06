use crate::app_ctx::AppCtx;
use crate::hotkeys;
use crate::window;
use lightproxy_core::{
    now_ms, ActorHandle, AppState, CoreError, Diagnostic, HealthResult, Profile, ProfileInput,
    RepairRecord, RepairResult, Settings,
};
use serde::Serialize;
use std::fs;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

pub(crate) async fn blocking<T, F>(work: F) -> Result<T, CoreError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, CoreError> + Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(work).await {
        Ok(result) => result,
        Err(err) => Err(CoreError::internal(format!("后台任务失败：{err}"))),
    }
}

fn actor(app: &AppHandle) -> ActorHandle {
    app.state::<AppCtx>().actor.clone()
}

#[tauri::command]
pub async fn get_state(app: AppHandle) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.get_state()).await
}

#[tauri::command]
pub async fn toggle_proxy(app: AppHandle, force: Option<bool>) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.toggle(force.unwrap_or(false))).await
}

#[tauri::command]
pub async fn enable_proxy(
    app: AppHandle,
    profile_id: String,
    force: Option<bool>,
) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.enable(profile_id, force.unwrap_or(false))).await
}

#[tauri::command]
pub async fn disable_proxy(app: AppHandle) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.disable()).await
}

#[tauri::command]
pub async fn switch_profile(
    app: AppHandle,
    profile_id: String,
    force: Option<bool>,
) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.switch_profile(profile_id, force.unwrap_or(false))).await
}

#[tauri::command]
pub async fn check_health(
    app: AppHandle,
    mode: Option<String>,
    force: Option<bool>,
) -> Result<HealthResult, CoreError> {
    let _ = mode;
    let actor = actor(&app);
    blocking(move || actor.check_health(force.unwrap_or(false))).await
}

#[tauri::command]
pub async fn list_profiles(app: AppHandle) -> Result<Vec<Profile>, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.list_profiles()).await
}

#[tauri::command]
pub async fn save_profile(app: AppHandle, input: ProfileInput) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.save_profile(input)).await
}

#[tauri::command]
pub async fn delete_profile(app: AppHandle, id: String) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.delete_profile(id)).await
}

#[tauri::command]
pub async fn set_default_profile(app: AppHandle, id: String) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.set_default_profile(id)).await
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<Settings, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.get_state().map(|state| state.settings)).await
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    patch: serde_json::Value,
) -> Result<AppState, CoreError> {
    let actor = actor(&app);
    let state = blocking(move || {
        actor.update_settings(patch)?;
        actor.get_state()
    })
    .await?;

    if state.settings.autostart {
        let _ = app.autolaunch().enable();
    } else {
        let _ = app.autolaunch().disable();
    }

    let (actor, failures) = {
        let ctx = app.state::<AppCtx>();
        let failures = hotkeys::apply(&app, &ctx, &state.settings.hotkeys);
        (ctx.actor.clone(), failures)
    };
    let _ = blocking(move || actor.set_hotkey_failures(failures)).await;
    Ok(state)
}

#[tauri::command]
pub async fn get_diagnostics(app: AppHandle) -> Result<Vec<Diagnostic>, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.diagnostics()).await
}

#[tauri::command]
pub async fn run_repair(app: AppHandle, ids: Vec<String>) -> Result<Vec<RepairResult>, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.run_repair(ids)).await
}

#[tauri::command]
pub async fn get_recent_repairs(app: AppHandle) -> Result<Vec<RepairRecord>, CoreError> {
    let actor = actor(&app);
    blocking(move || actor.recent_repairs()).await
}

fn log_dir(app: &AppHandle) -> std::path::PathBuf {
    app.path()
        .app_log_dir()
        .unwrap_or_else(|_| app.state::<AppCtx>().data_dir.join("logs"))
}

#[tauri::command]
pub fn open_log_dir(app: AppHandle) -> Result<String, CoreError> {
    let dir = log_dir(&app);
    fs::create_dir_all(&dir)?;
    std::process::Command::new("explorer.exe")
        .arg(&dir)
        .spawn()
        .map_err(|err| CoreError::internal(format!("无法打开日志目录：{err}")))?;
    Ok(dir.to_string_lossy().to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
}

#[tauri::command]
pub async fn export_diagnostics(app: AppHandle) -> Result<ExportResult, CoreError> {
    let actor = actor(&app);
    let (state, diagnostics, repairs) = blocking(move || {
        Ok::<_, CoreError>((
            actor.get_state()?,
            actor.diagnostics()?,
            actor.recent_repairs()?,
        ))
    })
    .await?;

    let dir = log_dir(&app);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("diagnostics-{}.json", now_ms()));
    let payload = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "generatedAtMs": now_ms(),
        "state": state,
        "diagnostics": diagnostics,
        "repairs": repairs,
    });
    fs::write(&path, serde_json::to_vec_pretty(&payload)?)?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> Result<(), CoreError> {
    let actor = actor(&app);
    blocking(move || actor.shutdown()).await?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), CoreError> {
    window::open(&app, "settings").map_err(|err| CoreError::internal(err.to_string()))
}

#[tauri::command]
pub fn open_panel(app: AppHandle) -> Result<(), CoreError> {
    window::open(&app, "panel").map_err(|err| CoreError::internal(err.to_string()))
}

#[tauri::command]
pub fn set_view(app: AppHandle, view: String) -> Result<(), CoreError> {
    window::set_view(&app, &view).map_err(|err| CoreError::internal(err.to_string()))
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), CoreError> {
    window::hide(&app).map_err(|err| CoreError::internal(err.to_string()))
}

#[tauri::command]
pub fn get_initial_view(app: AppHandle) -> String {
    app.state::<AppCtx>().current_view()
}
