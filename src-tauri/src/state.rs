use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Receives broadcasts (the default for any detected Dofus window).
    /// `main` from older profile files is silently coerced to this.
    #[serde(alias = "main")]
    Follower,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum MatchStrategy {
    /// Match by a substring of the window title (typically the character name).
    WindowTitleContains(String),
    /// Match by PID (only stable within a session).
    Pid(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterProfile {
    pub id: String,
    pub display_name: String,
    pub role: Role,
    pub match_strategy: MatchStrategy,
    /// Class slug captured at import time so the avatar still renders when no
    /// live Dofus window matches the profile.
    #[serde(default)]
    pub dofus_class: Option<String>,
}

impl CharacterProfile {
    pub fn matches_window(&self, title: &str, pid: u32) -> bool {
        match &self.match_strategy {
            MatchStrategy::WindowTitleContains(needle) => {
                !needle.is_empty() && title.contains(needle.as_str())
            }
            MatchStrategy::Pid(p) => *p == pid,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LiveWindow {
    pub hwnd: isize,
    pub pid: u32,
    pub title: String,
    pub class_name: String,
    /// Dofus class slug parsed from the title (e.g. "iop", "cra"), if recognisable.
    pub dofus_class: Option<String>,
    /// Character name parsed from the title (first " - " segment).
    pub character_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowEntry {
    pub hwnd: isize,
    pub pid: u32,
    pub title: String,
    pub class_name: String,
    pub dofus_class: Option<String>,
    pub character_name: Option<String>,
    pub profile: Option<CharacterProfile>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BroadcastReason {
    User,
    AutoDisabledForegroundMismatch,
    PanicHotkey,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::Horizontal
    }
}

/// Persisted user-resized overlay dimensions, per orientation. Stored in
/// logical pixels so the size is DPI-independent across reboots / monitor
/// swaps. Locked-axis values are still persisted as-is — the inner bar
/// pins itself anyway.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlaySizes {
    pub horizontal: Option<(u32, u32)>,
    pub vertical: Option<(u32, u32)>,
}

/// Configurable global-shortcut bindings. Each value is an accelerator string
/// (e.g. "Ctrl+Alt+B") or `None` if unbound. Defaults to all-`None` so the user
/// opts in explicitly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutBindings {
    pub toggle_broadcast: Option<String>,
    pub open_settings: Option<String>,
    pub close_app: Option<String>,
    /// 1-based slots for "switch to char N". Index 0 = char 1.
    pub focus_char: Vec<Option<String>>,
    pub focus_next: Option<String>,
    pub focus_prev: Option<String>,
    pub focus_main: Option<String>,
}

impl ShortcutBindings {
    pub fn ensure_focus_char_slots(&mut self) {
        while self.focus_char.len() < 8 {
            self.focus_char.push(None);
        }
    }
}

#[derive(Debug)]
pub struct InnerState {
    pub profiles: Vec<CharacterProfile>,
    pub live_windows: Vec<LiveWindow>,
    pub broadcast_enabled: bool,
    pub broadcast_keys: Vec<u32>,        // virtual-key codes whitelisted for relay
    pub panic_hotkey: String,            // accelerator string, e.g. "Ctrl+Shift+F12"
    pub pvp_warning_acknowledged: bool,
    pub overlay_position: Option<(i32, i32)>,
    pub overlay_sizes: OverlaySizes,
    pub main_character_id: Option<String>,
    pub profile_order: Vec<String>,
    pub orientation: Orientation,
    pub shortcuts: ShortcutBindings,
}

impl Default for InnerState {
    fn default() -> Self {
        let mut shortcuts = ShortcutBindings::default();
        shortcuts.ensure_focus_char_slots();
        Self {
            profiles: Vec::new(),
            live_windows: Vec::new(),
            broadcast_enabled: false,
            broadcast_keys: default_broadcast_keys(),
            panic_hotkey: "Ctrl+Shift+F12".into(),
            pvp_warning_acknowledged: false,
            overlay_position: None,
            overlay_sizes: OverlaySizes::default(),
            main_character_id: None,
            profile_order: Vec::new(),
            orientation: Orientation::default(),
            shortcuts,
        }
    }
}

pub fn default_broadcast_keys() -> Vec<u32> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    vec![
        VK_SPACE.0 as u32,
        VK_RETURN.0 as u32,
        VK_ESCAPE.0 as u32,
        VK_0.0 as u32,
        VK_1.0 as u32,
        VK_2.0 as u32,
        VK_3.0 as u32,
        VK_4.0 as u32,
        VK_5.0 as u32,
        VK_6.0 as u32,
        VK_7.0 as u32,
        VK_8.0 as u32,
        VK_9.0 as u32,
    ]
}

#[derive(Debug, Clone)]
pub struct AppState(Arc<RwLock<InnerState>>);

impl Default for AppState {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(InnerState::default())))
    }
}

impl AppState {
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, InnerState> {
        self.0.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, InnerState> {
        self.0.write()
    }

    /// Resolve every live window against profiles for the React store.
    pub fn snapshot_windows(&self) -> Vec<WindowEntry> {
        let inner = self.0.read();
        inner
            .live_windows
            .iter()
            .map(|w| WindowEntry {
                hwnd: w.hwnd,
                pid: w.pid,
                title: w.title.clone(),
                class_name: w.class_name.clone(),
                dofus_class: w.dofus_class.clone(),
                character_name: w.character_name.clone(),
                profile: inner
                    .profiles
                    .iter()
                    .find(|p| p.matches_window(&w.title, w.pid))
                    .cloned(),
            })
            .collect()
    }

    /// Returns the HWNDs that should receive a broadcast click whose source
    /// (the focused window when the user clicked) is `source_hwnd`.
    ///
    /// Every tracked Dofus window other than `source_hwnd` counts as a
    /// target as long as it matches an imported profile.
    pub fn broadcast_targets(&self, source_hwnd: isize) -> Vec<isize> {
        let inner = self.0.read();
        inner
            .live_windows
            .iter()
            .filter(|w| w.hwnd != source_hwnd)
            .filter(|w| {
                inner
                    .profiles
                    .iter()
                    .any(|p| p.matches_window(&w.title, w.pid))
            })
            .map(|w| w.hwnd)
            .collect()
    }

    /// All Dofus HWNDs we know about that are linked to an imported profile.
    pub fn all_hwnds(&self) -> Vec<isize> {
        let inner = self.0.read();
        inner
            .live_windows
            .iter()
            .filter(|w| {
                inner
                    .profiles
                    .iter()
                    .any(|p| p.matches_window(&w.title, w.pid))
            })
            .map(|w| w.hwnd)
            .collect()
    }

    /// Visible windows ordered by `profile_order`. Only windows linked to an
    /// imported profile are included.
    pub fn ordered_visible_hwnds(&self) -> Vec<isize> {
        let inner = self.0.read();
        let mut items: Vec<(isize, Option<usize>, String)> = inner
            .live_windows
            .iter()
            .filter_map(|w| {
                let profile = inner
                    .profiles
                    .iter()
                    .find(|p| p.matches_window(&w.title, w.pid))?;
                let idx = inner.profile_order.iter().position(|id| id == &profile.id);
                Some((w.hwnd, idx, w.title.clone()))
            })
            .collect();
        items.sort_by(|a, b| match (a.1, b.1) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.2.cmp(&b.2),
        });
        items.into_iter().map(|(h, _, _)| h).collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StateSnapshot {
    pub windows: Vec<WindowEntry>,
    pub profiles: Vec<CharacterProfile>,
    pub broadcast_enabled: bool,
    pub broadcast_keys: Vec<u32>,
    pub panic_hotkey: String,
    pub pvp_warning_acknowledged: bool,
    pub main_character_id: Option<String>,
    pub profile_order: Vec<String>,
    pub orientation: Orientation,
    pub overlay_sizes: OverlaySizes,
    pub shortcuts: ShortcutBindings,
}
