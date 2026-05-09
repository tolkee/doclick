import { LogicalSize } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Orientation } from "../types";

/** Inner-bar height in horizontal mode: chip (44px) + padding for the
 * 3px focus-ring outset. */
export const HORIZONTAL_BAR_HEIGHT = 64;

/** Inner-bar width in vertical mode: chip column (44px) + side padding
 * for the focus ring. */
export const VERTICAL_BAR_WIDTH = 76;

/** Vertical chrome height: kebab button + drag region. */
export const VERTICAL_TOPBAR_HEIGHT = 44;

const CHIP_BLOCK = 56;

/** BroadcastToggle (44px) + dividers + outer padding + kebab (44px). */
const HORIZONTAL_CONTROLS_WIDTH = 152;

const VERTICAL_CONTROLS_HEIGHT = 76;

/** Auto-fit chip-area extent when no chips are imported, so the empty
 * placeholder still fits at default size. The user can drag smaller and
 * the placeholder will clip. */
const HORIZONTAL_EMPTY_MAIN_AXIS = 360;
const VERTICAL_EMPTY_MAIN_AXIS = 240;

export const SETTINGS_DEFAULT_SIZE = { width: 440, height: 720 };
export const SETTINGS_MIN_SIZE = { width: 380, height: 560 };

/** Reject sizes below the minimum on either axis. Older builds occasionally
 * persisted overlay dimensions as settings_size; without this check the
 * settings view re-applies them on every open. */
export function isValidSettingsSize(s: [number, number] | null): boolean {
  return (
    s != null &&
    Number.isFinite(s[0]) &&
    Number.isFinite(s[1]) &&
    s[0] >= SETTINGS_MIN_SIZE.width &&
    s[1] >= SETTINGS_MIN_SIZE.height
  );
}

export interface OverlaySizeArgs {
  orientation: Orientation;
  visibleCount: number;
  /** Saved main-axis value (width in horizontal, height in vertical), or
   * null to use the auto-fit default. */
  savedMainAxis: number | null;
}

/** The cross axis is locked to the bar's natural size; only the main axis
 * is user-resizable and persists per orientation. */
export function computeOverlaySize(args: OverlaySizeArgs): {
  width: number;
  height: number;
} {
  const count = args.visibleCount;
  if (args.orientation === "horizontal") {
    const chipsAxis = count > 0 ? count * CHIP_BLOCK : HORIZONTAL_EMPTY_MAIN_AXIS;
    const min = computeOverlayMinSize("horizontal");
    return {
      width: Math.max(args.savedMainAxis ?? HORIZONTAL_CONTROLS_WIDTH + chipsAxis, min.width),
      height: HORIZONTAL_BAR_HEIGHT,
    };
  }
  const chipsAxis = count > 0 ? count * CHIP_BLOCK : VERTICAL_EMPTY_MAIN_AXIS;
  const min = computeOverlayMinSize("vertical");
  return {
    width: VERTICAL_BAR_WIDTH,
    height: Math.max(
      args.savedMainAxis ?? VERTICAL_TOPBAR_HEIGHT + VERTICAL_CONTROLS_HEIGHT + chipsAxis,
      min.height,
    ),
  };
}

/** OS-level minimum size: pins the cross axis and floors the main axis at
 * the visible non-chip controls. The chip area is allowed to fully collapse
 * past this — chips clip but the controls stay in frame. */
export function computeOverlayMinSize(orientation: Orientation): {
  width: number;
  height: number;
} {
  if (orientation === "horizontal") {
    return { width: HORIZONTAL_CONTROLS_WIDTH, height: HORIZONTAL_BAR_HEIGHT };
  }
  return { width: VERTICAL_BAR_WIDTH, height: VERTICAL_TOPBAR_HEIGHT + VERTICAL_CONTROLS_HEIGHT };
}

export async function applyWindowSize(width: number, height: number): Promise<void> {
  const win = await WebviewWindow.getByLabel("overlay");
  if (!win) return;
  await win.setSize(new LogicalSize(width, height));
}
