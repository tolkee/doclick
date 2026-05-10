import { useDoclickStore } from "../store/useDoclickStore";
import type { ShortcutBindings } from "../types";
import { HotkeyInput } from "./HotkeyInput";

const GLOBAL_ROWS: { key: keyof Omit<ShortcutBindings, "focus_char">; label: string }[] = [
  { key: "toggle_broadcast", label: "Activer/désactiver le broadcast" },
  { key: "open_settings", label: "Ouvrir les paramètres" },
  { key: "close_app", label: "Fermer l'application" },
  { key: "send_travel_command", label: "Envoyer /travel du presse-papiers" },
];

const NAV_ROWS: { key: keyof Omit<ShortcutBindings, "focus_char">; label: string }[] = [
  { key: "focus_next", label: "Personnage suivant" },
  { key: "focus_prev", label: "Personnage précédent" },
  { key: "focus_main", label: "Personnage principal" },
];

export function ShortcutsTab() {
  const shortcuts = useDoclickStore((s) => s.shortcuts);
  const setShortcuts = useDoclickStore((s) => s.setShortcuts);
  const panicHotkey = useDoclickStore((s) => s.panicHotkey);
  const setPanicHotkey = useDoclickStore((s) => s.setPanicHotkey);

  const updateField = async (
    field: keyof Omit<ShortcutBindings, "focus_char">,
    value: string | null,
  ) => {
    await setShortcuts({ ...shortcuts, [field]: value });
  };

  const updateFocusChar = async (i: number, value: string | null) => {
    const next = shortcuts.focus_char.slice();
    next[i] = value;
    await setShortcuts({ ...shortcuts, focus_char: next });
  };

  return (
    <div className="grid gap-8 md:grid-cols-2 max-w-3xl">
      <section className="space-y-3">
        <h2 className="text-lg font-semibold">Global</h2>
        <div className="space-y-1">
          {GLOBAL_ROWS.map((row) => (
            <Row key={row.key} label={row.label}>
              <HotkeyInput value={shortcuts[row.key]} onChange={(v) => updateField(row.key, v)} />
            </Row>
          ))}
          <Row label="Raccourci panique">
            <HotkeyInput value={panicHotkey || null} onChange={(v) => setPanicHotkey(v ?? "")} />
          </Row>
        </div>
      </section>

      <section className="space-y-3">
        <h2 className="text-lg font-semibold">Naviguer entre les personnages</h2>
        <div className="space-y-1">
          {NAV_ROWS.map((row) => (
            <Row key={row.key} label={row.label}>
              <HotkeyInput value={shortcuts[row.key]} onChange={(v) => updateField(row.key, v)} />
            </Row>
          ))}
          {Array.from({ length: 8 }, (_, i) => i).map((i) => (
            <Row key={i} label={`Perso ${i + 1}`}>
              <HotkeyInput
                value={shortcuts.focus_char[i] ?? null}
                onChange={(v) => updateFocusChar(i, v)}
              />
            </Row>
          ))}
        </div>
      </section>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-1.5">
      <span className="text-sm">{label}</span>
      {children}
    </div>
  );
}
