import { ArrowRight } from "lucide-react";
import { useDoclickStore } from "../store/useDoclickStore";
import type { Orientation } from "../types";
import { Button } from "../components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "../components/ui/toggle-group";
import type { SettingsTabId } from "../Settings";

interface Props {
  onNavigate: (tab: SettingsTabId) => void;
}

export function GlobalTab({ onNavigate }: Props) {
  const orientation = useDoclickStore((s) => s.orientation);
  const setOrientation = useDoclickStore((s) => s.setOrientation);
  const profiles = useDoclickStore((s) => s.profiles);

  return (
    <div className="space-y-6 max-w-2xl">
      <header className="space-y-3">
        <img
          src="/logo-white.png"
          alt="Doclick"
          className="h-16 w-16 select-none"
          draggable={false}
        />
        <h1 className="text-3xl font-bold tracking-tight">Doclick</h1>
        <p className="text-sm text-muted-foreground">
          Doclick est un outil d'aide pour les teams multicompte sur Dofus. Il
          permet de faciliter l'accomplissement de quêtes en reproduisant les
          interactions faites sur le personnage principal sur les autres
          comptes de la team. Doclick propose également des outils pour
          naviguer entre les comptes d'une team.
        </p>
      </header>

      {profiles.length === 0 && (
        <div className="flex items-center justify-between gap-3 rounded-md border p-4">
          <p className="text-sm font-medium">Aucun personnage importé</p>
          <Button size="sm" onClick={() => onNavigate("characters")}>
            Aller aux personnages
            <ArrowRight />
          </Button>
        </div>
      )}

      <Row label="Orientation">
        <ToggleGroup
          type="single"
          value={orientation}
          onValueChange={(v) => v && setOrientation(v as Orientation)}
        >
          <ToggleGroupItem value="horizontal">Horizontal</ToggleGroupItem>
          <ToggleGroupItem value="vertical">Vertical</ToggleGroupItem>
        </ToggleGroup>
      </Row>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-2">
      <span className="text-sm">{label}</span>
      {children}
    </div>
  );
}
