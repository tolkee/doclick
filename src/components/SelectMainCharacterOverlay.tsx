import { Crown } from "lucide-react";
import { useDoclickStore } from "../store/useDoclickStore";

interface Props {
  /// Open Settings → Personnages so the user can crown a character.
  onOpenCharacters: () => void;
}

/// Full-overlay CTA shown until a main character is set. Gated on
/// `profiles.length > 0` so it doesn't shadow AvatarBar's own
/// import-prompt empty states (no Dofus windows / unimported windows),
/// for which the destination tab would be a dead end.
export function SelectMainCharacterOverlay({ onOpenCharacters }: Props) {
  const mainId = useDoclickStore((s) => s.mainCharacterId);
  const hasProfiles = useDoclickStore((s) => s.profiles.length > 0);
  if (mainId !== null || !hasProfiles) return null;

  return (
    <div className="pointer-events-none absolute inset-0 z-20 flex items-center justify-center rounded-xl bg-background/85 p-2 backdrop-blur-sm">
      <button
        type="button"
        onClick={onOpenCharacters}
        data-tauri-drag-region="false"
        className="pointer-events-auto inline-flex max-w-full items-center justify-center gap-1.5 rounded-md border border-border/60 bg-foreground/10 px-3 py-1.5 text-center text-xs font-medium text-foreground shadow-sm hover:bg-foreground/20"
      >
        <Crown className="h-3.5 w-3.5 shrink-0 text-yellow-400" />
        <span className="whitespace-normal leading-tight">Choisir un meneur</span>
      </button>
    </div>
  );
}
