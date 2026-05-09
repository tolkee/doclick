import { useRef } from "react";
import { getCurrentWindow, LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";
import { MoreVertical } from "lucide-react";
import { Button } from "./ui/button";

const MENU_WIDTH = 160;
const MENU_GAP = 4;

// Module-level mirror so the kebab can detect "menu already open" synchronously
// at mousedown — the click's focus shift hides the menu before any async
// isVisible() check would resolve.
let menuVisible = false;
listen<void>("menu:opened", () => {
  menuVisible = true;
}).catch(() => {});
listen<void>("menu:closed", () => {
  menuVisible = false;
}).catch(() => {});

type Anchor = "below-right" | "right-top";

interface Props {
  /** `below-right` (horizontal mode): dropdown — menu's top-right at the
   * button's bottom-right. `right-top` (vertical mode): flyout — menu's
   * top-left to the right of the button. */
  anchor: Anchor;
  ariaLabel?: string;
}

export function KebabButton({ anchor, ariaLabel = "Menu" }: Props) {
  const ref = useRef<HTMLButtonElement | null>(null);
  // Captured at mousedown (before focus shifts) so onClick can decide
  // whether this click should open or no-op (the focus-loss path closes
  // an already-visible menu before onClick runs).
  const wasVisibleAtMousedown = useRef(false);

  const open = async () => {
    if (wasVisibleAtMousedown.current) return;
    const btn = ref.current;
    if (!btn) return;
    try {
      const overlay = getCurrentWindow();
      const factor = await overlay.scaleFactor();
      const pos = (await overlay.outerPosition()).toLogical(factor);
      const rect = btn.getBoundingClientRect();

      let x: number;
      let y: number;
      if (anchor === "below-right") {
        x = pos.x + rect.right - MENU_WIDTH;
        y = pos.y + rect.bottom + MENU_GAP;
      } else {
        x = pos.x + rect.right + MENU_GAP;
        y = pos.y + rect.top;
      }

      const menu = await WebviewWindow.getByLabel("menu");
      if (!menu) return;
      await menu.setPosition(new LogicalPosition(Math.round(x), Math.round(y)));
      await menu.show();
      await menu.setFocus();
    } catch (err) {
      console.warn("KebabButton.open failed", err);
    }
  };

  return (
    <Button
      ref={ref}
      type="button"
      variant="ghost"
      size="icon-lg"
      aria-label={ariaLabel}
      title={ariaLabel}
      data-tauri-drag-region="false"
      onMouseDown={() => {
        wasVisibleAtMousedown.current = menuVisible;
      }}
      onClick={open}
    >
      <MoreVertical strokeWidth={2} />
    </Button>
  );
}
