use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN,
};

use crate::broadcast::{
    dispatcher::{is_dispatching, try_enqueue},
    BroadcastJob,
};
use crate::shortcuts::{self, MOD_ALT, MOD_CTRL, MOD_META};
use crate::windows::focus::current_foreground;

use super::{current_modifiers, state, SELF_INJECTED};

/// Modifier keys themselves (Shift/Ctrl/Alt/Win, generic and L/R variants).
/// Replaying a bare modifier press on followers is never useful and confuses
/// the game's input state.
fn is_modifier_vk(vk: u32) -> bool {
    matches!(vk, 0x10..=0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
}

pub unsafe extern "system" fn ll_kbd_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code == HC_ACTION as i32 {
        let msg = w_param.0 as u32;
        if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && !is_dispatching() {
            let info = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
            let vk = info.vkCode;
            let self_injected = info.dwExtraInfo == SELF_INJECTED;
            if !self_injected && !is_modifier_vk(vk) {
                if let Some(app_state) = state() {
                    // Single read-lock snapshot — LL hooks have a system-wide
                    // timeout (~300ms); keep the callback fast and lock-light.
                    let known = {
                        let inner = app_state.read();
                        if inner.broadcast_enabled && inner.broadcast_keys_enabled {
                            inner.tracked_hwnds()
                        } else {
                            Vec::new()
                        }
                    };
                    // Modifier combos (Ctrl/Alt/Win+X) keep their app-local meaning —
                    // broadcasting Ctrl+C would replicate "copy" across every window.
                    let mods = current_modifiers();
                    if !known.is_empty() && mods & (MOD_CTRL | MOD_ALT | MOD_META) == 0 {
                        let fg = current_foreground();
                        // Keys bound to a doclick shortcut (e.g. "F1" = focus
                        // char 1) keep that meaning — never replay them on the
                        // followers.
                        if known.contains(&fg) && !shortcuts::is_reserved_key(mods, vk) {
                            let _ = try_enqueue(BroadcastJob::Key {
                                source_hwnd: fg,
                                vk,
                            });
                        }
                    }
                }
            }
        }
    }
    CallNextHookEx(None, n_code, w_param, l_param)
}
