use std::time::Duration;

use tauri::{Emitter, Manager};

mod broadcast;
mod commands;
mod config;
mod diagnostics;
mod events;
mod hooks;
mod shortcuts;
mod state;
mod travel;
mod windows;

use crate::events::{
    BroadcastStatePayload, FocusedWindowChangedPayload, UpdateState, UpdateStatePayload,
    WindowsChangedPayload, EVT_PREFS_CHANGED, EVT_UPDATE_STATE,
};
use crate::state::{AppState, BroadcastReason, Orientation};
use crate::windows::geometry::enable_per_monitor_dpi_awareness;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,doclick=debug")),
        )
        .init();

    diagnostics::install_panic_hook();

    enable_per_monitor_dpi_awareness();

    let app_state = AppState::default();

    let result = tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(shortcuts::dispatch)
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    // Hiding (vs closing) preserves the settings webview's tab
                    // and scroll state so re-opens are instant and don't lose
                    // user position.
                    "settings" => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    // Overlay close (via the kebab "Fermer" item) tears down
                    // the whole app — including the settings window.
                    _ => window.app_handle().exit(0),
                }
            }
        })
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::list_windows,
            commands::get_state_snapshot,
            commands::set_broadcast_enabled,
            commands::set_broadcast_keys_enabled,
            commands::set_panic_hotkey,
            commands::upsert_profile,
            commands::delete_profile,
            commands::acknowledge_pvp_warning,
            commands::focus_dofus_window,
            commands::cycle_focus,
            commands::save_overlay_position,
            commands::save_overlay_size,
            commands::save_settings_size,
            commands::save_settings_position,
            commands::open_settings,
            commands::set_main_character,
            commands::set_profile_order,
            commands::set_orientation,
            commands::set_overlay_scale,
            commands::set_shortcuts,
            commands::focus_character_at_index,
            commands::focus_next_character,
            commands::focus_prev_character,
            commands::focus_main_character,
            commands::check_for_update,
            commands::install_update_and_relaunch,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            let (saved_position, saved_size, saved_settings_size, saved_settings_position) =
                if let Ok(dir) = handle.path().app_data_dir() {
                    diagnostics::set_crash_dir(dir.clone());
                    let cfg = config::load(&dir);
                    let mut inner = app_state.write();
                    inner.profiles = cfg.profiles;
                    inner.broadcast_keys_enabled = cfg.broadcast_keys_enabled;
                    inner.panic_hotkey = cfg.panic_hotkey;
                    inner.pvp_warning_acknowledged = cfg.pvp_warning_acknowledged;
                    inner.overlay_position = cfg.overlay_position;
                    inner.overlay_sizes = cfg.overlay_sizes;
                    inner.settings_size = cfg.settings_size;
                    inner.settings_position = cfg.settings_position;
                    inner.main_character_id = cfg.main_character_id;
                    inner.profile_order = cfg.profile_order;
                    inner.orientation = cfg.orientation;
                    inner.overlay_scale = cfg.overlay_scale;
                    inner.shortcuts = cfg.shortcuts;
                    inner.shortcuts.ensure_focus_char_slots();
                    let saved_size = match inner.orientation {
                        Orientation::Horizontal => inner.overlay_sizes.horizontal,
                        Orientation::Vertical => inner.overlay_sizes.vertical,
                    };
                    (
                        cfg.overlay_position,
                        saved_size,
                        cfg.settings_size,
                        cfg.settings_position,
                    )
                } else {
                    (None, None, None, None)
                };

            // Window enumeration — sync init must precede `overlay.show()`.
            spawn_window_watcher(handle.clone(), app_state.clone());

            // Restore overlay position + size if previously saved, then show
            // the window. The overlay starts hidden in tauri.conf.json so the
            // first paint happens at the restored size, not the conf default.
            if let Some(overlay) = handle.get_webview_window("overlay") {
                // -32000 is the Win32 sentinel for a minimized window's
                // GetWindowPos result. Skip restoring such a position so
                // we don't spawn the window offscreen.
                if let Some((x, y)) = saved_position {
                    if x > -32000 && y > -32000 {
                        let _ = overlay.set_position(tauri::PhysicalPosition::new(x, y));
                    }
                }
                if let Some((w, h)) = saved_size {
                    let _ = overlay.set_size(tauri::LogicalSize::new(w, h));
                }
                let _ = overlay.show();
            }

            // Pre-apply the persisted settings size and position so the
            // first `open_settings` paints where the user left it. If no
            // position is persisted, fall back to centering — the conf
            // also has `center: true`, but on hide/show Tauri preserves
            // the last position, so a saved-then-cleared run would
            // otherwise stick at (0,0).
            if let Some(settings) = handle.get_webview_window("settings") {
                if let Some((w, h)) = saved_settings_size {
                    let _ = settings.set_size(tauri::LogicalSize::new(w, h));
                }
                match saved_settings_position {
                    // -32000 is the Win32 minimized sentinel — skip restoring
                    // such a position so we don't spawn offscreen.
                    Some((x, y)) if x > -32000 && y > -32000 => {
                        let _ = settings.set_position(tauri::PhysicalPosition::new(x, y));
                    }
                    _ => {
                        let _ = settings.center();
                    }
                }
            }

            // Hook thread (low-level mouse + keyboard).
            hooks::install(app_state.clone(), handle.clone());

            // Dispatcher thread (focus-cycle + SendInput worker).
            broadcast::dispatcher::start(handle.clone(), app_state.clone());

            // Foreground watchdog (auto-disable when no Dofus window is focused for ~5s).
            spawn_foreground_watchdog(handle.clone(), app_state.clone());

            // Focus tracker (emits which tracked Dofus window is focused, for the avatar bar).
            spawn_focus_tracker(handle.clone(), app_state.clone());

            // Register all configured global shortcuts (includes panic hotkey).
            shortcuts::reregister_all(&handle, &app_state);

            // Background updater check (~30s after startup, throttled to 6h).
            spawn_update_check(handle.clone(), app_state.clone());

            // One-shot re-hydrate ping ~750ms after launch. Covers the Tauri 2
            // startup race where the webview's very first `get_state_snapshot`
            // can resolve against partial state (config/window writes happen
            // synchronously in setup but the listener attaching for the catch-
            // up emit isn't ready until React mounts post-paint).
            spawn_boot_rehydrate(handle.clone());

            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!(?err, "fatal: tauri runtime exited");
        std::process::exit(1);
    }
}

fn spawn_window_watcher(app: tauri::AppHandle, state: AppState) {
    // Sync initial enum so the webview's first `get_state_snapshot` sees
    // pre-existing Dofus windows. Without this they stay invisible until
    // an HWND-level change forces the async loop's next delta-emit.
    let initial = windows::enumerate::enumerate_dofus_windows();
    let initial_signature: Vec<(isize, String)> =
        initial.iter().map(|w| (w.hwnd, w.title.clone())).collect();
    state.write().live_windows = initial;

    tauri::async_runtime::spawn(async move {
        let mut last_signature = initial_signature;
        let mut interval = tokio::time::interval(Duration::from_millis(1500));
        loop {
            interval.tick().await;
            let live = windows::enumerate::enumerate_dofus_windows();
            let signature: Vec<(isize, String)> =
                live.iter().map(|w| (w.hwnd, w.title.clone())).collect();
            let changed = signature != last_signature;
            {
                let mut inner = state.write();
                inner.live_windows = live;
            }
            if changed {
                last_signature = signature;
                let _ = app.emit(
                    events::EVT_WINDOWS_CHANGED,
                    WindowsChangedPayload {
                        windows: state.snapshot_windows(),
                    },
                );
            }
        }
    });
}

/// Emits `EVT_PREFS_CHANGED` once, ~750ms after launch. The overlay's
/// `App.tsx` wires `onPrefsChanged` to a full `hydrate()`, so this guarantees
/// a second snapshot round-trip after React has mounted and its listeners
/// are attached — repairing the initial paint when the first invoke raced
/// `setup`'s state writes.
fn spawn_boot_rehydrate(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(750)).await;
        let _ = app.emit(EVT_PREFS_CHANGED, ());
    });
}

fn spawn_focus_tracker(app: tauri::AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        // `None` = "not yet emitted"; `Some(None)` = "last emitted: no tracked window".
        let mut last: Option<Option<isize>> = None;
        loop {
            interval.tick().await;
            let fg = windows::focus::current_foreground();
            let known = state.all_hwnds();
            let current = if known.contains(&fg) { Some(fg) } else { None };
            if last != Some(current) {
                last = Some(current);
                let _ = app.emit(
                    events::EVT_FOCUSED_WINDOW_CHANGED,
                    FocusedWindowChangedPayload {
                        focused_hwnd: current,
                    },
                );
            }
        }
    });
}

/// HWNDs of every Tauri webview window we own. Used by the foreground
/// watchdog so clicking the overlay doesn't start the auto-disable countdown.
pub fn app_hwnds(app: &tauri::AppHandle) -> Vec<isize> {
    app.webview_windows()
        .values()
        .filter_map(|w| w.hwnd().ok().map(|h| h.0 as isize))
        .collect()
}

/// One-shot startup updater check. Sleeps 30s so it doesn't contend with
/// hook install and first window enumeration, then performs a single
/// network check. Errors are swallowed silently — only manual checks
/// (initiated from the About tab) surface failures to the user.
fn spawn_update_check(app: tauri::AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;

        {
            let inner = state.read();
            if inner.update_check_in_flight {
                return;
            }
            if let Some(last) = inner.last_update_check {
                if last.elapsed() < Duration::from_secs(6 * 3600) {
                    return;
                }
            }
        }
        state.write().update_check_in_flight = true;

        let result = commands::run_update_check(&app).await;

        {
            let mut inner = state.write();
            inner.update_check_in_flight = false;
            inner.last_update_check = Some(std::time::Instant::now());
        }

        match result {
            Ok(Some((version, notes))) => {
                tracing::info!(version = %version, "update available");
                let _ = app.emit(
                    EVT_UPDATE_STATE,
                    UpdateStatePayload {
                        state: UpdateState::Available,
                        version: Some(version),
                        notes,
                        error: None,
                    },
                );
            }
            Ok(None) => {
                tracing::debug!("startup update check: up to date");
            }
            Err(e) => {
                tracing::warn!(error = %e, "startup update check failed (silenced)");
            }
        }
    });
}

fn spawn_foreground_watchdog(app: tauri::AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1000));
        let mut non_dofus_streak: u32 = 0;
        loop {
            interval.tick().await;
            if !state.read().broadcast_enabled {
                non_dofus_streak = 0;
                continue;
            }
            let fg = windows::focus::current_foreground();
            // Both Dofus windows and our own app windows (overlay, settings)
            // count as valid foreground — otherwise clicking the broadcast
            // toggle on the overlay starts the auto-disable countdown.
            // Whitelisted companion apps (Ganymede) also count, so the user
            // can bounce between Dofus and Ganymede without tripping the gate.
            let dofus_known = state.all_hwnds();
            let app_known = app_hwnds(&app);
            let allowed = dofus_known.contains(&fg)
                || app_known.contains(&fg)
                || windows::focus::is_companion_window(fg);
            if allowed {
                non_dofus_streak = 0;
            } else {
                non_dofus_streak += 1;
                if non_dofus_streak >= 5 {
                    state.write().broadcast_enabled = false;
                    non_dofus_streak = 0;
                    tracing::info!(
                        fg = format!("{fg:#x}"),
                        "broadcast auto-disabled: no Dofus or doclick window foreground for 5s"
                    );
                    let _ = app.emit(
                        events::EVT_BROADCAST_STATE,
                        BroadcastStatePayload {
                            enabled: false,
                            reason: BroadcastReason::AutoDisabledForegroundMismatch,
                        },
                    );
                }
            }
        }
    });
}
