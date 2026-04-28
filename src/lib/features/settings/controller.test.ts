import { beforeEach, describe, expect, it, vi } from "vitest";

import { createDefaultAppSettings } from "$lib/settings/schema";
import type { RecordingShortcutStatus } from "$lib/native/shortcut";

const shortcutMocks = vi.hoisted(() => ({
  applyRecordingShortcut: vi.fn(),
  getRecordingShortcutStatus: vi.fn()
}));

const secretStoreMocks = vi.hoisted(() => ({
  clearGeminiApiKey: vi.fn(),
  hasGeminiApiKey: vi.fn(),
  saveGeminiApiKey: vi.fn()
}));

const settingsMocks = vi.hoisted(() => ({
  loadAppSettings: vi.fn(),
  saveAppSettings: vi.fn()
}));

vi.mock("$lib/native/shortcut", () => ({
  applyRecordingShortcut: shortcutMocks.applyRecordingShortcut,
  getRecordingShortcutStatus: shortcutMocks.getRecordingShortcutStatus
}));

vi.mock("$lib/native/secret-store", () => ({
  clearGeminiApiKey: secretStoreMocks.clearGeminiApiKey,
  hasGeminiApiKey: secretStoreMocks.hasGeminiApiKey,
  saveGeminiApiKey: secretStoreMocks.saveGeminiApiKey
}));

vi.mock("$lib/native/settings", () => ({
  loadAppSettings: settingsMocks.loadAppSettings,
  saveAppSettings: settingsMocks.saveAppSettings
}));

import { createSettingsController } from "./controller";

function createControllerHarness() {
  const state = {
    settingsDraft: createDefaultAppSettings(),
    settingsState: "idle" as "idle" | "loading" | "saving" | "error",
    settingsError: "",
    availableRecordingDevices: [{ name: "USB Mic", isDefault: true }],
    recordingInputOptionsArgs: null as {
      devices: Array<{ name: string; isDefault: boolean }>;
      selectedMicrophone: string;
    } | null,
    geminiApiKeyDraft: "",
    geminiApiKeyPresence: false,
    geminiApiKeyPresenceState: "missing" as "loading" | "present" | "missing" | "error",
    geminiApiKeyActionState: "idle" as "idle" | "checking" | "saving" | "clearing" | "error",
    geminiApiKeyStatusDetail: "",
    recordingShortcutDraft: "",
    recordingShortcutStatus: null as RecordingShortcutStatus | null,
    recordingShortcutActionState: "idle" as
      | "idle"
      | "loading"
      | "applying"
      | "reapplying"
      | "clearing"
      | "error",
    recordingShortcutError: "",
    canClearRecordingShortcut: true,
    idleStatusSyncs: 0
  };

  const controller = createSettingsController({
    getSettingsDraft: () => state.settingsDraft,
    patchSettingsDraft: (value) => {
      state.settingsDraft = { ...state.settingsDraft, ...value };
    },
    setSettingsState: (value) => {
      state.settingsState = value;
    },
    setSettingsError: (value) => {
      state.settingsError = value;
    },
    getAvailableRecordingDevices: () => state.availableRecordingDevices,
    setRecordingInputOptions: (devices, selectedMicrophone) => {
      state.recordingInputOptionsArgs = { devices, selectedMicrophone };
    },
    getActiveRecordingSession: () => false,
    getAppStatusPhase: () => "idle",
    syncIdleStatus: () => {
      state.idleStatusSyncs += 1;
    },
    getGeminiApiKeyDraft: () => state.geminiApiKeyDraft,
    setGeminiApiKeyDraft: (value) => {
      state.geminiApiKeyDraft = value;
    },
    getGeminiApiKeyPresence: () => state.geminiApiKeyPresence,
    setGeminiApiKeyPresence: (value) => {
      state.geminiApiKeyPresence = value;
    },
    setGeminiApiKeyPresenceState: (value) => {
      state.geminiApiKeyPresenceState = value;
    },
    getGeminiApiKeyPresenceState: () => state.geminiApiKeyPresenceState,
    setGeminiApiKeyActionState: (value) => {
      state.geminiApiKeyActionState = value;
    },
    getGeminiApiKeyActionState: () => state.geminiApiKeyActionState,
    setGeminiApiKeyStatusDetail: (value) => {
      state.geminiApiKeyStatusDetail = value;
    },
    getRecordingShortcutDraft: () => state.recordingShortcutDraft,
    setRecordingShortcutDraft: (value) => {
      state.recordingShortcutDraft = value;
    },
    setRecordingShortcutStatus: (value) => {
      state.recordingShortcutStatus = value;
    },
    getRecordingShortcutActionState: () => state.recordingShortcutActionState,
    setRecordingShortcutActionState: (value) => {
      state.recordingShortcutActionState = value;
    },
    setRecordingShortcutError: (value) => {
      state.recordingShortcutError = value;
    },
    canClearRecordingShortcut: () => state.canClearRecordingShortcut
  });

  return { controller, state };
}

describe("createSettingsController", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("hydrates the draft from persisted settings", async () => {
    settingsMocks.loadAppSettings.mockResolvedValue({
      ...createDefaultAppSettings(),
      defaultLanguage: "pt-BR",
      shortcut: "CommandOrControl+Alt+A"
    });

    const { controller, state } = createControllerHarness();

    await controller.hydrateSettings();

    expect(state.settingsDraft.defaultLanguage).toBe("pt-BR");
    expect(state.recordingShortcutDraft).toBe("CommandOrControl+Alt+A");
    expect(state.settingsState).toBe("idle");
  });

  it("persists settings and refreshes input options with the saved microphone", async () => {
    settingsMocks.saveAppSettings.mockImplementation(async (settings) => settings);

    const { controller, state } = createControllerHarness();

    await controller.persistSettings({
      ...state.settingsDraft,
      selectedMicrophone: "USB Mic",
      defaultLanguage: "pt-BR"
    });

    expect(settingsMocks.saveAppSettings).toHaveBeenCalledWith({
      ...createDefaultAppSettings(),
      selectedMicrophone: "USB Mic",
      defaultLanguage: "pt-BR"
    });
    expect(state.recordingInputOptionsArgs).toEqual({
      devices: [{ name: "USB Mic", isDefault: true }],
      selectedMicrophone: "USB Mic"
    });
    expect(state.idleStatusSyncs).toBe(1);
    expect(state.settingsState).toBe("idle");
  });

  it("restores the last saved settings when persistence fails", async () => {
    settingsMocks.loadAppSettings.mockResolvedValue({
      ...createDefaultAppSettings(),
      selectedMicrophone: "USB Mic"
    });

    const { controller, state } = createControllerHarness();
    await controller.hydrateSettings();

    settingsMocks.saveAppSettings.mockRejectedValue(new Error("Save failed"));

    await expect(
      controller.persistSettings(
        {
          ...state.settingsDraft,
          defaultLanguage: "pt-BR"
        },
        { rethrow: true }
      )
    ).rejects.toThrow("Save failed");

    expect(state.settingsDraft.defaultLanguage).toBe("auto");
    expect(state.settingsError).toBe("Save failed");
    expect(state.settingsState).toBe("error");
  });

  it("saves a Gemini API key and updates the local status", async () => {
    secretStoreMocks.saveGeminiApiKey.mockResolvedValue(undefined);

    const { controller, state } = createControllerHarness();
    state.geminiApiKeyDraft = "api-key-123";

    await controller.submitGeminiApiKey();

    expect(secretStoreMocks.saveGeminiApiKey).toHaveBeenCalledWith("api-key-123");
    expect(state.geminiApiKeyDraft).toBe("");
    expect(state.geminiApiKeyPresence).toBe(true);
    expect(state.geminiApiKeyPresenceState).toBe("present");
    expect(state.geminiApiKeyStatusDetail).toBe("Gemini API key saved to the OS keychain.");
  });

  it("saves and applies the recording shortcut", async () => {
    settingsMocks.saveAppSettings.mockImplementation(async (settings) => settings);
    shortcutMocks.applyRecordingShortcut.mockResolvedValue({
      state: "active",
      source: "saved",
      requestedShortcut: "CommandOrControl+Alt+A",
      activeShortcut: "CommandOrControl+Alt+A",
      detail: null
    });

    const { controller, state } = createControllerHarness();
    state.recordingShortcutDraft = " CommandOrControl+Alt+A ";

    await controller.submitRecordingShortcut();

    expect(settingsMocks.saveAppSettings).toHaveBeenCalledWith({
      ...createDefaultAppSettings(),
      shortcut: "CommandOrControl+Alt+A"
    });
    expect(shortcutMocks.applyRecordingShortcut).toHaveBeenCalledWith("CommandOrControl+Alt+A");
    expect(state.recordingShortcutDraft).toBe("CommandOrControl+Alt+A");
    expect(state.recordingShortcutStatus).toMatchObject({
      state: "active",
      activeShortcut: "CommandOrControl+Alt+A"
    });
    expect(state.recordingShortcutActionState).toBe("idle");
  });
});
