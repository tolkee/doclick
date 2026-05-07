import {
  getCurrentWindow,
  LogicalPosition,
  LogicalSize,
} from "@tauri-apps/api/window";
import { computeMinSize } from "../lib/overlaySize";
import { useDoclickStore } from "../store/useDoclickStore";
import type { Orientation } from "../types";

interface Props {
  orientation: Orientation;
}

type Direction = "East" | "West" | "North" | "South";

/// Custom edge resize for a transparent/decoration-less Tauri window.
///
/// The OS window has `resizable: false` so Windows does not add resize chrome
/// to any edge — that prevents the locked-axis cursor + the "drag the left
/// edge to move the window" behavior the user reported. This component then
/// injects pointer-driven resize on the variable-axis edges only:
///
///   • horizontal → x-only via left/right edges (`ew-resize`)
///   • vertical   → y-only via top/bottom edges (`ns-resize`)
///
/// For the West/North directions we also move the window position so the
/// opposite edge stays anchored, which is what users expect from a window
/// edge resize (drag the left edge → bar grows leftward).
export function ResizeHandles({ orientation }: Props) {
  const start =
    (direction: Direction) =>
    async (e: React.PointerEvent<HTMLDivElement>) => {
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
      const min = computeMinSize({ orientation, visibleCount: 0 });

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
        // setPosition first for West/North so the right/bottom edge doesn't
        // briefly overshoot before the size update lands.
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
        // Persist the final dragged size for this orientation. Skip when
        // the user clicked the handle without moving — `pending` stays
        // null and there's nothing new to save.
        if (pending) {
          useDoclickStore
            .getState()
            .saveOverlaySize(
              orientation,
              Math.round(pending.w),
              Math.round(pending.h),
            )
            .catch(() => {});
        }
      };

      handle.addEventListener("pointermove", onMove);
      handle.addEventListener("pointerup", cleanup);
      handle.addEventListener("pointercancel", cleanup);
    };

  if (orientation === "horizontal") {
    return (
      <>
        <div
          onPointerDown={start("West")}
          className="absolute left-0 top-0 bottom-0 w-2 cursor-ew-resize z-30"
          style={{ touchAction: "none" }}
          aria-hidden
        />
        <div
          onPointerDown={start("East")}
          className="absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize z-30"
          style={{ touchAction: "none" }}
          aria-hidden
        />
      </>
    );
  }

  return (
    <>
      <div
        onPointerDown={start("North")}
        className="absolute top-0 left-0 right-0 h-2 cursor-ns-resize z-30"
        style={{ touchAction: "none" }}
        aria-hidden
      />
      <div
        onPointerDown={start("South")}
        className="absolute bottom-0 left-0 right-0 h-2 cursor-ns-resize z-30"
        style={{ touchAction: "none" }}
        aria-hidden
      />
    </>
  );
}
