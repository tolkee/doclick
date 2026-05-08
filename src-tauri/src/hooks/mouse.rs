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

        // Mouse-bound shortcuts only — never left/right buttons. Left is
        // reserved for the broadcast click, right for the OS context menu.
        if !is_dispatching() {
            if let Some(t) = unsafe { trigger_from_message(msg, l_param) } {
                let mods = current_modifiers();
                if let Some(action) = shortcuts::lookup_mouse(MouseShortcut { mods, trigger: t }) {
                    if let Some(app) = app_handle() {
                        if shortcuts::should_run(&app, action) {
                            shortcuts::run_action(&app, action);
                            // Swallow so the underlying app doesn't double-process,
                            // matching how the OS handles keyboard accelerators.
                            return LRESULT(1);
                        }
                    }
                }
            }
        }

        // Trigger on UP, not DOWN, so the source window finishes processing
        // DOWN+UP before the dispatcher steals focus to the targets.
        if msg == WM_LBUTTONUP && !is_dispatching() {
            if let Some(app_state) = state() {
                // Snapshot under a single read-lock — the LL hook has a system
                // timeout (LowLevelHooksTimeout, ~300ms) and Windows silently
                // uninstalls hooks that exceed it.
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
                    let info = unsafe { &*(l_param.0 as *const MSLLHOOKSTRUCT) };
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
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

unsafe fn trigger_from_message(msg: u32, l_param: LPARAM) -> Option<MouseTrigger> {
    match msg {
        WM_MBUTTONDOWN => Some(MouseTrigger::Mouse3),
        WM_XBUTTONDOWN => {
            // HIWORD(mouseData) is XBUTTON1 (1) or XBUTTON2 (2).
            let info = unsafe { &*(l_param.0 as *const MSLLHOOKSTRUCT) };
            match (info.mouseData >> 16) as u16 {
                1 => Some(MouseTrigger::Mouse4),
                2 => Some(MouseTrigger::Mouse5),
                _ => None,
            }
        }
        WM_MOUSEWHEEL => {
            // HIWORD(mouseData) is the signed wheel delta.
            let info = unsafe { &*(l_param.0 as *const MSLLHOOKSTRUCT) };
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
    let pressed = |vk: VIRTUAL_KEY| {
        (unsafe { GetAsyncKeyState(vk.0 as i32) } as u16) & 0x8000 != 0
    };
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
