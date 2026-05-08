# Testing

doclick has two test surfaces: `cargo test` for the Rust backend and Vitest for the frontend. Both run in CI on every PR. The strategy is unit-first — test pure helpers exhaustively, skip integration tests that need a live Tauri runtime or a real Win32 window.

## Rust

### Where tests live

Tests sit in a `#[cfg(test)] mod tests` block at the **end** of each Rust file (clippy's `items_after_test_module` lint enforces this). New file → tests at the bottom.

```rust
// src/state.rs

pub struct AppState(...);
impl AppState { ... }
// ...all production code...

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_window_title_substring() { ... }
}
// EOF
```

### What to test

The 29 tests today cover:

- **`state::matches_window`** — substring + PID matching, empty needle never matches, case sensitivity.
- **`AppState::broadcast_targets` / `all_hwnds` / `ordered_visible_hwnds`** — exclusion rules, profile-order respect, edge cases (no followers, leader not found).
- **`config::save` / `config::load` round-trip** — full payload via `tempfile::NamedTempFile`, legacy `Role::main` coercion, malformed-JSON fallback.
- **`broadcast::translate::translate_proportional`** — same-size, scaling up/down, rounding, the zero-size guard.
- **`shortcuts::parse_shortcut`** — keyboard accelerators, mouse triggers, modifiers, rejection of unknown tokens.
- **`windows::enumerate::parse_dofus_class` + `parse_character_name`** — title parsing, accent folding, segment count requirement.

The pattern: take a function with side effects, extract the pure math/parsing, test that. The HWND-using `translate_click` is one line of plumbing on top of the pure `translate_proportional` — that's the testable one.

### What we don't test

- Anything that requires a real `tauri::AppHandle` (no fixture for it).
- The hook callbacks (LL hooks fire only on a real message-pump thread).
- The dispatcher worker's focus-cycle loop (depends on `SetForegroundWindow` against real windows).
- Anything that touches `windows/focus.rs::focus_window` — the `prime_focus_change_rights` Alt-tap is observable globally.

If a piece of logic *should* be tested but currently sits inside a Win32 wrapper, refactor it to expose the pure part — that's how `translate_proportional` came out of `translate_click`.

### Dev dependencies

`tempfile` is already in `[dev-dependencies]` for filesystem-touching tests. Add new test-only deps there, not under main `[dependencies]`.

### Running

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

CI runs this on `windows-latest`. Linux runners can't even compile the Win32-bound code, so no matter how pure the test, it has to ride the Windows job.

## Frontend

### Where tests live

Test files sit next to the source: `foo.ts` → `foo.test.ts`. Vitest auto-discovers `src/**/*.{test,spec}.{ts,tsx}`.

```
src/lib/
├── overlaySize.ts
├── overlaySize.test.ts
├── dofusClass.ts
├── dofusClass.test.ts
├── resizePointer.ts
└── resizePointer.test.ts
```

### What to test

Pure utilities only — no Tauri mocking, no Zustand store wiring. Today's coverage:

- **`overlaySize`** — orientation-driven size derivation, the empty-state floor, saved-size clamping.
- **`dofusClass`** — slug → display name, unknown class fallback, missing-class handling.
- **`resizePointer`** — the geometry math extracted from `ResizeHandles.tsx` (East/West shifts x, North/South shifts y, min clamp anchors the opposite edge).

### What we don't test

- Components that wire IPC + Zustand + DOM (would need extensive Tauri mocking; ROI is low at the current size).
- `App.tsx` itself — it's mostly an imperative controller for window state, hard to test without a Tauri runtime.
- shadcn-generated components in `src/components/ui/` — they're vendor code.

If a future regression motivates a component test, use `@testing-library/react` (already a devDep) with the jsdom environment configured in `vitest.config.ts`. Keep them focused on a single behaviour — long render-then-click-then-assert tests rot fast.

### Adding a frontend test

1. If the logic is buried inside a component, extract the pure part to `src/lib/<name>.ts` first. The test goes next to the new module.
2. Cover the obvious cases plus one or two edge cases (zero, negative, off-by-one boundaries).
3. Run `bun run test` locally before pushing.

### Running

```powershell
bun run test          # one-shot
bun run test:watch    # interactive
```

CI runs `bun run test` on `ubuntu-latest`. The frontend tests are platform-independent so they ride the Linux job for speed.

## What CI gates

`.github/workflows/ci.yml` runs on every PR + push to main:

- **Rust** (windows-latest): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **JS** (ubuntu-latest): `bun run typecheck`, `bun run lint`, `bun run format:check`, `bun run test`.

A red CI is a blocker. Run the equivalent commands locally before opening the PR — fast feedback is much cheaper than a CI round-trip.

## What CI does NOT gate

- Manual smoke testing of the actual broadcast (requires a Dofus install).
- The NSIS installer build (lives in the separate `release.yml` and only runs when a release-please PR merges).
- Visual regressions in the overlay (no screenshot tests today).

When in doubt, do a `bun run tauri dev` smoke run on the changed feature before requesting review.
