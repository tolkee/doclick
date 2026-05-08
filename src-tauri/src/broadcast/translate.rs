use crate::windows::geometry::{client_rect, client_to_screen, screen_to_client};

/// Pure proportional mapping: `(main_cx, main_cy)` inside a `main_w × main_h`
/// client rect → equivalent point inside a `follower_w × follower_h` client
/// rect. Sizes saturate at 1 to keep division safe on degenerate (minimized)
/// windows. Returns rounded integer client coordinates for the follower.
pub fn translate_proportional(
    main_cx: i32,
    main_cy: i32,
    main_w: i32,
    main_h: i32,
    follower_w: i32,
    follower_h: i32,
) -> (i32, i32) {
    let mw = main_w.max(1) as f64;
    let mh = main_h.max(1) as f64;
    let fw = follower_w.max(1) as f64;
    let fh = follower_h.max(1) as f64;
    let nx = main_cx as f64 / mw;
    let ny = main_cy as f64 / mh;
    ((nx * fw).round() as i32, (ny * fh).round() as i32)
}

/// Map a screen-coord click on `main_hwnd` to the equivalent screen-coord on
/// `follower_hwnd` so the click lands on the "same" UI element regardless of
/// window size.
pub fn translate_click(
    main_hwnd: isize,
    follower_hwnd: isize,
    main_screen_x: i32,
    main_screen_y: i32,
) -> Option<(i32, i32)> {
    let (main_cx, main_cy) = screen_to_client(main_hwnd, main_screen_x, main_screen_y)?;
    let main_rect = client_rect(main_hwnd)?;
    let f_rect = client_rect(follower_hwnd)?;

    let (f_cx, f_cy) = translate_proportional(
        main_cx,
        main_cy,
        main_rect.right - main_rect.left,
        main_rect.bottom - main_rect.top,
        f_rect.right - f_rect.left,
        f_rect.bottom - f_rect.top,
    );

    client_to_screen(follower_hwnd, f_cx, f_cy)
}

#[cfg(test)]
mod tests {
    use super::translate_proportional;

    #[test]
    fn same_size_windows_pass_through() {
        assert_eq!(translate_proportional(50, 25, 100, 50, 100, 50), (50, 25));
        assert_eq!(translate_proportional(0, 0, 100, 50, 100, 50), (0, 0));
        assert_eq!(translate_proportional(100, 50, 100, 50, 100, 50), (100, 50));
    }

    #[test]
    fn double_size_follower_scales_up() {
        assert_eq!(translate_proportional(50, 25, 100, 50, 200, 100), (100, 50));
        assert_eq!(translate_proportional(10, 10, 100, 100, 200, 200), (20, 20));
    }

    #[test]
    fn half_size_follower_scales_down() {
        assert_eq!(translate_proportional(100, 50, 200, 100, 100, 50), (50, 25));
    }

    #[test]
    fn rounds_to_nearest_pixel() {
        assert_eq!(translate_proportional(33, 0, 100, 1, 200, 1), (66, 0));
        assert_eq!(translate_proportional(34, 0, 100, 1, 199, 1), (68, 0));
    }

    #[test]
    fn zero_size_main_does_not_panic() {
        assert_eq!(translate_proportional(0, 0, 0, 0, 100, 100), (0, 0));
    }
}
