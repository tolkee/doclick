use serde::Serialize;
use tauri::Emitter;

use crate::state::{BroadcastReason, WindowEntry};

/// Emit an event and log a warning on failure.
///
/// Tauri's `emit` returns `Result`, but every emission failure in this app
/// is informational (a watcher updating the UI, a status change). We never
/// want a failed emit to crash a worker thread, but we also never want it
/// to disappear silently — silent emit failures hide IPC channel breakage
/// during reload, window teardown, and similar edge cases.
pub fn emit_or_log<P: Serialize + Clone>(app: &tauri::AppHandle, event: &str, payload: P) {
    if let Err(e) = app.emit(event, payload) {
        tracing::warn!(?e, event, "failed to emit event");
    }
}

pub const EVT_WINDOWS_CHANGED: &str = "windows-changed";
pub const EVT_BROADCAST_STATE: &str = "broadcast-state-changed";
pub const EVT_BROADCAST_TICK: &str = "broadcast-tick";
pub const EVT_ERROR: &str = "error";
pub const EVT_OPEN_SETTINGS: &str = "open-settings";
pub const EVT_PREFS_CHANGED: &str = "prefs-changed";
pub const EVT_FOCUSED_WINDOW_CHANGED: &str = "focused-window-changed";

#[derive(Debug, Clone, Serialize)]
pub struct WindowsChangedPayload {
    pub windows: Vec<WindowEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FocusedWindowChangedPayload {
    /// HWND of the foreground window if it matches a tracked Dofus window,
    /// otherwise `None`.
    pub focused_hwnd: Option<isize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BroadcastStatePayload {
    pub enabled: bool,
    pub reason: BroadcastReason,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum BroadcastTickPayload {
    Started { followers: usize },
    Finished { ok: usize, failed: usize },
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub message: String,
    pub context: Option<String>,
}
