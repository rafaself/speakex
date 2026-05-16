import type { RecordingShortcutStatus } from "$lib/native/shortcut";

export type ShortcutActionState =
  | "idle"
  | "loading"
  | "applying"
  | "reapplying"
  | "clearing"
  | "error";

interface LanguageOption {
  value: string;
  label: string;
}

export function buildRecordingShortcutStatusMessage(
  actionState: ShortcutActionState,
  status: RecordingShortcutStatus | null,
  savedShortcut: string | null,
  errorMessage: string
): string {
  if (actionState === "loading") {
    return "Checking the current recording shortcut status…";
  }

  if (actionState === "applying") {
    return "Saving the shortcut locally and applying it now…";
  }

  if (actionState === "reapplying") {
    return "Trying the saved shortcut again…";
  }

  if (actionState === "clearing") {
    return "Clearing the saved shortcut and unregistering it from the current runtime…";
  }

  if (actionState === "error") {
    return errorMessage || "Unable to update the recording shortcut.";
  }

  if (status === null) {
    return "Recording shortcut status is unavailable right now.";
  }

  if (status.state === "active") {
    const activeShortcut = status.activeShortcut ?? status.requestedShortcut ?? "the current shortcut";

    if (status.source === "default") {
      return `No saved shortcut exists, so SpeakEx registered the default ${activeShortcut}.`;
    }

    if (status.source === "saved") {
      return `The saved recording shortcut ${activeShortcut} is active.`;
    }

    return `The recording shortcut ${activeShortcut} is active in the current runtime.`;
  }

  if (status.state === "invalid") {
    return status.detail ?? "The saved recording shortcut could not be parsed.";
  }

  if (status.state === "unavailable") {
    return (
      status.detail ??
      "The requested recording shortcut could not be registered, likely because another app or the system already uses it."
    );
  }

  if (savedShortcut === null) {
    return "No shortcut override is saved. Startup still tries the default Ctrl+Alt+A when no saved shortcut exists.";
  }

  return "The saved recording shortcut is not active right now.";
}

export function formatDuration(durationMs: number | null): string {
  if (durationMs === null || durationMs < 0) {
    return "—";
  }

  const totalSeconds = Math.round(durationMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }

  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

export function formatFileName(path: string): string {
  return path.split(/[/\\]/u).pop() ?? path;
}

export function resolveLanguageOptionLabel(
  languageCode: string | null | undefined,
  languageOptions: LanguageOption[]
): string | null {
  if (!languageCode) {
    return null;
  }

  return languageOptions.find((option) => option.value === languageCode)?.label ?? languageCode;
}

export function resolveDetectedLanguageCodeFromHistory(
  languageValue: string | null | undefined,
  languageOptions: LanguageOption[]
): string | null {
  if (!languageValue || languageValue === "Auto / unspecified") {
    return null;
  }

  const matchedOption = languageOptions.find(
    (option) => option.value === languageValue || option.label === languageValue
  );

  return matchedOption?.value ?? languageValue;
}