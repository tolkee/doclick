//! Crash diagnostics: writes panic info to a per-app crash log so a user can
//! recover diagnostics without console access.
//!
//! Without this hook, a panic on a packaged release build dies silently —
//! Tauri's tracing layer flushes to the console, but the released installer
//! has no console attached.

use std::backtrace::Backtrace;
use std::fs;
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static CRASH_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let payload = panic_payload(info);
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let backtrace = Backtrace::force_capture();

        tracing::error!(%location, "panic: {payload}");

        if let Some(dir) = crash_dir() {
            let _ = write_log(dir, &payload, &location, &backtrace);
        }
    }));
}

/// Set the crash directory once an `AppHandle` becomes available. Before this
/// is called, panics still log to `tracing::error!` but no crash file is
/// produced.
pub fn set_crash_dir(app_data_dir: PathBuf) {
    let dir = app_data_dir.join("crashes");
    let _ = CRASH_DIR.set(dir);
}

fn crash_dir() -> Option<&'static PathBuf> {
    CRASH_DIR.get()
}

fn panic_payload(info: &panic::PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn write_log(
    dir: &PathBuf,
    payload: &str,
    location: &str,
    backtrace: &Backtrace,
) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("panic-{stamp}.log"));
    let mut f = fs::File::create(path)?;
    writeln!(f, "doclick panic")?;
    writeln!(f, "version: {}", env!("CARGO_PKG_VERSION"))?;
    writeln!(f, "timestamp_unix: {stamp}")?;
    writeln!(f, "location: {location}")?;
    writeln!(f, "payload: {payload}")?;
    writeln!(f)?;
    writeln!(f, "backtrace:")?;
    writeln!(f, "{backtrace}")?;
    Ok(())
}
