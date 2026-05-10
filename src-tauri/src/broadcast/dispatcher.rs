use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use once_cell::sync::OnceCell;
use tauri::{AppHandle, Emitter};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC, MOUSEEVENTF_ABSOLUTE,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK,
    MOUSEINPUT,
};

use crate::events::{BroadcastTickPayload, ErrorPayload, EVT_BROADCAST_TICK, EVT_ERROR};
use crate::state::AppState;
use crate::windows::focus::{current_foreground, focus_window};
use crate::windows::geometry::screen_to_absolute;

use super::translate::translate_click;
use super::BroadcastJob;

const QUEUE_CAPACITY: usize = 4;
const FOREGROUND_WAIT: Duration = Duration::from_millis(120);
/// Delay between the user's click and the first follower focus, so the source
/// window finishes processing DOWN+UP before we steal focus.
const PRE_DISPATCH_DELAY: Duration = Duration::from_millis(80);
/// Some games ignore zero-duration clicks; real human clicks are ~30-50ms.
const CLICK_DOWN_UP_GAP: Duration = Duration::from_millis(20);
/// Hold the follower foreground after sending input so its message pump can
/// ingest the click before we steal focus to the next target. Roughly 2-3
/// frames at 30 FPS — Dofus dips below that during loading screens.
const POST_SEND_HOLD: Duration = Duration::from_millis(80);
/// Times we re-check the foreground after `focus_with_retries` succeeds and
/// re-focus on drift. Single-shot recovery isn't enough: the FG can re-drift
/// in the microseconds between recovery and SendInput.
const DRIFT_RECOVERY_TRIES: u32 = 3;
/// Minimum gap between confirming foreground on the target and firing
/// SendInput, so the target's pump can drain WM_ACTIVATE / WM_SETFOCUS first.
/// Without this, Unity sees the window as "not yet focused" when the click
/// arrives and silently drops the input.
const POST_FOCUS_SETTLE: Duration = Duration::from_millis(30);
/// Gap between the synthetic mouse-move and the LEFTDOWN. Splitting move
/// from press lets the game update hover state on one frame before the press
/// lands on the next; combining them loses clicks during focus transitions.
const MOVE_TO_DOWN_GAP: Duration = Duration::from_millis(10);

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

pub fn start(app: AppHandle, state: AppState) {
    let (tx, rx) = bounded::<BroadcastJob>(QUEUE_CAPACITY);
    let _ = SENDER.set(tx);

    #[allow(clippy::expect_used)] // unrecoverable at startup
    std::thread::Builder::new()
        .name("doclick-dispatcher".into())
        .spawn(move || run(app, state, rx))
        .expect("spawn dispatcher thread");
}

fn run(app: AppHandle, state: AppState, rx: Receiver<BroadcastJob>) {
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
        let _ = app.emit(
            EVT_BROADCAST_TICK,
            BroadcastTickPayload::Started {
                followers: targets.len(),
            },
        );

        // Drain extras that piled up while we were idle. We're committed to
        // the first-arrived job; this clears the rest so the next `recv`
        // blocks on a fresh user click rather than serving stale ones.
        while rx.try_recv().is_ok() {}

        std::thread::sleep(PRE_DISPATCH_DELAY);

        let mut ok = 0usize;
        let mut failed = 0usize;
        for target in &targets {
            if dispatch_one(&app, &job, source_hwnd, *target) {
                ok += 1;
            } else {
                failed += 1;
            }
            std::thread::sleep(POST_SEND_HOLD);
        }

        tracing::debug!(ok, failed, "dispatcher: job complete");

        let restore_target = if original_fg != 0 {
            original_fg
        } else {
            source_hwnd
        };
        let _ = focus_window(restore_target, FOREGROUND_WAIT);

        DISPATCHING.store(false, Ordering::Release);

        let _ = app.emit(
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
) -> bool {
    // Translate before focusing the target. translate_click doesn't need the
    // target foreground, and pulling it out shrinks the critical window
    // between focus-confirmed and SendInput-fired.
    let translated_click = match job {
        BroadcastJob::Click {
            screen_x, screen_y, ..
        } => match translate_click(source_hwnd, target_hwnd, *screen_x, *screen_y) {
            Some(coords) => Some(coords),
            None => {
                tracing::warn!(
                    target = format!("{target_hwnd:#x}"),
                    reason = "translate_failed",
                    "dispatcher: coord translation failed (window minimized or just closed?)"
                );
                return false;
            }
        },
        BroadcastJob::Key { .. } => None,
    };

    if !focus_with_retries(target_hwnd) {
        tracing::warn!(
            target = format!("{target_hwnd:#x}"),
            reason = "focus_failed",
            "dispatcher: focus failed across retries (Win32 focus-stealing prevention or process integrity mismatch — try running doclick as admin if Dofus is elevated)"
        );
        let _ = app.emit(
            EVT_ERROR,
            ErrorPayload {
                message: format!("could not focus target {target_hwnd:#x}"),
                context: Some("dispatcher".into()),
            },
        );
        return false;
    }

    let mut stable = false;
    for _ in 0..DRIFT_RECOVERY_TRIES {
        if current_foreground() == target_hwnd {
            stable = true;
            break;
        }
        if !focus_with_retries(target_hwnd) {
            break;
        }
    }
    if !stable {
        tracing::warn!(
            target = format!("{target_hwnd:#x}"),
            reason = "drift_unrecoverable",
            "dispatcher: foreground would not stay on target across retries, click dropped"
        );
        let _ = app.emit(
            EVT_ERROR,
            ErrorPayload {
                message: format!("foreground drift on target {target_hwnd:#x}"),
                context: Some("dispatcher".into()),
            },
        );
        return false;
    }

    std::thread::sleep(POST_FOCUS_SETTLE);

    match job {
        BroadcastJob::Click { .. } => {
            // translate_for_target() above always returns Some for Click jobs;
            // for Key jobs it returns None and we never reach this arm.
            #[allow(clippy::expect_used)]
            let (tx, ty) = translated_click.expect("pre-translated for Click jobs");
            let ok = send_click(tx, ty);
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
/// `focus_window` call re-arms focus-stealing rights, so each attempt is
/// independent — important because the most common failure mode is the OS
/// denying focus changes during rapid back-to-back attempts.
fn focus_with_retries(target_hwnd: isize) -> bool {
    if focus_window(target_hwnd, FOREGROUND_WAIT) {
        return true;
    }
    std::thread::sleep(Duration::from_millis(15));
    if focus_window(target_hwnd, FOREGROUND_WAIT * 2) {
        return true;
    }
    std::thread::sleep(Duration::from_millis(30));
    focus_window(target_hwnd, FOREGROUND_WAIT * 2)
}

fn send_click(screen_x: i32, screen_y: i32) -> bool {
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
    std::thread::sleep(MOVE_TO_DOWN_GAP);
    let down_ok = unsafe { SendInput(&down, cb) == 1 };
    std::thread::sleep(CLICK_DOWN_UP_GAP);
    let up_ok = unsafe { SendInput(&up, cb) == 1 };
    move_ok && down_ok && up_ok
}

fn mouse_input(
    dx: i32,
    dy: i32,
    flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS,
) -> INPUT {
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

pub(crate) fn send_key(vk: u32) -> bool {
    let scan = unsafe { MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) } as u16;
    let inputs = [
        keyboard_input(vk as u16, scan, KEYEVENTF_SCANCODE),
        keyboard_input(vk as u16, scan, KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP),
    ];
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) == inputs.len() as u32 }
}

/// Atomic modifier+key combo as a single SendInput batch
/// (modifier-down → key-down → key-up → modifier-up).
pub(crate) fn send_key_combo(modifier_vk: u32, key_vk: u32) -> bool {
    let mod_scan = unsafe { MapVirtualKeyW(modifier_vk, MAPVK_VK_TO_VSC) } as u16;
    let key_scan = unsafe { MapVirtualKeyW(key_vk, MAPVK_VK_TO_VSC) } as u16;
    let inputs = [
        keyboard_input(modifier_vk as u16, mod_scan, KEYEVENTF_SCANCODE),
        keyboard_input(key_vk as u16, key_scan, KEYEVENTF_SCANCODE),
        keyboard_input(
            key_vk as u16,
            key_scan,
            KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
        ),
        keyboard_input(
            modifier_vk as u16,
            mod_scan,
            KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
        ),
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
