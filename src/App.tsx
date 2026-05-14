import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { useEffect, useRef } from "react";
import { AvatarBar } from "./components/AvatarBar";
import { BroadcastToggle } from "./components/BroadcastToggle";
import { KebabButton } from "./components/KebabButton";
import { PanicIndicator } from "./components/PanicIndicator";
import { ResizeHandles } from "./components/ResizeHandles";
import { VerticalOverlayChrome } from "./components/VerticalOverlayChrome";
import { openSettings, saveOverlayPosition } from "./ipc/commands";
import {
  onBroadcastState,
  onBroadcastTick,
  onError,
  onFocusedWindowChanged,
  onPrefsChanged,
  onUpdateProgress,
  onUpdateState,
  onWindowsChanged,
} from "./ipc/events";
import { computeOverlayMinSize, computeOverlaySize, presetOf } from "./lib/overlaySize";
import { useDoclickStore } from "./store/useDoclickStore";
import type { Orientation } from "./types";

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
      onFocusedWindowChanged((p) => useDoclickStore.setState({ focusedHwnd: p.focused_hwnd })),
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

    // Persist overlay position on move (debounced).
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
  }, [hydrate]);

  const orientation = useDoclickStore((s) => s.orientation);
  const overlayScale = useDoclickStore((s) => s.overlayScale);
  const overlaySizes = useDoclickStore((s) => s.overlaySizes);
  const hydrated = useDoclickStore((s) => s.hydrated);
  const visibleCount = useDoclickStore((s) => s.windows.filter((w) => w.profile != null).length);

  // Apply the overlay size when its derivation inputs change (orientation
  // toggle, chip count change, saved-size updates). The settings window
  // is a separate Tauri window now, so this effect doesn't need a view
  // guard — App.tsx only ever mounts in the overlay window.
  useEffect(() => {
    if (!hydrated) return;
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
  }, [hydrated, orientation, overlayScale, visibleCount, overlaySizes]);

  const openCharacters = () => {
    openSettings("characters").catch(() => {});
  };

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
      {/* `deep` so clicks anywhere in the subtree (separators, AvatarBar
          wrapper, empty-state text) initiate window drag. A bare
          `data-tauri-drag-region` would only fire on direct hits to this
          element, leaving the centered avatar zone non-draggable.
          Chips/BroadcastToggle/KebabButton each carry
          `data-tauri-drag-region="false"` to opt out. */}
      <div
        data-tauri-drag-region="deep"
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
