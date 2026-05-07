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
import { SettingsButton } from "./components/SettingsButton";
import { TitleBar } from "./components/TitleBar";
import Settings from "./Settings";
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
  applyOverlaySize,
  computeMinSize,
  HORIZONTAL_HEIGHT,
  SETTINGS_VIEW_MIN_SIZE,
  SETTINGS_VIEW_SIZE,
} from "./lib/overlaySize";
import { useDoclickStore } from "./store/useDoclickStore";

type View = "overlay" | "settings";

export default function App() {
  const hydrate = useDoclickStore((s) => s.hydrate);
  const moveDebounce = useRef<number | null>(null);

  const [view, setView] = useState<View>("overlay");
  // Read by closures (onMoved, event listeners) that capture stale state.
  const viewRef = useRef<View>(view);
  viewRef.current = view;
  // Snapshot of the overlay's size/position taken when entering settings,
  // so exiting can restore exactly what the user had. Intentionally a ref
  // (not zustand/disk) — settings size is ephemeral and must not pollute
  // the persisted overlay_size / overlay_position.
  const overlaySnapshot = useRef<{
    width: number;
    height: number;
    x: number;
    y: number;
  } | null>(null);

  const enterSettings = useCallback(async () => {
    if (viewRef.current === "settings") return;
    const win = getCurrentWindow();
    try {
      const factor = await win.scaleFactor();
      const size = (await win.outerSize()).toLogical(factor);
      const pos = (await win.outerPosition()).toLogical(factor);
      overlaySnapshot.current = {
        width: size.width,
        height: size.height,
        x: pos.x,
        y: pos.y,
      };
      // Property order matters: setSize after setResizable avoids a
      // 1-frame transparent flash on Windows.
      await win.setAlwaysOnTop(false);
      await win.setSkipTaskbar(false);
      await win.setResizable(true);
      await win.setMinSize(
        new LogicalSize(SETTINGS_VIEW_MIN_SIZE.width, SETTINGS_VIEW_MIN_SIZE.height),
      );
      await win.setSize(
        new LogicalSize(SETTINGS_VIEW_SIZE.width, SETTINGS_VIEW_SIZE.height),
      );
      await win.setFocus();
    } catch (err) {
      console.warn("enterSettings failed", err);
    }
    setView("settings");
  }, []);

  const exitSettings = useCallback(async () => {
    if (viewRef.current === "overlay") return;
    const win = getCurrentWindow();
    const snap = overlaySnapshot.current;
    const state = useDoclickStore.getState();
    const orientation = state.orientation;
    const visibleCount = state.windows.filter((w) => w.profile != null).length;
    const min = computeMinSize({ orientation, visibleCount });
    // Use the orientation's saved-or-default size, not the snapshot.size:
    // the user may have toggled orientation while inside Settings, in
    // which case the snapshot's size is for the wrong orientation.
    const saved = state.overlaySizes[orientation];
    try {
      await win.setAlwaysOnTop(true);
      await win.setSkipTaskbar(true);
      await win.setResizable(false);
      await win.setMinSize(new LogicalSize(min.width, min.height));
      await applyOverlaySize({
        orientation,
        visibleCount,
        override: saved ? { width: saved[0], height: saved[1] } : null,
      });
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

    // Persist overlay position on move (debounced). Only while in overlay
    // view — dragging the larger settings window must not clobber the
    // persisted overlay_position.
    const win = getCurrentWindow();
    const moveUnlistenP = win.onMoved(({ payload }) => {
      if (viewRef.current !== "overlay") return;
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

  // Apply the OS-level min size on orientation change. The locked axis is
  // pinned by the inner wrapper's fixed dimension — we don't setMaxSize on
  // the OS window because transient resizes (e.g. settings view) need to
  // grow it past that cap. If the user drags the locked-axis edge, the OS
  // window grows but the inner bar stays pinned (extra transparent space
  // on the locked side). Skipped while in settings view; exitSettings
  // re-applies the overlay min size.
  useEffect(() => {
    if (view !== "overlay") return;
    const win = getCurrentWindow();
    const min = computeMinSize({ orientation, visibleCount });
    (async () => {
      try {
        await win.setMinSize(new LogicalSize(min.width, min.height));
      } catch (err) {
        console.warn("overlay: set min size failed", err);
      }
    })();
    // visibleCount intentionally omitted from the dep array.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orientation, view]);

  // Apply size when the orientation changes (e.g. user toggles in
  // Settings) — restore the user's last manually-resized size for that
  // orientation if we have one persisted, else fall back to the
  // orientation's default size. We skip the first run (right after
  // hydrate completes) because Rust already restored the saved size
  // for the loaded orientation before the window was shown — re-running
  // here would clobber it with a default if no saved size exists yet.
  // Don't refire on visibleCount-only changes — that would clobber the
  // user's manual resize. Min size already grows the window if more
  // chips than fit are imported.
  const lastOrientation = useRef<string | null>(null);
  useEffect(() => {
    if (!hydrated) return;
    if (lastOrientation.current === null) {
      lastOrientation.current = orientation;
      return;
    }
    if (lastOrientation.current === orientation) return;
    // Settings view holds 440x720; defer the overlay resize until the
    // user clicks back. Don't update lastOrientation yet — once we
    // re-enter overlay view this effect refires (view is in the dep
    // array) and the orientation diff will still be visible.
    if (view !== "overlay") return;
    lastOrientation.current = orientation;
    const saved = overlaySizes[orientation];
    applyOverlaySize({
      orientation,
      visibleCount,
      override: saved ? { width: saved[0], height: saved[1] } : null,
    });
    // overlaySizes/visibleCount intentionally read at fire time only.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orientation, hydrated, view]);

  if (view === "settings") {
    return <Settings onBack={exitSettings} />;
  }

  if (orientation === "vertical") {
    // Pin the inner column to its natural width so transient view changes
    // (which temporarily widen the OS window) don't stretch the bar.
    const columnWidth = 76;
    return (
      <div className="flex h-screen w-screen flex-col overflow-hidden">
        <TitleBar title="Doclick" showMaximize={false} />
        <div
          className="relative flex-1 min-h-0 p-2"
          style={{ width: columnWidth + 16 }}
        >
          <div className="relative flex flex-col items-stretch gap-2 h-full w-full px-2 py-3 rounded-xl bg-background/70 backdrop-blur-md border border-border/50 shadow-xl">
            <div className="flex justify-center">
              <BroadcastToggle />
            </div>
            <div className="h-px w-full bg-border/60" />
            <div className="flex-1 min-h-0 w-full py-1">
              <AvatarBar />
            </div>
            <div className="h-px w-full bg-border/60" />
            <div className="flex flex-col items-center gap-1.5">
              <SettingsButton onOpenSettings={enterSettings} />
            </div>
            <PanicIndicator />
          </div>
          <ResizeHandles orientation={orientation} />
        </div>
      </div>
    );
  }

  // Horizontal layout: pin the bar to its locked height inside the column
  // below the TitleBar.
  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden">
      <TitleBar title="Doclick" showMaximize={false} />
      <div
        className="relative w-full p-2"
        style={{ height: HORIZONTAL_HEIGHT }}
      >
        <div className="relative flex items-center gap-2 h-full w-full px-3 rounded-xl bg-background/70 backdrop-blur-md border border-border/50 shadow-xl">
          <BroadcastToggle />
          <div className="w-px h-8 bg-border/60 mx-1" />
          <div className="flex-1 min-w-0">
            <AvatarBar />
          </div>
          <SettingsButton onOpenSettings={enterSettings} />
          <PanicIndicator />
        </div>
        <ResizeHandles orientation={orientation} />
      </div>
    </div>
  );
}
