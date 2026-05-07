import { LogicalSize } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Orientation } from "../types";

// =================== Layout constants ===================

/// Height of the TitleBar (matches Tailwind `h-9`).
export const TITLEBAR_HEIGHT = 36;

/// Locked inner-bar height in horizontal mode. Just enough for one chip
/// (44px) plus padding so the focus ring (3px outset) doesn't clip.
export const HORIZONTAL_BAR_HEIGHT = 64;

/// Locked inner-bar width in vertical mode. Chip column (44px) + side
/// padding so the chip's focus ring renders fully.
export const VERTICAL_BAR_WIDTH = 76;

/// Per-chip slot in the avatar bar (chip 44px + gap 12px).
const CHIP_BLOCK = 56;

/// Width of the "controls cluster" (separator + broadcast button) in
/// horizontal mode, including the bar's outer padding and gaps. Added
/// to chip slots to compute the auto-fit window width.
const HORIZONTAL_CONTROLS_WIDTH = 88;

/// Vertical equivalent — total height taken by the controls cluster
/// (separator + broadcast) and outer padding in vertical mode.
const VERTICAL_CONTROLS_HEIGHT = 76;

/// Floors so the empty-state placeholder ("Aucune fenêtre Dofus
/// ouverte" / "Aucun personnage importé · Importer") has room.
const HORIZONTAL_EMPTY_MAIN_AXIS = 360;
const VERTICAL_EMPTY_MAIN_AXIS = 240;

// =================== Settings view ===================

/// Default size for the settings view, used when no saved size exists.
export const SETTINGS_DEFAULT_SIZE = { width: 440, height: 720 };
export const SETTINGS_MIN_SIZE = { width: 380, height: 560 };

/// A saved settings size is "valid" if it's at least the minimum on both
/// axes. Anything smaller is treated as poisoned (e.g. left over from a
/// pre-fix bug where the overlay's dimensions leaked into settings_size)
/// and the caller should fall back to SETTINGS_DEFAULT_SIZE.
export function isValidSettingsSize(s: [number, number] | null): boolean {
  return (
    s != null &&
    Number.isFinite(s[0]) &&
    Number.isFinite(s[1]) &&
    s[0] >= SETTINGS_MIN_SIZE.width &&
    s[1] >= SETTINGS_MIN_SIZE.height
  );
}

// =================== Overlay view ===================

/// The overlay's *cross axis* is locked to the bar's natural size:
///
/// - horizontal: height = TitleBar + bar (fixed). Width is user-resizable.
/// - vertical:   width = bar column (fixed).      Height is user-resizable.
///
/// The user-resizable axis persists per orientation; if no value is
/// saved yet the overlay auto-fits the imported chip count.
export interface OverlaySizeArgs {
  orientation: Orientation;
  visibleCount: number;
  /// Saved main-axis value for this orientation (width in horizontal,
  /// height in vertical), or `null` to use the auto-fit default.
  savedMainAxis: number | null;
}

export function computeOverlaySize(args: OverlaySizeArgs): {
  width: number;
  height: number;
} {
  const count = args.visibleCount;
  if (args.orientation === "horizontal") {
    const chipsAxis =
      count > 0 ? count * CHIP_BLOCK : HORIZONTAL_EMPTY_MAIN_AXIS;
    const minWidth = HORIZONTAL_CONTROLS_WIDTH + HORIZONTAL_EMPTY_MAIN_AXIS;
    return {
      width: Math.max(
        args.savedMainAxis ?? HORIZONTAL_CONTROLS_WIDTH + chipsAxis,
        minWidth,
      ),
      height: TITLEBAR_HEIGHT + HORIZONTAL_BAR_HEIGHT,
    };
  }
  const chipsAxis = count > 0 ? count * CHIP_BLOCK : VERTICAL_EMPTY_MAIN_AXIS;
  const minHeight =
    TITLEBAR_HEIGHT + VERTICAL_CONTROLS_HEIGHT + VERTICAL_EMPTY_MAIN_AXIS;
  return {
    width: VERTICAL_BAR_WIDTH,
    height: Math.max(
      args.savedMainAxis ?? TITLEBAR_HEIGHT + VERTICAL_CONTROLS_HEIGHT + chipsAxis,
      minHeight,
    ),
  };
}

/// OS-level minimum size — pins the cross axis to its natural locked
/// dimension and gives the main axis enough room for the controls plus
/// the empty-state placeholder.
export function computeOverlayMinSize(orientation: Orientation): {
  width: number;
  height: number;
} {
  if (orientation === "horizontal") {
    return {
      width: HORIZONTAL_CONTROLS_WIDTH + HORIZONTAL_EMPTY_MAIN_AXIS,
      height: TITLEBAR_HEIGHT + HORIZONTAL_BAR_HEIGHT,
    };
  }
  return {
    width: VERTICAL_BAR_WIDTH,
    height: TITLEBAR_HEIGHT + VERTICAL_CONTROLS_HEIGHT + VERTICAL_EMPTY_MAIN_AXIS,
  };
}

/// Apply a size to the overlay window. Always targets the "overlay"
/// label — caller may run from any view.
export async function applyWindowSize(width: number, height: number): Promise<void> {
  const win = await WebviewWindow.getByLabel("overlay");
  if (!win) return;
  await win.setSize(new LogicalSize(width, height));
}
