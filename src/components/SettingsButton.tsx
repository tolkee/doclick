import { useState, useRef, useEffect } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Settings as SettingsIcon } from "lucide-react";
import { useDoclickStore } from "../store/useDoclickStore";
import { SettingsMenu } from "./SettingsMenu";

// Vertical space the popover needs in horizontal layout (bar + gap + menu body).
const MENU_EXPANDED_HEIGHT = 280;
// Horizontal space the popover needs in vertical layout (column + gap + menu body).
const MENU_EXPANDED_WIDTH = 320;

// The main window is now persistent (declared in tauri.conf.json), so this
// just brings it back to the foreground if the user had it minimized.
export async function openSettings(): Promise<void> {
  const win = await WebviewWindow.getByLabel("main");
  if (!win) return;
  await win.show();
  await win.unminimize();
  await win.setFocus();
}

export function SettingsButton() {
  const [open, setOpen] = useState(false);
  const wrapperRef = useRef<HTMLDivElement>(null);

  // Outside-click + Escape to close.
  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  // Grow the OS window while the popover is open so the dropdown isn't clipped:
  // taller in horizontal mode, wider in vertical mode. Restore the user's
  // pre-open size on close.
  useEffect(() => {
    if (!open) return;
    const win = getCurrentWindow();
    const state = useDoclickStore.getState();

    let cancelled = false;
    let previousSize: { width: number; height: number } | null = null;

    (async () => {
      try {
        const inner = await win.innerSize();
        const factor = await win.scaleFactor();
        const logical = inner.toLogical(factor);
        previousSize = { width: logical.width, height: logical.height };
        const expanded =
          state.orientation === "vertical"
            ? {
                width: Math.max(MENU_EXPANDED_WIDTH, logical.width),
                height: logical.height,
              }
            : {
                width: logical.width,
                height: Math.max(MENU_EXPANDED_HEIGHT, logical.height),
              };
        if (cancelled) return;
        await win.setSize(new LogicalSize(expanded.width, expanded.height));
      } catch (err) {
        console.warn("menu: expand failed", err);
      }
    })();

    return () => {
      cancelled = true;
      (async () => {
        try {
          if (previousSize) {
            await win.setSize(
              new LogicalSize(previousSize.width, previousSize.height),
            );
          }
        } catch (err) {
          console.warn("menu: restore failed", err);
        }
      })();
    };
  }, [open]);

  return (
    <div ref={wrapperRef} className="relative">
      <button
        onClick={() => setOpen((v) => !v)}
        className="w-9 h-9 rounded-full bg-zinc-800/80 hover:bg-zinc-700 text-zinc-200 flex items-center justify-center"
        title="Menu des paramètres"
      >
        <SettingsIcon className="h-4 w-4" />
      </button>
      {open && (
        <SettingsMenu
          onOpenSettings={() => {
            setOpen(false);
            openSettings();
          }}
        />
      )}
    </div>
  );
}
