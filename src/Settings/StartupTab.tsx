import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { Button } from "../components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "../components/ui/toggle-group";
import * as cmd from "../ipc/commands";
import { cn } from "../lib/utils";
import type { SettingsTabId } from "../Settings";
import { useDoclickStore } from "../store/useDoclickStore";
import type { StartupFlowConfig, StepState } from "../types";

interface Props {
  onNavigate: (tab: SettingsTabId) => void;
}

export function StartupTab({ onNavigate }: Props) {
  const config = useDoclickStore((s) => s.startupFlow);
  const runtime = useDoclickStore((s) => s.startupRuntime);
  const setStartupFlow = useDoclickStore((s) => s.setStartupFlow);
  const runStartupFlow = useDoclickStore((s) => s.runStartupFlow);
  const profileOrder = useDoclickStore((s) => s.profileOrder);
  const shortcut = useDoclickStore((s) => s.shortcuts.trigger_startup_flow);

  const update = async (patch: Partial<StartupFlowConfig>) => {
    await setStartupFlow({ ...config, ...patch });
  };

  return (
    <div className="space-y-6 max-w-2xl">
      <header className="space-y-1">
        <h1 className="text-2xl font-bold tracking-tight">Routine de démarrage</h1>
        <p className="text-sm text-muted-foreground">
          Automatise l'ouverture du launcher Ankama (avec autant de fenêtres Dofus que vous avez de
          personnages importés), puis de Ganymede. Les étapes déjà effectuées sont sautées
          automatiquement.
        </p>
      </header>

      <Row label="Activer la routine">
        <ToggleGroup
          type="single"
          value={config.enabled ? "on" : "off"}
          onValueChange={(v) => v && update({ enabled: v === "on" })}
        >
          <ToggleGroupItem value="off">Désactivée</ToggleGroupItem>
          <ToggleGroupItem value="on">Activée</ToggleGroupItem>
        </ToggleGroup>
      </Row>

      {config.enabled && (
        <>
          <section className="space-y-3">
            <h2 className="text-lg font-semibold">Déclencheurs</h2>
            <Row label="Lancer au démarrage de Doclick">
              <ToggleGroup
                type="single"
                value={config.run_on_app_start ? "on" : "off"}
                onValueChange={(v) => v && update({ run_on_app_start: v === "on" })}
              >
                <ToggleGroupItem value="off">Non</ToggleGroupItem>
                <ToggleGroupItem value="on">Oui</ToggleGroupItem>
              </ToggleGroup>
            </Row>
            <Row label="Raccourci manuel">
              <span className="text-xs text-muted-foreground">
                {shortcut ?? "Aucun"}
                <span className="mx-1">—</span>
                <button
                  type="button"
                  className="underline underline-offset-2 hover:text-foreground"
                  onClick={() => onNavigate("shortcuts")}
                >
                  Configurer…
                </button>
              </span>
            </Row>
          </section>

          <ActionSection
            title="Lancer les comptes Dofus"
            description={`Cible : ${profileOrder.length} compte(s) (basé sur vos personnages importés). Si des fenêtres Dofus sont déjà ouvertes, seule la différence sera lancée.`}
            kind="launcher"
            enabled={config.accounts.enabled}
            exePath={config.accounts.exe_path}
            error={config.accounts.last_path_error}
            status={runtime.accounts}
            onToggle={(enabled) => update({ accounts: { ...config.accounts, enabled } })}
            onPath={(exe_path) => update({ accounts: { ...config.accounts, exe_path } })}
          />

          <ActionSection
            title="Lancer Ganymede"
            description="Doclick ouvre Ganymede sauf si une fenêtre Ganymede est déjà détectée."
            kind="ganymede"
            enabled={config.ganymede.enabled}
            exePath={config.ganymede.exe_path}
            error={config.ganymede.last_path_error}
            status={runtime.ganymede}
            onToggle={(enabled) => update({ ganymede: { ...config.ganymede, enabled } })}
            onPath={(exe_path) => update({ ganymede: { ...config.ganymede, exe_path } })}
          />

          <section className="flex items-center gap-3">
            <Button onClick={() => runStartupFlow()} disabled={runtime.running}>
              {runtime.running ? "En cours…" : "Lancer maintenant"}
            </Button>
            {runtime.running && (
              <span className="text-xs text-muted-foreground">
                Suivez l'avancement dans les badges ci-dessus.
              </span>
            )}
          </section>
        </>
      )}
    </div>
  );
}

interface ActionSectionProps {
  title: string;
  description: string;
  kind: "launcher" | "ganymede";
  enabled: boolean;
  exePath: string | null;
  error: string | null;
  status: StepState;
  onToggle: (enabled: boolean) => void;
  onPath: (value: string | null) => void;
}

function ActionSection({
  title,
  description,
  kind,
  enabled,
  exePath,
  error,
  status,
  onToggle,
  onPath,
}: ActionSectionProps) {
  return (
    <section className="space-y-3 rounded-md border border-border/60 p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <h2 className="text-base font-semibold">{title}</h2>
          <p className="text-xs text-muted-foreground">{description}</p>
        </div>
        <StatusBadge status={status} />
      </div>
      <Row label="Activer">
        <ToggleGroup
          type="single"
          value={enabled ? "on" : "off"}
          onValueChange={(v) => v && onToggle(v === "on")}
        >
          <ToggleGroupItem value="off">Non</ToggleGroupItem>
          <ToggleGroupItem value="on">Oui</ToggleGroupItem>
        </ToggleGroup>
      </Row>
      <ExePathInput kind={kind} value={exePath} error={error} onChange={onPath} />
    </section>
  );
}

interface ExePathInputProps {
  kind: "launcher" | "ganymede";
  value: string | null;
  error: string | null;
  onChange: (value: string | null) => void;
}

function ExePathInput({ kind, value, error, onChange }: ExePathInputProps) {
  const [placeholder, setPlaceholder] = useState<string>("");

  useEffect(() => {
    let cancelled = false;
    cmd
      .getDefaultExePathHint(kind)
      .then((hint) => {
        if (!cancelled && hint) setPlaceholder(hint);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [kind]);

  const browse = async () => {
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "Exécutables", extensions: ["exe"] }],
        title: kind === "launcher" ? "Sélectionner le launcher Ankama" : "Sélectionner Ganymede",
      });
      if (typeof picked === "string" && picked.length > 0) {
        onChange(picked);
      }
    } catch (err) {
      console.warn("openDialog failed", err);
    }
  };

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between gap-3">
        <span className="text-sm">Exécutable</span>
        <div className="flex w-full max-w-md items-center gap-2">
          <input
            type="text"
            value={value ?? ""}
            placeholder={placeholder || "Détection auto…"}
            onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)}
            className={cn(
              "h-7 flex-1 rounded-md border bg-background px-2 text-xs outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30",
              error &&
                "border-destructive focus-visible:border-destructive/60 focus-visible:ring-destructive/20",
            )}
          />
          <Button size="sm" variant="outline" onClick={browse}>
            Parcourir…
          </Button>
        </div>
      </div>
      {error && (
        <p className="ml-auto max-w-md text-xs text-destructive">
          {error}{" "}
          <span className="text-muted-foreground">
            — corrigez le chemin pour réactiver cette action.
          </span>
        </p>
      )}
    </div>
  );
}

function StatusBadge({ status }: { status: StepState }) {
  const { label, tone } = describeStatus(status);
  return (
    <div className="flex flex-col items-end gap-1">
      <span
        className={cn(
          "rounded-full border px-2 py-0.5 text-[0.625rem] font-medium uppercase tracking-wide",
          tone,
        )}
      >
        {label}
      </span>
      {status.message && (
        <span className="max-w-[16rem] text-right text-[0.625rem] text-muted-foreground">
          {status.message}
        </span>
      )}
    </div>
  );
}

function describeStatus(state: StepState): { label: string; tone: string } {
  switch (state.status) {
    case "running":
      return {
        label: "En cours…",
        tone: "border-amber-500/40 bg-amber-500/10 text-amber-600 dark:text-amber-300",
      };
    case "done":
      return {
        label: "Terminé",
        tone: "border-emerald-500/40 bg-emerald-500/10 text-emerald-600 dark:text-emerald-300",
      };
    case "skipped":
      return {
        label: "Ignoré",
        tone: "border-border/60 bg-muted text-muted-foreground",
      };
    case "failed":
      return {
        label: "Échec",
        tone: "border-destructive/40 bg-destructive/10 text-destructive",
      };
    default:
      return {
        label: "En attente",
        tone: "border-border/60 bg-muted text-muted-foreground",
      };
  }
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-1.5">
      <span className="text-sm">{label}</span>
      {children}
    </div>
  );
}
