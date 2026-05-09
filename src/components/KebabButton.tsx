import { listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow, LogicalPosition } from "@tauri-apps/api/window";
import { MoreVertical } from "lucide-react";
import { useRef } from "react";
import { Button } from "./ui/button";

const MENU_WIDTH = 160;
const MENU_GAP = 4;

/// Module-level mirror of the menu window's visibility, kept in sync via
/// the events the Menu component emits on show/hide. Lets the kebab
/// detect the "menu was already open" case synchronously at mousedown
/// time — the focus shift caused by the click would otherwise hide the
/// menu before our async isVisible() check could run.
let menuVisible = false;
listen<void>("menu:opened", () => {
  menuVisible = true;
}).catch(() => {});
listen<void>("menu:closed", () => {
  menuVisible = false;
}).catch(() => {});

type Anchor = "below-right" | "right-top";

interface Props {
  /// Where the menu window opens relative to this button.
  ///   `below-right`: dropdown style — top-right of menu sits below the
  ///   button's bottom-right (used in horizontal mode).
  ///   `right-top`:   flyout style — top-left of menu sits to the right
  ///   of the button (used in vertical mode where there's no room
  ///   below).
  anchor: Anchor;
  ariaLabel?: string;
}

/// Single button that opens the dedicated "menu" Tauri window. The menu
/// window is pre-declared in `tauri.conf.json` (visible:false) — this
/// just positions it next to the button and shows it.
export function KebabButton({ anchor, ariaLabel = "Menu" }: Props) {
  const ref = useRef<HTMLButtonElement | null>(null);
  // Captured at mousedown (before focus shifts) so onClick can decide
  // whether this click is "open" or "close". If the menu was visible
  // at mousedown, the focus-loss path already hides it — we just no-op.
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
