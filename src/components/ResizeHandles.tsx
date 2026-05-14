import { getCurrentWindow, LogicalPosition, LogicalSize } from "@tauri-apps/api/window";
import type { PointerEvent as ReactPointerEvent } from "react";
import { computeOverlayMinSize, SETTINGS_MIN_SIZE } from "../lib/overlaySize";
import { useDoclickStore } from "../store/useDoclickStore";

type Direction = "East" | "West" | "North" | "South";

/// Three resize "modes" decide which edges get a handle and where the
/// final size gets persisted:
///
///   - `overlay-horizontal`: only E/W handles (width adjustable). The
///     bar's height is locked; user-saved width persists to
///     overlay_sizes.horizontal.
///   - `overlay-vertical`: only N/S handles (height adjustable). The
///     bar's width is locked; user-saved height persists to
///     overlay_sizes.vertical.
///   - `settings`: all four handles; full size persists to
///     settings_size.
export type ResizeMode = "overlay-horizontal" | "overlay-vertical" | "settings";

interface Props {
  mode: ResizeMode;
}

/// Custom edge resize for a transparent/decoration-less Tauri window.
/// The OS window has `decorations: false` so Windows draws no native
/// resize chrome; this component is the only resize affordance.
export function ResizeHandles({ mode }: Props) {
  const start = (direction: Direction) => async (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();

    const handle = e.currentTarget;
    const pointerId = e.pointerId;
    handle.setPointerCapture(pointerId);

    const win = getCurrentWindow();
    let initialW = 0;
    let initialH = 0;
    let initialX = 0;
    let initialY = 0;
    try {
      const factor = await win.scaleFactor();
      const size = (await win.outerSize()).toLogical(factor);
      const pos = (await win.outerPosition()).toLogical(factor);
      initialW = size.width;
      initialH = size.height;
      initialX = pos.x;
      initialY = pos.y;
    } catch (err) {
      console.warn("resize: read initial state failed", err);
      try {
        handle.releasePointerCapture(pointerId);
      } catch {}
      return;
    }

    const startSx = e.screenX;
    const startSy = e.screenY;
    const min = minSizeFor(mode);

    // Throttle setSize/setPosition IPC to one call per frame so a fast
    // mousemove doesn't queue dozens of redundant updates.
    let pendingFrame: number | null = null;
    let pending: {
      w: number;
      h: number;
      x: number;
      y: number;
      needsPos: boolean;
    } | null = null;

    const flush = () => {
      pendingFrame = null;
      const p = pending;
      if (!p) return;
      // setPosition first for West/North so the right/bottom edge
      // doesn't briefly overshoot before the size update lands.
      if (p.needsPos) {
        win.setPosition(new LogicalPosition(p.x, p.y)).catch(() => {});
      }
      win.setSize(new LogicalSize(p.w, p.h)).catch(() => {});
    };

    const onMove = (ev: PointerEvent) => {
      const dx = ev.screenX - startSx;
      const dy = ev.screenY - startSy;

      let newW = initialW;
      let newH = initialH;
      let newX = initialX;
      let newY = initialY;
      let needsPos = false;

      switch (direction) {
        case "East":
          newW = Math.max(min.width, initialW + dx);
          break;
        case "West":
          newW = Math.max(min.width, initialW - dx);
          newX = initialX + initialW - newW;
          needsPos = true;
          break;
        case "South":
          newH = Math.max(min.height, initialH + dy);
          break;
        case "North":
          newH = Math.max(min.height, initialH - dy);
          newY = initialY + initialH - newH;
          needsPos = true;
          break;
      }

      pending = { w: newW, h: newH, x: newX, y: newY, needsPos };
      if (pendingFrame !== null) return;
      pendingFrame = requestAnimationFrame(flush);
    };

    const cleanup = () => {
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", cleanup);
      handle.removeEventListener("pointercancel", cleanup);
      if (pendingFrame !== null) {
        cancelAnimationFrame(pendingFrame);
        flush();
      }
      try {
        handle.releasePointerCapture(pointerId);
      } catch {}
      if (pending) {
        persistFinalSize(mode, Math.round(pending.w), Math.round(pending.h));
      }
    };

    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", cleanup);
    handle.addEventListener("pointercancel", cleanup);
  };

  if (mode === "overlay-horizontal") {
    return (
      <>
        <Handle direction="West" cursor="ew-resize" axis="horizontal" onStart={start("West")} />
        <Handle direction="East" cursor="ew-resize" axis="horizontal" onStart={start("East")} />
      </>
    );
  }
  if (mode === "overlay-vertical") {
    return (
      <>
        <Handle direction="North" cursor="ns-resize" axis="vertical" onStart={start("North")} />
        <Handle direction="South" cursor="ns-resize" axis="vertical" onStart={start("South")} />
      </>
    );
  }
  // settings: all four edges
  return (
    <>
      <Handle direction="North" cursor="ns-resize" axis="vertical" onStart={start("North")} />
      <Handle direction="South" cursor="ns-resize" axis="vertical" onStart={start("South")} />
      <Handle direction="West" cursor="ew-resize" axis="horizontal" onStart={start("West")} />
      <Handle direction="East" cursor="ew-resize" axis="horizontal" onStart={start("East")} />
    </>
  );
}

function Handle({
  direction,
  cursor,
  axis,
  onStart,
}: {
  direction: Direction;
  cursor: string;
  axis: "horizontal" | "vertical";
  onStart: (e: ReactPointerEvent<HTMLDivElement>) => void;
}) {
  const base =
    axis === "horizontal" ? "absolute top-0 bottom-0 w-2 z-30" : "absolute left-0 right-0 h-2 z-30";
  const edge =
    direction === "North"
      ? "top-0"
      : direction === "South"
        ? "bottom-0"
        : direction === "East"
          ? "right-0"
          : "left-0";
  return (
    <div
      onPointerDown={onStart}
      className={`${base} ${edge}`}
      style={{ cursor, touchAction: "none" }}
      aria-hidden
    />
  );
}

function minSizeFor(mode: ResizeMode): { width: number; height: number } {
  if (mode === "settings") return SETTINGS_MIN_SIZE;
  const scale = useDoclickStore.getState().overlayScale;
  return computeOverlayMinSize(mode === "overlay-horizontal" ? "horizontal" : "vertical", scale);
}

function persistFinalSize(mode: ResizeMode, width: number, height: number) {
  const store = useDoclickStore.getState();
  if (mode === "settings") {
    store.saveSettingsSize(width, height).catch(() => {});
    return;
  }
  const orientation = mode === "overlay-horizontal" ? "horizontal" : "vertical";
  store.saveOverlaySize(orientation, width, height).catch(() => {});
}
