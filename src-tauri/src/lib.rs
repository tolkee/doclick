use std::time::Duration;

use tauri::{Emitter, Manager};

mod broadcast;
mod commands;
mod config;
mod events;
mod hooks;
mod shortcuts;
mod state;
mod windows;

use crate::events::{BroadcastStatePayload, FocusedWindowChangedPayload, WindowsChangedPayload};
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

    enable_per_monitor_dpi_awareness();

    let app_state = AppState::default();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(shortcuts::dispatch)
                .build(),
        )
        .on_window_event(|window, event| {
            // Closing any of our windows quits the whole app — neither the
            // main window nor the overlay should outlive the other. The
            // overlay has no decorations (no close button) so this fires
            // when the user closes the main window from its custom titlebar.
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.app_handle().exit(0);
            }
        })
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::list_windows,
            commands::get_state_snapshot,
            commands::set_broadcast_enabled,
            commands::set_broadcast_keys,
            commands::set_panic_hotkey,
            commands::upsert_profile,
            commands::delete_profile,
            commands::acknowledge_pvp_warning,
            commands::focus_dofus_window,
            commands::cycle_focus,
            commands::save_overlay_position,
            commands::save_overlay_size,
            commands::set_main_character,
            commands::set_profile_order,
            commands::set_orientation,
            commands::set_shortcuts,
            commands::focus_character_at_index,
            commands::focus_next_character,
            commands::focus_prev_character,
            commands::focus_main_character,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // Load persisted config.
            let (saved_position, saved_size) = if let Ok(dir) = handle.path().app_data_dir() {
                let cfg = config::load(&dir);
                let mut inner = app_state.write();
                inner.profiles = cfg.profiles;
                if !cfg.broadcast_keys.is_empty() {
                    inner.broadcast_keys = cfg.broadcast_keys;
                }
                inner.panic_hotkey = cfg.panic_hotkey;
                inner.pvp_warning_acknowledged = cfg.pvp_warning_acknowledged;
                inner.overlay_position = cfg.overlay_position;
                inner.overlay_sizes = cfg.overlay_sizes;
                inner.main_character_id = cfg.main_character_id;
                inner.profile_order = cfg.profile_order;
                inner.orientation = cfg.orientation;
                inner.shortcuts = cfg.shortcuts;
                inner.shortcuts.ensure_focus_char_slots();
                let saved_size = match inner.orientation {
                    Orientation::Horizontal => inner.overlay_sizes.horizontal,
                    Orientation::Vertical => inner.overlay_sizes.vertical,
                };
                (cfg.overlay_position, saved_size)
            } else {
                (None, None)
            };

            // Restore overlay position + size if previously saved. The
            // overlay starts hidden (configured in tauri.conf.json) and
            // spawn_overlay_visibility decides when to show it based on the
            // current foreground window.
            if let Some(overlay) = handle.get_webview_window("overlay") {
                if let Some((x, y)) = saved_position {
                    let _ = overlay.set_position(tauri::PhysicalPosition::new(x, y));
                }
                if let Some((w, h)) = saved_size {
                    let _ = overlay.set_size(tauri::LogicalSize::new(w, h));
                }
            }

            // Hook thread (low-level mouse + keyboard).
            hooks::install(app_state.clone(), handle.clone());

            // Dispatcher thread (focus-cycle + SendInput worker).
            broadcast::dispatcher::start(handle.clone(), app_state.clone());

            // Window enumeration timer.
            spawn_window_watcher(handle.clone(), app_state.clone());

            // Foreground watchdog (auto-disable when no Dofus window is focused for ~5s).
            spawn_foreground_watchdog(handle.clone(), app_state.clone());

            // Focus tracker (emits which tracked Dofus window is focused, for the avatar bar).
            spawn_focus_tracker(handle.clone(), app_state.clone());

            // Overlay visibility (auto-hide when no Dofus / app window focused).
            spawn_overlay_visibility(handle.clone(), app_state.clone());

            // Register all configured global shortcuts (includes panic hotkey).
            shortcuts::reregister_all(&handle, &app_state);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running doclick");
}

fn spawn_window_watcher(app: tauri::AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut last_signature: Vec<(isize, String)> = Vec::new();
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

/// HWNDs of every Tauri webview window we own (overlay, settings, ...).
/// Used so the overlay doesn't auto-hide when the user clicks it (taking
/// focus to the overlay itself), and so shortcuts keep working from the
/// settings window.
pub fn app_hwnds(app: &tauri::AppHandle) -> Vec<isize> {
    app.webview_windows()
        .values()
        .filter_map(|w| w.hwnd().ok().map(|h| h.0 as isize))
        .collect()
}

fn spawn_overlay_visibility(app: tauri::AppHandle, state: AppState) {
    // Hide after ~600ms of non-Dofus foreground (3 ticks at 200ms) to avoid
    // flicker when alt-tabbing through other windows.
    const HIDE_AFTER_TICKS: u32 = 3;
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        // Overlay starts hidden (see tauri.conf.json `visible: false`); track
        // that as our initial state so the first Dofus-foregrounded tick will
        // call show().
        let mut hidden = true;
        let mut non_dofus_streak: u32 = 0;
        let mut last_logged_fg: isize = 0;
        loop {
            interval.tick().await;
            let fg = windows::focus::current_foreground();
            let dofus_known = state.all_hwnds();
            let app_known = app_hwnds(&app);
            let allowed = dofus_known.contains(&fg) || app_known.contains(&fg);
            if fg != last_logged_fg {
                tracing::debug!(
                    fg = format!("{fg:#x}"),
                    dofus_count = dofus_known.len(),
                    app_count = app_known.len(),
                    allowed,
                    "overlay-visibility: foreground changed"
                );
                last_logged_fg = fg;
            }
            if allowed {
                if hidden {
                    if let Some(overlay) = app.get_webview_window("overlay") {
                        let _ = overlay.show();
                        tracing::debug!("overlay-visibility: showing overlay");
                    }
                    hidden = false;
                }
                non_dofus_streak = 0;
            } else {
                non_dofus_streak = non_dofus_streak.saturating_add(1);
                if non_dofus_streak >= HIDE_AFTER_TICKS && !hidden {
                    if let Some(overlay) = app.get_webview_window("overlay") {
                        let _ = overlay.hide();
                        tracing::debug!("overlay-visibility: hiding overlay");
                    }
                    hidden = true;
                }
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
            let dofus_known = state.all_hwnds();
            let app_known = app_hwnds(&app);
            let allowed = dofus_known.contains(&fg) || app_known.contains(&fg);
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
