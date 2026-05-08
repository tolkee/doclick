import { describe, expect, it } from "vitest";
import { computeResizeStep } from "./resizePointer";

const initial = { width: 600, height: 400, x: 100, y: 200 };
const min = { width: 300, height: 200 };

describe("computeResizeStep", () => {
  it("East drag grows width without moving x", () => {
    const step = computeResizeStep("East", 50, 0, initial, min);
    expect(step).toEqual({ w: 650, h: 400, x: 100, y: 200, needsPos: false });
  });

  it("East drag clamps to min width", () => {
    const step = computeResizeStep("East", -1000, 0, initial, min);
    expect(step.w).toBe(min.width);
  });

  it("West drag shrinks width and shifts x to keep right edge anchored", () => {
    const step = computeResizeStep("West", 100, 0, initial, min);
    expect(step.w).toBe(500);
    expect(step.x).toBe(200);
    expect(step.needsPos).toBe(true);
  });

  it("West drag clamps width and pins x at the original right edge minus min", () => {
    const step = computeResizeStep("West", 1000, 0, initial, min);
    expect(step.w).toBe(min.width);
    expect(step.x).toBe(initial.x + initial.width - min.width);
  });

  it("South drag grows height without moving y", () => {
    const step = computeResizeStep("South", 0, 50, initial, min);
    expect(step).toEqual({ w: 600, h: 450, x: 100, y: 200, needsPos: false });
  });

  it("North drag shrinks height and shifts y to keep bottom edge anchored", () => {
    const step = computeResizeStep("North", 0, 100, initial, min);
    expect(step.h).toBe(300);
    expect(step.y).toBe(300);
    expect(step.needsPos).toBe(true);
  });

  it("North drag clamps height and pins y at the original bottom edge minus min", () => {
    const step = computeResizeStep("North", 0, 1000, initial, min);
    expect(step.h).toBe(min.height);
    expect(step.y).toBe(initial.y + initial.height - min.height);
  });

  it("zero delta returns the initial state", () => {
    const step = computeResizeStep("East", 0, 0, initial, min);
    expect(step).toEqual({
      w: initial.width,
      h: initial.height,
      x: initial.x,
      y: initial.y,
      needsPos: false,
    });
  });
});
