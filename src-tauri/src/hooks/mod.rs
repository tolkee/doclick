pub mod keyboard;
pub mod mouse;

use once_cell::sync::OnceCell;
use tauri::AppHandle;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, MSG,
    WH_KEYBOARD_LL, WH_MOUSE_LL,
};

use crate::shortcuts::{MOD_ALT, MOD_CTRL, MOD_META, MOD_SHIFT};
use crate::state::AppState;

/// Marker stamped into `dwExtraInfo` of every synthetic input doclick sends
/// (broadcast replays, focus-priming Alt taps, travel keys). The LL hooks
/// skip events carrying it — without this, the Alt tap that `focus_window`
/// fires to earn focus-stealing rights is itself caught by the keyboard hook
/// and re-broadcast, focus-cycling every window on a simple focus shortcut.
pub(crate) const SELF_INJECTED: usize = 0xD0C11C;

/// Currently held modifiers as a MOD_* bitmask. Shared by both LL hook
/// callbacks for shortcut matching.
pub(crate) fn current_modifiers() -> u8 {
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

static GLOBAL_STATE: OnceCell<AppState> = OnceCell::new();
static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();

pub fn state() -> Option<&'static AppState> {
    GLOBAL_STATE.get()
}

pub fn app_handle() -> Option<AppHandle> {
    APP_HANDLE.get().cloned()
}

/// Install both low-level hooks on a dedicated thread that runs a Win32
/// message pump. Hooks fire on the installer's thread, and that thread MUST
/// pump messages or the hook callbacks never run.
pub fn install(state: AppState, app: AppHandle) {
    let _ = GLOBAL_STATE.set(state);
    let _ = APP_HANDLE.set(app);

    #[allow(clippy::expect_used)] // unrecoverable at startup
    std::thread::Builder::new()
        .name("doclick-hooks".into())
        .spawn(|| unsafe {
            let mouse_hook =
                SetWindowsHookExW(WH_MOUSE_LL, Some(mouse::ll_mouse_proc), None, 0).ok();
            let kbd_hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard::ll_kbd_proc), None, 0).ok();

            if mouse_hook.is_none() || kbd_hook.is_none() {
                tracing::error!("failed to install low-level hooks");
                return;
            }

            let mut msg = MSG::default();
            // Pump messages forever; the process exits when Tauri exits.
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if let Some(h) = mouse_hook {
                let _ = UnhookWindowsHookEx(h);
            }
            if let Some(h) = kbd_hook {
                let _ = UnhookWindowsHookEx(h);
            }
        })
        .expect("spawn hook thread");
}
