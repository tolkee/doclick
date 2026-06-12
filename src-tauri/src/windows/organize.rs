//! Window organizer: stack every tracked Dofus window on the same rect.
//!
//! Runs automatically whenever broadcast turns on. All windows land on the
//! full work area (taskbar excluded) of the monitor hosting the first window
//! in display order. Combined with the focus shortcuts this gives "one
//! screen, N accounts" flipping, and makes the proportional click
//! translation exactly 1:1.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IsIconic, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOZORDER, SW_RESTORE,
};

/// Stack `hwnds` on the work area of the monitor hosting the first one.
pub fn organize(hwnds: &[isize]) {
    let Some(&first) = hwnds.first() else {
        return;
    };
    let Some((left, top, width, height)) = monitor_work_area(first) else {
        return;
    };

    let moved = hwnds
        .iter()
        .filter(|&&hwnd| place_window(hwnd, left, top, width.max(1), height.max(1)))
        .count();
    tracing::debug!(moved, total = hwnds.len(), "organize: windows stacked");
}

fn place_window(hwnd: isize, x: i32, y: i32, w: i32, h: i32) -> bool {
    unsafe {
        let handle = HWND(hwnd as *mut _);
        if IsIconic(handle).as_bool() {
            let _ = ShowWindow(handle, SW_RESTORE);
        }
        SetWindowPos(handle, None, x, y, w, h, SWP_NOACTIVATE | SWP_NOZORDER).is_ok()
    }
}

/// `(left, top, width, height)` of the work area of the monitor hosting `hwnd`.
fn monitor_work_area(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let monitor = MonitorFromWindow(HWND(hwnd as *mut _), MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let r = info.rcWork;
        Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
    }
}
