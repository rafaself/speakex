import { invoke } from "@tauri-apps/api/core";

export type RecordingShortcutState = "unconfigured" | "active" | "invalid" | "unavailable";

export type RecordingShortcutSource = "none" | "saved" | "default" | "custom";

export interface RecordingShortcutStatus {
  state: RecordingShortcutState;
  source: RecordingShortcutSource;
  requestedShortcut: string | null;
  activeShortcut: string | null;
  detail: string | null;
}

export async function getRecordingShortcutStatus(): Promise<RecordingShortcutStatus> {
  return invoke<RecordingShortcutStatus>("get_recording_shortcut_status");
}

export async function applyRecordingShortcut(
  shortcut: string | null
): Promise<RecordingShortcutStatus> {
  return invoke<RecordingShortcutStatus>("apply_recording_shortcut", { shortcut });
}
