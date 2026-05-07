import { useEffect, useState } from "react";
import { useDoclickStore } from "./store/useDoclickStore";
import { onPrefsChanged, onWindowsChanged } from "./ipc/events";
import { GlobalTab } from "./Settings/GlobalTab";
import { CharactersTab } from "./Settings/CharactersTab";
import { ShortcutsTab } from "./Settings/ShortcutsTab";
import { AboutTab } from "./Settings/AboutTab";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { TitleBar } from "./components/TitleBar";

export type SettingsTabId = "global" | "characters" | "shortcuts" | "about";

const TABS: { id: SettingsTabId; label: string }[] = [
  { id: "global", label: "Général" },
  { id: "characters", label: "Personnages" },
  { id: "shortcuts", label: "Raccourcis" },
  { id: "about", label: "À propos" },
];

export default function Settings() {
  const hydrate = useDoclickStore((s) => s.hydrate);
  const [tab, setTab] = useState<SettingsTabId>("global");

  useEffect(() => {
    hydrate();
    const subs = [
      onWindowsChanged((p) =>
        useDoclickStore.setState({ windows: p.windows }),
      ),
      onPrefsChanged(() => hydrate()),
    ];
    return () => {
      subs.forEach((s) => s.then((off) => off()));
    };
  }, [hydrate]);

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
    </main>
  );
}
