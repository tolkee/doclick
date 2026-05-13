use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::UpdaterExt;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{IsIconic, ShowWindow, SW_RESTORE};

use crate::config::{self, PersistedConfig};
use crate::events::{
    BroadcastStatePayload, UpdateProgressPayload, UpdateState, UpdateStatePayload,
    WindowsChangedPayload, EVT_BROADCAST_STATE, EVT_PREFS_CHANGED, EVT_UPDATE_PROGRESS,
    EVT_UPDATE_STATE, EVT_WINDOWS_CHANGED,
};
use crate::startup_flow::{self, StartupFlowConfig};
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
    #[error("updater: {0}")]
    Updater(String),
}

impl From<std::io::Error> for CmdError {
    fn from(e: std::io::Error) -> Self {
        CmdError::Io(e.to_string())
    }
}

impl From<tauri_plugin_updater::Error> for CmdError {
    fn from(e: tauri_plugin_updater::Error) -> Self {
        CmdError::Updater(e.to_string())
    }
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, CmdError> {
    app.path()
        .app_data_dir()
        .map_err(|e| CmdError::Io(e.to_string()))
}

pub fn persist(app: &AppHandle, state: &AppState) -> Result<(), CmdError> {
    let dir = app_data_dir(app)?;
    let cfg = PersistedConfig::from_inner(&state.read());
    config::save(&dir, &cfg)?;
    Ok(())
}

fn emit_windows_changed(app: &AppHandle, state: &AppState) {
    let _ = app.emit(
        EVT_WINDOWS_CHANGED,
        WindowsChangedPayload {
            windows: state.snapshot_windows(),
        },
    );
}

fn emit_prefs_changed(app: &AppHandle) {
    let _ = app.emit(EVT_PREFS_CHANGED, ());
}

fn emit_broadcast_state(app: &AppHandle, enabled: bool, reason: BroadcastReason) {
    let _ = app.emit(
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
    let windows = state.snapshot_windows();
    state.read().to_snapshot(windows)
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
pub fn acknowledge_pvp_warning(app: AppHandle, state: State<'_, AppState>) -> Result<(), CmdError> {
    state.write().pvp_warning_acknowledged = true;
    persist(&app, &state)?;
    Ok(())
}

#[tauri::command]
pub fn focus_dofus_window(hwnd: isize) -> Result<(), CmdError> {
    unsafe {
        let h = HWND(hwnd as *mut _);
        if IsIconic(h).as_bool() {
            let _ = ShowWindow(h, SW_RESTORE);
        }
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

/// Set the entire startup-flow configuration in one call. Diffs the
/// per-action exe paths against the previous value: any change clears
/// the action's sticky `last_path_error` (the user is acknowledging
/// and correcting the broken path).
#[tauri::command]
pub fn set_startup_flow_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: StartupFlowConfig,
) -> Result<(), CmdError> {
    {
        let mut inner = state.write();
        let mut next = config;

        if next.accounts.exe_path != inner.startup_flow.accounts.exe_path {
            next.accounts.last_path_error = None;
        }
        if next.ganymede.exe_path != inner.startup_flow.ganymede.exe_path {
            next.ganymede.last_path_error = None;
        }

        inner.startup_flow = next;
    }
    persist(&app, &state)?;
    crate::shortcuts::reregister_all(&app, &state);
    emit_prefs_changed(&app);
    Ok(())
}

/// Trigger the startup flow asynchronously. Returns immediately; progress
/// arrives via `EVT_STARTUP_FLOW_STATE`. Concurrent calls are gated
/// internally — the second call is a no-op while the first is still
/// running.
#[tauri::command]
pub fn run_startup_flow(app: AppHandle, state: State<'_, AppState>) -> Result<(), CmdError> {
    if !state.read().startup_flow.enabled {
        return Err(CmdError::Invalid(
            "startup flow master switch is off".into(),
        ));
    }
    let app_clone = app.clone();
    let state_clone = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        startup_flow::run(app_clone, state_clone).await;
    });
    Ok(())
}

/// Resolve the default exe path for the given action kind, if the file
/// exists at the standard `%LOCALAPPDATA%` location. Returns the path as
/// a string for direct rendering in the UI (placeholder for empty
/// inputs). When `None`, the caller should fall back to the hint string.
#[tauri::command]
pub fn get_default_exe_path(kind: String) -> Result<Option<String>, CmdError> {
    let p = match kind.as_str() {
        "launcher" => startup_flow::probe::default_launcher_path(),
        "ganymede" => startup_flow::probe::default_ganymede_path(),
        other => return Err(CmdError::Invalid(format!("kind={other}"))),
    };
    Ok(p.map(|p| p.to_string_lossy().into_owned()))
}

/// Like `get_default_exe_path` but always returns the *would-be* path
/// even if it doesn't exist on disk. Used by the UI as the input
/// placeholder so the user sees where Doclick would look. Distinct from
/// `get_default_exe_path` to keep the "auto-fill when present" semantics
/// of the latter clean.
#[tauri::command]
pub fn get_default_exe_path_hint(kind: String) -> Result<Option<String>, CmdError> {
    let p = match kind.as_str() {
        "launcher" => startup_flow::probe::default_launcher_path_hint(),
        "ganymede" => startup_flow::probe::default_ganymede_path_hint(),
        other => return Err(CmdError::Invalid(format!("kind={other}"))),
    };
    Ok(p)
}

#[tauri::command]
pub fn focus_character_at_index(state: State<'_, AppState>, index: usize) -> Result<(), CmdError> {
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
    if let Some(h) = state.main_character_hwnd() {
        let _ = focus_window(h, std::time::Duration::from_millis(120));
    }
    Ok(())
}

pub fn emit_update_state(
    app: &AppHandle,
    state: UpdateState,
    version: Option<String>,
    notes: Option<String>,
    error: Option<String>,
) {
    let _ = app.emit(
        EVT_UPDATE_STATE,
        UpdateStatePayload {
            state,
            version,
            notes,
            error,
        },
    );
}

/// Single source of truth for "is there a newer release?" — used by both
/// the manual `check_for_update` command and the startup auto-check. Just
/// the network round-trip; emitting events and bookkeeping are the
/// caller's responsibility so manual vs. background can differ in UX.
pub async fn run_update_check(
    app: &AppHandle,
) -> Result<Option<(String, Option<String>)>, CmdError> {
    let updater = app.updater()?;
    let update = updater.check().await?;
    Ok(update.map(|u| (u.version.clone(), u.body.clone())))
}

// Updater APIs are async; the rest of this file is sync `#[tauri::command]`s
// by convention. Async is required here because `updater::check()` and
// `download_and_install()` perform network I/O.
#[tauri::command]
pub async fn check_for_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), CmdError> {
    {
        let mut inner = state.write();
        if inner.update_check_in_flight {
            return Ok(());
        }
        inner.update_check_in_flight = true;
    }
    emit_update_state(&app, UpdateState::Checking, None, None, None);

    let result = run_update_check(&app).await;

    {
        let mut inner = state.write();
        inner.update_check_in_flight = false;
        inner.last_update_check = Some(std::time::Instant::now());
    }

    match result {
        Ok(Some((version, notes))) => {
            emit_update_state(&app, UpdateState::Available, Some(version), notes, None);
            Ok(())
        }
        Ok(None) => {
            emit_update_state(&app, UpdateState::NoUpdate, None, None, None);
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            tracing::warn!(error = %msg, "update check failed");
            emit_update_state(&app, UpdateState::Error, None, None, Some(msg));
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn install_update_and_relaunch(app: AppHandle) -> Result<(), CmdError> {
    let updater = app.updater()?;
    let update = match updater.check().await? {
        Some(u) => u,
        None => {
            emit_update_state(&app, UpdateState::NoUpdate, None, None, None);
            return Ok(());
        }
    };

    let version = update.version.clone();
    let notes = update.body.clone();
    emit_update_state(
        &app,
        UpdateState::Downloading,
        Some(version.clone()),
        notes.clone(),
        None,
    );

    let downloaded = Arc::new(AtomicU64::new(0));
    let progress_app = app.clone();
    let progress_acc = downloaded.clone();
    let install_app = app.clone();

    let result = update
        .download_and_install(
            move |chunk_length, content_length| {
                let total = progress_acc.fetch_add(chunk_length as u64, Ordering::Relaxed)
                    + chunk_length as u64;
                let _ = progress_app.emit(
                    EVT_UPDATE_PROGRESS,
                    UpdateProgressPayload {
                        downloaded: total,
                        total: content_length,
                    },
                );
            },
            move || {
                let _ = install_app.emit(
                    EVT_UPDATE_STATE,
                    UpdateStatePayload {
                        state: UpdateState::Installing,
                        version: None,
                        notes: None,
                        error: None,
                    },
                );
            },
        )
        .await;

    match result {
        Ok(()) => {
            tracing::info!(version = %version, "update installed, relaunching");
            app.restart()
        }
        Err(e) => {
            let msg = e.to_string();
            tracing::warn!(error = %msg, "update install failed");
            emit_update_state(&app, UpdateState::Error, None, None, Some(msg));
            Err(e.into())
        }
    }
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
