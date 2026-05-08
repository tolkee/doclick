pub mod dispatcher;
pub mod translate;

use std::time::Duration;

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

/// Tunables for the broadcast dispatcher.
///
/// These were chosen empirically against Dofus 3 on Win11 and are exposed as
/// a struct (rather than `const` values) so they can be tweaked without a
/// recompile if a future client patch changes timing sensitivity. Defaults
/// match the values that shipped through 0.6.x.
#[derive(Debug, Clone, Copy)]
pub struct BroadcastTimings {
    /// Bounded queue capacity. Caps how many user clicks pile up while a job
    /// is in flight; older entries are dropped via `try_send`.
    pub queue_capacity: usize,
    /// Foreground-wait timeout used for both target focus and origin restore.
    pub foreground_wait: Duration,
    /// Delay between the user's click and the first follower focus, so the
    /// source window finishes processing DOWN+UP before we steal focus.
    pub pre_dispatch_delay: Duration,
    /// Gap between synthetic LEFTDOWN and LEFTUP. Real human clicks sit in
    /// the ~30–50ms range; some games drop zero-duration clicks.
    pub click_down_up_gap: Duration,
    /// Hold the follower foreground after sending input so its message pump
    /// can ingest the click before we steal focus to the next target.
    pub post_send_hold: Duration,
    /// Number of foreground re-checks performed after a successful focus,
    /// to recover from focus drift before SendInput fires.
    pub drift_recovery_tries: u32,
    /// Minimum gap between confirming foreground and firing SendInput, so
    /// the target's pump can drain WM_ACTIVATE / WM_SETFOCUS first.
    pub post_focus_settle: Duration,
    /// Gap between the synthetic mouse-move and the LEFTDOWN. Splitting move
    /// from press lets the game update hover state on one frame before the
    /// press lands on the next.
    pub move_to_down_gap: Duration,
}

impl Default for BroadcastTimings {
    fn default() -> Self {
        Self {
            queue_capacity: 4,
            foreground_wait: Duration::from_millis(120),
            pre_dispatch_delay: Duration::from_millis(80),
            click_down_up_gap: Duration::from_millis(20),
            post_send_hold: Duration::from_millis(80),
            drift_recovery_tries: 3,
            post_focus_settle: Duration::from_millis(30),
            move_to_down_gap: Duration::from_millis(10),
        }
    }
}
