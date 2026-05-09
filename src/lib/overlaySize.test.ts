import { describe, expect, it } from "vitest";
import {
  HORIZONTAL_BAR_HEIGHT,
  SETTINGS_DEFAULT_SIZE,
  SETTINGS_MIN_SIZE,
  VERTICAL_BAR_WIDTH,
  VERTICAL_TOPBAR_HEIGHT,
  computeOverlayMinSize,
  computeOverlaySize,
  isValidSettingsSize,
} from "./overlaySize";

describe("isValidSettingsSize", () => {
  it("accepts sizes at or above the minimum", () => {
    expect(isValidSettingsSize([SETTINGS_MIN_SIZE.width, SETTINGS_MIN_SIZE.height])).toBe(true);
    expect(isValidSettingsSize([SETTINGS_DEFAULT_SIZE.width, SETTINGS_DEFAULT_SIZE.height])).toBe(
      true,
    );
    expect(isValidSettingsSize([1000, 1200])).toBe(true);
  });

  it("rejects sizes below the minimum on either axis", () => {
    expect(isValidSettingsSize([SETTINGS_MIN_SIZE.width - 1, SETTINGS_MIN_SIZE.height])).toBe(
      false,
    );
    expect(isValidSettingsSize([SETTINGS_MIN_SIZE.width, SETTINGS_MIN_SIZE.height - 1])).toBe(
      false,
    );
  });

  it("rejects non-finite or null sizes (poisoned cache)", () => {
    expect(isValidSettingsSize(null)).toBe(false);
    expect(isValidSettingsSize([NaN, 720])).toBe(false);
    expect(isValidSettingsSize([440, Infinity])).toBe(false);
  });
});

describe("computeOverlaySize", () => {
  it("locks the cross axis in horizontal mode", () => {
    const size = computeOverlaySize({
      orientation: "horizontal",
      visibleCount: 4,
      savedMainAxis: null,
    });
    expect(size.height).toBe(HORIZONTAL_BAR_HEIGHT);
  });

  it("locks the cross axis in vertical mode", () => {
    const size = computeOverlaySize({
      orientation: "vertical",
      visibleCount: 4,
      savedMainAxis: null,
    });
    expect(size.width).toBe(VERTICAL_BAR_WIDTH);
  });

  it("auto-fits to chip count once it exceeds the empty-placeholder floor", () => {
    const empty = computeOverlaySize({
      orientation: "horizontal",
      visibleCount: 0,
      savedMainAxis: null,
    });
    const eight = computeOverlaySize({
      orientation: "horizontal",
      visibleCount: 8,
      savedMainAxis: null,
    });
    expect(eight.width).toBeGreaterThan(empty.width);
  });

  it("uses the saved main axis when above the floor", () => {
    const size = computeOverlaySize({
      orientation: "horizontal",
      visibleCount: 0,
      savedMainAxis: 900,
    });
    expect(size.width).toBe(900);
  });

  it("clamps the saved main axis to the per-orientation minimum", () => {
    const horizontal = computeOverlaySize({
      orientation: "horizontal",
      visibleCount: 0,
      savedMainAxis: 50,
    });
    const min = computeOverlayMinSize("horizontal");
    expect(horizontal.width).toBe(min.width);

    const vertical = computeOverlaySize({
      orientation: "vertical",
      visibleCount: 0,
      savedMainAxis: 30,
    });
    const verticalMin = computeOverlayMinSize("vertical");
    expect(vertical.height).toBe(verticalMin.height);
  });
});

describe("computeOverlayMinSize", () => {
  it("returns the locked cross axis in horizontal mode", () => {
    const min = computeOverlayMinSize("horizontal");
    expect(min.height).toBe(HORIZONTAL_BAR_HEIGHT);
  });

  it("returns the locked cross axis in vertical mode", () => {
    const min = computeOverlayMinSize("vertical");
    expect(min.width).toBe(VERTICAL_BAR_WIDTH);
    expect(min.height).toBeGreaterThanOrEqual(VERTICAL_TOPBAR_HEIGHT);
  });
});
