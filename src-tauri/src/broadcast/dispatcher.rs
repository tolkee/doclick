use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use once_cell::sync::OnceCell;
use tauri::AppHandle;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC, MOUSEEVENTF_ABSOLUTE,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK,
    MOUSEINPUT,
};

use crate::events::{
    emit_or_log, BroadcastTickPayload, ErrorPayload, EVT_BROADCAST_TICK, EVT_ERROR,
};
use crate::state::AppState;
use crate::windows::focus::{current_foreground, focus_window};
use crate::windows::geometry::screen_to_absolute;

use super::translate::translate_click;
use super::{BroadcastJob, BroadcastTimings};

static SENDER: OnceCell<Sender<BroadcastJob>> = OnceCell::new();
static DISPATCHING: AtomicBool = AtomicBool::new(false);

pub fn is_dispatching() -> bool {
    DISPATCHING.load(Ordering::Acquire)
}

pub fn try_enqueue(job: BroadcastJob) -> bool {
    let Some(tx) = SENDER.get() else {
        return false;
    };
    match tx.try_send(job) {
        Ok(_) => true,
        Err(TrySendError::Full(_)) => {
            tracing::debug!("broadcast queue full, dropping job");
            false
        }
        Err(TrySendError::Disconnected(_)) => false,
    }
}

pub fn start(app: AppHandle, state: AppState) -> std::io::Result<()> {
    let timings = BroadcastTimings::default();
    let (tx, rx) = bounded::<BroadcastJob>(timings.queue_capacity);
    let _ = SENDER.set(tx);

    std::thread::Builder::new()
        .name("doclick-dispatcher".into())
        .spawn(move || run(app, state, rx, timings))?;
    Ok(())
}

fn run(app: AppHandle, state: AppState, rx: Receiver<BroadcastJob>, timings: BroadcastTimings) {
    while let Ok(job) = rx.recv() {
        let source_hwnd = match &job {
            BroadcastJob::Click { source_hwnd, .. } => *source_hwnd,
            BroadcastJob::Key { source_hwnd, .. } => *source_hwnd,
        };
        let targets = state.broadcast_targets(source_hwnd);
        let broadcast_on = state.read().broadcast_enabled;
        tracing::debug!(
            ?job,
            broadcast_on,
            targets = targets.len(),
            source = format!("{source_hwnd:#x}"),
            "dispatcher: received job"
        );
        if !broadcast_on {
            continue;
        }
        if targets.is_empty() {
            tracing::warn!(
                "dispatcher: no broadcast targets (only the source window is tracked, or others are Ignored)"
            );
            continue;
        }

        let original_fg = current_foreground();
        DISPATCHING.store(true, Ordering::Release);
        emit_or_log(
            &app,
            EVT_BROADCAST_TICK,
            BroadcastTickPayload::Started {
                followers: targets.len(),
            },
        );

        // Committed to the first-arrived job: drop any that piled up so the
        // next recv blocks on a fresh user click instead of stale ones.
        while rx.try_recv().is_ok() {}

        std::thread::sleep(timings.pre_dispatch_delay);

        let mut ok = 0usize;
        let mut failed = 0usize;
        for target in &targets {
            if dispatch_one(&app, &job, source_hwnd, *target, &timings) {
                ok += 1;
            } else {
                failed += 1;
            }
            std::thread::sleep(timings.post_send_hold);
        }

        tracing::debug!(ok, failed, "dispatcher: job complete");

        let restore_target = if original_fg != 0 {
            original_fg
        } else {
            source_hwnd
        };
        let _ = focus_window(restore_target, timings.foreground_wait);

        DISPATCHING.store(false, Ordering::Release);

        emit_or_log(
            &app,
            EVT_BROADCAST_TICK,
            BroadcastTickPayload::Finished { ok, failed },
        );
    }
}

fn dispatch_one(
    app: &AppHandle,
    job: &BroadcastJob,
    source_hwnd: isize,
    target_hwnd: isize,
    timings: &BroadcastTimings,
) -> bool {
    // Translate before focusing the target — shrinks the focus-to-SendInput
    // window during which foreground drift can drop the click.
    let translated_click = match job {
        BroadcastJob::Click { screen_x, screen_y, .. } => {
            match translate_click(source_hwnd, target_hwnd, *screen_x, *screen_y) {
                Some(coords) => Some(coords),
                None => {
                    tracing::warn!(
                        target = format!("{target_hwnd:#x}"),
                        reason = "translate_failed",
                        "dispatcher: coord translation failed (window minimized or just closed?)"
                    );
                    return false;
                }
            }
        }
        BroadcastJob::Key { .. } => None,
    };

    if !focus_with_retries(target_hwnd, timings) {
        tracing::warn!(
            target = format!("{target_hwnd:#x}"),
            reason = "focus_failed",
            "dispatcher: focus failed across retries (Win32 focus-stealing prevention or process integrity mismatch — try running doclick as admin if Dofus is elevated)"
        );
        emit_or_log(
            app,
            EVT_ERROR,
            ErrorPayload {
                message: format!("could not focus target {target_hwnd:#x}"),
                context: Some("dispatcher".into()),
            },
        );
        return false;
    }

    let mut stable = false;
    for _ in 0..timings.drift_recovery_tries {
        if current_foreground() == target_hwnd {
            stable = true;
            break;
        }
        if !focus_with_retries(target_hwnd, timings) {
            break;
        }
    }
    if !stable {
        tracing::warn!(
            target = format!("{target_hwnd:#x}"),
            reason = "drift_unrecoverable",
            "dispatcher: foreground would not stay on target across retries, click dropped"
        );
        emit_or_log(
            app,
            EVT_ERROR,
            ErrorPayload {
                message: format!("foreground drift on target {target_hwnd:#x}"),
                context: Some("dispatcher".into()),
            },
        );
        return false;
    }

    std::thread::sleep(timings.post_focus_settle);

    match job {
        BroadcastJob::Click { .. } => {
            let (tx, ty) = translated_click.expect("pre-translated for Click jobs");
            let ok = send_click(tx, ty, timings);
            if !ok {
                tracing::warn!(
                    target = format!("{target_hwnd:#x}"),
                    reason = "sendinput_failed",
                    tx,
                    ty,
                    "dispatcher: SendInput rejected click event(s)"
                );
            }
            ok
        }
        BroadcastJob::Key { vk, .. } => {
            let ok = send_key(*vk);
            if !ok {
                tracing::warn!(
                    target = format!("{target_hwnd:#x}"),
                    reason = "sendinput_failed",
                    vk,
                    "dispatcher: SendInput rejected key event(s)"
                );
            }
            ok
        }
    }
}

/// Focus the target with up to 3 attempts and a backoff between them. Each
/// `focus_window` call re-arms focus-stealing rights, so attempts are
/// independent — necessary because the OS denies rapid focus changes.
fn focus_with_retries(target_hwnd: isize, timings: &BroadcastTimings) -> bool {
    if focus_window(target_hwnd, timings.foreground_wait) {
        return true;
    }
    std::thread::sleep(Duration::from_millis(15));
    if focus_window(target_hwnd, timings.foreground_wait * 2) {
        return true;
    }
    std::thread::sleep(Duration::from_millis(30));
    focus_window(target_hwnd, timings.foreground_wait * 2)
}

fn send_click(screen_x: i32, screen_y: i32, timings: &BroadcastTimings) -> bool {
    let (dx, dy) = screen_to_absolute(screen_x, screen_y);
    let mv = [mouse_input(
        dx,
        dy,
        MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK | MOUSEEVENTF_MOVE,
    )];
    let down = [mouse_input(
        dx,
        dy,
        MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK | MOUSEEVENTF_LEFTDOWN,
    )];
    let up = [mouse_input(
        dx,
        dy,
        MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK | MOUSEEVENTF_LEFTUP,
    )];
    let cb = std::mem::size_of::<INPUT>() as i32;
    let move_ok = unsafe { SendInput(&mv, cb) == 1 };
    std::thread::sleep(timings.move_to_down_gap);
    let down_ok = unsafe { SendInput(&down, cb) == 1 };
    std::thread::sleep(timings.click_down_up_gap);
    let up_ok = unsafe { SendInput(&up, cb) == 1 };
    move_ok && down_ok && up_ok
}

fn mouse_input(dx: i32, dy: i32, flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_key(vk: u32) -> bool {
    let scan = unsafe { MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) } as u16;
    let inputs = [
        keyboard_input(vk as u16, scan, KEYEVENTF_SCANCODE),
        keyboard_input(vk as u16, scan, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP),
    ];
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) == inputs.len() as u32 }
}

fn keyboard_input(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
