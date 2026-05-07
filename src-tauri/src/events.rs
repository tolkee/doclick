use serde::Serialize;

use crate::state::{BroadcastReason, WindowEntry};

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
