import { create } from "zustand";
import * as cmd from "../ipc/commands";
import {
  type BroadcastReason,
  type CharacterProfile,
  EMPTY_SHORTCUT_BINDINGS,
  type Orientation,
  type OverlaySizes,
  type ShortcutBindings,
  type WindowEntry,
} from "../types";

interface DoclickState {
  windows: WindowEntry[];
  profiles: CharacterProfile[];
  broadcastEnabled: boolean;
  broadcastLive: boolean;
  broadcastReason: BroadcastReason | null;
  panicHotkey: string;
  pvpWarningAcknowledged: boolean;
  mainCharacterId: string | null;
  profileOrder: string[];
  orientation: Orientation;
  overlaySizes: OverlaySizes;
  settingsSize: [number, number] | null;
  shortcuts: ShortcutBindings;
  broadcastKeys: number[];
  focusedHwnd: number | null;
  lastError: string | null;
  /// True once `hydrate()` has completed at least once. Effects that
  /// resize the overlay window must wait for this — before hydrate the
  /// store holds defaults that would otherwise clobber the size that
  /// Rust restored on app startup.
  hydrated: boolean;

  hydrate: () => Promise<void>;
  setWindows: (w: WindowEntry[]) => void;
  setBroadcast: (enabled: boolean, reason: BroadcastReason | null) => void;
  setBroadcastLive: (live: boolean) => void;
  setError: (msg: string | null) => void;

  toggleBroadcast: () => Promise<void>;
  upsertProfile: (p: CharacterProfile) => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
  focusWindow: (hwnd: number) => Promise<void>;
  acknowledgePvp: () => Promise<void>;
  setMainCharacter: (id: string | null) => Promise<void>;
  setProfileOrder: (ids: string[]) => Promise<void>;
  setOrientation: (orientation: Orientation) => Promise<void>;
  saveOverlaySize: (orientation: Orientation, width: number, height: number) => Promise<void>;
  saveSettingsSize: (width: number, height: number) => Promise<void>;
  setShortcuts: (shortcuts: ShortcutBindings) => Promise<void>;
  setPanicHotkey: (accelerator: string) => Promise<void>;
  setBroadcastKeys: (keys: number[]) => Promise<void>;
}

export const useDoclickStore = create<DoclickState>((set, get) => ({
  windows: [],
  profiles: [],
  broadcastEnabled: false,
  broadcastLive: false,
  broadcastReason: null,
  panicHotkey: "Ctrl+Shift+F12",
  pvpWarningAcknowledged: false,
  mainCharacterId: null,
  profileOrder: [],
  orientation: "horizontal",
  overlaySizes: { horizontal: null, vertical: null },
  settingsSize: null,
  shortcuts: EMPTY_SHORTCUT_BINDINGS,
  broadcastKeys: [],
  focusedHwnd: null,
  lastError: null,
  hydrated: false,

  hydrate: async () => {
    const snap = await cmd.getStateSnapshot();
    const shortcuts: ShortcutBindings = {
      ...EMPTY_SHORTCUT_BINDINGS,
      ...snap.shortcuts,
      focus_char: padFocusChar(snap.shortcuts?.focus_char ?? []),
    };
    set({
      windows: snap.windows,
      profiles: snap.profiles ?? [],
      broadcastEnabled: snap.broadcast_enabled,
      panicHotkey: snap.panic_hotkey,
      pvpWarningAcknowledged: snap.pvp_warning_acknowledged,
      mainCharacterId: snap.main_character_id,
      profileOrder: snap.profile_order,
      orientation: snap.orientation,
      overlaySizes: snap.overlay_sizes ?? { horizontal: null, vertical: null },
      settingsSize: snap.settings_size ?? null,
      shortcuts,
      broadcastKeys: snap.broadcast_keys,
      hydrated: true,
    });
  },

  setWindows: (w) => set({ windows: w }),
  setBroadcast: (enabled, reason) => set({ broadcastEnabled: enabled, broadcastReason: reason }),
  setBroadcastLive: (live) => set({ broadcastLive: live }),
  setError: (msg) => set({ lastError: msg }),

  toggleBroadcast: async () => {
    const next = !get().broadcastEnabled;
    await cmd.setBroadcastEnabled(next);
    set({ broadcastEnabled: next, broadcastReason: "user" });
  },
  upsertProfile: async (p) => {
    await cmd.upsertProfile(p);
    await get().hydrate();
  },
  deleteProfile: async (id) => {
    await cmd.deleteProfile(id);
    await get().hydrate();
  },
  focusWindow: async (hwnd) => {
    await cmd.focusDofusWindow(hwnd);
  },
  acknowledgePvp: async () => {
    await cmd.acknowledgePvpWarning();
    set({ pvpWarningAcknowledged: true });
  },
  setMainCharacter: async (id) => {
    await cmd.setMainCharacter(id);
    set({ mainCharacterId: id });
  },
  setProfileOrder: async (ids) => {
    await cmd.setProfileOrder(ids);
    set({ profileOrder: ids });
  },
  setOrientation: async (orientation) => {
    await cmd.setOrientation(orientation);
    set({ orientation });
    // App.tsx applies the overlay window size in response to the
    // orientation change — and skips it while the settings view is open
    // so the user's edits aren't shrunk away mid-toggle.
  },
  saveOverlaySize: async (orientation, width, height) => {
    await cmd.saveOverlaySize(orientation, width, height);
    set((s) => ({
      overlaySizes: { ...s.overlaySizes, [orientation]: [width, height] },
    }));
  },
  saveSettingsSize: async (width, height) => {
    await cmd.saveSettingsSize(width, height);
    set({ settingsSize: [width, height] });
  },
  setShortcuts: async (shortcuts) => {
    const padded: ShortcutBindings = {
      ...shortcuts,
      focus_char: padFocusChar(shortcuts.focus_char),
    };
    await cmd.setShortcuts(padded);
    set({ shortcuts: padded });
  },
  setPanicHotkey: async (accelerator) => {
    await cmd.setPanicHotkey(accelerator);
    set({ panicHotkey: accelerator });
  },
  setBroadcastKeys: async (keys) => {
    await cmd.setBroadcastKeys(keys);
    set({ broadcastKeys: keys });
  },
}));

function padFocusChar(slots: (string | null)[]): (string | null)[] {
  const out = slots.slice(0, 8);
  while (out.length < 8) out.push(null);
  return out;
}

// (Selectors that allocate new arrays/objects each call must NOT be passed to
// `useDoclickStore(...)` directly — Zustand's snapshot equality treats every
// new ref as a change and infinite-loops. Compute derived data with `useMemo`
// at the call site over stable raw refs instead. See AvatarBar for the pattern.)
