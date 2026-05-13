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
  close_all: string | null;
  /// 8 slots; element N maps to "switch to char N+1".
  focus_char: (string | null)[];
  focus_next: string | null;
  focus_prev: string | null;
  focus_main: string | null;
  send_travel_command: string | null;
  trigger_startup_flow: string | null;
}

export const EMPTY_SHORTCUT_BINDINGS: ShortcutBindings = {
  toggle_broadcast: null,
  open_settings: null,
  close_app: null,
  close_all: null,
  focus_char: [null, null, null, null, null, null, null, null],
  focus_next: null,
  focus_prev: null,
  focus_main: null,
  send_travel_command: null,
  trigger_startup_flow: null,
};

export interface LaunchAccountsAction {
  enabled: boolean;
  exe_path: string | null;
  last_path_error: string | null;
}

export interface LaunchGanymedeAction {
  enabled: boolean;
  exe_path: string | null;
  last_path_error: string | null;
}

export interface StartupFlowConfig {
  enabled: boolean;
  run_on_app_start: boolean;
  accounts: LaunchAccountsAction;
  ganymede: LaunchGanymedeAction;
}

export const EMPTY_STARTUP_FLOW_CONFIG: StartupFlowConfig = {
  enabled: false,
  run_on_app_start: false,
  accounts: { enabled: false, exe_path: null, last_path_error: null },
  ganymede: { enabled: false, exe_path: null, last_path_error: null },
};

export type StepStatus = "idle" | "running" | "skipped" | "done" | "failed";

export interface StepState {
  status: StepStatus;
  message: string | null;
  last_run_ms: number | null;
}

export interface StartupRuntimeState {
  running: boolean;
  accounts: StepState;
  ganymede: StepState;
}

export const EMPTY_STARTUP_RUNTIME: StartupRuntimeState = {
  running: false,
  accounts: { status: "idle", message: null, last_run_ms: null },
  ganymede: { status: "idle", message: null, last_run_ms: null },
};

export interface StartupFlowStatePayload {
  running: boolean;
  accounts: StepState;
  ganymede: StepState;
}

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
  settings_size: [number, number] | null;
  shortcuts: ShortcutBindings;
  startup_flow: StartupFlowConfig;
  startup_flow_runtime: StartupRuntimeState;
}

export type BroadcastReason = "user" | "auto-disabled-foreground-mismatch" | "panic-hotkey";

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

export type UpdateState =
  | "idle"
  | "checking"
  | "available"
  | "no-update"
  | "downloading"
  | "installing"
  | "error";

export interface UpdateStatePayload {
  state: UpdateState;
  version: string | null;
  notes: string | null;
  error: string | null;
}

export interface UpdateProgressPayload {
  downloaded: number;
  total: number | null;
}
