import { LogicalSize } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Orientation } from "../types";

interface SizeArgs {
  orientation: Orientation;
  visibleCount: number;
  /// Optional explicit size to apply instead of the orientation default.
  /// Used when restoring a previously persisted user-resized size.
  override?: { width: number; height: number } | null;
}

/// Compute and apply the overlay window size. ALWAYS targets the "overlay"
/// window by label — calling code may run from the settings webview, where
/// `getCurrentWindow()` would resize the settings window instead.
export async function applyOverlaySize(args: SizeArgs): Promise<void> {
  const { width, height } = args.override ?? computeSize(args);
  try {
    const overlay = await WebviewWindow.getByLabel("overlay");
    if (!overlay) return;
    await overlay.setSize(new LogicalSize(width, height));
  } catch (err) {
    console.warn("applyOverlaySize: setSize failed", err);
  }
}

/// Default size when an orientation is freshly entered.
export function computeSize({ orientation, visibleCount }: SizeArgs): {
  width: number;
  height: number;
} {
  if (orientation === "horizontal") {
    const count = Math.max(1, visibleCount);
    return {
      width: Math.max(600, HORIZONTAL_MIN_WIDTH + (count - 1) * CHIP_BLOCK),
      height: HORIZONTAL_HEIGHT,
    };
  }
  const count = Math.max(1, visibleCount);
  return {
    width: VERTICAL_WIDTH,
    height: VERTICAL_MIN_HEIGHT + (count - 1) * CHIP_BLOCK,
  };
}

// Window size constants. The locked axis (height in horizontal, width in
// vertical) is pinned only by the inner wrapper's fixed dimension — the OS
// window itself isn't capped, so the settings popover can freely grow the
// window past the bar's locked side without fighting a setMaxSize cap. The
// variable axis has a fixed minimum sized for the bar's controls plus one
// chip slot; beyond that the avatar bar scrolls.

/// Locked inner-bar height in horizontal mode. 68 = 8 outer padding + 52
/// inner panel; the panel needs to be at least chip(44) + 2*4px padding so
/// the focus ring (3px outset) doesn't overflow the panel chrome.
export const HORIZONTAL_HEIGHT = 68;
/// Locked inner-bar width in vertical mode. Matches the inner column wrapper
/// (`columnWidth + p-2 padding` in App.tsx) so the bar's right edge — and
/// its rounded border — render fully inside the OS window.
export const VERTICAL_WIDTH = 92;

/// Per-chip slot in the avatar bar (chip 44px + gap 12px).
const CHIP_BLOCK = 56;

/// Min width for the controls + one chip slot in horizontal mode. Below
/// this, the avatar bar starts hiding chips, so the OS resize stops here
/// and the bar scrolls when more chips than fit are imported.
const HORIZONTAL_MIN_WIDTH = 280;
/// Vertical equivalent — min height for the controls + one chip slot.
const VERTICAL_MIN_HEIGHT = 280;

export function computeMinSize(args: SizeArgs): {
  width: number;
  height: number;
} {
  if (args.orientation === "horizontal") {
    return { width: HORIZONTAL_MIN_WIDTH, height: HORIZONTAL_HEIGHT };
  }
  return { width: VERTICAL_WIDTH, height: VERTICAL_MIN_HEIGHT };
}
