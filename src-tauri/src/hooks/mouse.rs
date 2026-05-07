use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, MSLLHOOKSTRUCT, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MOUSEWHEEL,
    WM_XBUTTONDOWN,
};

use crate::broadcast::{
    dispatcher::{is_dispatching, try_enqueue},
    BroadcastJob,
};
use crate::shortcuts::{self, MouseShortcut, MouseTrigger, MOD_ALT, MOD_CTRL, MOD_META, MOD_SHIFT};
use crate::windows::focus::current_foreground;
use crate::windows::geometry::screen_point_in_client;

use super::{app_handle, state};

pub unsafe extern "system" fn ll_mouse_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code == HC_ACTION as i32 {
        let msg = w_param.0 as u32;

        // Mouse-bound shortcuts (XButton, middle button, scroll wheel). We
        // never reuse the left/right buttons here — those are reserved for
        // the broadcast click and the OS context menu.
        if !is_dispatching() {
            let trigger = trigger_from_message(msg, l_param);
            if let Some(t) = trigger {
                let mods = current_modifiers();
                if let Some(action) = shortcuts::lookup_mouse(MouseShortcut { mods, trigger: t }) {
                    if let Some(app) = app_handle() {
                        if shortcuts::should_run(&app, action) {
                            shortcuts::run_action(&app, action);
                            // Swallow the event so the underlying app doesn't also
                            // process it (matches the global-shortcut behaviour the
                            // OS provides for keyboard accelerators).
                            return LRESULT(1);
                        }
                        // Gate denied — let the underlying app receive the
                        // click/wheel naturally.
                    }
                }
            }
        }

        // Trigger on UP, not DOWN, so the source window finishes processing
        // DOWN+UP before the dispatcher steals focus to the targets.
        if msg == WM_LBUTTONUP && !is_dispatching() {
            if let Some(app_state) = state() {
                // Snapshot in a single read-lock so we don't hold the lock
                // across the Win32 syscalls below. LL hooks have a
                // system-wide timeout (LowLevelHooksTimeout, ~300ms default);
                // Windows quietly uninstalls hooks that exceed it.
                let (broadcast_on, known) = {
                    let inner = app_state.read();
                    if !inner.broadcast_enabled {
                        (false, Vec::new())
                    } else {
                        let known: Vec<isize> = inner
                            .live_windows
                            .iter()
                            .filter(|w| {
                                inner
                                    .profiles
                                    .iter()
                                    .any(|p| p.matches_window(&w.title, w.pid))
                            })
                            .map(|w| w.hwnd)
                            .collect();
                        (true, known)
                    }
                };
                if broadcast_on {
                    let info = &*(l_param.0 as *const MSLLHOOKSTRUCT);
                    let sx = info.pt.x;
                    let sy = info.pt.y;
                    let fg = current_foreground();
                    if !known.contains(&fg) {
                        tracing::debug!(
                            sx,
                            sy,
                            fg = format!("{fg:#x}"),
                            "click: foreground is not a tracked Dofus window, dropped"
                        );
                    } else if !screen_point_in_client(fg, sx, sy) {
                        tracing::debug!(
                            sx,
                            sy,
                            fg = format!("{fg:#x}"),
                            "click: outside source client rect, dropped"
                        );
                    } else {
                        let queued = try_enqueue(BroadcastJob::Click {
                            source_hwnd: fg,
                            screen_x: sx,
                            screen_y: sy,
                        });
                        tracing::debug!(
                            sx,
                            sy,
                            source = format!("{fg:#x}"),
                            queued,
                            "click: enqueued for broadcast"
                        );
                    }
                }
            }
        }
    }
    CallNextHookEx(None, n_code, w_param, l_param)
}

unsafe fn trigger_from_message(msg: u32, l_param: LPARAM) -> Option<MouseTrigger> {
    match msg {
        WM_MBUTTONDOWN => Some(MouseTrigger::Mouse3),
        WM_XBUTTONDOWN => {
            let info = &*(l_param.0 as *const MSLLHOOKSTRUCT);
            // HIWORD(mouseData) is XBUTTON1 (1) or XBUTTON2 (2).
            match (info.mouseData >> 16) as u16 {
                1 => Some(MouseTrigger::Mouse4),
                2 => Some(MouseTrigger::Mouse5),
                _ => None,
            }
        }
        WM_MOUSEWHEEL => {
            let info = &*(l_param.0 as *const MSLLHOOKSTRUCT);
            // HIWORD(mouseData) is the signed wheel delta.
            let delta = (info.mouseData >> 16) as u16 as i16;
            if delta > 0 {
                Some(MouseTrigger::WheelUp)
            } else if delta < 0 {
                Some(MouseTrigger::WheelDown)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn current_modifiers() -> u8 {
    unsafe {
        let pressed = |vk: VIRTUAL_KEY| (GetAsyncKeyState(vk.0 as i32) as u16) & 0x8000 != 0;
        let mut m = 0u8;
        if pressed(VK_CONTROL) {
            m |= MOD_CTRL;
        }
        if pressed(VK_SHIFT) {
            m |= MOD_SHIFT;
        }
        if pressed(VK_MENU) {
            m |= MOD_ALT;
        }
        if pressed(VK_LWIN) || pressed(VK_RWIN) {
            m |= MOD_META;
        }
        m
    }
}
