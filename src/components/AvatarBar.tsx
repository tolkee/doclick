import { useMemo } from "react";
import { useDoclickStore } from "../store/useDoclickStore";
import type { WindowEntry } from "../types";
import { CharacterChip } from "./CharacterChip";

interface Props {
  /// Open Settings → Personnages. Used by the empty-state CTA when there
  /// are unimported Dofus windows.
  onOpenCharacters: () => void;
}

export function AvatarBar({ onOpenCharacters }: Props) {
  const orientation = useDoclickStore((s) => s.orientation);
  const windows = useDoclickStore((s) => s.windows);
  const profileOrder = useDoclickStore((s) => s.profileOrder);

  // Sort/filter via useMemo so the selector itself returns stable refs and
  // doesn't trip Zustand's snapshot-equality detection (would otherwise loop).
  const visible = useMemo(
    () => orderVisible(windows, profileOrder),
    [windows, profileOrder],
  );

  if (visible.length === 0) {
    if (windows.length === 0) {
      return (
        <div className="flex h-full w-full items-center justify-center px-2 text-center text-[11px] text-muted-foreground">
          Aucune fenêtre Dofus ouverte
        </div>
      );
    }
    return (
      <div className="flex h-full w-full items-center justify-center gap-2 px-2 text-center text-[11px] text-muted-foreground">
        <span>Aucun personnage importé</span>
        <button
          type="button"
          onClick={onOpenCharacters}
          data-tauri-drag-region="false"
          className="rounded-md bg-foreground/10 px-2 py-0.5 text-foreground/90 hover:bg-foreground/20"
        >
          Importer
        </button>
      </div>
    );
  }

  // Inner px/py keeps the focus ring (ring-2 + ring-offset-1 = 3px outset)
  // from being clipped by the overflow scroll container at the bar edges and
  // at the top/bottom of every chip in horizontal mode (where the bar's
  // height equals the chip's height).
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

function orderVisible(
  windows: WindowEntry[],
  profileOrder: string[],
): WindowEntry[] {
  // Only imported characters (i.e. those resolved to a profile) appear in the
  // overlay.
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
