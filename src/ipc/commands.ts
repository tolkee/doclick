import { invoke } from "@tauri-apps/api/core";
import type {
  CharacterProfile,
  Orientation,
  OverlayScale,
  SettingsTabId,
  ShortcutBindings,
  StateSnapshot,
  WindowEntry,
} from "../types";

export const listWindows = () => invoke<WindowEntry[]>("list_windows");

export const getStateSnapshot = () => invoke<StateSnapshot>("get_state_snapshot");

export const setBroadcastEnabled = (enabled: boolean) =>
  invoke<void>("set_broadcast_enabled", { enabled });

export const setBroadcastKeys = (keys: number[]) => invoke<void>("set_broadcast_keys", { keys });

export const setPanicHotkey = (accelerator: string) =>
  invoke<void>("set_panic_hotkey", { accelerator });

export const upsertProfile = (profile: CharacterProfile) =>
  invoke<void>("upsert_profile", { profile });

export const deleteProfile = (id: string) => invoke<void>("delete_profile", { id });

export const acknowledgePvpWarning = () => invoke<void>("acknowledge_pvp_warning");

export const focusDofusWindow = (hwnd: number) => invoke<void>("focus_dofus_window", { hwnd });

export const cycleFocus = () => invoke<void>("cycle_focus");

export const saveOverlayPosition = (x: number, y: number) =>
  invoke<void>("save_overlay_position", { x, y });

export const saveOverlaySize = (orientation: Orientation, width: number, height: number) =>
  invoke<void>("save_overlay_size", { orientation, width, height });

export const saveSettingsSize = (width: number, height: number) =>
  invoke<void>("save_settings_size", { width, height });

export const saveSettingsPosition = (x: number, y: number) =>
  invoke<void>("save_settings_position", { x, y });

export const openSettings = (tab?: SettingsTabId) =>
  invoke<void>("open_settings", { tab: tab ?? null });

export const setMainCharacter = (id: string | null) => invoke<void>("set_main_character", { id });

export const setProfileOrder = (ids: string[]) => invoke<void>("set_profile_order", { ids });

export const setOrientation = (orientation: Orientation) =>
  invoke<void>("set_orientation", { orientation });

export const setOverlayScale = (scale: OverlayScale) =>
  invoke<void>("set_overlay_scale", { scale });

export const setShortcuts = (shortcuts: ShortcutBindings) =>
  invoke<void>("set_shortcuts", { shortcuts });

export const focusCharacterAtIndex = (index: number) =>
  invoke<void>("focus_character_at_index", { index });

export const focusNextCharacter = () => invoke<void>("focus_next_character");

export const focusPrevCharacter = () => invoke<void>("focus_prev_character");

export const focusMainCharacter = () => invoke<void>("focus_main_character");

export const checkForUpdate = () => invoke<void>("check_for_update");

export const installUpdateAndRelaunch = () => invoke<void>("install_update_and_relaunch");
