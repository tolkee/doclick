export type ResizeDirection = "East" | "West" | "North" | "South";

export interface ResizeInitial {
  width: number;
  height: number;
  x: number;
  y: number;
}

export interface ResizeMin {
  width: number;
  height: number;
}

export interface ResizeStep {
  w: number;
  h: number;
  x: number;
  y: number;
  needsPos: boolean;
}

/**
 * Compute the next window size + position for a single pointer-move event.
 *
 * West/North drags both shrink the window AND move it: when the user pulls
 * the left edge to the right, the window's right edge must stay anchored,
 * so x increases by the same amount the width decreased. East/South drags
 * leave position alone.
 *
 * Sizes are clamped to `min` but positions are not — we trust the caller's
 * window manager to keep the window onscreen.
 */
export function computeResizeStep(
  direction: ResizeDirection,
  dx: number,
  dy: number,
  initial: ResizeInitial,
  min: ResizeMin,
): ResizeStep {
  let w = initial.width;
  let h = initial.height;
  let x = initial.x;
  let y = initial.y;
  let needsPos = false;

  switch (direction) {
    case "East":
      w = Math.max(min.width, initial.width + dx);
      break;
    case "West":
      w = Math.max(min.width, initial.width - dx);
      x = initial.x + initial.width - w;
      needsPos = true;
      break;
    case "South":
      h = Math.max(min.height, initial.height + dy);
      break;
    case "North":
      h = Math.max(min.height, initial.height - dy);
      y = initial.y + initial.height - h;
      needsPos = true;
      break;
  }

  return { w, h, x, y, needsPos };
}
