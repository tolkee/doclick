use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindow, ShowWindow, SW_RESTORE};

use crate::config::{self, PersistedConfig};
use crate::events::{
    emit_or_log, BroadcastStatePayload, WindowsChangedPayload, EVT_BROADCAST_STATE,
    EVT_PREFS_CHANGED, EVT_WINDOWS_CHANGED,
};
use crate::state::{
    AppState, BroadcastReason, CharacterProfile, Orientation, ShortcutBindings, StateSnapshot,
    WindowEntry,
};
use crate::windows::focus::focus_window;

#[derive(Debug, Serialize, thiserror::Error)]
pub enum CmdError {
    #[error("io: {0}")]
    Io(String),
    #[error("invalid: {0}")]
    Invalid(String),
}

impl From<std::io::Error> for CmdError {
    fn from(e: std::io::Error) -> Self {
        CmdError::Io(e.to_string())
    }
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, CmdError> {
    app.path()
        .app_data_dir()
        .map_err(|e| CmdError::Io(e.to_string()))
}

pub fn persist(app: &AppHandle, state: &AppState) -> Result<(), CmdError> {
    let dir = app_data_dir(app)?;
    let inner = state.read();
    let cfg = PersistedConfig {
        profiles: inner.profiles.clone(),
        broadcast_keys: inner.broadcast_keys.clone(),
        panic_hotkey: inner.panic_hotkey.clone(),
        pvp_warning_acknowledged: inner.pvp_warning_acknowledged,
        overlay_position: inner.overlay_position,
        overlay_sizes: inner.overlay_sizes,
        settings_size: inner.settings_size,
        main_character_id: inner.main_character_id.clone(),
        profile_order: inner.profile_order.clone(),
        orientation: inner.orientation,
        shortcuts: inner.shortcuts.clone(),
    };
    drop(inner);
    config::save(&dir, &cfg)?;
    Ok(())
}

fn emit_windows_changed(app: &AppHandle, state: &AppState) {
    emit_or_log(
        app,
        EVT_WINDOWS_CHANGED,
        WindowsChangedPayload {
            windows: state.snapshot_windows(),
        },
    );
}

fn emit_prefs_changed(app: &AppHandle) {
    emit_or_log(app, EVT_PREFS_CHANGED, ());
}

fn emit_broadcast_state(app: &AppHandle, enabled: bool, reason: BroadcastReason) {
    emit_or_log(
        app,
        EVT_BROADCAST_STATE,
        BroadcastStatePayload { enabled, reason },
    );
}

#[tauri::command]
pub fn list_windows(state: State<'_, AppState>) -> Vec<WindowEntry> {
    state.snapshot_windows()
}

#[tauri::command]
pub fn get_state_snapshot(state: State<'_, AppState>) -> StateSnapshot {
    let inner = state.read();
    StateSnapshot {
        windows: state.snapshot_windows(),
        profiles: inner.profiles.clone(),
        broadcast_enabled: inner.broadcast_enabled,
        broadcast_keys: inner.broadcast_keys.clone(),
        panic_hotkey: inner.panic_hotkey.clone(),
        pvp_warning_acknowledged: inner.pvp_warning_acknowledged,
        main_character_id: inner.main_character_id.clone(),
        profile_order: inner.profile_order.clone(),
        orientation: inner.orientation,
        overlay_sizes: inner.overlay_sizes,
        settings_size: inner.settings_size,
        shortcuts: inner.shortcuts.clone(),
    }
}

#[tauri::command]
pub fn set_broadcast_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), CmdError> {
    state.write().broadcast_enabled = enabled;
    emit_broadcast_state(&app, enabled, BroadcastReason::User);
    Ok(())
}

#[tauri::command]
pub fn set_broadcast_keys(
    app: AppHandle,
    state: State<'_, AppState>,
    keys: Vec<u32>,
) -> Result<(), CmdError> {
    state.write().broadcast_keys = keys;
    persist(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn set_panic_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    accelerator: String,
) -> Result<(), CmdError> {
    state.write().panic_hotkey = accelerator;
    persist(&app, &state)?;
    crate::shortcuts::reregister_all(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn upsert_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: CharacterProfile,
) -> Result<(), CmdError> {
    {
        let mut inner = state.write();
        if let Some(existing) = inner.profiles.iter_mut().find(|p| p.id == profile.id) {
            *existing = profile.clone();
        } else {
            inner.profiles.push(profile.clone());
            if !inner.profile_order.contains(&profile.id) {
                inner.profile_order.push(profile.id.clone());
            }
        }
    }
    persist(&app, &state)?;
    emit_windows_changed(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn delete_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), CmdError> {
    {
        let mut inner = state.write();
        inner.profiles.retain(|p| p.id != id);
        inner.profile_order.retain(|x| x != &id);
        if inner.main_character_id.as_deref() == Some(id.as_str()) {
            inner.main_character_id = None;
        }
    }
    persist(&app, &state)?;
    emit_windows_changed(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn acknowledge_pvp_warning(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CmdError> {
    state.write().pvp_warning_acknowledged = true;
    persist(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn focus_dofus_window(hwnd: isize) -> Result<(), CmdError> {
    let h = HWND(hwnd as *mut _);
    if !unsafe { IsWindow(Some(h)) }.as_bool() {
        return Err(CmdError::Invalid("invalid window handle".into()));
    }
    if unsafe { IsIconic(h) }.as_bool() {
        let _ = unsafe { ShowWindow(h, SW_RESTORE) };
    }
    let _ = focus_window(hwnd, std::time::Duration::from_millis(120));
    Ok(())
}

#[tauri::command]
pub fn save_overlay_position(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
) -> Result<(), CmdError> {
    state.write().overlay_position = Some((x, y));
    persist(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn save_overlay_size(
    app: AppHandle,
    state: State<'_, AppState>,
    orientation: String,
    width: u32,
    height: u32,
) -> Result<(), CmdError> {
    {
        let mut inner = state.write();
        match orientation.as_str() {
            "horizontal" => inner.overlay_sizes.horizontal = Some((width, height)),
            "vertical" => inner.overlay_sizes.vertical = Some((width, height)),
            other => return Err(CmdError::Invalid(format!("orientation={other}"))),
        }
    }
    persist(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn save_settings_size(
    app: AppHandle,
    state: State<'_, AppState>,
    width: u32,
    height: u32,
) -> Result<(), CmdError> {
    state.write().settings_size = Some((width, height));
    persist(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn cycle_focus(state: State<'_, AppState>) -> Result<(), CmdError> {
    let hwnds = state.all_hwnds();
    if hwnds.is_empty() {
        return Ok(());
    }
    let current = crate::windows::focus::current_foreground();
    let next_index = hwnds
        .iter()
        .position(|h| *h == current)
        .map(|i| (i + 1) % hwnds.len())
        .unwrap_or(0);
    let _ = focus_window(hwnds[next_index], std::time::Duration::from_millis(120));
    Ok(())
}

#[tauri::command]
pub fn set_main_character(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Option<String>,
) -> Result<(), CmdError> {
    state.write().main_character_id = id;
    persist(&app, &state)?;
    emit_prefs_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_profile_order(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), CmdError> {
    state.write().profile_order = ids;
    persist(&app, &state)?;
    emit_prefs_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_orientation(
    app: AppHandle,
    state: State<'_, AppState>,
    orientation: String,
) -> Result<(), CmdError> {
    let parsed = match orientation.as_str() {
        "horizontal" => Orientation::Horizontal,
        "vertical" => Orientation::Vertical,
        other => return Err(CmdError::Invalid(format!("orientation={other}"))),
    };
    state.write().orientation = parsed;
    persist(&app, &state)?;
    emit_prefs_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_shortcuts(
    app: AppHandle,
    state: State<'_, AppState>,
    shortcuts: ShortcutBindings,
) -> Result<(), CmdError> {
    {
        let mut s = shortcuts;
        s.ensure_focus_char_slots();
        state.write().shortcuts = s;
    }
    persist(&app, &state)?;
    crate::shortcuts::reregister_all(&app, &state);
    emit_prefs_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn focus_character_at_index(
    state: State<'_, AppState>,
    index: usize,
) -> Result<(), CmdError> {
    let hwnds = state.ordered_visible_hwnds();
    if let Some(&hwnd) = hwnds.get(index) {
        let _ = focus_window(hwnd, std::time::Duration::from_millis(120));
    }
    Ok(())
}

#[tauri::command]
pub fn focus_next_character(state: State<'_, AppState>) -> Result<(), CmdError> {
    cycle_visible(&state, 1);
    Ok(())
}

#[tauri::command]
pub fn focus_prev_character(state: State<'_, AppState>) -> Result<(), CmdError> {
    cycle_visible(&state, -1);
    Ok(())
}

#[tauri::command]
pub fn focus_main_character(state: State<'_, AppState>) -> Result<(), CmdError> {
    let target_hwnd = {
        let inner = state.read();
        let main_id = inner.main_character_id.clone();
        match main_id {
            Some(id) => {
                let title_match = inner
                    .profiles
                    .iter()
                    .find(|p| p.id == id)
                    .cloned();
                title_match.and_then(|p| {
                    inner
                        .live_windows
                        .iter()
                        .find(|w| p.matches_window(&w.title, w.pid))
                        .map(|w| w.hwnd)
                })
            }
            None => None,
        }
    };
    if let Some(h) = target_hwnd {
        let _ = focus_window(h, std::time::Duration::from_millis(120));
    }
    Ok(())
}

fn cycle_visible(state: &AppState, delta: i32) {
    let hwnds = state.ordered_visible_hwnds();
    if hwnds.is_empty() {
        return;
    }
    let current = crate::windows::focus::current_foreground();
    let pos = hwnds.iter().position(|h| *h == current);
    let len = hwnds.len() as i32;
    let next = match pos {
        Some(i) => ((i as i32 + delta).rem_euclid(len)) as usize,
        None => {
            if delta >= 0 {
                0
            } else {
                (len - 1) as usize
            }
        }
    };
    let _ = focus_window(hwnds[next], std::time::Duration::from_millis(120));
}
