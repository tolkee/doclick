import { listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow, LogicalPosition } from "@tauri-apps/api/window";
import { MoreVertical } from "lucide-react";
import { useRef } from "react";
import { cn } from "../lib/utils";
import { useDoclickStore } from "../store/useDoclickStore";
import type { OverlayScale } from "../types";
import { Button } from "./ui/button";

const MENU_WIDTH = 160;
const MENU_GAP = 4;

/// Kebab is intentionally smaller than the avatar/broadcast to preserve
/// the visual hierarchy at every preset. Sizes (28/32/40) match the
/// kebabSize field in OVERLAY_SCALE_PRESETS. The `[&_svg...]:size-X`
/// suffix scales the lucide icon inside to keep its proportion.
const KEBAB_CLASS: Record<OverlayScale, string> = {
  small: "size-7 [&_svg:not([class*='size-'])]:size-3.5",
  medium: "size-8 [&_svg:not([class*='size-'])]:size-4",
  large: "size-10 [&_svg:not([class*='size-'])]:size-5",
};

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
  const updateAvailable = useDoclickStore(
    (s) =>
      s.updateState === "available" ||
      s.updateState === "downloading" ||
      s.updateState === "installing",
  );
  const scale = useDoclickStore((s) => s.overlayScale);

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
      className={cn("relative", KEBAB_CLASS[scale])}
    >
      <MoreVertical strokeWidth={2} />
      {updateAvailable && (
        <span
          aria-hidden="true"
          className="pointer-events-none absolute right-1 top-1 h-1.5 w-1.5 rounded-full bg-emerald-400 ring-1 ring-background"
        />
      )}
    </Button>
  );
}
