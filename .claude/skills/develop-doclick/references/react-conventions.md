# React conventions

The frontend is React 19 + TypeScript (strict mode) + Vite + Tailwind 4 + shadcn/ui + Zustand. Code lives in `src/`.

## TypeScript

`tsconfig.json` has `strict: true`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`. ESLint enforces no `any`, no `as any`, no `// @ts-ignore`. `// @ts-expect-error` is allowed only with a description (`@typescript-eslint/ban-ts-comment` config).

Path alias `@/*` maps to `./src/*`. Use it in tests and configs; relative imports are fine inside the src tree.

JSDoc is the documentation convention. `///`-style triple-slash comments (Rust syntax that snuck in earlier) are not idiomatic TS — convert to `/**` JSDoc when you touch them.

## State management

Single Zustand store at `src/store/useDoclickStore.ts`. The store owns:

- The hydrated app state mirror (windows, profiles, settings) populated by `hydrate()` and event listeners.
- Action methods that wrap an IPC call + a local `set(...)` (the IPC call is the source of truth; the local set is an optimistic UI update).

**Never use raw `invoke('...')` in components or the store.** Every Tauri command goes through `src/ipc/commands.ts`. Adding a new command means adding the wrapper in the same PR — that's how the frontend stays type-safe across the IPC boundary.

### Selectors and re-renders

Zustand's snapshot equality treats a fresh `[]`/`{}` as a state change and infinite-loops the component. Two safe shapes:

```ts
// 1. Return a primitive — Zustand bails out when the number doesn't change.
const visibleCount = useDoclickStore((s) => s.windows.filter((w) => w.profile != null).length);

// 2. Return a stable raw ref, then derive with useMemo at the call site.
const windows = useDoclickStore((s) => s.windows);
const liveByProfileId = useMemo(() => {
  const m = new Map<string, WindowEntry>();
  for (const w of windows) if (w.profile) m.set(w.profile.id, w);
  return m;
}, [windows]);
```

Banned shape:

```ts
// Allocates a new Map every store change → infinite render loop.
const liveByProfileId = useDoclickStore((s) => {
  const m = new Map();
  for (const w of s.windows) if (w.profile) m.set(w.profile.id, w);
  return m;
});
```

This is documented inline in `useDoclickStore.ts` — the comment is intentional and worth keeping.

## Component organization

- `src/components/` — feature-agnostic UI building blocks (`AvatarBar`, `BroadcastToggle`, `CharacterChip`, `TitleBar`, `ResizeHandles`, `PanicIndicator`, `ErrorBoundary`, `VerticalOverlayChrome`).
- `src/components/ui/` — shadcn-generated primitives. These are auto-generated; the `react-refresh/only-export-components` warnings are known and accepted (shadcn co-exports config alongside the component).
- `src/Settings/` — settings tab implementations, one per file.
- `src/Settings.tsx` — the tab shell.
- `src/App.tsx` — the view router (`overlay` ↔ `settings`) + window management.

## Hooks and effects

The `react-hooks/exhaustive-deps` rule is on — keep dependency arrays honest. The `react-hooks/set-state-in-effect` rule is **off** in `eslint.config.js`: the "sync prop to state" pattern is used in a couple of places (`Settings.tsx`, `HotkeyInput.tsx`) where the prop is the only source of truth. The override is documented in the eslint config; don't switch it back on without rewriting those components.

Long imperative effects (the `enterSettings`/`exitSettings` flow, the resize-on-input effect, the settings-resize-persist effect) live in `App.tsx` because they share refs and the `view` state. Don't extract them into separate hooks unless you can also collapse the shared mutable refs — past attempts produced a hook signature with 4–5 setters and refs threaded through it, which was harder to reason about than the inline version.

## Styling

Tailwind 4 with a custom `oklch` theme defined in `src/index.css`. The theme tokens (`--background`, `--card`, `--muted`, `--ring`, `--primary`, etc.) are the source of truth for surface and text colors.

The chip palette (`CharacterChip.tsx`) intentionally uses bright accent colors (`ring-sky-400`, `bg-emerald-500`, `text-yellow-400`) that don't map onto theme tokens — these are deliberate UX signals (focus ring, online dot, main-character crown), not theming oversights. Don't replace them with `ring-ring` or similar; that changes the design.

Custom keyframes (`pulse-red` on the broadcast button live state) live in `src/index.css`. Add new ones there, not inline.

## Performance

The overlay renders a small tree (≤8 chips, a toggle, a title bar). `React.memo` is not used and isn't needed. `useMemo` is appropriate for the few derived maps in `CharactersTab.tsx` and `App.tsx`; don't sprinkle it elsewhere as cargo-cult.

No code splitting / lazy loading. The single overlay bundle (~350 KB minified) loads in <100ms. Adding `lazy()` for the settings tabs would save almost nothing and add complexity.

## Error handling

Top-level `<ErrorBoundary>` wraps `<App />` in `main.tsx`. It catches render-time exceptions and shows a "Reload overlay" recovery card. Async errors (event handlers, promises) bypass it — surface those via the store's `lastError` and the `PanicIndicator` component (existing pattern).

`.catch(() => {})` to swallow expected Tauri-IPC failures (e.g. window already closed, hot-reload race) is acceptable when the failure is genuinely informational. Anything that should surface to the user goes through `setError` on the store.

## Testing

Vitest with the jsdom environment. Test files sit next to the source: `foo.ts` → `foo.test.ts`. The pattern is to extract pure helpers from IO-heavy components so they're testable without a window:

- `src/lib/overlaySize.ts` — size math, fully tested.
- `src/lib/dofusClass.ts` — slug → display name, tested.
- `src/lib/resizePointer.ts` — pure resize geometry extracted from `ResizeHandles.tsx`, tested.

When a component does both pure computation and DOM/Tauri work, extract the pure part into `src/lib/` and unit-test that. Component tests for shadcn-driven UI haven't been worth the maintenance cost yet — Playwright would be the right tool if we ever go there.

See [testing.md](testing.md) for the testing approach in detail.

## Linting / formatting

```powershell
bun run typecheck
bun run lint
bun run format          # write
bun run format:check    # CI uses this
```

Prettier config: 100-col, double quotes, semicolons, trailing commas, LF endings. Settings live in `.prettierrc.json`, scope in `.prettierignore` (curated markdown like skill/release-flow files is excluded).
