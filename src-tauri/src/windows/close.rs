use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE,
};

use crate::windows::enumerate::{process_basename, DOFUS_PROCESS_NAMES};
use crate::windows::focus::COMPANION_PROCESS_NAMES;

/// Post WM_CLOSE to every visible top-level window owned by a Dofus or
/// Ganymede process. Fire-and-forget: receiving processes run their own
/// shutdown (logout prompts, save dialogs) on their own schedule, and we
/// don't wait. Caller is expected to exit Doclick immediately after.
pub fn close_dofus_and_companion_windows() {
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(0));
    }
}

unsafe extern "system" fn enum_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return true.into();
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return true.into();
    }

    let exe = match process_basename(pid) {
        Some(name) => name,
        None => return true.into(),
    };

    let is_target = DOFUS_PROCESS_NAMES
        .iter()
        .chain(COMPANION_PROCESS_NAMES.iter())
        .any(|n| n.eq_ignore_ascii_case(&exe));

    if is_target {
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
    }

    true.into()
}
