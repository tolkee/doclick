import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef, useState } from "react";
import { AvatarBar } from "./components/AvatarBar";
import { BroadcastToggle } from "./components/BroadcastToggle";
import { KebabButton } from "./components/KebabButton";
import { PanicIndicator } from "./components/PanicIndicator";
import { ResizeHandles } from "./components/ResizeHandles";
import { VerticalOverlayChrome } from "./components/VerticalOverlayChrome";
import { saveOverlayPosition } from "./ipc/commands";
import {
  onBroadcastState,
  onBroadcastTick,
  onError,
  onFocusedWindowChanged,
  onOpenSettings,
  onPrefsChanged,
  onUpdateProgress,
  onUpdateState,
  onWindowsChanged,
} from "./ipc/events";
import {
  computeOverlayMinSize,
  computeOverlaySize,
  isValidSettingsSize,
  presetOf,
  SETTINGS_DEFAULT_SIZE,
  SETTINGS_MIN_SIZE,
} from "./lib/overlaySize";
import Settings, { type SettingsTabId } from "./Settings";
import { useDoclickStore } from "./store/useDoclickStore";
import type { Orientation } from "./types";

type View = "overlay" | "settings";

export default function App() {
  const hydrate = useDoclickStore((s) => s.hydrate);
  const moveDebounce = useRef<number | null>(null);

  const [view, setView] = useState<View>("overlay");
  const [settingsTab, setSettingsTab] = useState<SettingsTabId>("global");
  // Mirrored in a ref for closures (event listeners, debounced callbacks)
  // that capture stale state. Transition handlers update it before resizing
  // so late Tauri resize events are attributed to the destination view.
  const viewRef = useRef<View>("overlay");
  useEffect(() => {
    viewRef.current = view;
  }, [view]);

  const enterSettings = useCallback(async (tab: SettingsTabId = "global") => {
    setSettingsTab(tab);
    if (viewRef.current === "settings") return;
    const win = getCurrentWindow();
    try {
      // Apply size BEFORE flipping view so the settings UI never paints
      // at overlay dimensions (no flash). Order: enable resize so the
      // current overlay min doesn't clamp, set new min, then set size.
      // Reject saved sizes smaller than SETTINGS_MIN_SIZE — they can only
      // come from poisoned cache (older builds occasionally persisted the
      // overlay's dimensions as settings_size on view transitions).
      const saved = useDoclickStore.getState().settingsSize;
      const target: [number, number] = isValidSettingsSize(saved)
        ? (saved as [number, number])
        : [SETTINGS_DEFAULT_SIZE.width, SETTINGS_DEFAULT_SIZE.height];
      await win.setResizable(true);
      await win.setMinSize(new LogicalSize(SETTINGS_MIN_SIZE.width, SETTINGS_MIN_SIZE.height));
      await win.setSize(new LogicalSize(target[0], target[1]));
      viewRef.current = "settings";
      setView("settings");
      await win.setFocus();
    } catch (err) {
      console.warn("enterSettings failed", err);
    }
  }, []);

  const exitSettings = useCallback(async () => {
    if (viewRef.current === "overlay") return;
    // Mark the destination view before the programmatic shrink. The settings
    // resize listener may still be subscribed for a moment, and must not
    // persist the overlay's thin dimensions as settings_size.
    viewRef.current = "overlay";
    const win = getCurrentWindow();
    const state = useDoclickStore.getState();
    const orientation = state.orientation;
    const scale = state.overlayScale;
    const visibleCount = state.windows.filter((w) => w.profile != null).length;
    const min = computeOverlayMinSize(orientation, scale);
    const size = computeOverlaySize({
      orientation,
      scale,
      visibleCount,
      savedMainAxis: savedMainAxis(state.overlaySizes, orientation),
    });
    try {
      await win.setMinSize(new LogicalSize(min.width, min.height));
      await win.setSize(new LogicalSize(size.width, size.height));
      await win.setResizable(false);
    } catch (err) {
      console.warn("exitSettings failed", err);
    }
    setView("overlay");
  }, []);

  useEffect(() => {
    hydrate();
    const subs = [
      onWindowsChanged((p) => useDoclickStore.setState({ windows: p.windows })),
      onBroadcastState((p) =>
        useDoclickStore.setState({
          broadcastEnabled: p.enabled,
          broadcastReason: p.reason,
        }),
      ),
      onBroadcastTick((p) => {
        if (p.kind === "Started") useDoclickStore.setState({ broadcastLive: true });
        else useDoclickStore.setState({ broadcastLive: false });
      }),
      onError((p) => useDoclickStore.setState({ lastError: p.message })),
      onFocusedWindowChanged((p) => useDoclickStore.setState({ focusedHwnd: p.focused_hwnd })),
      onOpenSettings(() => {
        if (viewRef.current === "overlay") enterSettings();
      }),
      onPrefsChanged(() => hydrate()),
      onUpdateState((p) =>
        useDoclickStore.setState({
          updateState: p.state,
          updateAvailableVersion: p.version,
          updateNotes: p.notes,
          updateError: p.error,
          // Reset progress whenever the state isn't actively downloading.
          updateProgress:
            p.state === "downloading" ? useDoclickStore.getState().updateProgress : null,
        }),
      ),
      onUpdateProgress((p) => useDoclickStore.setState({ updateProgress: p })),
    ];

    // Persist window position on move (debounced). Overlay and settings
    // are the same Tauri window, so its position is shared between
    // views — moves in either view persist to the same overlay_position.
    const win = getCurrentWindow();
    const moveUnlistenP = win.onMoved(({ payload }) => {
      // Win32 reports `-32000` for both axes while a window is
      // minimized — never persist that as the overlay's position, or
      // the window will spawn offscreen on next launch.
      if (payload.x <= -32000 || payload.y <= -32000) return;
      if (moveDebounce.current !== null) window.clearTimeout(moveDebounce.current);
      moveDebounce.current = window.setTimeout(() => {
        saveOverlayPosition(payload.x, payload.y).catch(() => {});
      }, 400);
    });

    return () => {
      for (const s of subs) {
        s.then((off) => off());
      }
      moveUnlistenP.then((off) => off());
      if (moveDebounce.current !== null) window.clearTimeout(moveDebounce.current);
    };
  }, [hydrate, enterSettings]);

  const orientation = useDoclickStore((s) => s.orientation);
  const overlayScale = useDoclickStore((s) => s.overlayScale);
  const overlaySizes = useDoclickStore((s) => s.overlaySizes);
  const hydrated = useDoclickStore((s) => s.hydrated);
  const visibleCount = useDoclickStore((s) => s.windows.filter((w) => w.profile != null).length);

  // Apply the overlay size when its derivation inputs change. View
  // transitions are *not* driven from here — enterSettings/exitSettings
  // apply size imperatively before flipping `view`, so the new view
  // never paints at the wrong dimensions. This effect is just the
  // passive backup that catches orientation toggles, chip count
  // changes, and the user's saved-size updates while in overlay view.
  useEffect(() => {
    if (!hydrated) return;
    if (view !== "overlay") return;
    const win = getCurrentWindow();
    (async () => {
      try {
        const size = computeOverlaySize({
          orientation,
          scale: overlayScale,
          visibleCount,
          savedMainAxis: savedMainAxis(overlaySizes, orientation),
        });
        const min = computeOverlayMinSize(orientation, overlayScale);
        await win.setMinSize(new LogicalSize(min.width, min.height));
        await win.setSize(new LogicalSize(size.width, size.height));
        await win.setResizable(false);
      } catch (err) {
        console.warn("apply overlay size failed", err);
      }
    })();
  }, [hydrated, view, orientation, overlayScale, visibleCount, overlaySizes]);

  // Persist the settings window size while the user is in settings
  // view. `onResized` fires for *any* size change — custom handle
  // drags, native edge drags (resizable: true), or programmatic
  // setSize from this code. Debounced + dedup'd so a steady-state size
  // doesn't loop back into the store.
  useEffect(() => {
    if (view !== "settings") return;
    const win = getCurrentWindow();
    let timer: number | null = null;
    const unlistenP = win.onResized(({ payload }) => {
      // Skip resize events that arrive after we've already left settings
      // view (e.g. exitSettings's setSize(overlaySize) firing late while
      // the async unlisten hasn't completed yet). Without this guard the
      // overlay's tiny dimensions would be persisted as settings_size and
      // applied on every subsequent open.
      if (viewRef.current !== "settings") return;
      if (timer !== null) window.clearTimeout(timer);
      timer = window.setTimeout(async () => {
        if (viewRef.current !== "settings") return;
        try {
          const factor = await win.scaleFactor();
          const w = Math.round(payload.width / factor);
          const h = Math.round(payload.height / factor);
          const next: [number, number] = [w, h];
          if (!isValidSettingsSize(next)) return;
          const cur = useDoclickStore.getState().settingsSize;
          if (cur && cur[0] === w && cur[1] === h) return;
          await useDoclickStore.getState().saveSettingsSize(next[0], next[1]);
        } catch {}
      }, 250);
    });
    return () => {
      unlistenP.then((off) => off());
      if (timer !== null) window.clearTimeout(timer);
    };
  }, [view]);

  if (view === "settings") {
    return (
      <>
        <Settings onBack={exitSettings} initialTab={settingsTab} />
        <ResizeHandles mode="settings" />
      </>
    );
  }

  const openCharacters = () => enterSettings("characters");

  if (orientation === "vertical") {
    return (
      <div className="relative flex h-screen w-screen flex-col overflow-hidden rounded-xl border border-border/50 bg-background shadow-2xl">
        <VerticalOverlayChrome />
        {/* biome-ignore lint/a11y/noStaticElementInteractions: drag-region surface for a frameless Tauri window. The mousedown only initiates window drag — there is no UI action to gate behind a role. */}
        <div
          onMouseDown={(e) => {
            if (e.button !== 0) return;
            const t = e.target as HTMLElement;
            if (t.closest('button, [data-tauri-drag-region="false"]')) return;
            void getCurrentWindow().startDragging();
          }}
          className="drag-surface relative flex flex-1 min-h-0 w-full flex-col items-stretch gap-2 px-2 py-2"
        >
          <div className="flex-1 min-h-0 w-full">
            <AvatarBar onOpenCharacters={openCharacters} />
          </div>
          <div className="h-px w-full bg-border/60" />
          <div className="flex justify-center">
            <BroadcastToggle />
          </div>
        </div>
        <PanicIndicator />
        <ResizeHandles mode="overlay-vertical" />
      </div>
    );
  }

  return (
    <div className="relative flex h-screen w-screen flex-col overflow-hidden rounded-xl border border-border/50 bg-background shadow-2xl">
      <div
        data-tauri-drag-region
        className="relative flex items-center gap-2 w-full px-3"
        style={{ height: presetOf(overlayScale).horizontalBarHeight }}
      >
        <BroadcastToggle />
        <div className="w-px h-8 bg-border/60 mx-1" />
        <div className="flex-1 min-w-0">
          <AvatarBar onOpenCharacters={openCharacters} />
        </div>
        <div className="w-px h-8 bg-border/60 mx-1" />
        <KebabButton anchor="below-right" />
      </div>
      <PanicIndicator />
      <ResizeHandles mode="overlay-horizontal" />
    </div>
  );
}

function savedMainAxis(
  sizes: { horizontal: [number, number] | null; vertical: [number, number] | null },
  orientation: Orientation,
): number | null {
  // Horizontal mode: only width is user-resizable → tuple[0].
  // Vertical mode:   only height is user-resizable → tuple[1].
  if (orientation === "horizontal") return sizes.horizontal?.[0] ?? null;
  return sizes.vertical?.[1] ?? null;
}
