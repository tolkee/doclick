import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/window";
import type { Orientation, OverlayScale } from "../types";

// =================== Scale presets ===================

/// Per-scale layout constants. Each preset scales the cross-axis of the
/// bar (height in horizontal mode, width in vertical) plus the per-chip
/// stride; the user-resizable main axis is unaffected by the preset.
/// `chipSize + 12 (gap) = chipBlock` for every row, so adjusting one means
/// adjusting both.
export interface OverlayScalePreset {
  /// Avatar chip and broadcast toggle pixel size.
  chipSize: number;
  /// Kebab menu button pixel size.
  kebabSize: number;
  /// Per-chip slot stride along the main axis (chip + gap).
  chipBlock: number;
  /// Locked inner-bar height in horizontal mode. Sized to fit one chip
  /// plus padding so the focus ring (3px outset) doesn't clip.
  horizontalBarHeight: number;
  /// Locked inner-bar width in vertical mode. Chip column plus side
  /// padding so the chip's focus ring renders fully.
  verticalBarWidth: number;
  /// Vertical overlay chrome height: hosts the kebab menu button and
  /// serves as the top drag region.
  verticalTopbarHeight: number;
  /// Total non-chip width in horizontal mode: BroadcastToggle + left
  /// divider + outer padding + right divider + kebab button. Added to
  /// chip slots to compute the auto-fit width.
  horizontalControlsWidth: number;
  /// Vertical equivalent — total height taken by the controls cluster
  /// (separator + broadcast) and outer padding in vertical mode.
  verticalControlsHeight: number;
  /// Auto-fit chip-area extent when no chips are imported, so the empty
  /// state placeholder still has room at default size. The user can drag
  /// the window smaller than this and the placeholder will clip.
  horizontalEmptyMainAxis: number;
  verticalEmptyMainAxis: number;
}

export const OVERLAY_SCALE_PRESETS: Record<OverlayScale, OverlayScalePreset> = {
  small: {
    chipSize: 36,
    kebabSize: 28,
    chipBlock: 48,
    horizontalBarHeight: 52,
    verticalBarWidth: 64,
    verticalTopbarHeight: 36,
    horizontalControlsWidth: 128,
    verticalControlsHeight: 64,
    horizontalEmptyMainAxis: 320,
    verticalEmptyMainAxis: 200,
  },
  medium: {
    chipSize: 44,
    kebabSize: 32,
    chipBlock: 56,
    horizontalBarHeight: 64,
    verticalBarWidth: 76,
    verticalTopbarHeight: 44,
    horizontalControlsWidth: 152,
    verticalControlsHeight: 76,
    horizontalEmptyMainAxis: 360,
    verticalEmptyMainAxis: 240,
  },
  large: {
    chipSize: 52,
    kebabSize: 40,
    chipBlock: 64,
    horizontalBarHeight: 76,
    verticalBarWidth: 88,
    verticalTopbarHeight: 52,
    horizontalControlsWidth: 176,
    verticalControlsHeight: 88,
    horizontalEmptyMainAxis: 400,
    verticalEmptyMainAxis: 280,
  },
};

export function presetOf(scale: OverlayScale): OverlayScalePreset {
  return OVERLAY_SCALE_PRESETS[scale];
}

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
/// - horizontal: height = bar (fixed).        Width is user-resizable.
/// - vertical:   width = bar column (fixed).  Height is user-resizable.
///
/// The user-resizable axis persists per orientation; if no value is
/// saved yet the overlay auto-fits the imported chip count.
export interface OverlaySizeArgs {
  orientation: Orientation;
  scale: OverlayScale;
  visibleCount: number;
  /// Saved main-axis value for this orientation (width in horizontal,
  /// height in vertical), or `null` to use the auto-fit default.
  savedMainAxis: number | null;
}

export function computeOverlaySize(args: OverlaySizeArgs): {
  width: number;
  height: number;
} {
  const p = presetOf(args.scale);
  const count = args.visibleCount;
  if (args.orientation === "horizontal") {
    const chipsAxis = count > 0 ? count * p.chipBlock : p.horizontalEmptyMainAxis;
    const min = computeOverlayMinSize("horizontal", args.scale);
    return {
      width: Math.max(args.savedMainAxis ?? p.horizontalControlsWidth + chipsAxis, min.width),
      height: p.horizontalBarHeight,
    };
  }
  const chipsAxis = count > 0 ? count * p.chipBlock : p.verticalEmptyMainAxis;
  const min = computeOverlayMinSize("vertical", args.scale);
  return {
    width: p.verticalBarWidth,
    height: Math.max(
      args.savedMainAxis ?? p.verticalTopbarHeight + p.verticalControlsHeight + chipsAxis,
      min.height,
    ),
  };
}

/// OS-level minimum size — pins the cross axis to its locked natural
/// dimension and floors the main axis at the visible non-chip controls.
/// The chip area itself is allowed to fully collapse: shrunk past this
/// floor, chips clip but the broadcast/window-buttons cluster stays in
/// frame.
export function computeOverlayMinSize(
  orientation: Orientation,
  scale: OverlayScale,
): {
  width: number;
  height: number;
} {
  const p = presetOf(scale);
  if (orientation === "horizontal") {
    return {
      width: p.horizontalControlsWidth,
      height: p.horizontalBarHeight,
    };
  }
  return {
    width: p.verticalBarWidth,
    height: p.verticalTopbarHeight + p.verticalControlsHeight,
  };
}

/// Apply a size to the overlay window. Always targets the "overlay"
/// label — caller may run from any view.
export async function applyWindowSize(width: number, height: number): Promise<void> {
  const win = await WebviewWindow.getByLabel("overlay");
  if (!win) return;
  await win.setSize(new LogicalSize(width, height));
}
