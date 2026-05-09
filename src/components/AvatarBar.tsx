import { UserPlus } from "lucide-react";
import { useMemo } from "react";
import { useDoclickStore } from "../store/useDoclickStore";
import type { WindowEntry } from "../types";
import { CharacterChip } from "./CharacterChip";

interface Props {
  onOpenCharacters: () => void;
}

export function AvatarBar({ onOpenCharacters }: Props) {
  const orientation = useDoclickStore((s) => s.orientation);
  const windows = useDoclickStore((s) => s.windows);
  const profileOrder = useDoclickStore((s) => s.profileOrder);

  // useMemo here (not in the selector) — Zustand's snapshot equality treats
  // a fresh array as a state change and loops the component.
  const visible = useMemo(() => orderVisible(windows, profileOrder), [windows, profileOrder]);

  // Launcher / non-character Dofus windows have character_name === null and
  // must not trigger the "import" CTA.
  const characterWindowCount = useMemo(
    () => windows.filter((w) => w.character_name !== null).length,
    [windows],
  );

  if (visible.length === 0) {
    if (characterWindowCount === 0) {
      return (
        <div className="flex h-full w-full items-center justify-center px-2 text-center text-[11px] text-muted-foreground">
          Aucune fenêtre Dofus ouverte
        </div>
      );
    }
    const detectedLabel =
      characterWindowCount === 1
        ? "1 personnage détecté"
        : `${characterWindowCount} personnages détectés`;
    if (orientation === "vertical") {
      return (
        <div className="flex h-full w-full flex-col items-center justify-center gap-1.5 px-1 text-center text-[11px] text-muted-foreground">
          <span>{detectedLabel}</span>
          <button
            type="button"
            onClick={onOpenCharacters}
            aria-label="Importer"
            title="Importer"
            className="inline-flex size-6 items-center justify-center rounded-md bg-foreground/10 text-foreground/90 hover:bg-foreground/20"
          >
            <UserPlus className="h-3.5 w-3.5" strokeWidth={2} />
          </button>
        </div>
      );
    }
    return (
      <div className="flex h-full w-full items-center justify-center gap-2 px-2 text-center text-[11px] text-muted-foreground">
        <span>{detectedLabel}</span>
        <button
          type="button"
          onClick={onOpenCharacters}
          data-tauri-drag-region="false"
          className="inline-flex h-6 items-center justify-center rounded-md bg-foreground/10 px-2 text-foreground/90 hover:bg-foreground/20"
        >
          Importer
        </button>
      </div>
    );
  }

  // px/py prevents the chip's 3px focus-ring outset from being clipped by
  // the overflow container — bar height equals chip height in horizontal.
  const cls =
    orientation === "horizontal"
      ? "flex flex-row items-center gap-1.5 overflow-x-auto no-scrollbar px-1 py-1"
      : "flex flex-col items-center gap-1.5 overflow-y-auto w-full no-scrollbar py-1 px-1";

  return (
    <div className={cls}>
      {visible.map((w) => (
        <CharacterChip key={w.hwnd} entry={w} />
      ))}
    </div>
  );
}

function orderVisible(windows: WindowEntry[], profileOrder: string[]): WindowEntry[] {
  const filtered = windows.filter((w) => w.profile != null);
  const idx = (w: WindowEntry) => {
    const id = w.profile?.id;
    if (!id) return Infinity;
    const i = profileOrder.indexOf(id);
    return i < 0 ? Infinity : i;
  };
  filtered.sort((a, b) => {
    const ia = idx(a);
    const ib = idx(b);
    if (ia !== ib) return ia - ib;
    const an = a.character_name ?? a.title;
    const bn = b.character_name ?? b.title;
    return an.localeCompare(bn);
  });
  return filtered;
}
