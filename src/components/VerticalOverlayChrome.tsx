import { VERTICAL_TOPBAR_HEIGHT } from "../lib/overlaySize";
import { KebabButton } from "./KebabButton";

export function VerticalOverlayChrome() {
  return (
    <div
      data-tauri-drag-region
      className="flex shrink-0 items-center justify-center bg-background/70 backdrop-blur-md"
      style={{ height: VERTICAL_TOPBAR_HEIGHT }}
    >
      <KebabButton anchor="right-top" />
    </div>
  );
}
