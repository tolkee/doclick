# Architecture

doclick is a Windows-only Tauri 2 desktop overlay that broadcasts mouse clicks
and key presses from one Dofus 3 window to every other tracked Dofus window
on the same machine. The user always interacts with one "source" window — the
foreground one at click time — and doclick replays the input across the
followers via Win32 `SetForegroundWindow` + `SendInput`.

## Stack

| Layer | Tech |
| --- | --- |
| Frontend | React 19, TypeScript, Vite, Tailwind 4, shadcn/ui, Zustand |
| IPC | Tauri 2 commands + events |
| Backend | Rust 2021, parking_lot, crossbeam-channel, tokio (watchers) |
| Win32 | `windows` crate 0.61 (Foundation, WindowsAndMessaging, Input, HiDpi, Gdi, ProcessStatus) |
| Build | Bun (frontend), Cargo (backend), `bun run tauri dev/build` |

## Directory layout

```
src/                       React frontend
├── App.tsx                view router (overlay ↔ settings) + window mgmt
├── Settings.tsx           tabbed settings shell
├── Settings/<Tab>.tsx     per-tab UI
├── components/            UI building blocks (AvatarBar, BroadcastToggle, ResizeHandles, ...)
├── components/ui/         shadcn-generated primitives
├── store/useDoclickStore  Zustand store, owns hydrated app state
├── ipc/{commands,events}  thin typed wrappers over Tauri's invoke + listen
├── lib/                   pure utilities (overlaySize, dofusClass, resizePointer)
└── types.ts               IPC payload types

src-tauri/src/             Rust backend
├── lib.rs                 Tauri builder, watchers, app bootstrap
├── main.rs                entry point
├── state.rs               AppState (Arc<RwLock<InnerState>>) + serde types
├── config.rs              JSON persistence at app_data_dir/profiles.json
├── commands.rs            #[tauri::command] handlers (one IPC entrypoint per action)
├── events.rs              event names + payload structs + emit_or_log helper
├── shortcuts.rs           keyboard + mouse-button shortcut registration & dispatch
├── broadcast/
│   ├── mod.rs             BroadcastJob enum + tunable BroadcastTimings
│   ├── dispatcher.rs      focus-cycle + SendInput worker thread
│   └── translate.rs       proportional client-coord remapping (pure math + HWND wrapper)
├── hooks/
│   ├── mod.rs             dedicated message-pump thread, owns the LL hooks
│   ├── keyboard.rs        WH_KEYBOARD_LL callback (whitelisted broadcast)
│   └── mouse.rs           WH_MOUSE_LL callback (broadcast clicks + mouse shortcuts)
└── windows/               Win32 helpers
    ├── enumerate.rs       EnumWindows-driven Dofus discovery + title parsing
    ├── focus.rs           focus_window (SendInput Alt-tap + AttachThreadInput + SetForegroundWindow)
    └── geometry.rs        DPI awareness, virtual desktop coords, client/screen conversions
```

## Threads & concurrency

doclick runs on five concurrent contexts:

1. **Tauri main / UI thread** — runs the WebView, services all
   `#[tauri::command]` calls. Should never block on disk or Win32 input.
2. **Tokio runtime** — drives the three async watchers spawned in
   `lib.rs::run`: `spawn_window_watcher` (1.5s EnumWindows poll),
   `spawn_focus_tracker` (200ms GetForegroundWindow), and
   `spawn_foreground_watchdog` (1s loop that disables broadcast after 5s of
   non-Dofus / non-doclick foreground).
3. **Hook thread** — `hooks::install` spawns a dedicated `std::thread`
   that calls `SetWindowsHookExW` for both LL mouse and LL keyboard, then
   pumps Win32 messages. The hooks MUST run on this thread because LL hooks
   fire on the installer's thread, and Windows enforces a system timeout
   (`LowLevelHooksTimeout`, ~300ms). Long lock holds in the hook callbacks
   silently uninstall the hook — see `hooks/mouse.rs::ll_mouse_proc` for
   the snapshot-under-read-lock pattern.
4. **Dispatcher thread** — `broadcast::dispatcher::start` spawns a worker
   that `recv`s `BroadcastJob`s from a bounded `crossbeam-channel`. The
   worker owns the focus-cycling state machine: it does the
   pre-dispatch sleep, focuses each follower with retries + drift recovery,
   sleeps to let the message pump catch up, sends the synthetic input,
   then restores the original foreground window.
5. **Frontend** — single-threaded React with a small Zustand store. The
   store hydrates from `commands::get_state_snapshot` once on mount and
   then subscribes to events for incremental updates.

`AppState` is shared across all of these via `Arc<RwLock<InnerState>>`
(parking_lot, no poisoning). Read locks are held briefly inside callbacks
and watchers; write locks are held only inside command handlers and the
panic / auto-disable paths.

## Data flow: a single broadcast click

```
User left-clicks in the source Dofus window
 │
 ▼ WH_MOUSE_LL (hook thread)
hooks/mouse.rs::ll_mouse_proc
 ├─ snapshots {broadcast_enabled, known_hwnds} under one read lock
 ├─ checks foreground is a tracked window AND click is inside the client rect
 └─ try_enqueue(BroadcastJob::Click { source_hwnd, screen_x, screen_y })
 │
 ▼ crossbeam channel (bounded, capacity 4)
broadcast/dispatcher.rs::run (dispatcher thread)
 ├─ emit_or_log(EVT_BROADCAST_TICK, Started)
 ├─ for each follower hwnd from state.broadcast_targets(source_hwnd):
 │    ├─ translate_click(source, target, x, y)  → translate.rs (pure math)
 │    ├─ focus_with_retries(target_hwnd, timings)  → windows/focus.rs
 │    ├─ drift recovery loop
 │    ├─ sleep(post_focus_settle)
 │    └─ SendInput(MOVE) → sleep → SendInput(LDOWN) → sleep → SendInput(LUP)
 ├─ focus_window(original_fg) to restore
 └─ emit_or_log(EVT_BROADCAST_TICK, Finished {ok, failed})
 │
 ▼ Tauri event channel (UI thread)
src/App.tsx onBroadcastTick listener
 └─ store.setBroadcastLive(true|false)  → triggers BroadcastToggle re-render
```

The whole cycle is gated on the foreground watchdog: if the foreground
hasn't been a tracked Dofus window or a doclick window for 5 consecutive
1s ticks, `lib.rs::spawn_foreground_watchdog` flips `broadcast_enabled`
to `false` and emits `BroadcastReason::AutoDisabledForegroundMismatch`.

## Persistence

Configuration is stored as a single JSON file at
`<app_data_dir>/profiles.json`, written through a temp-file rename in
`config::save` so a crashed write never leaves a corrupted file. Schema
lives in `config::PersistedConfig`; legacy fields use `#[serde(default)]`
or `#[serde(alias = "...")]` to keep older config files readable across
upgrades (e.g. `Role::main` → `Role::Follower`).

The frontend never writes to disk; every persisted change goes through
a Tauri command which calls `commands::persist`.

## Tauri configuration notes

- The overlay is a single window, label `"overlay"`, with
  `decorations: false`, `transparent: true`, `alwaysOnTop: true`, and
  `skipTaskbar: true`. Resize handles are drawn by `ResizeHandles.tsx`
  because Windows draws no native chrome at this configuration.
- `tauri.conf.json` sets `"csp": null` deliberately. The overlay does
  not load remote content; CSP would only restrict already-bundled
  assets. Document any future remote-asset addition here before flipping
  CSP on.
- The release build is configured for size + speed in `Cargo.toml`'s
  `[profile.release]` (LTO fat, opt-level 3, codegen-units 1, strip,
  panic abort). NSIS installer is produced by the existing release-please
  → tauri-build pipeline in `.github/workflows/release.yml`.
