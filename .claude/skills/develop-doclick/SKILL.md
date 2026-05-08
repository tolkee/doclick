---
name: develop-doclick
description: Use when writing, refactoring, or reviewing code in the doclick repo (a Tauri 2 + React 19 Windows overlay for Dofus 3 multi-account broadcast). Covers Rust backend conventions (no panics in IPC paths, parking_lot over std::sync, HWND validation before unsafe Win32, logged event emission), Tauri command/event patterns, React/Zustand patterns (typed IPC SDK only, no raw invoke, stable selectors), the project's strict comments policy (default off, only architectural/algorithmic/invariant comments survive), and the testing approach (pure helpers extracted from IO, Vitest for frontend, cargo test for Rust). Triggers on any code change touching `src/`, `src-tauri/`, or `.github/workflows/` — including new features, bug fixes, refactors, dependency bumps, and config edits — even if the user doesn't explicitly mention conventions. Use it before writing code in this repo so you don't reinvent patterns or violate guardrails the team has already settled on.
---

# develop-doclick

Conventions for working in the doclick codebase. doclick is a Windows-only Tauri 2 desktop overlay that broadcasts mouse/keyboard input from a "source" Dofus 3 window to every other tracked Dofus window via Win32 `SetForegroundWindow` + `SendInput`. Architecture overview lives in [ARCHITECTURE.md](../../../ARCHITECTURE.md); commit and release rules live in the [release-flow](../release-flow/SKILL.md) skill.

This skill is the conventions hub. The reference files in `references/` go deep on each area — load the one that matches what you're touching.

## Top-line rules (apply everywhere)

1. **Don't change UX without a clear ask.** Features have been validated end-to-end. Refactor freely; rename labels, change shortcuts, or alter broadcast behaviour only when the user requests it.
2. **No panics in production paths.** `unwrap`/`expect` are fine in tests and in code with a written invariant explaining why the panic can't happen. Anywhere else — IPC handlers, watchers, hook callbacks, dispatcher worker — propagate the error or log and recover.
3. **Validate untrusted IPC inputs before unsafe Win32.** HWNDs, PIDs, and other handles cross the WebView ↔ Rust boundary as raw integers. Run `IsWindow`/equivalent before any `unsafe { ... }` Win32 call that consumes them.
4. **`parking_lot::Mutex`/`RwLock` over `std::sync::*`.** `parking_lot` is already a dep, doesn't poison, and is faster. Drop `.unwrap()` on `.lock()` once migrated.
5. **Never silently drop `app.emit` failures.** Use `events::emit_or_log` (Rust) so emission failures land in `tracing::warn!` instead of `let _ =`.
6. **React: only call Tauri through the typed SDK in `src/ipc/`.** No raw `invoke('command_name', ...)` strings in components or the store.
7. **Zustand selectors return primitives or stable refs.** Allocating a new array/object inside `useDoclickStore((s) => …)` re-renders forever. Compute derived data with `useMemo` at the call site over stable raw inputs (see `AvatarBar` for the canonical pattern).
8. **Comments off by default.** A comment exists only when it documents a non-obvious architectural decision, a complex algorithm, a subtle invariant/pitfall, or a public API (rustdoc / JSDoc on exports). Never narrate the implementation, reference the current PR/fix, or leave `// removed X` markers. See [comments-policy.md](references/comments-policy.md) for keep/delete examples.
9. **Tests live in `#[cfg(test)] mod tests` at the END of the Rust file.** Clippy's `items_after_test_module` lint enforces this — putting tests in the middle breaks CI.
10. **Conventional Commits, one concern per PR.** Squash-merge with a Conventional title. The [release-flow](../release-flow/SKILL.md) skill has the full bump-and-changelog rules.

## Decision tree — what to read next

Pick the reference based on what you're editing:

| If you are editing… | Load this reference |
| --- | --- |
| anything under `src-tauri/src/**` | [rust-conventions.md](references/rust-conventions.md) |
| `src-tauri/src/commands.rs`, `src-tauri/src/events.rs`, `src-tauri/tauri.conf.json`, capabilities | [tauri-conventions.md](references/tauri-conventions.md) |
| anything under `src/**` (React, Zustand, Tailwind) | [react-conventions.md](references/react-conventions.md) |
| any source file's comments — Rust OR TypeScript | [comments-policy.md](references/comments-policy.md) |
| adding tests (Rust or frontend), or refactoring to make code testable | [testing.md](references/testing.md) |
| `.github/workflows/ci.yml` | both [rust-conventions.md](references/rust-conventions.md) and [react-conventions.md](references/react-conventions.md) — keep CI in sync with the local toolchain |
| anything that produces a commit | the [release-flow](../release-flow/SKILL.md) skill |

If you're touching multiple areas, read each relevant reference. They're each ~100–200 lines and designed to be skimmed.

## What this skill is NOT

- It's not a tutorial on Rust, React, Tauri, or Win32. Assume the reader has working knowledge.
- It's not a complete API reference. Inline rustdoc on `AppState` and JSDoc on the IPC SDK are authoritative for signatures.
- It does not cover the release pipeline (commit format, version bumps, NSIS build, GitHub release flow). That's [release-flow](../release-flow/SKILL.md).

## Quick-validate before opening a PR

```powershell
# Rust
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend
bun run typecheck
bun run lint
bun run format:check
bun run test
```

CI runs the same set on every PR (`.github/workflows/ci.yml`). Keep them green locally first — CI failures on lint/format are pure friction.
