use crate::app_ctx::{AppCtx, HotkeyAction};
use crate::window;
use lightproxy_core::Hotkeys;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

pub fn apply(app: &AppHandle, ctx: &AppCtx, hotkeys: &Hotkeys) -> Vec<String> {
    let _ = app.global_shortcut().unregister_all();
    let mut map = Vec::new();
    let mut failures = Vec::new();
    let bindings = [
        (&hotkeys.toggle, HotkeyAction::Toggle),
        (&hotkeys.panel, HotkeyAction::Panel),
        (&hotkeys.next_profile, HotkeyAction::NextProfile),
    ];
    for (spec, action) in bindings {
        if spec.trim().is_empty() {
            continue;
        }
        match parse(spec) {
            Some(shortcut) => match app.global_shortcut().register(shortcut) {
                Ok(()) => map.push((shortcut, action)),
                Err(err) => failures.push(format!("{spec}：{err}")),
            },
            None => failures.push(format!("{spec}：组合无法识别")),
        }
    }
    if let Ok(mut existing) = ctx.hotkey_map.lock() {
        *existing = map;
    }
    failures
}

pub fn resolve(app: &AppHandle, shortcut: &Shortcut) -> Option<HotkeyAction> {
    let ctx = app.state::<AppCtx>();
    {
        let mut last = ctx.last_hotkey.lock().ok()?;
        if let Some(at) = *last {
            if at.elapsed() < Duration::from_millis(200) {
                return None;
            }
        }
        *last = Some(Instant::now());
    }
    let map = ctx.hotkey_map.lock().ok()?;
    let action = map
        .iter()
        .find(|(candidate, _)| candidate == shortcut)
        .map(|(_, action)| *action);
    action
}

pub fn run(app: &AppHandle, action: HotkeyAction) {
    let actor = app.state::<AppCtx>().actor.clone();
    match action {
        HotkeyAction::Toggle => {
            std::thread::spawn(move || {
                let _ = actor.toggle(false);
            });
        }
        HotkeyAction::Panel => {
            let _ = window::open(app, "panel");
        }
        HotkeyAction::NextProfile => {
            let Ok(state) = actor.get_state() else {
                return;
            };
            if state.profiles.is_empty() {
                return;
            }
            let current = state
                .active_profile_id
                .as_deref()
                .or(state.default_profile_id.as_deref());
            let index = current
                .and_then(|id| state.profiles.iter().position(|profile| profile.id == id))
                .unwrap_or(0);
            let next = state.profiles[(index + 1) % state.profiles.len()]
                .id
                .clone();
            std::thread::spawn(move || {
                let _ = actor.switch_profile(next, false);
            });
        }
    }
}

pub fn parse(spec: &str) -> Option<Shortcut> {
    let mut modifiers = Modifiers::empty();
    let mut code = None;
    for part in spec
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "win" | "meta" | "super" => modifiers |= Modifiers::SUPER,
            key => code = key_code(key),
        }
    }
    let code = code?;
    let modifiers = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };
    Some(Shortcut::new(modifiers, code))
}

fn key_code(key: &str) -> Option<Code> {
    let code = match key {
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        "space" => Code::Space,
        "enter" => Code::Enter,
        "escape" | "esc" => Code::Escape,
        "tab" => Code::Tab,
        "backspace" => Code::Backspace,
        "delete" => Code::Delete,
        "home" => Code::Home,
        "end" => Code::End,
        "pageup" => Code::PageUp,
        "pagedown" => Code::PageDown,
        "arrowup" | "up" => Code::ArrowUp,
        "arrowdown" | "down" => Code::ArrowDown,
        "arrowleft" | "left" => Code::ArrowLeft,
        "arrowright" | "right" => Code::ArrowRight,
        _ => return None,
    };
    Some(code)
}
