// Mutex<HashMap> locks below are held only across HashMap ops (clear, insert,
// get, contains_key) which cannot panic, so the locks cannot be poisoned.
// `.unwrap()` on `.lock()` is therefore infallible — module-level allow keeps
// the call sites readable.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::commands;
use crate::events::{BroadcastStatePayload, EVT_BROADCAST_STATE};
use crate::state::{AppState, BroadcastReason, ShortcutBindings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutAction {
    PanicHotkey,
    ToggleBroadcast,
    OpenSettings,
    CloseApp,
    CloseAll,
    FocusChar(usize),
    FocusNext,
    FocusPrev,
    FocusMain,
    SendTravelCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseTrigger {
    Mouse3,
    Mouse4,
    Mouse5,
    WheelUp,
    WheelDown,
}

pub const MOD_CTRL: u8 = 1 << 0;
pub const MOD_SHIFT: u8 = 1 << 1;
pub const MOD_ALT: u8 = 1 << 2;
pub const MOD_META: u8 = 1 << 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseShortcut {
    pub mods: u8,
    pub trigger: MouseTrigger,
}

static REGISTERED: Lazy<Mutex<HashMap<Shortcut, ShortcutAction>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static MOUSE_REGISTERED: Lazy<Mutex<HashMap<MouseShortcut, ShortcutAction>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Re-register every configured shortcut. Idempotent: unregisters the previous
/// set first so the registry stays clean across config changes. Keyboard
/// accelerators go through the global_shortcut plugin; mouse-button and wheel
/// triggers live in our own map and are dispatched from the low-level mouse
/// hook.
pub fn reregister_all(app: &AppHandle, state: &AppState) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    REGISTERED.lock().unwrap().clear();
    MOUSE_REGISTERED.lock().unwrap().clear();

    let (panic_accel, bindings) = {
        let inner = state.read();
        (inner.panic_hotkey.clone(), inner.shortcuts.clone())
    };

    let mut new_kbd: HashMap<Shortcut, ShortcutAction> = HashMap::new();
    let mut new_mouse: HashMap<MouseShortcut, ShortcutAction> = HashMap::new();
    register_one(
        app,
        &panic_accel,
        ShortcutAction::PanicHotkey,
        &mut new_kbd,
        &mut new_mouse,
    );
    register_bindings(app, &bindings, &mut new_kbd, &mut new_mouse);

    *REGISTERED.lock().unwrap() = new_kbd;
    *MOUSE_REGISTERED.lock().unwrap() = new_mouse;
}

fn register_bindings(
    app: &AppHandle,
    b: &ShortcutBindings,
    kbd: &mut HashMap<Shortcut, ShortcutAction>,
    mouse: &mut HashMap<MouseShortcut, ShortcutAction>,
) {
    if let Some(a) = b.toggle_broadcast.as_deref() {
        register_one(app, a, ShortcutAction::ToggleBroadcast, kbd, mouse);
    }
    if let Some(a) = b.open_settings.as_deref() {
        register_one(app, a, ShortcutAction::OpenSettings, kbd, mouse);
    }
    if let Some(a) = b.close_app.as_deref() {
        register_one(app, a, ShortcutAction::CloseApp, kbd, mouse);
    }
    if let Some(a) = b.close_all.as_deref() {
        register_one(app, a, ShortcutAction::CloseAll, kbd, mouse);
    }
    for (i, slot) in b.focus_char.iter().enumerate() {
        if let Some(a) = slot.as_deref() {
            register_one(app, a, ShortcutAction::FocusChar(i), kbd, mouse);
        }
    }
    if let Some(a) = b.focus_next.as_deref() {
        register_one(app, a, ShortcutAction::FocusNext, kbd, mouse);
    }
    if let Some(a) = b.focus_prev.as_deref() {
        register_one(app, a, ShortcutAction::FocusPrev, kbd, mouse);
    }
    if let Some(a) = b.focus_main.as_deref() {
        register_one(app, a, ShortcutAction::FocusMain, kbd, mouse);
    }
    if let Some(a) = b.send_travel_command.as_deref() {
        register_one(app, a, ShortcutAction::SendTravelCommand, kbd, mouse);
    }
}

fn register_one(
    app: &AppHandle,
    accel: &str,
    action: ShortcutAction,
    kbd: &mut HashMap<Shortcut, ShortcutAction>,
    mouse: &mut HashMap<MouseShortcut, ShortcutAction>,
) {
    if accel.is_empty() {
        return;
    }
    let parsed = match parse_shortcut(accel) {
        Some(p) => p,
        None => {
            tracing::warn!(accel, ?action, "invalid shortcut, skipping");
            return;
        }
    };
    match parsed {
        ParsedShortcut::Keyboard(s) => {
            if kbd.contains_key(&s) {
                tracing::warn!(
                    accel,
                    ?action,
                    "duplicate keyboard shortcut, last write wins"
                );
            }
            if let Err(err) = app.global_shortcut().register(s) {
                tracing::warn!(?err, accel, ?action, "failed to register keyboard shortcut");
                return;
            }
            kbd.insert(s, action);
        }
        ParsedShortcut::Mouse(m) => {
            if mouse.contains_key(&m) {
                tracing::warn!(accel, ?action, "duplicate mouse shortcut, last write wins");
            }
            mouse.insert(m, action);
        }
    }
}

/// Look up a mouse-trigger binding. Called from the low-level mouse hook.
pub fn lookup_mouse(s: MouseShortcut) -> Option<ShortcutAction> {
    MOUSE_REGISTERED.lock().unwrap().get(&s).copied()
}

pub fn dispatch(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let action = {
        let map = REGISTERED.lock().unwrap();
        map.get(shortcut).copied()
    };
    let Some(action) = action else { return };
    if !should_run(app, action) {
        return;
    }
    run_action(app, action);
}

/// Returns whether `action` should fire given the current foreground window.
///
/// Gates everything except the escape-hatch actions (panic, open-settings,
/// close-app) so the user can always recover from a stuck state. The action
/// fires only when a tracked Dofus window or one of our own app windows is
/// in the foreground.
pub fn should_run(app: &AppHandle, action: ShortcutAction) -> bool {
    match action {
        ShortcutAction::PanicHotkey
        | ShortcutAction::OpenSettings
        | ShortcutAction::CloseApp
        | ShortcutAction::CloseAll => return true,
        _ => {}
    }
    let state = match app.try_state::<AppState>() {
        Some(s) => s,
        None => return true,
    };
    let fg = crate::windows::focus::current_foreground();
    if state.all_hwnds().contains(&fg) {
        return true;
    }
    if crate::app_hwnds(app).contains(&fg) {
        return true;
    }
    crate::windows::focus::is_companion_window(fg)
}

/// Execute the side-effect for `action`. Used by both the keyboard plugin
/// dispatcher and the low-level mouse hook.
pub fn run_action(app: &AppHandle, action: ShortcutAction) {
    let state = match app.try_state::<AppState>() {
        Some(s) => s,
        None => return,
    };
    let app_handle = app.clone();
    match action {
        ShortcutAction::PanicHotkey => {
            state.write().broadcast_enabled = false;
            let _ = app_handle.emit(
                EVT_BROADCAST_STATE,
                BroadcastStatePayload {
                    enabled: false,
                    reason: BroadcastReason::PanicHotkey,
                },
            );
        }
        ShortcutAction::ToggleBroadcast => {
            let new_enabled = {
                let mut inner = state.write();
                inner.broadcast_enabled = !inner.broadcast_enabled;
                inner.broadcast_enabled
            };
            let _ = app_handle.emit(
                EVT_BROADCAST_STATE,
                BroadcastStatePayload {
                    enabled: new_enabled,
                    reason: BroadcastReason::User,
                },
            );
        }
        ShortcutAction::OpenSettings => {
            commands::show_settings_window(&app_handle, None);
        }
        ShortcutAction::CloseApp => {
            let _ = commands::persist(&app_handle, state.inner());
            app_handle.exit(0);
        }
        ShortcutAction::CloseAll => {
            crate::windows::close::close_external_app_windows();
            let _ = commands::persist(&app_handle, state.inner());
            // Hard-exit: `app_handle.exit(0)` from the global-hotkey thread
            // was observed to leave Doclick running after the other windows
            // closed. Persistence already ran, so skipping destructors is safe.
            std::process::exit(0);
        }
        ShortcutAction::FocusChar(i) => {
            let _ = commands::focus_character_at_index(state, i);
        }
        ShortcutAction::FocusNext => {
            let _ = commands::focus_next_character(state);
        }
        ShortcutAction::FocusPrev => {
            let _ = commands::focus_prev_character(state);
        }
        ShortcutAction::FocusMain => {
            let _ = commands::focus_main_character(state);
        }
        ShortcutAction::SendTravelCommand => {
            crate::travel::run_travel_from_clipboard(&app_handle, state);
        }
    }
}

pub enum ParsedShortcut {
    Keyboard(Shortcut),
    Mouse(MouseShortcut),
}

pub fn parse_shortcut(s: &str) -> Option<ParsedShortcut> {
    let mut mods_kbd = Modifiers::empty();
    let mut mods_mouse: u8 = 0;
    let mut key: Option<Code> = None;
    let mut mouse_trigger: Option<MouseTrigger> = None;

    for tok in s.split('+').map(str::trim) {
        match tok.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => {
                mods_kbd |= Modifiers::CONTROL;
                mods_mouse |= MOD_CTRL;
            }
            "shift" => {
                mods_kbd |= Modifiers::SHIFT;
                mods_mouse |= MOD_SHIFT;
            }
            "alt" => {
                mods_kbd |= Modifiers::ALT;
                mods_mouse |= MOD_ALT;
            }
            "meta" | "win" | "super" => {
                mods_kbd |= Modifiers::SUPER;
                mods_mouse |= MOD_META;
            }
            other => {
                if let Some(t) = mouse_trigger_from_str(other) {
                    mouse_trigger = Some(t);
                } else if let Some(c) = code_from_str(other) {
                    key = Some(c);
                }
            }
        }
    }

    if let Some(t) = mouse_trigger {
        return Some(ParsedShortcut::Mouse(MouseShortcut {
            mods: mods_mouse,
            trigger: t,
        }));
    }
    let key = key?;
    Some(ParsedShortcut::Keyboard(Shortcut::new(Some(mods_kbd), key)))
}

fn mouse_trigger_from_str(s: &str) -> Option<MouseTrigger> {
    Some(match s.to_ascii_lowercase().as_str() {
        "mouse3" | "mmb" | "middle" => MouseTrigger::Mouse3,
        "mouse4" | "xbutton1" | "back" => MouseTrigger::Mouse4,
        "mouse5" | "xbutton2" | "forward" => MouseTrigger::Mouse5,
        "wheelup" | "scrollup" => MouseTrigger::WheelUp,
        "wheeldown" | "scrolldown" => MouseTrigger::WheelDown,
        _ => return None,
    })
}

fn code_from_str(s: &str) -> Option<Code> {
    Some(match s.to_ascii_uppercase().as_str() {
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "ESC" | "ESCAPE" => Code::Escape,
        "SPACE" => Code::Space,
        "ENTER" | "RETURN" => Code::Enter,
        "TAB" => Code::Tab,
        "BACKSPACE" => Code::Backspace,
        "DELETE" | "DEL" => Code::Delete,
        "LEFT" => Code::ArrowLeft,
        "RIGHT" => Code::ArrowRight,
        "UP" => Code::ArrowUp,
        "DOWN" => Code::ArrowDown,
        s if s.len() == 1 => {
            let c = s.chars().next()?;
            if c.is_ascii_alphabetic() {
                match c.to_ascii_uppercase() {
                    'A' => Code::KeyA,
                    'B' => Code::KeyB,
                    'C' => Code::KeyC,
                    'D' => Code::KeyD,
                    'E' => Code::KeyE,
                    'F' => Code::KeyF,
                    'G' => Code::KeyG,
                    'H' => Code::KeyH,
                    'I' => Code::KeyI,
                    'J' => Code::KeyJ,
                    'K' => Code::KeyK,
                    'L' => Code::KeyL,
                    'M' => Code::KeyM,
                    'N' => Code::KeyN,
                    'O' => Code::KeyO,
                    'P' => Code::KeyP,
                    'Q' => Code::KeyQ,
                    'R' => Code::KeyR,
                    'S' => Code::KeyS,
                    'T' => Code::KeyT,
                    'U' => Code::KeyU,
                    'V' => Code::KeyV,
                    'W' => Code::KeyW,
                    'X' => Code::KeyX,
                    'Y' => Code::KeyY,
                    'Z' => Code::KeyZ,
                    _ => return None,
                }
            } else if c.is_ascii_digit() {
                match c {
                    '0' => Code::Digit0,
                    '1' => Code::Digit1,
                    '2' => Code::Digit2,
                    '3' => Code::Digit3,
                    '4' => Code::Digit4,
                    '5' => Code::Digit5,
                    '6' => Code::Digit6,
                    '7' => Code::Digit7,
                    '8' => Code::Digit8,
                    '9' => Code::Digit9,
                    _ => return None,
                }
            } else {
                return None;
            }
        }
        _ => return None,
    })
}
