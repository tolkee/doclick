---
name: develop-doclick
description: Project conventions, recipes, and guardrails for working on Doclick — the Tauri 2 + React 19 + Win32 overlay that broadcasts clicks/keystrokes across Dofus 3 windows. Triggers on any edit to `src/`, `src-tauri/`, `tauri.conf.json`, `biome.json`, or release/CI configs in this repo. Use proactively whenever adding a Tauri command, an event, a global shortcut, or refactoring overlay/settings/state code. Also applies whenever Claude is about to add comments to any file in this repo — Doclick has a strict anti-slop comment policy that this skill enforces. Read this skill before making *any* code change to Doclick, even ones that look trivial; the guardrails about UX preservation, IPC wire format, and persistence schema are easy to violate accidentally.
---

# Developing on Doclick

## Product scope

Doclick is a Windows-only Tauri 2 overlay that broadcasts clicks and keystrokes from one tracked Dofus 3 window to others. The implementation rule is **no memory injection, no packet inspection, no Dofus-specific automation** — only synthetic input via Win32 public APIs (`SendInput`, `SetForegroundWindow`, `WH_MOUSE_LL`, `WH_KEYBOARD_LL`). If a proposed change reaches outside that envelope, stop and surface it to the user.

The user-facing UX is **validated and tested**. Don't change layouts, copy, sizes, color, motion, drag/drop behavior, broadcast logic, focus-cycle behavior, panic-hotkey behavior, or auto-disable rules unless the user explicitly asks. The whole point of refactors is internal cleanup; preserving observable behavior is a hard constraint.

## What never to change without explicit user approval

| Concern | Why it's load-bearing |
|---|---|
| Visible UX (layouts, sizes, copy, color) | Validated with users; surprise changes break muscle memory |
| Persistence schema (`profiles.json` shape) | Older builds and the live client must round-trip the same file |
| Tauri IPC wire format (command params, event payload fields) | Frontend↔backend contract; mismatches surface as silent failures |
| Shortcut accelerator strings | User-visible config that's persisted in `profiles.json` |
| Async timer cadences (window watcher 1.5s, focus tracker 200ms, foreground watchdog 1s, broadcast `POST_FOCUS_SETTLE` etc.) | Tuned against Win32 hook timeouts and Unity's input pipeline |

## Architecture map

```
Rust host (src-tauri/src)                React client (src)
─────────────────────────                ──────────────────
lib.rs       Tauri builder, async        App.tsx       View orchestration
             tasks, panic hook                          + window-size effects
state.rs     AppState (Arc<RwLock>)      Settings.tsx  Tab shell
             InnerState, snapshots       Settings/     Per-tab forms
config.rs    profiles.json persistence   Menu.tsx      Kebab-menu window
commands.rs  All #[tauri::command]       components/   UI primitives
events.rs    Payload types + EVT_*       store/        Zustand store
shortcuts.rs Global keyboard + mouse     ipc/          Typed invoke / listen
hooks/       LL mouse/keyboard hooks     lib/          Pure utilities
broadcast/   Dispatcher + translation    types.ts      Shared TS types
windows/     Win32 enumerate/focus/      
             geometry helpers
diagnostics  Panic hook → crash log
```

The Rust host owns all state and Win32 interaction. The React client renders and dispatches. They communicate exclusively through:
- **Commands**: frontend calls `invoke<T>("name", args)`. Define in `commands.rs`, register in `lib.rs::generate_handler!`, mirror the typed wrapper in `src/ipc/commands.ts`.
- **Events**: backend emits `app.emit(EVT_NAME, payload)`. Define payload + `EVT_*` constant in `events.rs`, mirror the typed `listen<T>` wrapper in `src/ipc/events.ts`, subscribe in App.tsx's effect.

Persistence flows: state mutation → `state.write()` → `commands::persist(&app, &state)` → `PersistedConfig::from_inner(&inner)` → atomic rename to `app_data_dir/profiles.json`.

## Comment philosophy (the anti-slop rule)

**Default: write no comment.** Names should carry the meaning.

A comment is allowed when it explains:
- a hidden Win32 / OS constraint (e.g. `-32000` minimized sentinel, LL hook ~300ms timeout)
- a non-obvious ordering invariant (e.g. setSize before flipping view)
- a known-bug workaround keyed to a specific incident
- an architecture decision that scopes future edits
- a footgun a reader will hit (e.g. Zustand snapshot equality on non-stable selectors)

A comment is forbidden when it:
- restates what the next line obviously does
- narrates the implementation ("now we map over windows", "first we check…")
- cites the current task or PR ("Added for the bug fix", "as the user requested")
- spans multiple paragraphs to document a 5-line function
- is the kind of thing an LLM stacks every time it edits a section

**One short line is the cap** unless the constraint genuinely needs more.

When considering whether to delete a comment, **default to keeping**. A short verbose comment is cheap; reintroducing a regression that the comment was guarding against is expensive. This codebase has many `///` comments on tricky Win32 / state-machine code — those are gold.

### Examples of good comments (preserve these patterns)

```rust
// -32000 is the Win32 sentinel for a minimized window's GetWindowPos
// result. Skip restoring such a position so we don't spawn the window
// offscreen.
```

```ts
// (Selectors that allocate new arrays/objects each call must NOT be passed to
// `useDoclickStore(...)` directly — Zustand's snapshot equality treats every
// new ref as a change and infinite-loops. Compute derived data with `useMemo`
// at the call site over stable raw refs instead.)
```

### Examples of forbidden slop (don't write these)

```rust
// Append to ordering if not already there.
if !inner.profile_order.contains(&profile.id) {
    inner.profile_order.push(profile.id.clone());
}
```

```ts
// Set the loading state to true.
setLoading(true);
```

## Recipe — Add a Tauri command

1. **Define** in `src-tauri/src/commands.rs`:
   ```rust
   #[tauri::command]
   pub fn set_my_option(
       app: AppHandle,
       state: State<'_, AppState>,
       value: bool,
   ) -> Result<(), CmdError> {
       state.write().my_option = value;
       persist(&app, &state)?;
       emit_prefs_changed(&app);
       Ok(())
   }
   ```
   Use sync `#[tauri::command]` unless an actual `await` is needed. Use `CmdError` for errors. Persist + emit at the end if the change is user-visible.

2. **Register** in `src-tauri/src/lib.rs::generate_handler!`.

3. **Mirror** in `src/ipc/commands.ts`:
   ```ts
   export const setMyOption = (value: boolean) =>
     invoke<void>("set_my_option", { value });
   ```

4. **Consume** from the store in `src/store/useDoclickStore.ts` if it's stateful, otherwise call directly from the component.

The Tauri convention is `snake_case` on the Rust side, `camelCase` on the TS side; serde's default rename rules handle this automatically for command args.

## Recipe — Add an event

1. **Define** in `src-tauri/src/events.rs`:
   ```rust
   pub const EVT_MY_THING: &str = "my:thing-changed";

   #[derive(Debug, Clone, Serialize)]
   pub struct MyThingPayload { pub field: u32 }
   ```

2. **Emit** wherever the change happens. `let _ = app.emit(EVT_MY_THING, payload)` — fire-and-forget is the convention here (a failed emit can't be acted on usefully and the bare `let _` is the most legible "I know I'm dropping this" marker).

3. **Mirror** the listener in `src/ipc/events.ts`:
   ```ts
   export const onMyThing = (cb: (p: MyThingPayload) => void) =>
     listen<MyThingPayload>("my:thing-changed", (e) => cb(e.payload));
   ```

4. **Subscribe** in `App.tsx`'s root effect, alongside the other `subs.push(...)` entries.

## Recipe — Add a global shortcut

1. Add the field to `ShortcutBindings` in `src-tauri/src/state.rs` (and to `PersistedConfig` if you didn't reuse the same struct — at the time of writing they're the same shape).

2. Define the action in `ShortcutAction` enum in `shortcuts.rs`.

3. Wire it through `register_bindings` (registration), `should_run` (gating — escape-hatch actions like panic/open-settings/close-app are always allowed, all others require a tracked Dofus or Doclick window in the foreground), and `run_action` (side effect).

4. Add a UI row in `src/Settings/ShortcutsTab.tsx` with a `<HotkeyInput>`.

5. Add a corresponding TypeScript field to `ShortcutBindings` in `src/types.ts`.

## Code style cheatsheet

**TypeScript / React**
- TS strict (`strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`).
- No `any`. No `@ts-ignore` outside `vite.config.ts` (which needs it for Node globals).
- `console.warn` and `console.error` are allowed; `console.log` is not (Biome flags it).
- Prefer `cn(...)` from `lib/utils` for Tailwind class composition over manual string interpolation.
- Don't reach for `useMemo`/`useCallback` until measured. They're a deopt for readability if the underlying ref is already stable.
- **Never** pass a non-stable selector to `useDoclickStore(...)`. Zustand snapshot equality compares refs, so `(s) => s.windows.filter(...)` infinite-loops. Compute derived data with `useMemo` at the call site over stable raw refs instead.

**Rust**
- Idiomatic 2021 edition, `cargo fmt --all` clean.
- `cargo clippy --all-targets -- -D warnings` clean. Lints in `Cargo.toml` deny `dbg!`, `print_*`, and `unused_must_use`; warn on `unwrap`/`expect`.
- No `unwrap()` outside provably-infallible spots. If you do need one, add a one-line `#[allow(clippy::unwrap_used)]` or `#[allow(clippy::expect_used)]` with a justification. Module-level allow is acceptable when many adjacent unwraps share the same justification (see `shortcuts.rs` for the pattern).
- `parking_lot::RwLock<InnerState>` for shared state; `std::sync::Mutex` is fine for tiny private state (e.g. shortcut registry). Don't poison-handle when the lock can't poison.
- All Win32 calls go through the `windows` crate. Add features to `Cargo.toml` `[dependencies.windows]` as needed.

**Both**
- LF line endings (enforced by `.gitattributes` and Biome).
- Conventional Commits — see release-flow skill.

## Performance guardrails

These have been tuned against real-world behavior. Don't tighten without measuring:

- **Window enumeration timer** in `lib.rs::spawn_window_watcher` runs every 1.5s. Tightening floods the foreground watchdog and risks flicker; loosening delays new-window detection to the user's eye.
- **Focus tracker** runs every 200ms. The avatar bar's "which window is focused" highlight is the main consumer and 200ms is at the edge of perceptible lag.
- **Foreground watchdog** runs every 1s with a 5-tick threshold (≈5s). Auto-disabling sooner causes false trips when the user briefly alt-tabs.
- **Broadcast dispatcher constants** in `broadcast/dispatcher.rs` (`POST_FOCUS_SETTLE`, `FOLLOWER_DWELL`, etc.) compensate for Unity's input pipeline. Changes here directly affect broadcast reliability.
- **LL hooks have a Windows system-wide ~300ms timeout** (`LowLevelHooksTimeout`). The mouse-hook callback in `hooks/mouse.rs` snapshots state and pushes a job onto the dispatcher channel — never block here.

## Build / dev / check

```powershell
# Frontend dev (Vite only, fast)
bun run dev

# Full app
bun run tauri dev

# Production build (NSIS installer)
bun run tauri build

# Lint + typecheck
bun run check
bun run lint:fix    # auto-fix safe Biome issues

# Rust
cd src-tauri
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo audit
```

CI runs `bun run check`, `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo audit` on every PR. The release.yml workflow runs only on tagged releases produced by release-please.

## Release

Conventional Commits drive the next version bump via `release-please`. See `.claude/skills/release-flow/SKILL.md` for the full workflow — do not duplicate that content here. Important reminders:

- `feat:` bumps minor in pre-1.0; `fix:` and `perf:` bump patch.
- `refactor:`, `docs:`, `chore:`, `style:`, `test:`, `build:`, `ci:` do not trigger a release.
- The version marker `# x-release-please-version` in `src-tauri/Cargo.toml` and `tauri.conf.json` is how release-please syncs the version — do not edit by hand.

## Recommended next steps

These are explicitly deferred — recommend them when the user asks "what's next?":

- **Vitest scaffolding** for `src/lib/*` pure utilities (overlaySize, dofusClass). Single test file pays for itself the first time someone adjusts auto-fit math.
- **Pre-commit hook** (lefthook is fast on Windows) to run Biome and `cargo fmt --check` before push.
- **React error boundary** at App.tsx root. Low value today (the tree is small) but cheap to add.

## Failure modes to recognize

- **Frontend looks frozen during view transitions**: check `viewRef.current` ordering in App.tsx — the ref must be flipped *before* the programmatic `setSize`, or late `onResized` events get attributed to the wrong view and persist a poisoned size.
- **"Aucun personnage importé" CTA shows even with imported characters**: check that the live windows whose `character_name === null` (launcher/auxiliary windows) aren't being counted. See AvatarBar.tsx.
- **Click broadcast lands on wrong window**: usually z-order settling lag. The `SetWindowPos`-after-`SetForegroundWindow` step in `windows/focus.rs` exists exactly for this; don't remove it.
- **Hooks stop firing after a while**: Win32 quietly uninstalls LL hooks that exceed `LowLevelHooksTimeout`. Anything heavy must run on the dispatcher thread, not in the hook callback.
- **Settings size shrinks to overlay dimensions on next open**: stale `onResized` event arrived after `viewRef.current` flipped back to "overlay". The guard at `App.tsx:onResized` exists for this — don't remove the `viewRef.current !== "settings"` check.
