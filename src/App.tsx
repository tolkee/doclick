import { useCallback, useEffect, useRef, useState } from "react";
import {
  getCurrentWindow,
  LogicalPosition,
  LogicalSize,
} from "@tauri-apps/api/window";
import { AvatarBar } from "./components/AvatarBar";
import { BroadcastToggle } from "./components/BroadcastToggle";
import { PanicIndicator } from "./components/PanicIndicator";
import { ResizeHandles } from "./components/ResizeHandles";
import { TitleBar } from "./components/TitleBar";
import Settings, { type SettingsTabId } from "./Settings";
import { saveOverlayPosition } from "./ipc/commands";
import {
  onBroadcastState,
  onBroadcastTick,
  onError,
  onFocusedWindowChanged,
  onOpenSettings,
  onPrefsChanged,
  onWindowsChanged,
} from "./ipc/events";
import {
  computeOverlayMinSize,
  computeOverlaySize,
  HORIZONTAL_BAR_HEIGHT,
  SETTINGS_DEFAULT_SIZE,
  SETTINGS_MIN_SIZE,
} from "./lib/overlaySize";
import { useDoclickStore } from "./store/useDoclickStore";
import type { Orientation } from "./types";

type View = "overlay" | "settings";

export default function App() {
  const hydrate = useDoclickStore((s) => s.hydrate);
  const moveDebounce = useRef<number | null>(null);

  const [view, setView] = useState<View>("overlay");
  const [settingsTab, setSettingsTab] = useState<SettingsTabId>("global");
  // Mirrored in a ref for closures (event listeners, debounced callbacks)
  // that capture stale state.
  const viewRef = useRef<View>(view);
  viewRef.current = view;
  // Snapshot of the overlay's position taken when entering settings, so
  // exiting can restore exactly where the bar sat. Size is *not*
  // snapshotted — it's fully derived from store state by the size
  // effect below, so settings dimensions can never bleed into the
  // overlay (and vice versa).
  const overlayPositionSnapshot = useRef<{ x: number; y: number } | null>(null);

  const enterSettings = useCallback(async (tab: SettingsTabId = "global") => {
    setSettingsTab(tab);
    if (viewRef.current === "settings") return;
    const win = getCurrentWindow();
    try {
      const factor = await win.scaleFactor();
      const pos = (await win.outerPosition()).toLogical(factor);
      overlayPositionSnapshot.current = { x: pos.x, y: pos.y };
      // Apply size BEFORE flipping view so the settings UI never paints
      // at overlay dimensions (no flash). Order: enable resize so the
      // current overlay min doesn't clamp, set new min, then set size.
      const target =
        useDoclickStore.getState().settingsSize ??
        ([SETTINGS_DEFAULT_SIZE.width, SETTINGS_DEFAULT_SIZE.height] as [number, number]);
      await win.setResizable(true);
      await win.setMinSize(
        new LogicalSize(SETTINGS_MIN_SIZE.width, SETTINGS_MIN_SIZE.height),
      );
      await win.setSize(new LogicalSize(target[0], target[1]));
      await win.setAlwaysOnTop(false);
      await win.setSkipTaskbar(false);
      await win.setFocus();
    } catch (err) {
      console.warn("enterSettings failed", err);
    }
    setView("settings");
  }, []);

  const exitSettings = useCallback(async () => {
    if (viewRef.current === "overlay") return;
    const win = getCurrentWindow();
    const snap = overlayPositionSnapshot.current;
    const state = useDoclickStore.getState();
    const orientation = state.orientation;
    const visibleCount = state.windows.filter((w) => w.profile != null).length;
    const min = computeOverlayMinSize(orientation);
    const size = computeOverlaySize({
      orientation,
      visibleCount,
      savedMainAxis: savedMainAxis(state.overlaySizes, orientation),
    });
    try {
      await win.setSize(new LogicalSize(size.width, size.height));
      await win.setMinSize(new LogicalSize(min.width, min.height));
      await win.setResizable(false);
      await win.setAlwaysOnTop(true);
      await win.setSkipTaskbar(true);
      if (snap) {
        await win.setPosition(new LogicalPosition(snap.x, snap.y));
      }
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
      onFocusedWindowChanged((p) =>
        useDoclickStore.setState({ focusedHwnd: p.focused_hwnd }),
      ),
      onOpenSettings(() => {
        if (viewRef.current === "overlay") enterSettings();
      }),
      onPrefsChanged(() => hydrate()),
    ];

    // Persist overlay position on move (debounced). Only while in
    // overlay view — dragging the larger settings window must not
    // clobber the persisted overlay_position.
    const win = getCurrentWindow();
    const moveUnlistenP = win.onMoved(({ payload }) => {
      if (viewRef.current !== "overlay") return;
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
      subs.forEach((s) => s.then((off) => off()));
      moveUnlistenP.then((off) => off());
      if (moveDebounce.current !== null) window.clearTimeout(moveDebounce.current);
    };
  }, [hydrate, enterSettings]);

  const orientation = useDoclickStore((s) => s.orientation);
  const overlaySizes = useDoclickStore((s) => s.overlaySizes);
  const hydrated = useDoclickStore((s) => s.hydrated);
  const visibleCount = useDoclickStore(
    (s) => s.windows.filter((w) => w.profile != null).length,
  );

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
          visibleCount,
          savedMainAxis: savedMainAxis(overlaySizes, orientation),
        });
        const min = computeOverlayMinSize(orientation);
        await win.setSize(new LogicalSize(size.width, size.height));
        await win.setMinSize(new LogicalSize(min.width, min.height));
        await win.setResizable(false);
      } catch (err) {
        console.warn("apply overlay size failed", err);
      }
    })();
  }, [hydrated, view, orientation, visibleCount, overlaySizes]);

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
      if (timer !== null) window.clearTimeout(timer);
      timer = window.setTimeout(async () => {
        try {
          const factor = await win.scaleFactor();
          const w = Math.round(payload.width / factor);
          const h = Math.round(payload.height / factor);
          const cur = useDoclickStore.getState().settingsSize;
          if (cur && cur[0] === w && cur[1] === h) return;
          await useDoclickStore.getState().saveSettingsSize(w, h);
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

  // Overlay view. Outer card matches the settings window's chrome:
  // solid bg, `rounded-xl`, `border-border/50`, `shadow-2xl`. The
  // TitleBar and the bar below sit flush as one block.
  const openCharacters = () => enterSettings("characters");

  if (orientation === "vertical") {
    return (
      <div className="relative flex h-screen w-screen flex-col overflow-hidden rounded-xl border border-border/50 bg-background shadow-2xl">
        <TitleBar
          title="Doclick"
          showMaximize={false}
          onOpenSettings={() => enterSettings()}
        />
        <div className="relative flex flex-1 min-h-0 w-full flex-col items-stretch gap-2 px-2 py-2">
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
      <TitleBar
        title="Doclick"
        showMaximize={false}
        onOpenSettings={() => enterSettings()}
      />
      <div
        className="relative flex items-center gap-2 w-full px-3"
        style={{ height: HORIZONTAL_BAR_HEIGHT }}
      >
        <BroadcastToggle />
        <div className="w-px h-8 bg-border/60 mx-1" />
        <div className="flex-1 min-w-0">
          <AvatarBar onOpenCharacters={openCharacters} />
        </div>
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
