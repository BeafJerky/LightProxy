use crate::app_ctx::AppCtx;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

const PANEL: (f64, f64) = (360.0, 560.0);
const SETTINGS: (f64, f64) = (780.0, 640.0);
const BLUR_SETTLE: Duration = Duration::from_millis(300);

fn size_for(view: &str) -> (f64, f64) {
    if view == "settings" {
        SETTINGS
    } else {
        PANEL
    }
}

pub fn open(app: &AppHandle, view: &str) -> tauri::Result<()> {
    let view = if view == "settings" {
        "settings"
    } else {
        "panel"
    };
    if let Some(ctx) = app.try_state::<AppCtx>() {
        ctx.set_current_view(view);
        ctx.cancel_autohide();
    }
    if let Some(window) = app.get_webview_window("main") {
        window.set_size(tauri::LogicalSize::new(size_for(view).0, size_for(view).1))?;
        let _ = window.emit("navigate", view);
        reveal(&window)?;
        return Ok(());
    }

    let (width, height) = size_for(view);
    let first_load = Arc::new(AtomicBool::new(false));
    let first_load_flag = first_load.clone();
    let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("LightProxy")
        .inner_size(width, height)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .transparent(true)
        .visible(false)
        .shadow(true)
        .on_page_load(move |window, payload| {
            // 首帧就绪后再显示，避免先出现一个透明空窗。
            if payload.event() == PageLoadEvent::Finished
                && !first_load_flag.swap(true, Ordering::SeqCst)
            {
                let _ = reveal(&window);
            }
        })
        .build()?;
    attach(&window)?;
    Ok(())
}

pub fn set_view(app: &AppHandle, view: &str) -> tauri::Result<()> {
    let view = if view == "settings" {
        "settings"
    } else {
        "panel"
    };
    if let Some(ctx) = app.try_state::<AppCtx>() {
        ctx.set_current_view(view);
        ctx.cancel_autohide();
    }
    if let Some(window) = app.get_webview_window("main") {
        window.set_size(tauri::LogicalSize::new(size_for(view).0, size_for(view).1))?;
        let _ = window.emit("navigate", view);
        reveal(&window)?;
        Ok(())
    } else {
        open(app, view)
    }
}

pub fn hide(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        if let Some(ctx) = app.try_state::<AppCtx>() {
            ctx.reveal.reset();
        }
        window.hide()?;
        schedule_destroy(app);
    }
    Ok(())
}

fn reveal(window: &WebviewWindow) -> tauri::Result<()> {
    position(window)?;
    if let Some(ctx) = window.app_handle().try_state::<AppCtx>() {
        ctx.reveal.mark_revealed();
    }
    window.show()?;
    window.set_focus()?;
    Ok(())
}

pub fn schedule_destroy(app: &AppHandle) {
    let Some(ctx) = app.try_state::<AppCtx>() else {
        return;
    };
    let generation = ctx.cancel_autohide();
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(300));
        let Some(ctx) = handle.try_state::<AppCtx>() else {
            return;
        };
        if ctx.autohide_generation.load(Ordering::SeqCst) != generation {
            return;
        }
        if let Some(window) = handle.get_webview_window("main") {
            if !window.is_visible().unwrap_or(false) {
                let _ = window.destroy();
            }
        }
    });
}

fn attach(window: &WebviewWindow) -> tauri::Result<()> {
    let handle = window.app_handle().clone();
    let target = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(focused) => {
            if *focused {
                return;
            }
            let Some(ctx) = handle.try_state::<AppCtx>() else {
                return;
            };
            // 面板显示瞬间的失焦可能来自托盘弹出层的焦点回弹，不能当作点击外部。
            let hide = ctx.current_view() == "panel" && ctx.reveal.should_hide_on_blur(BLUR_SETTLE);
            if hide {
                ctx.reveal.reset();
                let _ = target.hide();
                schedule_destroy(&handle);
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let Some(ctx) = handle.try_state::<AppCtx>() else {
                return;
            };
            if ctx.current_view() == "settings" {
                let exit_on_close = ctx
                    .actor
                    .get_state()
                    .map(|state| state.settings.close_action == "exit")
                    .unwrap_or(false);
                if exit_on_close {
                    crate::tray::quit(&handle);
                    return;
                }
            }
            ctx.reveal.reset();
            let _ = target.hide();
            schedule_destroy(&handle);
        }
        _ => {}
    });
    Ok(())
}

fn position(window: &WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let size = window.outer_size()?;
    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();
    let margin = (16.0 * scale) as i32;
    let taskbar = (56.0 * scale) as i32;
    let x = monitor_pos.x + monitor_size.width as i32 - size.width as i32 - margin;
    let y = monitor_pos.y + monitor_size.height as i32 - size.height as i32 - taskbar;
    window.set_position(PhysicalPosition::new(
        x.max(monitor_pos.x),
        y.max(monitor_pos.y),
    ))?;
    Ok(())
}
