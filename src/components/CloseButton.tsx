import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-react";

export function CloseButton() {
  return (
    <button
      onClick={() => getCurrentWindow().close()}
      className="w-9 h-9 rounded-full bg-zinc-800/80 hover:bg-red-600 hover:text-white text-zinc-300 flex items-center justify-center transition-colors"
      title="Fermer doclick"
    >
      <X className="h-3.5 w-3.5" strokeWidth={2.5} />
    </button>
  );
}
