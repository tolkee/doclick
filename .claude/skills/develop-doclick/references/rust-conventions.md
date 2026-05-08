# Rust conventions

doclick's Rust backend lives in `src-tauri/src/`. It runs across five concurrent contexts (Tauri main thread, Tokio watchers, the LL-hook message-pump thread, the broadcast dispatcher thread, and the frontend via IPC) — see [ARCHITECTURE.md](../../../../ARCHITECTURE.md) for the full picture.

## Error handling

**No panics in IPC paths, watchers, hooks, or the dispatcher.** That means `unwrap`/`expect` are reserved for:

- `#[cfg(test)]` test bodies (no concurrency, no recovery).
- Code with a written invariant explaining why the panic is impossible (e.g. `translated_click.expect("pre-translated for Click jobs")` in the dispatcher — the value is always `Some` in this branch and a panic indicates a logic bug we want to surface in dev).
- The very last line of `lib::run` (`tauri::Builder::run(...).expect("error while running doclick")`) — there's no recovery from a Tauri runtime failure on a desktop binary.

Anywhere else, propagate via `Result`. The custom `CmdError` enum in `commands.rs` covers IO, validation, and "invalid HWND" today; extend it before reaching for `String` errors.

```rust
// Good:
pub fn focus_dofus_window(hwnd: isize) -> Result<(), CmdError> {
    let h = HWND(hwnd as *mut _);
    if !unsafe { IsWindow(Some(h)) }.as_bool() {
        return Err(CmdError::Invalid("invalid window handle".into()));
    }
    /* ... */
}

// Bad — frontend-supplied hwnd reaches unsafe Win32 unchecked:
pub fn focus_dofus_window(hwnd: isize) {
    unsafe {
        let h = HWND(hwnd as *mut _);
        ShowWindow(h, SW_RESTORE);  // UB on bogus handle
    }
}
```

Thread spawns also return `Result`. `hooks::install` and `broadcast::dispatcher::start` each propagate `std::io::Result<()>` so the Tauri `setup` callback can `?`-bubble a startup failure into the runtime instead of crashing the worker.

## Concurrency primitives

Use `parking_lot::Mutex` and `parking_lot::RwLock` everywhere. They're already a dep, faster than std, and **never poison**, so you don't need `.unwrap()` after `.lock()`/`.read()`/`.write()`. The codebase has zero remaining `std::sync::Mutex` references — keep it that way.

`AppState(Arc<RwLock<InnerState>>)` is the shared state. Hold guards as briefly as possible: the dispatcher, watchers, and hook thread all take read locks, and a long-held write lock starves them. The hook thread is especially sensitive — Windows has a `LowLevelHooksTimeout` (~300ms by default) and silently uninstalls hooks that exceed it. Pattern (from `hooks/mouse.rs`):

```rust
// Snapshot under one lock, then release it before any Win32 call.
let (broadcast_on, known) = {
    let inner = app_state.read();
    (inner.broadcast_enabled, inner.live_windows.iter().map(|w| w.hwnd).collect::<Vec<_>>())
};
// inner is dropped here. Now do the Win32 work.
```

`InnerState` is intentionally not `Clone` — clones must go through `AppState::clone()` so every owner sees the same `Arc<RwLock<_>>`.

## Win32 / unsafe code

The crate sets `#![deny(unsafe_op_in_unsafe_fn)]` in `lib.rs`. Inside `unsafe extern "system" fn` callbacks (the LL hook procs), each unsafe op needs its own `unsafe { ... }` block — wrap raw-pointer derefs and `Call*Ex` calls explicitly:

```rust
// Inside unsafe extern "system" fn ll_kbd_proc:
let info = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
let next = unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
```

Always validate handles you didn't create yourself (anything from IPC, anything stored across enumerations) before passing to Win32. `IsWindow`, `GetWindowThreadProcessId == 0` checks, etc. are cheap.

DPI awareness is set per-process at startup (`enable_per_monitor_dpi_awareness` in `lib.rs`). Don't re-enable it elsewhere; do trust it everywhere.

## Event emission

Tauri's `app.emit(...)` returns `Result`. Failed emits are informational (a watcher failed to push a UI update during reload, a window-close race, etc.) — they shouldn't panic the worker, but they shouldn't disappear silently either. Use `events::emit_or_log`:

```rust
use crate::events::emit_or_log;

emit_or_log(&app, EVT_BROADCAST_TICK, payload);   // Good.
let _ = app.emit(EVT_BROADCAST_TICK, payload);    // Banned.
```

`emit_or_log` is the only allowed shape for emissions across the codebase — the team's checked it in `commands.rs`, `lib.rs`, `shortcuts.rs`, and `broadcast/dispatcher.rs`.

## Configurable tunables

Hardcoded `Duration` constants for timing-sensitive code go in a struct with `Default::default()` matching the shipped values. The pattern is `BroadcastTimings` in `broadcast/mod.rs`: 12 fields documenting *why* each value was chosen, threaded through the dispatcher worker. This keeps the magic-number-explaining comments next to the values and lets future tuning happen without a recompile (no UI for it yet, but the option exists).

If you find yourself adding a `const FOO: Duration` for a Win32 race window, put it on the relevant `Timings` struct instead.

## Module layout

Tests go at the **end** of the file, not the middle. Clippy's `items_after_test_module` is on (`-D warnings` in CI), so an item declared after a `#[cfg(test)] mod tests { ... }` block fails the build.

```rust
// state.rs
pub struct AppState(...);
impl AppState { ... }
// ...all production code...

#[cfg(test)]
mod tests {
    use super::*;
    // ...
}
// EOF
```

Modules use Rust 2018+ style — no `mod.rs` files for new modules; prefer `foo.rs` + `foo/bar.rs`.

## Documentation

Every public item on `AppState` and the public surface of `state.rs`, `events.rs`, and `broadcast/mod.rs` has a `///` rustdoc comment. Keep this discipline when adding new public APIs:

- One-line summary.
- Locking implications if it acquires a guard.
- A "Why" line for non-obvious design decisions.

`#[derive(Default)]` with `#[default]` on a variant is preferred over a manual `impl Default` (clippy's `derivable_impls`).

## Linting and formatting

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

`rustfmt.toml` pins `max_width = 100`. CI runs the exact same commands on `windows-latest` (Linux runners can't compile the Win32-bound code).

## Release profile

`Cargo.toml` ships with:

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
```

`panic = "abort"` is safe here — unwinding past the Tauri runtime isn't useful for a desktop binary. Don't relax these to speed up a release build; if you need a faster non-debug build for development, use the default `cargo build` (debug) or add a custom profile.
