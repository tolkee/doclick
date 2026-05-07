# doclick

Minimalist always-on-top overlay for Dofus 3 (Unity) team play. Toggle
broadcast, click on whichever Dofus window is on top, and the same click is
replayed on every other tracked Dofus window &mdash; useful for plowing
through per-account NPC dialogs, bank menus, and HUD interactions without
alt-tabbing eight times.

> **Scope.** Click broadcast, key broadcast, basic window organizer. Dofus 3
> already gives you combat auto-focus, auto-follow, and auto-join &mdash;
> doclick doesn't try to replace those. Use them for movement; use doclick
> for per-account UI clicks.

## Disclaimer

doclick is an **alt-tab + click simulator**, like a macro. It runs entirely
on Windows public APIs &mdash; no process injection, no memory hooks, no
packet reading, no interaction with the Dofus client beyond the synthetic
input you would otherwise produce yourself.

> **PvP warning.** Broadcasting input in **Kolossium**, **Prism**, or
> **Alliance-vs-Alliance** PvP violates Ankama's Terms of Use. doclick can't
> detect those modes. Disable broadcast (panic hotkey) before entering.

## Download &amp; install

Grab the latest installer from the
[Releases](https://github.com/tolkee/doclick/releases/latest) page and run
it. The installer is an unsigned NSIS `.exe`; on first launch Windows
SmartScreen will warn that the publisher is unknown &mdash; click **More
info** &rarr; **Run anyway**. The installer bundles the WebView2 bootstrapper
and will fetch the runtime if it isn't already present (Windows 11 ships it
preinstalled).

A code-signing certificate is on the long-term roadmap; until then the
SmartScreen prompt is unavoidable.

## How it works

Dofus 3 is a Unity client and ignores Win32 `PostMessage`/`SendMessage` for
input, so doclick uses the only thing Unity *does* respect: synthetic input
via `SendInput` &mdash; which only goes to the foreground window. To reach
each follower, doclick rapidly cycles `SetForegroundWindow` through the
team, fires the input, and returns focus to your main. Each follower costs
~30&ndash;50 ms; an 8-account team round-trip is ~300 ms. Fine for menus
and dialogs, **not** fast enough for real-time combat.

## Build from source

Prerequisites:

- Windows 10 / 11
- [Bun](https://bun.sh) (1.3+)
- Rust (stable, MSVC toolchain) &mdash; install from <https://rustup.rs>
- Microsoft C++ Build Tools (Tauri prerequisite)
- WebView2 runtime (preinstalled on Windows 11)

Build:

```powershell
bun install
bun run tauri build
```

The NSIS installer lands at
`src-tauri/target/release/bundle/nsis/doclick_<version>_x64-setup.exe`.

## Develop

```powershell
bun install
bun run tauri dev
```

The first `cargo build` will take several minutes while it compiles the
`windows` crate. Subsequent runs are incremental.

### Layout

```
src/                  React UI (overlay bar + settings window)
src-tauri/            Rust backend
  src/
    state.rs          shared AppState
    config.rs         JSON profile persistence
    commands.rs       Tauri command surface
    events.rs         event payload types
    windows/          enumerate / geometry / focus (SetForegroundWindow trick)
    hooks/            WH_MOUSE_LL / WH_KEYBOARD_LL on a dedicated thread
    broadcast/        focus-cycle dispatcher + proportional coord translation
```

### Stack

- Tauri 2 (Rust + Vite + React 19 + TypeScript)
- Tailwind v4 for styling, Zustand for state
- Win32 via the `windows` crate

### Risks / things to verify on a live install

These are unknowns the implementation guesses at; verify with Spy++ /
Process Explorer once you have a Dofus 3 install:

1. **Process &amp; window class names.** `enumerate.rs` looks for
   `Dofus.exe` / `DofusInvoker.exe`. If the launcher wraps a child with a
   different name, add it there.
2. **`SetForegroundWindow` reliability on Windows 11 22H2+.** The
   `AttachThreadInput` trick + Alt-tap workaround usually wins, but there
   can be edge cases. If broadcast misses windows, click into Dofus once
   after launching doclick to prime focus state.
3. **Scancode vs virtual key.** doclick sends `KEYEVENTF_SCANCODE`; Unity
   generally reads scancodes in preference. If a dialog ignores keys, try
   sending VK without `KEYEVENTF_SCANCODE` for that case.

## Contributing

Issues and pull requests are welcome. A few ground rules:

- Windows 10 / 11 only &mdash; cross-platform changes will be closed.
- doclick stays a click/key simulator. No memory reading, no packet
  inspection, no Dofus-specific automation that goes beyond replaying user
  input.
- Keep the dependency footprint small.
- Code submitted to this repo must be compatible with the project's
  noncommercial license (see below).

## License

[PolyForm Noncommercial 1.0.0](./LICENSE). Personal and non-commercial use,
modification, and redistribution are permitted. Commercial use is
prohibited.
