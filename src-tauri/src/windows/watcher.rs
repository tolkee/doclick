//! Event-driven window and foreground tracking.
//!
//! Earlier versions polled: `EnumWindows` every 1.5s and
//! `GetForegroundWindow` every 200ms, forever. This module instead installs
//! WinEvent hooks (`SetWinEventHook`, WINEVENT_OUTOFCONTEXT) on a dedicated
//! message-pump thread and wakes two async watcher tasks through tokio
//! `Notify` handles. Updates land in ~150ms instead of up to 1.5s, and idle
//! CPU drops to zero between events. A slow fallback rescan papers over any
//! missed events.
//!
//! The WinEvent hooks get their own pump thread — NOT the LL-hook thread —
//! so a burst of desktop events can never delay low-level mouse/keyboard
//! processing (LL hooks are silently uninstalled by Windows when their
//! thread stalls past LowLevelHooksTimeout).

use std::time::Duration;

use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, CHILDID_SELF, EVENT_OBJECT_CREATE,
    EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND,
    EVENT_SYSTEM_MINIMIZESTART, MSG, OBJID_WINDOW, WINEVENT_OUTOFCONTEXT,
};

use crate::events::{
    FocusedWindowChangedPayload, WindowsChangedPayload, EVT_FOCUSED_WINDOW_CHANGED,
    EVT_WINDOWS_CHANGED,
};
use crate::state::AppState;
use crate::windows::enumerate::{enumerate_dofus_windows, PidNameCache};
use crate::windows::focus::current_foreground;

/// Coalesces the burst of WinEvents around a window opening/closing into a
/// single rescan.
const DEBOUNCE: Duration = Duration::from_millis(150);
/// Fallback full rescan when no event arrived — safety net only.
const FALLBACK_RESCAN: Duration = Duration::from_secs(10);
/// Fallback foreground re-check — EVENT_SYSTEM_FOREGROUND is reliable, this
/// only covers exotic cases (e.g. foreground lost to a closing window).
const FALLBACK_FOCUS: Duration = Duration::from_secs(2);

/// Pending-wakeup flags for the watcher tasks. `Notify` stores at most one
/// permit, so events firing while a rescan is in flight are never lost — the
/// next `notified().await` returns immediately.
static WINDOWS_DIRTY: Lazy<Notify> = Lazy::new(Notify::new);
static FOREGROUND_DIRTY: Lazy<Notify> = Lazy::new(Notify::new);

/// Ask the focus watcher to re-evaluate which tracked window is focused.
/// Called when profiles change: the foreground window didn't move, but its
/// tracked/untracked status may have flipped.
pub fn nudge_focus() {
    FOREGROUND_DIRTY.notify_one();
}

pub fn start(app: AppHandle, state: AppState) {
    install_win_event_hooks();

    // Sync initial enumeration — must complete before `overlay.show()` so
    // the webview's first `get_state_snapshot` sees pre-existing Dofus
    // windows. Without this they stay invisible until the first WinEvent
    // (or fallback rescan) forces a delta-emit.
    let mut cache = PidNameCache::default();
    let initial = enumerate_dofus_windows(&mut cache);
    let initial_signature: Vec<(isize, String)> =
        initial.iter().map(|w| (w.hwnd, w.title.clone())).collect();
    state.write().live_windows = initial;

    spawn_windows_watcher(app.clone(), state.clone(), cache, initial_signature);
    spawn_focus_watcher(app, state);
}

/// WinEvent callback. Runs on the doclick-winevents pump thread; must stay
/// cheap (filter + notify) because it fires for desktop-wide events.
unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _id_event_thread: u32,
    _time: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND {
        FOREGROUND_DIRTY.notify_one();
        return;
    }
    // Only whole top-level windows. Child controls and non-window
    // accessibility objects fire CREATE/NAMECHANGE constantly.
    if hwnd.is_invalid() || id_object != OBJID_WINDOW.0 || id_child != CHILDID_SELF as i32 {
        return;
    }
    WINDOWS_DIRTY.notify_one();
}

fn install_win_event_hooks() {
    #[allow(clippy::expect_used)] // unrecoverable at startup
    std::thread::Builder::new()
        .name("doclick-winevents".into())
        .spawn(|| unsafe {
            // Narrow per-range hooks rather than one broad range so the
            // high-frequency events in between never reach our callback.
            let ranges = [
                (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
                // CREATE(0x8000), DESTROY, SHOW, HIDE(0x8003)
                (EVENT_OBJECT_CREATE, EVENT_OBJECT_HIDE),
                (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
                (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            ];
            let mut installed = 0;
            for (min, max) in ranges {
                let hook = SetWinEventHook(
                    min,
                    max,
                    None,
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );
                if hook.is_invalid() {
                    tracing::warn!(min, max, "failed to install WinEvent hook range");
                } else {
                    installed += 1;
                }
            }
            if installed == 0 {
                // The fallback rescans in the watcher tasks keep the app
                // functional, just slower to notice changes.
                tracing::error!("no WinEvent hooks installed, relying on fallback rescans only");
            }

            // WINEVENT_OUTOFCONTEXT events are delivered as messages to this
            // thread — it must pump or the callback never runs.
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .expect("spawn winevents thread");
}

fn spawn_windows_watcher(
    app: AppHandle,
    state: AppState,
    mut cache: PidNameCache,
    mut last_signature: Vec<(isize, String)>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = tokio::time::timeout(FALLBACK_RESCAN, WINDOWS_DIRTY.notified()).await;
            tokio::time::sleep(DEBOUNCE).await;
            let live = enumerate_dofus_windows(&mut cache);
            let signature: Vec<(isize, String)> =
                live.iter().map(|w| (w.hwnd, w.title.clone())).collect();
            if signature != last_signature {
                last_signature = signature;
                state.write().live_windows = live;
                let _ = app.emit(
                    EVT_WINDOWS_CHANGED,
                    WindowsChangedPayload {
                        windows: state.snapshot_windows(),
                    },
                );
                // The tracked set changed, so "which tracked window is
                // focused" may have a new answer.
                nudge_focus();
            }
        }
    });
}

fn spawn_focus_watcher(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        // `None` = "not yet emitted"; `Some(None)` = "last emitted: no tracked window".
        let mut last: Option<Option<isize>> = None;
        loop {
            let fg = current_foreground();
            let tracked = state.read().tracked_hwnds().contains(&fg);
            let current = if tracked { Some(fg) } else { None };
            if last != Some(current) {
                last = Some(current);
                let _ = app.emit(
                    EVT_FOCUSED_WINDOW_CHANGED,
                    FocusedWindowChangedPayload {
                        focused_hwnd: current,
                    },
                );
            }
            let _ = tokio::time::timeout(FALLBACK_FOCUS, FOREGROUND_DIRTY.notified()).await;
        }
    });
}
