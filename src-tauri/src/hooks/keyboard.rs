use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN,
};

use crate::broadcast::{
    dispatcher::{is_dispatching, try_enqueue},
    BroadcastJob,
};
use crate::windows::focus::current_foreground;

use super::state;

pub unsafe extern "system" fn ll_kbd_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code == HC_ACTION as i32 {
        let msg = w_param.0 as u32;
        if (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) && !is_dispatching() {
            if let Some(app_state) = state() {
                // Modifier combos (Ctrl/Alt/Win+X) keep their app-local meaning —
                // broadcasting Ctrl+C would replicate "copy" across every window.
                let should_broadcast = {
                    let inner = app_state.read();
                    inner.broadcast_enabled
                        && inner.broadcast_keys_enabled
                        && !modifiers_held()
                };
                if should_broadcast {
                    let info = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
                    let vk = info.vkCode;
                    let fg = current_foreground();
                    if app_state.all_hwnds().contains(&fg) {
                        let _ = try_enqueue(BroadcastJob::Key {
                            source_hwnd: fg,
                            vk,
                        });
                    }
                }
            }
        }
    }
    CallNextHookEx(None, n_code, w_param, l_param)
}

fn modifiers_held() -> bool {
    unsafe {
        let pressed = |vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| {
            (GetAsyncKeyState(vk.0 as i32) as u16) & 0x8000 != 0
        };
        pressed(VK_CONTROL) || pressed(VK_MENU) || pressed(VK_LWIN) || pressed(VK_RWIN)
    }
}
