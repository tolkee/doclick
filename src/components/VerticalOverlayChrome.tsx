import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Settings, X } from "lucide-react";
import { VERTICAL_TOPBAR_HEIGHT } from "../lib/overlaySize";
import { Button } from "./ui/button";

interface Props {
  onOpenSettings: () => void;
}

export function VerticalOverlayChrome({ onOpenSettings }: Props) {
  const win = getCurrentWindow();

  return (
    <div
      data-tauri-drag-region
      className="flex shrink-0 flex-col items-center bg-background/70 backdrop-blur-md"
      style={{ height: VERTICAL_TOPBAR_HEIGHT }}
    >
      <div className="flex h-9 w-full items-center">
        <button
          type="button"
          aria-label="Minimiser"
          title="Minimiser"
          data-tauri-drag-region="false"
          className="flex h-9 flex-1 items-center justify-center text-foreground/90 transition-colors hover:bg-foreground/10"
          onClick={() => void win.minimize()}
        >
          <Minus className="h-3.5 w-3.5" strokeWidth={2} />
        </button>
        <button
          type="button"
          aria-label="Fermer"
          title="Fermer"
          data-tauri-drag-region="false"
          className="flex h-9 flex-1 items-center justify-center text-foreground/90 transition-colors hover:bg-red-600 hover:text-white"
          onClick={() => void win.close()}
        >
          <X className="h-4 w-4" strokeWidth={2} />
        </button>
      </div>
      <Button
        type="button"
        variant="ghost"
        size="icon-lg"
        aria-label="Paramètres"
        title="Paramètres"
        data-tauri-drag-region="false"
        className="mt-2 text-foreground/90 hover:bg-foreground/10"
        onClick={onOpenSettings}
      >
        <Settings className="h-4.5 w-4.5" strokeWidth={2} />
      </Button>
    </div>
  );
}
