import { useDoclickStore } from "../store/useDoclickStore";
import type { Orientation } from "../types";
import { Button } from "./ui/button";
import { Separator } from "./ui/separator";
import { ToggleGroup, ToggleGroupItem } from "./ui/toggle-group";
import { cn } from "@/lib/utils";

interface Props {
  onOpenSettings: () => void;
}

export function SettingsMenu({ onOpenSettings }: Props) {
  const orientation = useDoclickStore((s) => s.orientation);
  const setOrientation = useDoclickStore((s) => s.setOrientation);

  // In vertical mode the overlay is a narrow column; open the menu to the
  // right of the gear instead of below it. SettingsButton expands the window
  // width on open so the popover has room.
  const popoverPos =
    orientation === "vertical"
      ? "left-full top-0 ml-2"
      : "right-0 top-full mt-2";

  return (
    <div
      data-tauri-drag-region="false"
      className={cn(
        "absolute z-50 w-56 rounded-xl bg-zinc-900/95 p-2 shadow-xl ring-1 ring-zinc-700 backdrop-blur-md",
        popoverPos,
      )}
    >
      <Row label="Disposition">
        <ToggleGroup
          type="single"
          value={orientation}
          onValueChange={(v) => v && setOrientation(v as Orientation)}
        >
          <ToggleGroupItem value="horizontal" className="h-8 px-2.5">
            Horiz
          </ToggleGroupItem>
          <ToggleGroupItem value="vertical" className="h-8 px-2.5">
            Vert
          </ToggleGroupItem>
        </ToggleGroup>
      </Row>
      <Separator className="my-1.5" />
      <Button
        variant="ghost"
        size="sm"
        onClick={onOpenSettings}
        className="w-full justify-start"
      >
        Ouvrir les paramètres…
      </Button>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-2 px-1.5 py-1.5">
      <span className="text-xs text-zinc-400">{label}</span>
      {children}
    </div>
  );
}
