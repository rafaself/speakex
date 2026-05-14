import {
  applyRecordingShortcut,
  getRecordingShortcutStatus,
  type RecordingShortcutStatus
} from "$lib/native/shortcut";
import {
  clearGeminiApiKey,
  hasGeminiApiKey,
  saveGeminiApiKey
} from "$lib/native/secret-store";
import { loadAppSettings, saveAppSettings } from "$lib/native/settings";
import { type DraftToggleKey } from "$lib/stores/app-shell";
import type {
  GeminiApiKeyActionState,
  GeminiApiKeyPresenceState,
  SettingsDraft
} from "$lib/types/app-shell";
import type { RecordingInputDevice } from "$lib/native/recording";

interface SettingsControllerContext {
  getSettingsDraft: () => SettingsDraft;
  patchSettingsDraft: (value: Partial<SettingsDraft>) => void;
  setSettingsState: (state: "idle" | "loading" | "saving" | "error") => void;
  setSettingsError: (message: string) => void;
  getAvailableRecordingDevices: () => RecordingInputDevice[];
  setRecordingInputOptions: (devices: RecordingInputDevice[], selectedMicrophone: string) => void;
  getActiveRecordingSession: () => boolean;
  getAppStatusPhase: () => string;
  syncIdleStatus: () => void;
  getGeminiApiKeyDraft: () => string;
  setGeminiApiKeyDraft: (value: string) => void;
  getGeminiApiKeyPresence: () => boolean;
  setGeminiApiKeyPresence: (value: boolean) => void;
  setGeminiApiKeyPresenceState: (value: GeminiApiKeyPresenceState) => void;
  getGeminiApiKeyPresenceState: () => GeminiApiKeyPresenceState;
  setGeminiApiKeyActionState: (value: GeminiApiKeyActionState) => void;
  getGeminiApiKeyActionState: () => GeminiApiKeyActionState;
  setGeminiApiKeyStatusDetail: (value: string) => void;
  getRecordingShortcutDraft: () => string;
  setRecordingShortcutDraft: (value: string) => void;
  setRecordingShortcutStatus: (value: RecordingShortcutStatus | null) => void;
  getRecordingShortcutActionState: () => "idle" | "loading" | "applying" | "reapplying" | "clearing" | "error";
  setRecordingShortcutActionState: (
    value: "idle" | "loading" | "applying" | "reapplying" | "clearing" | "error"
  ) => void;
  setRecordingShortcutError: (value: string) => void;
  canClearRecordingShortcut: () => boolean;
}

function summarizeGeminiApiKeyFailure(detail: string, action: "check" | "save" | "clear") {
  const normalized = detail.replace(/\s+/gu, " ").trim();

  if (normalized === "") {
    return "SpeakEx could not determine the secure storage failure cause.";
  }

  if (/^secure storage is unavailable$/iu.test(normalized)) {
    if (action === "save") {
      return "Secure storage is unavailable, so SpeakEx could not save the Gemini API key.";
    }

    if (action === "clear") {
      return "Secure storage is unavailable, so SpeakEx could not clear the Gemini API key.";
    }

    return "Secure storage is unavailable, so SpeakEx could not check the Gemini API key.";
  }

  if (/^Gemini API key cannot be empty$/iu.test(normalized)) {
    return "Enter a Gemini API key before saving.";
  }

  const summarized = normalized.endsWith(".") ? normalized : `${normalized}.`;
  return summarized;
}

export function createSettingsController(context: SettingsControllerContext) {
  let lastSavedSettings: SettingsDraft | null = null;
  let settingsSaveQueue = Promise.resolve();
  let latestSettingsRequest = 0;

  async function hydrateSettings() {
    context.setSettingsState("loading");
    context.setSettingsError("");

    try {
      const persistedSettings = await loadAppSettings();

      context.patchSettingsDraft(persistedSettings);
      lastSavedSettings = persistedSettings;
      context.setRecordingShortcutDraft(persistedSettings.shortcut ?? "");
      context.setSettingsState("idle");
    } catch (error) {
      context.setSettingsError(
        error instanceof Error ? error.message : "Unable to load saved preferences"
      );
      context.setSettingsState("error");
    }
  }

  async function persistSettings(
    nextSettings: SettingsDraft,
    options: { rethrow?: boolean } = {}
  ) {
    const requestId = ++latestSettingsRequest;

    context.patchSettingsDraft(nextSettings);
    context.setSettingsState("saving");
    context.setSettingsError("");

    const saveOperation = settingsSaveQueue
      .catch(() => undefined)
      .then(() => saveAppSettings(nextSettings));

    settingsSaveQueue = saveOperation.then(
      () => undefined,
      () => undefined
    );

    try {
      const persistedSettings = await saveOperation;

      lastSavedSettings = persistedSettings;

      if (requestId !== latestSettingsRequest) {
        return;
      }

      context.patchSettingsDraft(persistedSettings);
      context.setRecordingInputOptions(
        context.getAvailableRecordingDevices(),
        persistedSettings.selectedMicrophone
      );

      if (!context.getActiveRecordingSession() && context.getAppStatusPhase() === "idle") {
        context.syncIdleStatus();
      }

      context.setSettingsState("idle");
    } catch (error) {
      const resolvedError =
        error instanceof Error ? error : new Error("Unable to save preferences");

      if (requestId !== latestSettingsRequest) {
        if (options.rethrow) {
          throw resolvedError;
        }

        return;
      }

      context.setSettingsError(resolvedError.message);
      context.setSettingsState("error");

      if (lastSavedSettings) {
        context.patchSettingsDraft(lastSavedSettings);
        context.setRecordingInputOptions(
          context.getAvailableRecordingDevices(),
          lastSavedSettings.selectedMicrophone
        );
      }

      if (options.rethrow) {
        throw resolvedError;
      }
    }
  }

  async function refreshRecordingShortcutStatus(showFeedback = true) {
    if (
      context.getRecordingShortcutActionState() === "applying" ||
      context.getRecordingShortcutActionState() === "reapplying" ||
      context.getRecordingShortcutActionState() === "clearing"
    ) {
      return;
    }

    context.setRecordingShortcutActionState("loading");

    if (showFeedback) {
      context.setRecordingShortcutError("");
    }

    try {
      context.setRecordingShortcutStatus(await getRecordingShortcutStatus());
      context.setRecordingShortcutActionState("idle");
    } catch (error) {
      context.setRecordingShortcutActionState("error");
      context.setRecordingShortcutError(
        error instanceof Error ? error.message : "Unable to load the recording shortcut status."
      );
    }
  }

  async function refreshGeminiApiKeyPresence(showFeedback = true) {
    if (
      context.getGeminiApiKeyActionState() === "saving" ||
      context.getGeminiApiKeyActionState() === "clearing"
    ) {
      return;
    }

    context.setGeminiApiKeyActionState("checking");
    context.setGeminiApiKeyPresenceState("loading");

    if (showFeedback) {
      context.setGeminiApiKeyStatusDetail("");
    }

    try {
      const geminiApiKeyPresence = await hasGeminiApiKey();
      context.setGeminiApiKeyPresence(geminiApiKeyPresence);
      context.setGeminiApiKeyPresenceState(geminiApiKeyPresence ? "present" : "missing");
      context.setGeminiApiKeyActionState("idle");

      if (showFeedback) {
        context.setGeminiApiKeyStatusDetail("");
      }
    } catch (error) {
      const detail =
        error instanceof Error ? error.message : "Unable to check the Gemini API key status.";

      context.setGeminiApiKeyPresenceState("error");
      context.setGeminiApiKeyActionState("error");
      context.setGeminiApiKeyStatusDetail(summarizeGeminiApiKeyFailure(detail, "check"));
    }
  }

  function updateLanguage(defaultLanguage: string) {
    void persistSettings({
      ...context.getSettingsDraft(),
      defaultLanguage
    });
  }

  function updateMicrophone(selectedMicrophone: string) {
    context.setRecordingInputOptions(context.getAvailableRecordingDevices(), selectedMicrophone);

    void persistSettings({
      ...context.getSettingsDraft(),
      selectedMicrophone
    });
  }

  function toggleSetting(key: DraftToggleKey) {
    const draft = context.getSettingsDraft();

    void persistSettings({
      ...draft,
      [key]: !draft[key]
    });
  }

  async function saveAndApplyRecordingShortcut(
    shortcut: string | null,
    action: "applying" | "clearing"
  ) {
    context.setRecordingShortcutActionState(action);
    context.setRecordingShortcutError("");

    try {
      await persistSettings(
        {
          ...context.getSettingsDraft(),
          shortcut
        },
        { rethrow: true }
      );

      const savedShortcut = context.getSettingsDraft().shortcut;
      context.setRecordingShortcutDraft(savedShortcut ?? "");
      context.setRecordingShortcutStatus(await applyRecordingShortcut(savedShortcut));
      context.setRecordingShortcutActionState("idle");
    } catch (error) {
      context.setRecordingShortcutDraft(context.getSettingsDraft().shortcut ?? "");
      context.setRecordingShortcutActionState("error");
      context.setRecordingShortcutError(
        error instanceof Error
          ? error.message
          : action === "clearing"
            ? "Unable to clear the saved recording shortcut."
            : "Unable to save and apply the recording shortcut."
      );
    }
  }

  async function submitRecordingShortcut() {
    const nextShortcut = context.getRecordingShortcutDraft().trim();

    if (
      (context.getRecordingShortcutActionState() !== "idle" &&
        context.getRecordingShortcutActionState() !== "error") ||
      nextShortcut.length === 0
    ) {
      return;
    }

    await saveAndApplyRecordingShortcut(nextShortcut, "applying");
  }

  async function reapplySavedRecordingShortcut() {
    const savedShortcut = context.getSettingsDraft().shortcut;

    if (
      (context.getRecordingShortcutActionState() !== "idle" &&
        context.getRecordingShortcutActionState() !== "error") ||
      savedShortcut === null
    ) {
      return;
    }

    context.setRecordingShortcutActionState("reapplying");
    context.setRecordingShortcutError("");

    try {
      context.setRecordingShortcutStatus(await applyRecordingShortcut(savedShortcut));
      context.setRecordingShortcutDraft(savedShortcut);
      context.setRecordingShortcutActionState("idle");
    } catch (error) {
      context.setRecordingShortcutActionState("error");
      context.setRecordingShortcutError(
        error instanceof Error
          ? error.message
          : "Unable to re-apply the saved recording shortcut."
      );
    }
  }

  async function clearRecordingShortcutSetting() {
    if (!context.canClearRecordingShortcut()) {
      return;
    }

    context.setRecordingShortcutDraft("");
    await saveAndApplyRecordingShortcut(null, "clearing");
  }

  async function submitGeminiApiKey() {
    const geminiApiKeyDraftValue = context.getGeminiApiKeyDraft().trim();

    if (
      context.getGeminiApiKeyActionState() === "checking" ||
      context.getGeminiApiKeyActionState() === "saving" ||
      context.getGeminiApiKeyActionState() === "clearing" ||
      geminiApiKeyDraftValue.length === 0
    ) {
      return;
    }

    context.setGeminiApiKeyActionState("saving");
    context.setGeminiApiKeyStatusDetail("");

    try {
      await saveGeminiApiKey(geminiApiKeyDraftValue);
      context.setGeminiApiKeyDraft("");
      context.setGeminiApiKeyPresence(true);
      context.setGeminiApiKeyPresenceState("present");
      context.setGeminiApiKeyActionState("idle");
      context.setGeminiApiKeyStatusDetail("");
    } catch (error) {
      const detail = error instanceof Error ? error.message : "Unable to save the Gemini API key.";

      context.setGeminiApiKeyActionState("error");
      context.setGeminiApiKeyPresenceState(
        context.getGeminiApiKeyPresence() ? "present" : "missing"
      );
      context.setGeminiApiKeyStatusDetail(summarizeGeminiApiKeyFailure(detail, "save"));
    }
  }

  async function removeGeminiApiKey() {
    if (
      context.getGeminiApiKeyActionState() === "checking" ||
      context.getGeminiApiKeyActionState() === "saving" ||
      context.getGeminiApiKeyActionState() === "clearing" ||
      !context.getGeminiApiKeyPresence()
    ) {
      return;
    }

    context.setGeminiApiKeyActionState("clearing");
    context.setGeminiApiKeyStatusDetail("");

    try {
      const cleared = await clearGeminiApiKey();

      context.setGeminiApiKeyDraft("");
      context.setGeminiApiKeyPresence(false);
      context.setGeminiApiKeyPresenceState("missing");
      context.setGeminiApiKeyActionState("idle");
      context.setGeminiApiKeyStatusDetail(
        cleared
          ? "Gemini API key cleared from the OS keychain."
          : "No Gemini API key was stored in the OS keychain."
      );
    } catch (error) {
      const detail = error instanceof Error ? error.message : "Unable to clear the Gemini API key.";

      context.setGeminiApiKeyActionState("error");
      context.setGeminiApiKeyPresenceState(
        context.getGeminiApiKeyPresence() ? "present" : "missing"
      );
      context.setGeminiApiKeyStatusDetail(summarizeGeminiApiKeyFailure(detail, "clear"));
    }
  }

  return {
    clearRecordingShortcutSetting,
    hydrateSettings,
    persistSettings,
    reapplySavedRecordingShortcut,
    refreshGeminiApiKeyPresence,
    refreshRecordingShortcutStatus,
    removeGeminiApiKey,
    saveAndApplyRecordingShortcut,
    submitGeminiApiKey,
    submitRecordingShortcut,
    toggleSetting,
    updateLanguage,
    updateMicrophone
  };
}
