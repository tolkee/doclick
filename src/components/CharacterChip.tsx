import { useDoclickStore } from "../store/useDoclickStore";
import { avatarUrlFor, classDisplayName } from "../lib/dofusClass";
import type { WindowEntry } from "../types";
import { Crown } from "./Crown";

interface Props {
  entry: WindowEntry;
}

export function CharacterChip({ entry }: Props) {
  const focusWindow = useDoclickStore((s) => s.focusWindow);
  const mainId = useDoclickStore((s) => s.mainCharacterId);
  const focusedHwnd = useDoclickStore((s) => s.focusedHwnd);

  const profile = entry.profile;
  const name = profile?.display_name ?? entry.character_name ?? entry.title.slice(0, 24);
  const initials = (name.match(/[A-Za-z0-9]/g) ?? ["?"]).slice(0, 2).join("").toUpperCase();

  const dofusClass = entry.dofus_class ?? profile?.dofus_class ?? null;
  const avatarSrc = avatarUrlFor(dofusClass);
  const isMain = !!profile && profile.id === mainId;
  const isFocused = focusedHwnd === entry.hwnd;
  const className = classDisplayName(dofusClass);

  const avatarRing = isFocused ? "ring-2 ring-sky-400 ring-offset-1 ring-offset-zinc-900" : "";

  return (
    <button
      onClick={() => focusWindow(entry.hwnd)}
      className="group relative flex-none rounded-full hover:brightness-110 transition"
      title={`${name}${className ? ` — ${className}` : ""}\n${entry.title}`}
    >
      <div
        className={`relative w-11 h-11 rounded-full overflow-hidden bg-zinc-700 flex items-center justify-center text-xs font-semibold text-zinc-200 ${avatarRing}`}
      >
        {avatarSrc ? (
          <img
            src={avatarSrc}
            alt={name}
            className="w-full h-full object-cover"
            draggable={false}
          />
        ) : (
          <span>{initials}</span>
        )}
        {isFocused && <span className="absolute inset-0 bg-sky-400/30 pointer-events-none" />}
        <span className="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full bg-emerald-500 ring-2 ring-zinc-900" />
        {isMain && (
          <span className="absolute inset-0 flex items-center justify-center pointer-events-none text-yellow-400 drop-shadow-[0_0_3px_rgba(0,0,0,0.85)]">
            <Crown size={22} />
          </span>
        )}
      </div>
    </button>
  );
}
