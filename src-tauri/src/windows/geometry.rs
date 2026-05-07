use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN,
};

pub fn enable_per_monitor_dpi_awareness() {
    unsafe {
        // Best-effort. If a manifest already set awareness, this is a no-op.
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VirtualDesktop {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

pub fn virtual_desktop() -> VirtualDesktop {
    unsafe {
        VirtualDesktop {
            left: GetSystemMetrics(SM_XVIRTUALSCREEN),
            top: GetSystemMetrics(SM_YVIRTUALSCREEN),
            width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
            height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
        }
    }
}

/// Converts an absolute screen point to MOUSEEVENTF_ABSOLUTE coords (0..=65535)
/// over the virtual desktop. Caller must include `MOUSEEVENTF_VIRTUALDESK` in
/// the SendInput flags.
pub fn screen_to_absolute(screen_x: i32, screen_y: i32) -> (i32, i32) {
    let vd = virtual_desktop();
    let w = vd.width.max(1) as f64;
    let h = vd.height.max(1) as f64;
    let nx = ((screen_x - vd.left) as f64 / w * 65535.0).round() as i32;
    let ny = ((screen_y - vd.top) as f64 / h * 65535.0).round() as i32;
    (nx.clamp(0, 65535), ny.clamp(0, 65535))
}

pub fn client_rect(hwnd: isize) -> Option<RECT> {
    unsafe {
        let mut r = RECT::default();
        if GetClientRect(HWND(hwnd as *mut _), &mut r).is_ok() {
            Some(r)
        } else {
            None
        }
    }
}

pub fn screen_to_client(hwnd: isize, screen_x: i32, screen_y: i32) -> Option<(i32, i32)> {
    unsafe {
        let mut p = POINT {
            x: screen_x,
            y: screen_y,
        };
        if ScreenToClient(HWND(hwnd as *mut _), &mut p).as_bool() {
            Some((p.x, p.y))
        } else {
            None
        }
    }
}

pub fn client_to_screen(hwnd: isize, client_x: i32, client_y: i32) -> Option<(i32, i32)> {
    unsafe {
        let mut p = POINT {
            x: client_x,
            y: client_y,
        };
        if ClientToScreen(HWND(hwnd as *mut _), &mut p).as_bool() {
            Some((p.x, p.y))
        } else {
            None
        }
    }
}

/// True if the screen point lies inside the window's client area.
pub fn screen_point_in_client(hwnd: isize, screen_x: i32, screen_y: i32) -> bool {
    let Some((cx, cy)) = screen_to_client(hwnd, screen_x, screen_y) else {
        return false;
    };
    let Some(rect) = client_rect(hwnd) else {
        return false;
    };
    cx >= rect.left && cx < rect.right && cy >= rect.top && cy < rect.bottom
}
