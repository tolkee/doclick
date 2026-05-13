use std::time::{Duration, Instant};

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow, SetWindowPos, HWND_TOP,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
};

/// Bring `hwnd` to the foreground despite Win32 focus-stealing prevention.
///
/// Strategy:
///   1. Synthesize a no-op Alt-key tap via `SendInput` (Windows allows focus
///      changes for ~5s after any user input, real or synthetic). SendInput
///      batches both events atomically; consecutive `keybd_event` calls in a
///      retry loop can be coalesced by the input subsystem and leave the
///      next attempt unprimed.
///   2. Attach our thread's input queue to the current foreground window's
///      thread, call `SetForegroundWindow`, force z-order with `SetWindowPos`,
///      then detach. The explicit z-order step matters for stacked overlapping
///      siblings (e.g. multiple Dofus instances): without it, z-order can
///      settle a frame after focus and `WindowFromPoint` can still return the
///      previous top-of-stack at SendInput delivery time, routing the click
///      to the wrong window.
///
/// Returns `true` if the foreground window became `hwnd` within `timeout`.
pub fn focus_window(hwnd: isize, timeout: Duration) -> bool {
    let target = HWND(hwnd as *mut _);

    unsafe {
        prime_focus_change_rights();

        let our_tid = GetCurrentThreadId();
        let fg_hwnd = GetForegroundWindow();
        let mut fg_tid = 0u32;
        if !fg_hwnd.is_invalid() {
            fg_tid = GetWindowThreadProcessId(fg_hwnd, None);
        }

        let attached = if fg_tid != 0 && fg_tid != our_tid {
            AttachThreadInput(our_tid, fg_tid, true).as_bool()
        } else {
            false
        };

        let _ = SetForegroundWindow(target);
        let _ = SetWindowPos(
            target,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        );

        if attached {
            let _ = AttachThreadInput(our_tid, fg_tid, false);
        }
    }

    wait_until_foreground(hwnd, timeout)
}

unsafe fn prime_focus_change_rights() {
    let scan = MapVirtualKeyW(VK_MENU.0 as u32, MAPVK_VK_TO_VSC) as u16;
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: scan,
                    dwFlags: KEYEVENTF_SCANCODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: scan,
                    dwFlags: KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
}

pub fn wait_until_foreground(hwnd: isize, timeout: Duration) -> bool {
    let target = HWND(hwnd as *mut _);
    let deadline = Instant::now() + timeout;
    loop {
        let fg = unsafe { GetForegroundWindow() };
        if fg == target {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub fn current_foreground() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

/// Companion-app exe filenames that count as a "valid" foreground for the
/// shortcut gate and the broadcast auto-disable watchdog. Hardcoded for
/// now — Ganymede is the only companion the user regularly bounces between.
/// Promote to a Settings-managed list when a second use case appears.
pub(crate) const COMPANION_PROCESS_NAMES: &[&str] = &["ganymede.exe"];

/// Process basename for the given HWND, or `None` if the process can't be
/// opened.
pub(crate) fn process_basename_of(hwnd: isize) -> Option<String> {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid));
    }
    if pid == 0 {
        return None;
    }
    crate::windows::enumerate::process_basename(pid)
}

/// True if `hwnd` belongs to a whitelisted companion app (Ganymede).
pub(crate) fn is_companion_window(hwnd: isize) -> bool {
    process_basename_of(hwnd)
        .map(|name| {
            COMPANION_PROCESS_NAMES
                .iter()
                .any(|w| name.eq_ignore_ascii_case(w))
        })
        .unwrap_or(false)
}
