import { beforeEach, describe, expect, it, vi } from "vitest";

import { createDefaultAppSettings } from "$lib/settings/schema";
import type { RecordedAudioMetadata } from "$lib/types/app-shell";
import type { RunCompletedRecordingTranscriptionResult, Transcript } from "$lib/native/transcription";
import type { LatestManualOutcomeSettings } from "./controller";

const recordingMocks = vi.hoisted(() => ({
  cancelRecording: vi.fn(),
  getRecordingStatus: vi.fn(),
  listRecordingInputDevices: vi.fn(),
  startRecording: vi.fn(),
  stopRecording: vi.fn()
}));

const transcriptionMocks = vi.hoisted(() => ({
  hasCompletedRecordingAudio: vi.fn(),
  runCompletedRecordingTranscription: vi.fn()
}));

vi.mock("$lib/native/recording", () => ({
  cancelRecording: recordingMocks.cancelRecording,
  getRecordingStatus: recordingMocks.getRecordingStatus,
  listRecordingInputDevices: recordingMocks.listRecordingInputDevices,
  startRecording: recordingMocks.startRecording,
  stopRecording: recordingMocks.stopRecording
}));

vi.mock("$lib/native/transcription", () => ({
  hasCompletedRecordingAudio: transcriptionMocks.hasCompletedRecordingAudio,
  runCompletedRecordingTranscription: transcriptionMocks.runCompletedRecordingTranscription
}));

import { createRecordingController } from "./controller";

function createControllerHarness() {
  const state = {
    settingsDraft: createDefaultAppSettings(),
    availableRecordingDevices: [] as Array<{ name: string; label: string; isDefault: boolean }>,
    recordingDevicesState: "loading" as "loading" | "ready" | "error",
    recordingDevicesError: "",
    recordingInputOptionsArgs: null as {
      devices: Array<{ name: string; label: string; isDefault: boolean }>;
      selected: string;
    } | null,
    activeRecordingSession: null as { id: string; inputDeviceName: string } | null,
    appStatus: null as ReturnType<typeof import("$lib/stores/app-shell").getAppStatusForPhase> | null,
    idleStatusMessage: null as string | null,
    recordingCommandState: null as "starting" | "stopping" | "cancelling" | null,
    latestRecordingStatus: null as object | null,
    canStartRecording: true,
    canDiscardRecording: false,
    canConfirmRecordingAndTranscribe: false,
    resetTranscriptionRunCalls: [] as Array<{ clearCompletedRecordingMetadata?: boolean } | undefined>,
    latestCompletedRecordingMetadata: null as RecordedAudioMetadata | null,
    latestTranscript: null as Transcript | null,
    latestManualTranscriptionResult: null as RunCompletedRecordingTranscriptionResult | null,
    latestManualOutcomeSettings: null as LatestManualOutcomeSettings | null,
    latestManualTranscriptionFailure: null as string | null,
    activeTranscriptionCommand: null as "manual" | null,
    transcribableRecordedAudio: null as {
      sessionId: string;
      path: string;
      mimeType: string;
      durationMs: number | null;
      inputDeviceName: string;
      sampleRateHz: number;
      channels: number;
      fileSizeBytes: number;
      limitReached: boolean;
      maxDurationMs: number | null;
    } | null,
    loadedHistoryId: null as string | null
  };

  const controller = createRecordingController({
    getSettingsDraft: () => state.settingsDraft,
    setAvailableRecordingDevices: (value) => {
      state.availableRecordingDevices = value;
    },
    setRecordingDevicesState: (value) => {
      state.recordingDevicesState = value;
    },
    setRecordingDevicesError: (value) => {
      state.recordingDevicesError = value;
    },
    setRecordingInputOptions: (devices, selectedMicrophone) => {
      state.recordingInputOptionsArgs = { devices, selected: selectedMicrophone };
    },
    getActiveRecordingSession: () => state.activeRecordingSession,
    setActiveRecordingSession: (value) => {
      state.activeRecordingSession = value;
    },
    getAppStatusPhase: () => state.appStatus?.phase ?? "idle",
    getRecordedAudio: () => state.appStatus?.recordedAudio ?? null,
    setAppStatus: (value) => {
      state.appStatus = value;
    },
    syncIdleStatus: (detail) => {
      state.idleStatusMessage = detail ?? "";
    },
    getSelectedMicrophoneLabel: () => "USB Mic",
    getSelectedMicrophoneUnavailable: () => false,
    getRecordingCommandState: () => state.recordingCommandState,
    setRecordingCommandState: (value) => {
      state.recordingCommandState = value;
    },
    getCanStartRecording: () => state.canStartRecording,
    getCanDiscardRecording: () => state.canDiscardRecording,
    getCanConfirmRecordingAndTranscribe: () => state.canConfirmRecordingAndTranscribe,
    getCanRunManualTranscription: () => state.transcribableRecordedAudio !== null,
    resolveSelectedDeviceName: () => null,
    resolveRecordingLimitMs: () => 900_000,
    setLatestRecordingStatus: (value) => {
      state.latestRecordingStatus = value;
    },
    resetTranscriptionRun: (value) => {
      state.resetTranscriptionRunCalls.push(value);
    },
    setLatestCompletedRecordingMetadata: (value) => {
      state.latestCompletedRecordingMetadata = value;
    },
    setLatestTranscript: (value) => {
      state.latestTranscript = value;
    },
    setLatestManualTranscriptionResult: (value) => {
      state.latestManualTranscriptionResult = value;
    },
    setLatestManualOutcomeSettings: (value) => {
      state.latestManualOutcomeSettings = value;
    },
    setLatestManualTranscriptionFailure: (value) => {
      state.latestManualTranscriptionFailure = value;
    },
    setActiveTranscriptionCommand: (value) => {
      state.activeTranscriptionCommand = value;
    },
    getTranscribableRecordedAudio: () => state.transcribableRecordedAudio,
    getManualOutcomeSettingsSnapshot: () => ({
      autoCopy: state.settingsDraft.autoCopy,
      saveAudioFiles: state.settingsDraft.saveAudioFiles,
      saveTranscriptionHistory: state.settingsDraft.saveTranscriptionHistory
    }),
    loadHistoryEntries: async (preferredSelectionId) => {
      state.loadedHistoryId = preferredSelectionId;
    },
    formatDuration: (durationMs) => (durationMs === null ? "—" : `${durationMs}ms`),
    formatFileName: (path) => path.split("/").pop() ?? path,
    hiddenManualNotificationCompletedMessage: "Completed notification.",
    hiddenManualNotificationFailedMessage: "Failed notification."
  });

  return { controller, state };
}

describe("createRecordingController", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loads recording devices and refreshes idle status when ready", async () => {
    recordingMocks.listRecordingInputDevices.mockResolvedValue([
      { name: "USB Mic", label: "USB Mic", isDefault: true }
    ]);

    const { controller, state } = createControllerHarness();

    await controller.loadRecordingDevices();

    expect(state.availableRecordingDevices).toEqual([
      { name: "USB Mic", label: "USB Mic", isDefault: true }
    ]);
    expect(state.recordingDevicesState).toBe("ready");
    expect(state.recordingInputOptionsArgs).toEqual({
      devices: [{ name: "USB Mic", label: "USB Mic", isDefault: true }],
      selected: "default"
    });
    expect(state.idleStatusMessage).toBe("");
  });

  it("shows an error state when recording cannot start because the microphone is unavailable", async () => {
    const { state } = createControllerHarness();
    const controller = createRecordingController({
      getSettingsDraft: () => state.settingsDraft,
      setAvailableRecordingDevices: () => undefined,
      setRecordingDevicesState: () => undefined,
      setRecordingDevicesError: () => undefined,
      setRecordingInputOptions: () => undefined,
      getActiveRecordingSession: () => state.activeRecordingSession,
      setActiveRecordingSession: () => undefined,
      getAppStatusPhase: () => "idle",
      getRecordedAudio: () => null,
      setAppStatus: (value) => {
        state.appStatus = value;
      },
      syncIdleStatus: () => undefined,
      getSelectedMicrophoneLabel: () => "Unavailable mic",
      getSelectedMicrophoneUnavailable: () => true,
      getRecordingCommandState: () => null,
      setRecordingCommandState: () => undefined,
      getCanStartRecording: () => false,
      getCanDiscardRecording: () => false,
      getCanConfirmRecordingAndTranscribe: () => false,
      getCanRunManualTranscription: () => false,
      resolveSelectedDeviceName: () => null,
      resolveRecordingLimitMs: () => 900_000,
      setLatestRecordingStatus: () => undefined,
      resetTranscriptionRun: () => undefined,
      setLatestCompletedRecordingMetadata: () => undefined,
      setLatestTranscript: () => undefined,
      setLatestManualTranscriptionResult: () => undefined,
      setLatestManualOutcomeSettings: () => undefined,
      setLatestManualTranscriptionFailure: () => undefined,
      setActiveTranscriptionCommand: () => undefined,
      getTranscribableRecordedAudio: () => null,
      getManualOutcomeSettingsSnapshot: () => ({
        autoCopy: true,
        saveAudioFiles: false,
        saveTranscriptionHistory: false
      }),
      loadHistoryEntries: async () => undefined,
      formatDuration: (value) => String(value ?? "—"),
      formatFileName: (path) => path,
      hiddenManualNotificationCompletedMessage: "Completed notification.",
      hiddenManualNotificationFailedMessage: "Failed notification."
    });

    await controller.beginRecording();

    expect(recordingMocks.startRecording).not.toHaveBeenCalled();
    expect(state.appStatus).toMatchObject({
      phase: "error",
      detail: "The saved microphone is unavailable. Choose one of the loaded inputs before starting a recording."
    });
  });

  it("confirms an active recording and transcribes it immediately", async () => {
    recordingMocks.stopRecording.mockResolvedValue({
      sessionId: "session-1",
      audioInput: {
        path: "/tmp/session-1.wav",
        mimeType: "audio/wav",
        durationMs: 42_000
      },
      inputDeviceName: "USB Mic",
      sampleRateHz: 48_000,
      channels: 2,
      fileSizeBytes: 128_000,
      limitReached: false
    });
    transcriptionMocks.hasCompletedRecordingAudio.mockResolvedValue(true);
    transcriptionMocks.runCompletedRecordingTranscription.mockResolvedValue({
      transcript: {
        text: "Transcript ready",
        provider: "Gemini",
        model: "1.5-pro",
        language: "en-US",
        durationMs: 42_000
      },
      historyId: "history-1",
      historySaved: true,
      historyError: null,
      copiedToClipboard: true,
      clipboardError: null,
      audioDeleted: false,
      audioDeleteError: null,
      retainedAudioPath: "/tmp/retained.wav"
    });

    const { controller, state } = createControllerHarness();
    state.activeRecordingSession = {
      id: "session-1",
      inputDeviceName: "USB Mic"
    };
    state.canDiscardRecording = true;
    state.canConfirmRecordingAndTranscribe = true;

    await controller.confirmRecordingAndTranscribe();

    expect(recordingMocks.stopRecording).toHaveBeenCalledTimes(1);
    expect(transcriptionMocks.runCompletedRecordingTranscription).toHaveBeenCalledWith({
      sessionId: "session-1",
      path: "/tmp/session-1.wav",
      mimeType: "audio/wav",
      durationMs: 42_000,
      inputDeviceName: "USB Mic",
      sampleRateHz: 48_000,
      channels: 2,
      fileSizeBytes: 128_000,
      limitReached: false,
      maxDurationMs: 900_000
    });
    expect(state.activeRecordingSession).toBeNull();
    expect(state.latestTranscript?.text).toBe("Transcript ready");
    expect(state.latestCompletedRecordingMetadata?.path).toBe("/tmp/session-1.wav");
    expect(state.appStatus).toMatchObject({
      phase: "completed",
      headline: "Transcript ready."
    });
  });

  it("stops manual transcription early when the completed audio file is gone", async () => {
    transcriptionMocks.hasCompletedRecordingAudio.mockResolvedValue(false);

    const { controller, state } = createControllerHarness();
    state.transcribableRecordedAudio = {
      sessionId: "session-1",
      path: "/tmp/session-1.wav",
      mimeType: "audio/wav",
      durationMs: 42_000,
      inputDeviceName: "USB Mic",
      sampleRateHz: 48_000,
      channels: 2,
      fileSizeBytes: 128_000,
      limitReached: false,
      maxDurationMs: 900_000
    };

    await controller.startManualTranscription();

    expect(state.resetTranscriptionRunCalls).toEqual([undefined]);
    expect(state.latestManualTranscriptionFailure).toBe(
      "The completed recording is no longer available at /tmp/session-1.wav."
    );
    expect(state.appStatus).toMatchObject({
      phase: "error",
      headline: "Manual transcription needs a new recording."
    });
  });

  it("stores the manual transcription result and refreshes history on success", async () => {
    transcriptionMocks.hasCompletedRecordingAudio.mockResolvedValue(true);
    transcriptionMocks.runCompletedRecordingTranscription.mockResolvedValue({
      transcript: {
        text: "Transcript ready",
        provider: "Gemini",
        model: "1.5-pro",
        language: "en-US",
        durationMs: 42_000
      },
      historyId: "history-1",
      historySaved: true,
      historyError: null,
      copiedToClipboard: true,
      clipboardError: null,
      audioDeleted: false,
      audioDeleteError: null,
      retainedAudioPath: "/tmp/retained.wav"
    });

    const { controller, state } = createControllerHarness();
    state.transcribableRecordedAudio = {
      sessionId: "session-1",
      path: "/tmp/session-1.wav",
      mimeType: "audio/wav",
      durationMs: 42_000,
      inputDeviceName: "USB Mic",
      sampleRateHz: 48_000,
      channels: 2,
      fileSizeBytes: 128_000,
      limitReached: false,
      maxDurationMs: 900_000
    };

    await controller.startManualTranscription();

    expect(state.latestTranscript).toEqual({
      text: "Transcript ready",
      provider: "Gemini",
      model: "1.5-pro",
      language: "en-US",
      durationMs: 42_000
    });
    expect(state.latestManualTranscriptionResult).toMatchObject({
      historyId: "history-1",
      historySaved: true
    });
    expect(state.latestManualOutcomeSettings).toEqual({
      autoCopy: true,
      saveAudioFiles: false,
      saveTranscriptionHistory: false
    });
    expect(state.loadedHistoryId).toBe("history-1");
    expect(state.appStatus).toMatchObject({
      phase: "completed",
      headline: "Transcript ready."
    });
    expect(state.activeTranscriptionCommand).toBeNull();
  });
});
