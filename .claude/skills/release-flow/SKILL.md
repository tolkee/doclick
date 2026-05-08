---
name: release-flow
description: Use when committing or merging code in the doclick repo. doclick uses Conventional Commits + release-please to auto-bump versions, generate CHANGELOG entries, and ship signed GitHub releases of the NSIS installer. Triggers on any work that produces a commit (writing code, fixing bugs, refactors, docs, dependency bumps), and on questions about how to cut a new release.
---

# Release flow

doclick releases are fully automated by a single workflow at `.github/workflows/release.yml`. It runs on every push to `main` (and via `workflow_dispatch`) with two sequential jobs:

| Job | Runs on | What it does |
|---|---|---|
| `release-please` | `ubuntu-latest` | Maintains an open "Release PR" that bumps versions + writes CHANGELOG. When a Release PR merges, creates a **draft** GitHub Release at `v<version>` (`draft: true` in `release-please-config.json`). The actual git tag is created when the release is published — see the build job. |
| `build-windows` | `windows-latest` (only when `release-please` set `release_created == 'true'`) | Builds the NSIS installer with `bun run tauri build`, uploads the `.exe` to the draft release with `gh release upload`, then flips it to published with `gh release edit --draft=false` — which is the moment GitHub creates the `v<version>` git tag. One release per tag, with the installer attached at publish time. |

The two jobs live in one workflow file deliberately: GitHub blocks `GITHUB_TOKEN`-pushed events from triggering separate workflows, so a tag-triggered build job never fires when release-please pushes the tag. Job-to-job `needs:` chaining sidesteps this.

The single human action is **using Conventional Commits**. Everything else is automatic.

## How a release happens

1. You merge PRs into `main` with Conventional Commit messages (or with a Conventional title if you squash-merge).
2. `release-please-action` re-runs on every push to `main` and (re-)opens a PR titled `chore(main): release <next-version>`. That PR:
   - Bumps the version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
   - Updates `CHANGELOG.md` with grouped entries from the commits since the last release.
   - Updates `.release-please-manifest.json`.
3. You review and merge the Release PR.
4. `release.yml` fires on the same push (job chained via `needs:`). The `release-please` job creates a draft GH Release for `v<version>`; `build-windows` then builds the NSIS `.exe`, uploads it to that draft, and publishes the release — which is when the git tag becomes a real ref. The release goes from invisible-draft to published-with-installer in a single workflow run.

## Conventional Commit cheat sheet

The commit type drives the version bump. Pre-1.0.0, breaking changes bump the **minor** version (not major) — this is release-please's default for 0.x versions.

| Prefix | Bump | Appears in CHANGELOG | Example |
|---|---|---|---|
| `feat:` | minor | "Features" | `feat: add per-character delay slider` |
| `fix:` | patch | "Bug Fixes" | `fix: focus loss on Win11 22H2` |
| `feat!:` / `BREAKING CHANGE:` footer | minor (pre-1.0) | "Breaking Changes" | `feat!: drop legacy SendMessage path` |
| `perf:` | patch | "Performance" | `perf: cache window enumeration` |
| `revert:` | patch | "Reverts" | `revert: focus cycle change` |
| `docs:`, `chore:`, `style:`, `refactor:`, `test:`, `build:`, `ci:` | none | not shown by default | `chore: bump windows crate to 0.62` |

A scope is optional but recommended for cross-cutting code: `feat(broadcast): ...`, `fix(hooks): ...`, `chore(deps): ...`.

## Rules of thumb when writing commits / PR titles

- **Squash-merge with a Conventional PR title.** That title becomes the single commit on `main` and feeds CHANGELOG.
- **One concern per commit/PR.** If you bundle a feat + a fix, only one will show up in the right CHANGELOG section.
- **User-facing language.** The CHANGELOG ships to users — write `fix: prevent broadcast in Kolossium` not `fix: nullcheck on line 42`.
- **No version bumps by hand.** Don't edit `package.json`, `Cargo.toml`, `tauri.conf.json`, or the manifest manually. release-please owns them.
- **No manual tags.** Merging the Release PR + the build job publishing the draft is what creates the tag.

## Cutting a release as a maintainer

Normal path:
1. Merge feature PRs into `main` with Conventional titles.
2. Wait for the Release PR to update (it usually does within ~1 min).
3. Review the proposed CHANGELOG diff. If something looks wrong (e.g. a `chore:` got grouped weirdly), fix the commit history with a follow-up PR before merging the Release PR.
4. Merge the Release PR.
5. Watch `release.yml` build the installer (~5–10 min on `windows-latest`). The release auto-publishes once the upload step finishes.
6. Verify the published release has the `.exe` attached. If you want to add release notes beyond what release-please generated, edit them on GitHub after publish.

Manual override (rare):
- To force a release on a specific date even with no `feat`/`fix` commits, push an empty commit: `git commit --allow-empty -m "fix: prepare release"`.
- To force a major bump pre-1.0.0, add `Release-As: 1.0.0` in a commit footer.

## Common pitfalls

- **`x-release-please-version` annotation in `src-tauri/Cargo.toml`.** The version line ends with `# x-release-please-version` — that's how release-please's generic updater finds it. **Do not strip the comment.** If the line ever gets reformatted (e.g. `cargo fmt` or an auto-edit), version bumps stop working silently. release-please's TOML support doesn't extend to `extra-files` in v17, hence the comment-based annotation.
- **`Cargo.lock` is not auto-bumped.** Cargo regenerates the workspace version field on `cargo build`, so the CI release build produces a consistent binary. The merged main branch will briefly carry a stale `Cargo.lock` until the next local build commits an update — harmless.
- **`bun.lock`** does not encode the workspace version, so it doesn't need bumping. If a future Bun version starts encoding it, drop `--frozen-lockfile` from `release.yml` and add `bun.lock` as an extra-file.
- **Release PR doesn't appear after merging to `main`.** Check that the merged PR's title (or the squashed commit message) starts with a Conventional prefix. `release-please` ignores non-Conventional commits — they don't trigger a release.
- **Tag created but no release artifact.** The `release.yml` workflow needs `contents: write` (it does). If the Windows runner fails, check Bun + Rust setup steps; the cache key may need busting. If the build succeeded but the release stayed in draft, check the `gh release edit --draft=false` step's logs.
- **Never set `skip-github-release: true`.** In release-please-action v4 it skips both the GitHub Release **and** the tag push, breaking the build trigger. The 0.6.0 release was lost this way once. Use `draft: true` if you want the release hidden until the build finishes.

## Files that own this flow

- [`release-please-config.json`](../../../release-please-config.json) — package definition + extra-files list
- [`.release-please-manifest.json`](../../../.release-please-manifest.json) — current version per package
- [`.github/workflows/release.yml`](../../../.github/workflows/release.yml) — single workflow with two jobs: release-please (always runs on push to main), then build-windows (runs only when `release_created == 'true'`)

## When NOT to use this flow

- Hot-fixing a release that's already published. Cherry-pick to a branch, manually tag `v0.x.y`, push the tag — the existing `release.yml` will build it without needing release-please. release-please will catch up on the next normal commit.
