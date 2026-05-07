export type Role = "follower" | "ignored";

export type Orientation = "horizontal" | "vertical";

export type MatchStrategy =
  | { kind: "WindowTitleContains"; value: string }
  | { kind: "Pid"; value: number };

export interface CharacterProfile {
  id: string;
  display_name: string;
  role: Role;
  match_strategy: MatchStrategy;
  /// Class slug captured at import time so the avatar still renders when no
  /// live Dofus window matches the profile.
  dofus_class: string | null;
}

export interface WindowEntry {
  hwnd: number;
  pid: number;
  title: string;
  class_name: string;
  dofus_class: string | null;
  character_name: string | null;
  profile: CharacterProfile | null;
}

export interface ShortcutBindings {
  toggle_broadcast: string | null;
  open_settings: string | null;
  close_app: string | null;
  /// 8 slots; element N maps to "switch to char N+1".
  focus_char: (string | null)[];
  focus_next: string | null;
  focus_prev: string | null;
  focus_main: string | null;
}

export const EMPTY_SHORTCUT_BINDINGS: ShortcutBindings = {
  toggle_broadcast: null,
  open_settings: null,
  close_app: null,
  focus_char: [null, null, null, null, null, null, null, null],
  focus_next: null,
  focus_prev: null,
  focus_main: null,
};

export interface OverlaySizes {
  horizontal: [number, number] | null;
  vertical: [number, number] | null;
}

export interface StateSnapshot {
  windows: WindowEntry[];
  profiles: CharacterProfile[];
  broadcast_enabled: boolean;
  broadcast_keys: number[];
  panic_hotkey: string;
  pvp_warning_acknowledged: boolean;
  main_character_id: string | null;
  profile_order: string[];
  orientation: Orientation;
  overlay_sizes: OverlaySizes;
  shortcuts: ShortcutBindings;
}

export type BroadcastReason =
  | "user"
  | "auto-disabled-foreground-mismatch"
  | "panic-hotkey";

export interface WindowsChangedPayload {
  windows: WindowEntry[];
}

export interface FocusedWindowChangedPayload {
  focused_hwnd: number | null;
}

export interface BroadcastStatePayload {
  enabled: boolean;
  reason: BroadcastReason;
}

export type BroadcastTickPayload =
  | { kind: "Started"; data: { followers: number } }
  | { kind: "Finished"; data: { ok: number; failed: number } };

export interface ErrorPayload {
  message: string;
  context: string | null;
}
