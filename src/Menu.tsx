import { useCallback, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { emit } from "@tauri-apps/api/event";
import { Minus, Settings as SettingsIcon, X } from "lucide-react";
import { EVT_OPEN_SETTINGS } from "./ipc/events";

/**
 * Content for the dedicated "menu" Tauri window (declared in
 * tauri.conf.json). Shown by KebabButton, hides on blur. Items act on the
 * overlay window then close this one.
 */
export default function Menu() {
  const hide = useCallback(async () => {
    try {
      // Emit before hiding so the kebab's visibility mirror flips to false
      // before the next click reads it.
      await emit("menu:closed");
      await getCurrentWindow().hide();
    } catch {}
  }, []);

  useEffect(() => {
    const win = getCurrentWindow();
    const unlistenFocusP = win.onFocusChanged(({ payload: focused }) => {
      if (focused) void emit("menu:opened");
      else void hide();
    });
    return () => {
      unlistenFocusP.then((off) => off()).catch(() => {});
    };
  }, [hide]);

  const onSettings = async () => {
    await emit(EVT_OPEN_SETTINGS);
    await hide();
  };

  const onMinimize = async () => {
    const overlay = await WebviewWindow.getByLabel("overlay");
    if (overlay) await overlay.minimize();
    await hide();
  };

  const onClose = async () => {
    const overlay = await WebviewWindow.getByLabel("overlay");
    await hide();
    if (overlay) await overlay.close();
  };

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden rounded-md border border-border/50 bg-background py-1 shadow-2xl">
      <MenuItem
        onClick={onSettings}
        icon={<SettingsIcon className="h-3.5 w-3.5" strokeWidth={2} />}
      >
        Paramètres
      </MenuItem>
      <MenuItem onClick={onMinimize} icon={<Minus className="h-3.5 w-3.5" strokeWidth={2} />}>
        Minimiser
      </MenuItem>
      <MenuItem onClick={onClose} icon={<X className="h-4 w-4" strokeWidth={2} />} danger>
        Fermer
      </MenuItem>
    </div>
  );
}

function MenuItem({
  onClick,
  icon,
  danger,
  children,
}: {
  onClick: () => void;
  icon: React.ReactNode;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex h-8 w-full items-center gap-2 px-3 text-left text-xs text-foreground/90 transition-colors ${
        danger ? "hover:bg-red-600 hover:text-white" : "hover:bg-foreground/10"
      }`}
    >
      <span className="flex h-4 w-4 items-center justify-center">{icon}</span>
      <span>{children}</span>
    </button>
  );
}
