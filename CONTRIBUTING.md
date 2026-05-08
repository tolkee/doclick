# Contributing

Quick reference for development. The full architecture lives in
[ARCHITECTURE.md](./ARCHITECTURE.md).

## Prerequisites

- Windows 10/11 (the codebase is Win32-bound; macOS/Linux can't compile it).
- [Rust stable](https://rustup.rs/) — `rustup component add rustfmt clippy`.
- [Bun 1.3+](https://bun.sh/) for the frontend.
- A Dofus 3 install if you want to test broadcast end-to-end.

## Develop

```powershell
bun install
bun run tauri dev
```

This starts Vite on port 1420 and launches the Tauri webview with hot
reload. The Rust side rebuilds incrementally on save.

## Test

```powershell
# Rust
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend
bun run test
```

Both run in CI (`.github/workflows/ci.yml`).

## Lint & format

```powershell
# Rust
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# Frontend
bun run typecheck
bun run lint
bun run format        # write
bun run format:check  # CI uses this
```

Configuration lives in `rustfmt.toml`, `eslint.config.js`, `.prettierrc.json`.

## Build a release artifact locally

```powershell
bun run tauri build
```

Produces an NSIS installer at
`src-tauri/target/release/bundle/nsis/doclick_<version>_x64-setup.exe`.

## Commits

doclick uses [Conventional Commits](https://www.conventionalcommits.org/)
plus release-please. The full flow is documented in the
[release-flow](./.claude/skills/release-flow/SKILL.md) skill — read it
before opening a PR. Quick rules:

- `feat:` minor bump, `fix:` / `perf:` patch bump (pre-1.0, breaking
  changes are still minor).
- `chore:`, `refactor:`, `docs:`, `style:`, `test:`, `ci:`, `build:` do
  not bump versions and do not appear in the CHANGELOG.
- One concern per PR. Squash-merge with a Conventional title.
- Don't edit `package.json`, `Cargo.toml`, `tauri.conf.json`, or
  `.release-please-manifest.json` versions by hand. release-please owns
  them.
- Don't push tags by hand. Merging the Release PR is what produces the tag.

## Working with Claude Code in this repo

The repo ships a `develop-doclick` skill at
`.claude/skills/develop-doclick/SKILL.md` that documents project
conventions (no-panic IPC paths, parking_lot over std::sync, typed Tauri
SDK in the frontend, comments policy, etc.). It auto-triggers when you
edit `src/`, `src-tauri/`, or `.github/workflows/`. Read it once; future
agent runs in this repo will pull it in automatically.

## Worktrees

Long-running refactors are best done in a `git worktree` so the main
checkout keeps building. Claude's worktrees live under `.claude/worktrees/`
and are tracked in `.gitignore`.
