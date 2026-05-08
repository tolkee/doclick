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
                let info = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
                let vk = info.vkCode;
                let (broadcast_on, in_whitelist) = {
                    let inner = app_state.read();
                    (
                        inner.broadcast_enabled && !modifiers_held(),
                        inner.broadcast_keys.contains(&vk),
                    )
                };
                if broadcast_on && in_whitelist {
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
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

fn modifiers_held() -> bool {
    let pressed = |vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| {
        (unsafe { GetAsyncKeyState(vk.0 as i32) } as u16) & 0x8000 != 0
    };
    pressed(VK_CONTROL) || pressed(VK_MENU) || pressed(VK_LWIN) || pressed(VK_RWIN)
}
