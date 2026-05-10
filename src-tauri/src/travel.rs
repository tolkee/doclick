//! Reads `/travel X,Y` from the clipboard and submits it to the main
//! character's Dofus window via Space → Ctrl+V → Enter.

use std::time::Duration;

use tauri::{AppHandle, Emitter, State};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_RETURN, VK_SPACE, VK_V};

use crate::broadcast::dispatcher::{send_key, send_key_combo};
use crate::events::{BroadcastStatePayload, EVT_BROADCAST_STATE};
use crate::state::{AppState, BroadcastReason};
use crate::windows::focus::focus_window;

const FOCUS_WAIT: Duration = Duration::from_millis(120);

/// Wait for Dofus to actually focus the chat input after Space is pressed.
/// If we paste before the chat is open, the command lands in the game world
/// (e.g. as a movement input) instead of in chat. 80ms covers Unity's input
/// pipeline at 30 FPS with margin.
const CHAT_OPEN_DELAY: Duration = Duration::from_millis(80);

/// Settle gap between Ctrl+V and Enter so the pasted text is committed to
/// the chat input before we submit. Without it, Enter occasionally fires on
/// an empty input.
const POST_PASTE_DELAY: Duration = Duration::from_millis(30);

pub fn run_travel_from_clipboard(app: &AppHandle, state: State<'_, AppState>) {
    let Some(_command) = read_validated_travel_command() else {
        tracing::debug!("travel: clipboard does not contain a valid /travel command");
        return;
    };
    let Some(hwnd) = state.main_character_hwnd() else {
        tracing::debug!("travel: no main character set, or its window is not live");
        return;
    };

    // Disable broadcast before sending keys, otherwise our Space/Enter would
    // be replayed to followers and they'd all open chat. The user re-enables
    // manually after the trip — auto-restore would surprise the user mid-action.
    disable_broadcast_if_on(app, &state);

    if !focus_window(hwnd, FOCUS_WAIT) {
        tracing::warn!(
            target = format!("{hwnd:#x}"),
            "travel: could not focus main character window"
        );
        return;
    }

    if !send_key(VK_SPACE.0 as u32) {
        tracing::warn!("travel: SendInput rejected Space");
        return;
    }
    std::thread::sleep(CHAT_OPEN_DELAY);

    if !send_key_combo(VK_CONTROL.0 as u32, VK_V.0 as u32) {
        tracing::warn!("travel: SendInput rejected Ctrl+V");
        return;
    }
    std::thread::sleep(POST_PASTE_DELAY);

    if !send_key(VK_RETURN.0 as u32) {
        tracing::warn!("travel: SendInput rejected Enter");
    }
}

fn disable_broadcast_if_on(app: &AppHandle, state: &State<'_, AppState>) {
    let was_on = {
        let mut inner = state.write();
        let prev = inner.broadcast_enabled;
        inner.broadcast_enabled = false;
        prev
    };
    if was_on {
        let _ = app.emit(
            EVT_BROADCAST_STATE,
            BroadcastStatePayload {
                enabled: false,
                reason: BroadcastReason::User,
            },
        );
    }
}

fn read_validated_travel_command() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let raw = clipboard.get_text().ok()?;
    let s = raw.trim();
    let rest = s.strip_prefix("/travel ")?;
    // No `.trim()` on the halves — `i32::from_str` rejects whitespace, so
    // `/travel 4, -19` and `/travel  4,-19` both fail to parse here. Strict
    // whole-match per the feature spec.
    let (x, y) = rest.split_once(',')?;
    x.parse::<i32>().ok()?;
    y.parse::<i32>().ok()?;
    Some(s.to_owned())
}
