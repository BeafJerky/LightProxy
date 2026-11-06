mod app_ctx;
mod commands;
mod hotkeys;
mod tray;
mod window;

use app_ctx::AppCtx;
use lightproxy_core::{ActorHandle, ProxyService, StateStore, Watcher, WinInetBackend};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::ShortcutState;

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = window::open(app, "panel");
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if let Some(action) = hotkeys::resolve(app, shortcut) {
                        hotkeys::run(app, action);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::toggle_proxy,
            commands::enable_proxy,
            commands::disable_proxy,
            commands::switch_profile,
            commands::check_health,
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::set_default_profile,
            commands::get_settings,
            commands::update_settings,
            commands::get_diagnostics,
            commands::run_repair,
            commands::get_recent_repairs,
            commands::open_log_dir,
            commands::export_diagnostics,
            commands::quit_app,
            commands::open_settings,
            commands::open_panel,
            commands::set_view,
            commands::hide_window,
            commands::get_initial_view
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let _ = APP_HANDLE.set(handle.clone());

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = StateStore::new(&data_dir);
            let loaded = store.load();

            let on_state: Arc<dyn Fn(&lightproxy_core::AppState) + Send + Sync> =
                Arc::new(|state| {
                    let Some(app) = APP_HANDLE.get() else {
                        return;
                    };
                    let _ = app.emit("state-changed", state);
                    let app = app.clone();
                    let state = state.clone();
                    let tray_app = app.clone();
                    let _ = app.run_on_main_thread(move || tray::refresh(&tray_app, &state));
                });

            let service = ProxyService::new(
                store,
                WinInetBackend::default(),
                loaded.state,
                loaded.warning.clone(),
                loaded.readonly,
                on_state,
            );
            let actor = ActorHandle::spawn(service)
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
            app.manage(AppCtx::new(actor.clone(), data_dir.clone()));

            let state = actor
                .get_state()
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;
            tray::create(&handle, &state)
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;

            let failures = {
                let ctx = app.state::<AppCtx>();
                hotkeys::apply(&handle, &ctx, &state.settings.hotkeys)
            };
            let _ = actor.set_hotkey_failures(failures);

            if state.settings.autostart {
                let _ = app.autolaunch().enable();
            } else {
                let _ = app.autolaunch().disable();
            }

            let watch_actor = actor.clone();
            let watcher = Watcher::spawn(Duration::from_secs(2), move || {
                let _ = watch_actor.poll();
            });
            if let Ok(mut slot) = app.state::<AppCtx>().watcher.lock() {
                *slot = Some(watcher);
            }

            let recovery_actor = actor.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(250));
                let _ = recovery_actor.recover();
            });

            Ok(())
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running LightProxy");
}
