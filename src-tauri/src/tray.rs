use crate::app_ctx::AppCtx;
use crate::window;
use lightproxy_core::{AppState, ProxyState};
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

fn status_text(state: &AppState) -> String {
    match state.state {
        ProxyState::Off => "代理已关闭".into(),
        ProxyState::Enabling => "正在启用…".into(),
        ProxyState::Disabling => "正在关闭…".into(),
        ProxyState::Repairing => "正在修复…".into(),
        ProxyState::On => {
            let server = current_server(state).unwrap_or_else(|| "未配置".into());
            match &state.last_health {
                Some(health) if health.ok => {
                    format!("代理已开启 · {server} · {}ms", health.latency_ms)
                }
                _ => format!("代理已开启 · {server}"),
            }
        }
        ProxyState::Degraded => format!(
            "代理异常：{}",
            state.message.clone().unwrap_or_else(|| "检测失败".into())
        ),
        ProxyState::Error => format!(
            "代理不可用：{}",
            state.message.clone().unwrap_or_else(|| "未知错误".into())
        ),
    }
}

fn current_server(state: &AppState) -> Option<String> {
    let id = state.active_profile_id.as_deref()?;
    state
        .profiles
        .iter()
        .find(|profile| profile.id == id)
        .map(|profile| profile.server())
}

fn tooltip(state: &AppState) -> String {
    let text = status_text(state);
    if text.chars().count() > 120 {
        text.chars().take(117).collect::<String>() + "…"
    } else {
        text
    }
}

pub fn build_menu(app: &AppHandle, state: &AppState) -> tauri::Result<Menu<Wry>> {
    let active = state.state.is_active();
    let status = MenuItem::with_id(app, "status", status_text(state), false, None::<&str>)?;
    let toggle_label = if active {
        "关闭代理"
    } else {
        "开启代理"
    };
    let toggle = MenuItem::with_id(app, "toggle", toggle_label, true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "检查连通性", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 LightProxy", true, None::<&str>)?;

    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = vec![
        Box::new(status),
        Box::new(PredefinedMenuItem::separator(app)?),
        Box::new(toggle),
        Box::new(PredefinedMenuItem::separator(app)?),
    ];

    for profile in state.profiles.iter().take(8) {
        let checked = active && state.active_profile_id.as_deref() == Some(profile.id.as_str());
        let marker = if checked { "●" } else { "○" };
        let label = format!("{marker} {}  {}", profile.name, profile.server());
        let item = MenuItem::with_id(
            app,
            format!("profile:{}", profile.id),
            label,
            true,
            None::<&str>,
        )?;
        items.push(Box::new(item));
    }
    if state.profiles.len() > 8 {
        let more = MenuItem::with_id(app, "more", "更多配置…", true, None::<&str>)?;
        items.push(Box::new(more));
    }

    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(check));
    items.push(Box::new(settings));
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(quit));

    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|item| item.as_ref()).collect();
    Menu::with_items(app, &refs)
}

pub fn create(app: &AppHandle, state: &AppState) -> tauri::Result<()> {
    let menu = build_menu(app, state)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
        .expect("embedded tray icon must be a valid PNG");
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip(tooltip(state))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    let toggle_on_click = app
                        .state::<AppCtx>()
                        .actor
                        .get_state()
                        .map(|state| state.settings.tray_click_action == "toggle")
                        .unwrap_or(false);
                    if toggle_on_click {
                        let actor = app.state::<AppCtx>().actor.clone();
                        std::thread::spawn(move || {
                            let _ = actor.toggle(false);
                        });
                    } else {
                        let _ = window::open(app, "panel");
                    }
                }
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => {
                    let _ = window::open(app, "panel");
                }
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

pub fn refresh(app: &AppHandle, state: &AppState) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    if let Ok(menu) = build_menu(app, state) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = tray.set_tooltip(Some(tooltip(state)));
}

fn handle_menu(app: &AppHandle, id: &str) {
    let actor = app.state::<AppCtx>().actor.clone();
    match id {
        "toggle" => {
            std::thread::spawn(move || {
                let _ = actor.toggle(false);
            });
        }
        "check" => {
            std::thread::spawn(move || {
                let _ = actor.check_health(true);
            });
        }
        "settings" => {
            let _ = window::open(app, "settings");
        }
        "more" => {
            let _ = window::open(app, "settings");
        }
        "quit" => quit(app),
        _ => {
            if let Some(profile_id) = id.strip_prefix("profile:") {
                let profile_id = profile_id.to_string();
                std::thread::spawn(move || {
                    let _ = actor.switch_profile(profile_id);
                });
            }
        }
    }
}

pub fn quit(app: &AppHandle) {
    if let Some(ctx) = app.try_state::<AppCtx>() {
        let _ = ctx.actor.shutdown();
        if let Ok(mut watcher) = ctx.watcher.lock() {
            if let Some(watcher) = watcher.as_mut() {
                watcher.stop();
            }
        }
    }
    app.exit(0);
}
