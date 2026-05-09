# CLAUDE.md

Always-loaded context for agents working on this repo. The deeper rules live in the `develop-doclick` skill — this file is the index.

## What this is

Doclick is a Windows-only Tauri 2 + React 19 overlay for Dofus 3. It broadcasts clicks and keystrokes from one tracked Dofus window to others. Pure user-input simulation via Win32 public APIs: no memory injection, no packet inspection, no Dofus-specific automation.

## Architecture in one paragraph

The Rust host (`src-tauri/`) owns shared state, Win32 input hooks, the broadcast dispatcher, and a global-shortcut registry. The React client (`src/`) renders the always-on-top overlay bar and the settings window. They communicate over Tauri's typed IPC: commands in `commands.rs` ↔ `src/ipc/commands.ts`, events in `events.rs` ↔ `src/ipc/events.ts`. Profiles persist to `profiles.json` in `app_data_dir` via `config.rs`.

## Module map

```
src/
  App.tsx                  Overlay/settings view orchestration, window-size effects
  Settings.tsx             Tab shell (Global / Caractères / Raccourcis / À propos)
  Settings/                Per-tab forms
  Menu.tsx                 Standalone kebab-menu Tauri window
  components/              UI primitives (BroadcastToggle, AvatarBar, ResizeHandles, …)
  store/useDoclickStore.ts Zustand store + IPC-backed actions
  ipc/                     Typed `invoke` / `listen` wrappers
  lib/                     Pure utilities (overlaySize, dofusClass, cn)

src-tauri/src/
  lib.rs                   Tauri builder, async tasks (window watcher / focus tracker / FG watchdog)
  state.rs                 AppState (Arc<RwLock<InnerState>>) + snapshot helpers
  config.rs                Persistence (profiles.json + atomic rename)
  commands.rs              All #[tauri::command] entry points
  events.rs                Event payload types + EVT_* constants
  shortcuts.rs             Global keyboard + mouse shortcut registry & dispatch
  diagnostics.rs           Panic hook → app_data_dir/crashes/<ts>.log
  hooks/                   WH_MOUSE_LL / WH_KEYBOARD_LL on a dedicated thread
  broadcast/               Focus-cycle dispatcher + proportional coord translation
  windows/                 Enumerate / focus / geometry helpers
```

## Comment philosophy

Default: **write no comment.** Names should carry the meaning.

Allowed comments explain:
- a hidden constraint (e.g. Win32 `-32000` minimized sentinel)
- a non-obvious invariant (e.g. ordering: setSize before flipping view)
- a known-bug workaround
- an architecture decision that scopes future edits
- a footgun a reader will hit (e.g. Zustand snapshot equality)

Forbidden:
- restating what the next line does
- narrating the implementation
- citing the current task or PR ("Added for X")
- multi-paragraph docstrings
- comments that an LLM stacks every time it edits a section

When in doubt, prefer keeping a short verbose comment to deleting it. Re-introducing the bug it guarded against costs more than the reading time.

## Dev commands

```powershell
# Frontend dev (Vite only)
bun run dev

# Full app (Tauri + Vite)
bun run tauri dev

# Production build (NSIS installer)
bun run tauri build

# Lint + typecheck
bun run check

# Auto-fix safe issues
bun run lint:fix
```

```powershell
# Rust
cd src-tauri
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo audit
```

## What never to change without explicit user approval

- Visible UX (layouts, sizes, copy, color, motion).
- Persistence schema (`profiles.json` shape) — must remain readable across versions.
- IPC wire format (Tauri command params, event payload fields).
- Shortcut accelerator strings exposed to users.
- The async timers in `lib.rs` (window watcher 1.5s, focus tracker 200ms, foreground watchdog 1s) — they balance responsiveness against Win32 hook timeouts.

## Skills

- `.claude/skills/develop-doclick/` — full conventions, recipes, code style cheatsheet. Loads automatically when editing `src/` or `src-tauri/`.
- `.claude/skills/release-flow/` — Conventional Commits + release-please workflow.
