pub mod keyboard;
pub mod mouse;

use once_cell::sync::OnceCell;
use tauri::AppHandle;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, MSG,
    WH_KEYBOARD_LL, WH_MOUSE_LL,
};

use crate::state::AppState;

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
pub fn install(state: AppState, app: AppHandle) -> std::io::Result<()> {
    let _ = GLOBAL_STATE.set(state);
    let _ = APP_HANDLE.set(app);

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
        })?;
    Ok(())
}
