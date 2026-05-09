import {
  closestCenter,
  DndContext,
  type DragEndEvent,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Crown, Download, GripVertical, Trash2 } from "lucide-react";
import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { Button } from "../components/ui/button";
import { avatarUrlFor, classDisplayName } from "../lib/dofusClass";
import { useDoclickStore } from "../store/useDoclickStore";
import type { CharacterProfile, WindowEntry } from "../types";

function newId(): string {
  // `crypto.randomUUID` requires a secure context which some Tauri webview
  // configurations don't provide.
  return `p_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`;
}

function profileFromWindow(w: WindowEntry): CharacterProfile {
  if (w.profile) return w.profile;
  const guess = w.character_name ?? w.title.split(" - ")[0]?.trim() ?? w.title;
  const name = (guess || "Personnage").slice(0, 32);
  return {
    id: newId(),
    display_name: name,
    role: "follower",
    match_strategy: { kind: "WindowTitleContains", value: name },
    dofus_class: w.dofus_class,
  };
}

export function CharactersTab() {
  const windows = useDoclickStore((s) => s.windows);
  const profiles = useDoclickStore((s) => s.profiles);
  const profileOrder = useDoclickStore((s) => s.profileOrder);
  const mainId = useDoclickStore((s) => s.mainCharacterId);
  const upsert = useDoclickStore((s) => s.upsertProfile);
  const remove = useDoclickStore((s) => s.deleteProfile);
  const setMain = useDoclickStore((s) => s.setMainCharacter);
  const setProfileOrder = useDoclickStore((s) => s.setProfileOrder);

  const orderedProfiles = useMemo(
    () => orderProfiles(profiles, profileOrder),
    [profiles, profileOrder],
  );

  const liveByProfileId = useMemo(() => {
    const map = new Map<string, WindowEntry>();
    for (const w of windows) {
      if (w.profile) map.set(w.profile.id, w);
    }
    return map;
  }, [windows]);

  const importable = useMemo(
    () => windows.filter((w) => w.profile == null && w.character_name != null),
    [windows],
  );

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  const onDragEnd = (e: DragEndEvent) => {
    const { active, over } = e;
    if (!over || active.id === over.id) return;
    const ids = orderedProfiles.map((p) => p.id);
    const oldIdx = ids.indexOf(String(active.id));
    const newIdx = ids.indexOf(String(over.id));
    if (oldIdx < 0 || newIdx < 0) return;
    const next = arrayMove(ids, oldIdx, newIdx);
    setProfileOrder(next);
  };

  const handleImport = async (w: WindowEntry) => {
    await upsert(profileFromWindow(w));
  };

  return (
    <div className="space-y-8 max-w-3xl">
      <section className="space-y-3">
        <h2 className="text-lg font-semibold">Vos personnages</h2>
        {orderedProfiles.length === 0 ? (
          <p className="text-sm italic text-muted-foreground">Aucun personnage importé.</p>
        ) : (
          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
            <SortableContext
              items={orderedProfiles.map((p) => p.id)}
              strategy={verticalListSortingStrategy}
            >
              <ul className="space-y-2">
                {orderedProfiles.map((p, idx) => {
                  const live = liveByProfileId.get(p.id);
                  return (
                    <ImportedRow
                      key={p.id}
                      profile={p}
                      live={live}
                      index={idx}
                      isMain={mainId === p.id}
                      onToggleMain={() => setMain(mainId === p.id ? null : p.id)}
                      onForget={() => remove(p.id)}
                    />
                  );
                })}
              </ul>
            </SortableContext>
          </DndContext>
        )}
      </section>

      <section className="space-y-3">
        <h2 className="text-lg font-semibold">Trouver un personnage</h2>
        {importable.length === 0 ? (
          <p className="text-sm italic text-muted-foreground">Aucun personnage connecté.</p>
        ) : (
          <ul className="space-y-2">
            {importable.map((w) => (
              <ImportableRow key={w.hwnd} entry={w} onImport={() => handleImport(w)} />
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

interface ImportedRowProps {
  profile: CharacterProfile;
  live: WindowEntry | undefined;
  index: number;
  isMain: boolean;
  onToggleMain: () => void;
  onForget: () => void;
}

function ImportedRow({ profile, live, index, isMain, onToggleMain, onForget }: ImportedRowProps) {
  const dofusClass = live?.dofus_class ?? profile.dofus_class ?? null;
  const cls = classDisplayName(dofusClass);
  const avatar = avatarUrlFor(dofusClass);
  const online = live != null;
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: profile.id,
  });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <li
      ref={setNodeRef}
      style={style}
      className={cn(
        "flex items-center gap-3 rounded-md border bg-card px-3 py-2",
        isDragging && "opacity-50",
      )}
    >
      <button
        type="button"
        className="flex h-8 w-6 shrink-0 cursor-grab items-center justify-center text-muted-foreground hover:text-foreground active:cursor-grabbing"
        {...attributes}
        {...listeners}
      >
        <GripVertical className="h-4 w-4" />
      </button>
      <span className="w-5 text-center text-xs text-muted-foreground">{index + 1}</span>
      <CharacterAvatar src={avatar} className={cls} />
      <span
        className={cn(
          "h-2.5 w-2.5 shrink-0 rounded-full",
          online ? "bg-emerald-500" : "bg-red-500",
        )}
        title={online ? "Connecté" : "Aucune fenêtre"}
      />
      <div className="flex flex-1 flex-col leading-tight">
        <span className="truncate text-sm font-medium">{profile.display_name}</span>
        {cls && <span className="truncate text-xs text-muted-foreground">{cls}</span>}
      </div>
      <Button
        variant="ghost"
        size="icon"
        onClick={onToggleMain}
        className={cn("h-8 w-8", isMain && "text-yellow-400")}
      >
        <Crown className={cn("h-4 w-4", isMain && "fill-yellow-400")} />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        onClick={onForget}
        className="h-8 w-8 text-muted-foreground hover:text-destructive"
      >
        <Trash2 className="h-4 w-4" />
      </Button>
    </li>
  );
}

interface ImportableRowProps {
  entry: WindowEntry;
  onImport: () => void;
}

function ImportableRow({ entry, onImport }: ImportableRowProps) {
  const cls = classDisplayName(entry.dofus_class);
  const avatar = avatarUrlFor(entry.dofus_class);
  const guessedName = entry.character_name ?? entry.title.split(" - ")[0]?.trim() ?? entry.title;

  return (
    <li className="flex items-center gap-3 rounded-md border px-3 py-2">
      <CharacterAvatar src={avatar} className={cls} />
      <div className="flex flex-1 flex-col leading-tight">
        <span className="truncate text-sm font-medium">{guessedName}</span>
        {cls && <span className="truncate text-xs text-muted-foreground">{cls}</span>}
      </div>
      <Button size="sm" onClick={onImport}>
        <Download />
        Importer
      </Button>
    </li>
  );
}

function CharacterAvatar({ src, className }: { src: string | null; className: string | null }) {
  return (
    <div className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-muted text-xs">
      {src ? (
        <img src={src} alt={className ?? ""} className="h-full w-full object-cover" />
      ) : (
        <span className="text-muted-foreground">?</span>
      )}
    </div>
  );
}

function orderProfiles(profiles: CharacterProfile[], profileOrder: string[]): CharacterProfile[] {
  const idx = (p: CharacterProfile) => {
    const i = profileOrder.indexOf(p.id);
    return i < 0 ? Infinity : i;
  };
  const copy = profiles.slice();
  copy.sort((a, b) => {
    const ia = idx(a);
    const ib = idx(b);
    if (ia !== ib) return ia - ib;
    return a.display_name.localeCompare(b.display_name);
  });
  return copy;
}
