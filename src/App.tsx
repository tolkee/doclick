import { useEffect, useRef } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { AvatarBar } from "./components/AvatarBar";
import { BroadcastToggle } from "./components/BroadcastToggle";
import { CloseButton } from "./components/CloseButton";
import { PanicIndicator } from "./components/PanicIndicator";
import { ResizeHandles } from "./components/ResizeHandles";
import { openSettings, SettingsButton } from "./components/SettingsButton";
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
import { applyOverlaySize, computeMinSize, HORIZONTAL_HEIGHT } from "./lib/overlaySize";
import { useDoclickStore } from "./store/useDoclickStore";

export default function App() {
  const hydrate = useDoclickStore((s) => s.hydrate);
  const moveDebounce = useRef<number | null>(null);

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
      onOpenSettings(() => openSettings()),
      onPrefsChanged(() => hydrate()),
    ];

    // Persist overlay position on move (debounced).
    const win = getCurrentWindow();
    const moveUnlistenP = win.onMoved(({ payload }) => {
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
  }, [hydrate]);

  const orientation = useDoclickStore((s) => s.orientation);
  const overlaySizes = useDoclickStore((s) => s.overlaySizes);
  const hydrated = useDoclickStore((s) => s.hydrated);
  const visibleCount = useDoclickStore(
    (s) => s.windows.filter((w) => w.profile != null).length,
  );

  // Apply the min size on orientation change. The locked axis is pinned by
  // the inner wrapper's fixed dimension — we don't setMaxSize on the OS
  // window because the settings popover needs to grow it past that cap to
  // render below/beside the bar. If the user drags the locked-axis edge,
  // the OS window grows but the inner bar stays pinned (extra transparent
  // space appears on the locked side).
  useEffect(() => {
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
  }, [orientation]);

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
    lastOrientation.current = orientation;
    const saved = overlaySizes[orientation];
    applyOverlaySize({
      orientation,
      visibleCount,
      override: saved ? { width: saved[0], height: saved[1] } : null,
    });
    // overlaySizes/visibleCount intentionally read at fire time only.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orientation, hydrated]);

  if (orientation === "vertical") {
    // Pin the inner column to its natural width so opening the settings menu
    // (which temporarily widens the OS window) doesn't stretch the bar.
    const columnWidth = 76;
    return (
      <div className="relative h-full p-2" style={{ width: columnWidth + 16 }}>
        <div
          data-tauri-drag-region="deep"
          className="relative flex flex-col items-stretch gap-2 h-full w-full px-2 py-3 rounded-2xl bg-zinc-900/80 backdrop-blur-md ring-1 ring-zinc-700/60 shadow-xl"
        >
          <div className="flex justify-center">
            <BroadcastToggle />
          </div>
          <div className="h-px w-full bg-zinc-700/70" />
          <div className="flex-1 min-h-0 w-full py-1">
            <AvatarBar />
          </div>
          <div className="h-px w-full bg-zinc-700/70" />
          <div className="flex flex-col items-center gap-1.5">
            <SettingsButton />
            <CloseButton />
          </div>
          <PanicIndicator />
        </div>
        <ResizeHandles orientation={orientation} />
      </div>
    );
  }

  // Horizontal layout: pin the bar to its locked height inside a possibly
  // taller window, so the SettingsMenu popover (rendered inside the window)
  // has room to extend below without stretching the bar itself.
  return (
    <div className="relative w-full p-2" style={{ height: HORIZONTAL_HEIGHT }}>
      <div
        data-tauri-drag-region="deep"
        className="relative flex items-center gap-2 h-full w-full px-3 rounded-2xl bg-zinc-900/80 backdrop-blur-md ring-1 ring-zinc-700/60 shadow-xl"
      >
        <BroadcastToggle />
        <div className="w-px h-8 bg-zinc-700/70 mx-1" />
        <div className="flex-1 min-w-0">
          <AvatarBar />
        </div>
        <SettingsButton />
        <CloseButton />
        <PanicIndicator />
      </div>
      <ResizeHandles orientation={orientation} />
    </div>
  );
}
