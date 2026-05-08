# Tauri conventions

doclick is a Tauri 2 app with a single window (label `"overlay"`), `decorations: false`, `transparent: true`, `alwaysOnTop: true`, and `skipTaskbar: true`. The window does double duty: it renders the thin always-on-top overlay AND, after `enterSettings()`, it resizes to host the settings tabs. This is intentional — see [ARCHITECTURE.md](../../../../ARCHITECTURE.md).

## Commands

Every Rust → frontend boundary goes through `#[tauri::command]` handlers in `src-tauri/src/commands.rs` and is registered in `lib.rs::run`'s `invoke_handler`. Conventions:

- One handler per logical action. Keep them small (~10–20 LOC); push real work into `state.rs` / `windows/` / `broadcast/` modules.
- Return `Result<T, CmdError>`. `CmdError` is a `serde::Serialize + thiserror::Error` enum with `Io`, `Invalid` variants today — extend it before reaching for stringly-typed errors.
- Validate untrusted inputs before any unsafe Win32 call. The HWND is the obvious case; PIDs, indices into orderings, and orientation strings should also be range-checked.
- Persist after every state mutation: `commands::persist(&app, &state)?;` at the end of the handler. The persist call writes the full config via temp-file + rename in `config::save` — atomic and safe.
- After persisting, emit any UI-relevant change. Use the helper functions at the top of `commands.rs` (`emit_windows_changed`, `emit_prefs_changed`, `emit_broadcast_state`) so the event names stay centralized.

The frontend never invokes a command by raw string — it goes through `src/ipc/commands.ts`, which wraps `invoke(...)` in typed functions. When you add a Rust command, add the matching wrapper in `commands.ts` in the same PR.

## Events

Event names are `&'static str` constants in `events.rs`. Payloads are `#[derive(Debug, Clone, Serialize)]` structs in the same file. Mirror types in `src/types.ts` and the typed listener in `src/ipc/events.ts`.

**Always emit through `events::emit_or_log`:**

```rust
use crate::events::emit_or_log;

emit_or_log(&app, EVT_BROADCAST_STATE, BroadcastStatePayload { enabled, reason });
```

`let _ = app.emit(...)` is banned across the Rust codebase — silent failures hide IPC channel breakage during reload, window teardown, and similar edge cases. The helper logs failures via `tracing::warn!` and never panics.

On the frontend, every event has an `on*` listener factory in `src/ipc/events.ts` that returns an `() => Promise<UnlistenFn>`. The pattern from `App.tsx`:

```ts
const subs = [
  onWindowsChanged((p) => useDoclickStore.setState({ windows: p.windows })),
  onBroadcastState((p) => useDoclickStore.setState({ broadcastEnabled: p.enabled, ... })),
  // ...
];
return () => {
  subs.forEach((s) => s.then((off) => off()));
};
```

The `useEffect` cleanup MUST unlisten — leaked listeners survive a hot reload and you'll see ghost handlers.

## State

`AppState` is the only `#[state]`-managed value. It's an `Arc<RwLock<InnerState>>` that all watchers, hooks, and the dispatcher share. Don't add a second managed state; extend `InnerState` instead.

Inside command handlers, use `state.read()` for reads and `state.write()` for mutations. Drop guards before any IO (file write, Win32 call, event emit) — see `rust-conventions.md` for why.

## Window management

The single `"overlay"` window swaps between two layouts driven by `view: "overlay" | "settings"` in `App.tsx`. The transitions (`enterSettings`, `exitSettings`) apply size + min-size + alwaysOnTop + skipTaskbar imperatively BEFORE flipping `view` so the destination layout never paints at the wrong dimensions. There's a passive backup effect that re-applies overlay size when orientation/chip-count/saved-size change while in overlay view — never use that effect for view transitions, it'll fight `enterSettings`.

A `viewRef` mirrors `view` for use inside event listener closures and the move-position-debounce timer (which can fire after the unmount). When you add a new effect that subscribes to a Tauri event AND cares which view is currently shown, read `viewRef.current`, not the React state.

## Settings size handling

Older builds occasionally persisted the overlay's tiny dimensions as `settings_size`. The `isValidSettingsSize` helper in `src/lib/overlaySize.ts` rejects anything below `SETTINGS_MIN_SIZE` on either axis — treat invalid sizes as "poisoned cache" and fall back to `SETTINGS_DEFAULT_SIZE`. Don't remove this defensive check; users who upgraded from an old install still have the bad value on disk.

## Capabilities

`src-tauri/capabilities/default.json` lists the capabilities granted to the `"overlay"` window. Adding a new Tauri plugin or command surface usually requires extending this — Tauri's permission model means a missing entry shows up as a runtime IPC error, not a compile error.

## CSP

`tauri.conf.json` deliberately sets `"security": { "csp": null }`. The overlay loads no remote content — every asset is bundled — so CSP would only restrict already-trusted code. If you ever add a remote-asset use case (web font CDN, telemetry, image hosting), enable CSP with an explicit allowlist in the same PR.

## Single-instance / autostart / global shortcut

The only Tauri plugin in use is `tauri-plugin-global-shortcut`. Don't add `single-instance` — running two instances is a user choice (e.g. testing a release vs a dev build); the codebase does not currently coordinate state across instances. Don't add `autostart` without a corresponding settings UI.

## Build / release

`Cargo.toml`'s `[profile.release]` is tuned for a small NSIS installer (LTO fat, opt-level 3, codegen-units 1, strip, panic abort). The release pipeline (`.github/workflows/release.yml`) is owned by release-please — see the [release-flow](../../release-flow/SKILL.md) skill for the bump rules, and don't edit version numbers by hand.

The `# x-release-please-version` comment on `src-tauri/Cargo.toml`'s `version` line is load-bearing — release-please's TOML support relies on it. If a future cargo edit strips the comment, version bumps stop working silently.
