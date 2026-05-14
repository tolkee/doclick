import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { ResizeHandles } from "./components/ResizeHandles";
import { TitleBar } from "./components/TitleBar";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { saveSettingsPosition } from "./ipc/commands";
import { onOpenSettings, onUpdateProgress, onUpdateState } from "./ipc/events";
import { isValidSettingsSize } from "./lib/overlaySize";
import { AboutTab } from "./Settings/AboutTab";
import { CharactersTab } from "./Settings/CharactersTab";
import { GlobalTab } from "./Settings/GlobalTab";
import { ShortcutsTab } from "./Settings/ShortcutsTab";
import { useDoclickStore } from "./store/useDoclickStore";
import type { SettingsTabId } from "./types";

const TABS: { id: SettingsTabId; label: string }[] = [
  { id: "global", label: "Général" },
  { id: "characters", label: "Personnages" },
  { id: "shortcuts", label: "Raccourcis" },
  { id: "about", label: "À propos" },
];

export default function Settings() {
  const hydrate = useDoclickStore((s) => s.hydrate);
  const [tab, setTab] = useState<SettingsTabId>("global");

  // The settings window has its own React tree and its own store, so it
  // must hydrate from the Rust snapshot independently of the overlay.
  useEffect(() => {
    hydrate();
  }, [hydrate]);

  // Switch tab whenever the Rust side asks us to open with a specific tab
  // (e.g. AvatarBar's "Personnages" CTA → openSettings("characters")).
  useEffect(() => {
    const offP = onOpenSettings((p) => {
      if (p.tab) setTab(p.tab);
    });
    return () => {
      offP.then((off) => off()).catch(() => {});
    };
  }, []);

  // The settings webview has its own store — without these listeners the
  // À propos tab's "Vérifier les mises à jour" button never reflects the
  // check's progress (the overlay window does, which is why the kebab dot
  // works in isolation).
  useEffect(() => {
    const subs = [
      onUpdateState((p) =>
        useDoclickStore.setState({
          updateState: p.state,
          updateAvailableVersion: p.version,
          updateNotes: p.notes,
          updateError: p.error,
          updateProgress:
            p.state === "downloading" ? useDoclickStore.getState().updateProgress : null,
        }),
      ),
      onUpdateProgress((p) => useDoclickStore.setState({ updateProgress: p })),
    ];
    return () => {
      for (const s of subs) s.then((off) => off()).catch(() => {});
    };
  }, []);

  // Persist user-resized window size. `onResized` also fires for
  // programmatic setSize from the Rust setup path, so we debounce + dedup
  // to avoid loops. Rejects sizes below SETTINGS_MIN_SIZE (poisoned cache
  // from earlier builds occasionally persisted overlay dimensions here).
  useEffect(() => {
    const win = getCurrentWindow();
    let timer: number | null = null;
    const offP = win.onResized(({ payload }) => {
      if (timer !== null) window.clearTimeout(timer);
      timer = window.setTimeout(async () => {
        try {
          const factor = await win.scaleFactor();
          const w = Math.round(payload.width / factor);
          const h = Math.round(payload.height / factor);
          if (!isValidSettingsSize([w, h])) return;
          const cur = useDoclickStore.getState().settingsSize;
          if (cur && cur[0] === w && cur[1] === h) return;
          await useDoclickStore.getState().saveSettingsSize(w, h);
        } catch {}
      }, 250);
    });
    return () => {
      offP.then((off) => off()).catch(() => {});
      if (timer !== null) window.clearTimeout(timer);
    };
  }, []);

  // Persist user-moved window position (debounced, ignoring the Win32
  // minimized sentinel so a stray -32000 doesn't spawn the window
  // offscreen on next launch).
  useEffect(() => {
    const win = getCurrentWindow();
    let timer: number | null = null;
    const offP = win.onMoved(({ payload }) => {
      if (payload.x <= -32000 || payload.y <= -32000) return;
      if (timer !== null) window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        saveSettingsPosition(payload.x, payload.y).catch(() => {});
      }, 400);
    });
    return () => {
      offP.then((off) => off()).catch(() => {});
      if (timer !== null) window.clearTimeout(timer);
    };
  }, []);

  return (
    <main className="flex h-screen flex-col select-text overflow-hidden rounded-xl border border-border/50 bg-background text-foreground shadow-2xl">
      <TitleBar title="Doclick" />
      <Tabs
        value={tab}
        onValueChange={(v) => setTab(v as SettingsTabId)}
        className="flex min-h-0 flex-1 flex-col"
      >
        <header className="flex justify-center border-b px-6 py-4">
          <TabsList>
            {TABS.map((t) => (
              <TabsTrigger key={t.id} value={t.id}>
                {t.label}
              </TabsTrigger>
            ))}
          </TabsList>
        </header>
        <section className="flex-1 overflow-y-auto px-6 py-6">
          <TabsContent value="global" className="mt-0">
            <GlobalTab onNavigate={setTab} />
          </TabsContent>
          <TabsContent value="characters" className="mt-0">
            <CharactersTab />
          </TabsContent>
          <TabsContent value="shortcuts" className="mt-0">
            <ShortcutsTab />
          </TabsContent>
          <TabsContent value="about" className="mt-0">
            <AboutTab />
          </TabsContent>
        </section>
      </Tabs>
      <ResizeHandles mode="settings" />
    </main>
  );
}
