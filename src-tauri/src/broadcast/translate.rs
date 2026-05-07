use crate::windows::geometry::{client_rect, client_to_screen, screen_to_client};

/// Map a screen-coord click on `main_hwnd` to the equivalent screen-coord on `follower_hwnd`.
/// Uses proportional scaling against each window's client rect so the click lands on the
/// "same" UI element regardless of window size.
pub fn translate_click(
    main_hwnd: isize,
    follower_hwnd: isize,
    main_screen_x: i32,
    main_screen_y: i32,
) -> Option<(i32, i32)> {
    let (main_cx, main_cy) = screen_to_client(main_hwnd, main_screen_x, main_screen_y)?;
    let main_rect = client_rect(main_hwnd)?;
    let main_w = (main_rect.right - main_rect.left).max(1);
    let main_h = (main_rect.bottom - main_rect.top).max(1);

    let nx = main_cx as f64 / main_w as f64;
    let ny = main_cy as f64 / main_h as f64;

    let f_rect = client_rect(follower_hwnd)?;
    let f_w = (f_rect.right - f_rect.left).max(1);
    let f_h = (f_rect.bottom - f_rect.top).max(1);

    let f_cx = (nx * f_w as f64).round() as i32;
    let f_cy = (ny * f_h as f64).round() as i32;

    client_to_screen(follower_hwnd, f_cx, f_cy)
}
