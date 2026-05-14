import { presetOf } from "../lib/overlaySize";
import { useDoclickStore } from "../store/useDoclickStore";
import { KebabButton } from "./KebabButton";

export function VerticalOverlayChrome() {
  const scale = useDoclickStore((s) => s.overlayScale);
  return (
    <div
      data-tauri-drag-region
      className="flex shrink-0 items-center justify-center bg-background/70 backdrop-blur-md"
      style={{ height: presetOf(scale).verticalTopbarHeight }}
    >
      <KebabButton anchor="right-top" />
    </div>
  );
}
