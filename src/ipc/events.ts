import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  BroadcastStatePayload,
  BroadcastTickPayload,
  ErrorPayload,
  FocusedWindowChangedPayload,
  WindowsChangedPayload,
} from "../types";

export const EVT_WINDOWS_CHANGED = "windows-changed";
export const EVT_BROADCAST_STATE = "broadcast-state-changed";
export const EVT_BROADCAST_TICK = "broadcast-tick";
export const EVT_ERROR = "error";
export const EVT_OPEN_SETTINGS = "open-settings";
export const EVT_PREFS_CHANGED = "prefs-changed";
export const EVT_FOCUSED_WINDOW_CHANGED = "focused-window-changed";

export const onWindowsChanged = (cb: (p: WindowsChangedPayload) => void): Promise<UnlistenFn> =>
  listen<WindowsChangedPayload>(EVT_WINDOWS_CHANGED, (e) => cb(e.payload));

export const onFocusedWindowChanged = (
  cb: (p: FocusedWindowChangedPayload) => void,
): Promise<UnlistenFn> =>
  listen<FocusedWindowChangedPayload>(EVT_FOCUSED_WINDOW_CHANGED, (e) => cb(e.payload));

export const onBroadcastState = (cb: (p: BroadcastStatePayload) => void): Promise<UnlistenFn> =>
  listen<BroadcastStatePayload>(EVT_BROADCAST_STATE, (e) => cb(e.payload));

export const onBroadcastTick = (cb: (p: BroadcastTickPayload) => void): Promise<UnlistenFn> =>
  listen<BroadcastTickPayload>(EVT_BROADCAST_TICK, (e) => cb(e.payload));

export const onError = (cb: (p: ErrorPayload) => void): Promise<UnlistenFn> =>
  listen<ErrorPayload>(EVT_ERROR, (e) => cb(e.payload));

export const onOpenSettings = (cb: () => void): Promise<UnlistenFn> =>
  listen<void>(EVT_OPEN_SETTINGS, () => cb());

export const onPrefsChanged = (cb: () => void): Promise<UnlistenFn> =>
  listen<void>(EVT_PREFS_CHANGED, () => cb());
