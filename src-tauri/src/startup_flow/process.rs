use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
};

use crate::windows::enumerate::process_basename;

/// Spawn `exe` and immediately detach. The returned PID is the only handle
/// we keep — dropping the `Child` is intentional; the OS keeps the process
/// alive after we exit and we don't want to block waiting for it.
///
/// All three stdio handles are nulled out so an inherited console (e.g. a
/// `bun run tauri dev` parent) doesn't keep stray pipes open into the
/// spawned launcher process.
pub fn spawn_detached(exe: &Path) -> io::Result<u32> {
    Command::new(exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|c| c.id())
}

/// `io::ErrorKind`s we treat as "the user-provided path is wrong" — these
/// poison the action's `last_path_error` and gate future runs until the
/// path is edited. Anything else (memory pressure, handle exhaustion…) is
/// transient and the next run will retry.
pub fn is_path_error(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
    )
}

struct ProbeCtx<'a> {
    wanted: &'a str,
    hit: bool,
}

/// True if any visible top-level window's owning process basename matches
/// `wanted` (case-insensitive). Cheaper than a full process snapshot and
/// sufficient for our consumers — Ankama Launcher and Ganymede both have
/// a visible window when they're running.
pub fn is_process_running(wanted: &str) -> bool {
    let mut ctx = ProbeCtx { wanted, hit: false };
    unsafe {
        let _ = EnumWindows(Some(probe_proc), LPARAM(&mut ctx as *mut _ as isize));
    }
    ctx.hit
}

unsafe extern "system" fn probe_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut ProbeCtx);
    if ctx.hit {
        return false.into();
    }
    if !IsWindowVisible(hwnd).as_bool() {
        return true.into();
    }
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return true.into();
    }
    if let Some(name) = process_basename(pid) {
        if name.eq_ignore_ascii_case(ctx.wanted) {
            ctx.hit = true;
            return false.into();
        }
    }
    true.into()
}

struct FindCtx<'a> {
    prefer_pid: Option<u32>,
    wanted: &'a str,
    best: Option<isize>,
    pid_match: bool,
}

/// Poll for a visible top-level window whose owning process matches
/// `wanted_basename`, preferring a window whose PID matches `prefer_pid`
/// when set. Returns the HWND once found, or `None` after `timeout`
/// elapses. Sleeps 250ms between polls — CPU stays near zero while a
/// launcher takes its 5–15s to paint.
pub fn poll_for_window(
    prefer_pid: Option<u32>,
    wanted_basename: &str,
    timeout: Duration,
) -> Option<isize> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(h) = find_window(prefer_pid, wanted_basename) {
            return Some(h);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn find_window(prefer_pid: Option<u32>, wanted: &str) -> Option<isize> {
    let mut ctx = FindCtx {
        prefer_pid,
        wanted,
        best: None,
        pid_match: false,
    };
    unsafe {
        let _ = EnumWindows(Some(find_proc), LPARAM(&mut ctx as *mut _ as isize));
    }
    ctx.best
}

unsafe extern "system" fn find_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut FindCtx);
    if !IsWindowVisible(hwnd).as_bool() {
        return true.into();
    }
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return true.into();
    }
    let pid_match = ctx.prefer_pid.is_some_and(|p| p == pid);
    // Once we've locked onto a PID match, the rest of the enumeration is
    // a waste of cycles — and we don't want a same-name basename in
    // another stray window to displace it.
    if !pid_match && ctx.pid_match {
        return true.into();
    }
    if let Some(name) = process_basename(pid) {
        if name.eq_ignore_ascii_case(ctx.wanted) {
            ctx.best = Some(hwnd.0 as isize);
            if pid_match {
                ctx.pid_match = true;
                return false.into();
            }
        }
    }
    true.into()
}
