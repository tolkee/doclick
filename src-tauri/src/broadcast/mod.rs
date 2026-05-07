pub mod dispatcher;
pub mod translate;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum BroadcastJob {
    /// Single click at the given screen point on the source window.
    /// `source_hwnd` is whichever tracked Dofus window the user clicked on
    /// (the foreground at click time, not necessarily the designated Main).
    /// Translation to each target's coords happens at dispatch time.
    Click {
        source_hwnd: isize,
        screen_x: i32,
        screen_y: i32,
    },
    /// Single key tap. Source is the foreground tracked window.
    Key { source_hwnd: isize, vk: u32 },
}
