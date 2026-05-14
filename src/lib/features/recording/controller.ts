import { createRecordingInputOptions, getAppStatusForPhase } from "$lib/stores/app-shell";
import {
  cancelRecording,
  getRecordingStatus,
  listRecordingInputDevices,
  startRecording,
  stopRecording,
  type ActiveRecordingSession,
  type RecordingInputDevice,
  type RecordingStatus
} from "$lib/native/recording";
import {
  hasCompletedRecordingAudio,
  runCompletedRecordingTranscription,
  type RunCompletedRecordingTranscriptionResult,
  type Transcript
} from "$lib/native/transcription";
import type { RecordedAudioMetadata, SettingsDraft } from "$lib/types/app-shell";

import {
  buildActiveRecordingStatus,
  buildCompletedManualStatus,
  buildCompletedRecordingStatus,
  buildDurationSummaryLabel,
  buildFallbackRecordingStatus,
  buildMissingManualRecordingStatus,
  buildRecordingTimingFromRecordedAudio,
  buildRecordingTimingFromStatus,
  buildRetainedRecordedAudio
} from "./status";

const recordingStatusPollIntervalMs = 1000;
const recordingDevicesPollIntervalMs = 3000;

export interface LatestManualOutcomeSettings {
  autoCopy: boolean;
  saveAudioFiles: boolean;
  saveTranscriptionHistory: boolean;
}

interface RecordingControllerContext {
  getSettingsDraft: () => SettingsDraft;
  getAvailableRecordingDevices: () => RecordingInputDevice[];
  setAvailableRecordingDevices: (devices: RecordingInputDevice[]) => void;
  setRecordingDevicesState: (state: "loading" | "ready" | "error") => void;
  setRecordingDevicesError: (message: string) => void;
  setRecordingInputOptions: (options: RecordingInputDevice[], selectedMicrophone: string) => void;
  getActiveRecordingSession: () => ActiveRecordingSession | null;
  setActiveRecordingSession: (session: ActiveRecordingSession | null) => void;
  getAppStatusPhase: () => string;
  getRecordedAudio: () => RecordedAudioMetadata | null;
  setAppStatus: (status: ReturnType<typeof getAppStatusForPhase>) => void;
  syncIdleStatus: (detail?: string) => void;
  getSelectedMicrophoneLabel: () => string;
  getSelectedMicrophoneUnavailable: () => boolean;
  getRecordingCommandState: () => "starting" | "stopping" | "cancelling" | null;
  setRecordingCommandState: (state: "starting" | "stopping" | "cancelling" | null) => void;
  getCanStartRecording: () => boolean;
  getCanDiscardRecording: () => boolean;
  getCanConfirmRecordingAndTranscribe: () => boolean;
  getCanRunManualTranscription: () => boolean;
  resolveSelectedDeviceName: () => string | null;
  resolveRecordingLimitMs: () => number;
  setLatestRecordingStatus: (status: RecordingStatus | null) => void;
  resetTranscriptionRun: (options?: { clearCompletedRecordingMetadata?: boolean }) => void;
  setLatestCompletedRecordingMetadata: (value: RecordedAudioMetadata | null) => void;
  setLatestTranscript: (value: Transcript | null) => void;
  setLatestManualTranscriptionResult: (
    value: RunCompletedRecordingTranscriptionResult | null
  ) => void;
  setLatestManualOutcomeSettings: (value: LatestManualOutcomeSettings | null) => void;
  setLatestManualTranscriptionFailure: (value: string | null) => void;
  setActiveTranscriptionCommand: (value: "manual" | null) => void;
  getTranscribableRecordedAudio: () => RecordedAudioMetadata | null;
  getManualOutcomeSettingsSnapshot: () => LatestManualOutcomeSettings;
  loadHistoryEntries: (preferredSelectionId: string | null) => Promise<void>;
  formatDuration: (durationMs: number | null) => string;
  formatFileName: (path: string) => string;
  hiddenManualNotificationCompletedMessage: string;
  hiddenManualNotificationFailedMessage: string;
}

export function createRecordingController(context: RecordingControllerContext) {
  let recordingStatusPoller: number | null = null;
  let recordingDevicesPoller: number | null = null;
  let isRefreshingRecordingStatus = false;

  function areRecordingDevicesEqual(
    currentDevices: RecordingInputDevice[],
    nextDevices: RecordingInputDevice[]
  ) {
    return (
      currentDevices.length === nextDevices.length &&
      currentDevices.every(
        (device, index) =>
          device.name === nextDevices[index]?.name &&
          device.label === nextDevices[index]?.label &&
          device.isDefault === nextDevices[index]?.isDefault
      )
    );
  }

  function stopRecordingStatusPolling() {
    if (recordingStatusPoller === null) {
      return;
    }

    window.clearInterval(recordingStatusPoller);
    recordingStatusPoller = null;
  }

  function startRecordingStatusPolling() {
    if (recordingStatusPoller !== null) {
      return;
    }

    recordingStatusPoller = window.setInterval(() => {
      void syncRecorderFromNative(true);
    }, recordingStatusPollIntervalMs);
  }

  function stopRecordingDevicePolling() {
    if (recordingDevicesPoller === null) {
      return;
    }

    window.clearInterval(recordingDevicesPoller);
    recordingDevicesPoller = null;
  }

  function startRecordingDevicePolling() {
    if (recordingDevicesPoller !== null) {
      return;
    }

    recordingDevicesPoller = window.setInterval(() => {
      if (
        context.getActiveRecordingSession() !== null ||
        context.getRecordingCommandState() !== null
      ) {
        return;
      }

      void refreshRecordingDevices({ showLoading: false, suppressErrors: true });
    }, recordingDevicesPollIntervalMs);
  }

  async function refreshRecordingDevices(options?: {
    showLoading?: boolean;
    suppressErrors?: boolean;
  }) {
    const showLoading = options?.showLoading ?? true;
    const suppressErrors = options?.suppressErrors ?? false;

    if (showLoading) {
      context.setRecordingDevicesState("loading");
    }

    context.setRecordingDevicesError("");

    try {
      const devices = await listRecordingInputDevices();
      const currentDevices = context.getAvailableRecordingDevices();

      if (showLoading || !areRecordingDevicesEqual(currentDevices, devices)) {
        context.setAvailableRecordingDevices(devices);
        context.setRecordingInputOptions(devices, context.getSettingsDraft().selectedMicrophone);
      }

      context.setRecordingDevicesState("ready");
    } catch (error) {
      if (suppressErrors) {
        return;
      }

      context.setAvailableRecordingDevices([]);
      context.setRecordingDevicesError(
        error instanceof Error ? error.message : "Unable to load recording input devices"
      );
      context.setRecordingInputOptions([], context.getSettingsDraft().selectedMicrophone);
      context.setRecordingDevicesState("error");
    }

    if (context.getActiveRecordingSession() === null && context.getAppStatusPhase() === "idle") {
      context.syncIdleStatus();
    }
  }

  async function loadRecordingDevices() {
    await refreshRecordingDevices();
    startRecordingDevicePolling();
  }

  async function syncRecorderFromNative(suppressErrors = false) {
    if (isRefreshingRecordingStatus) {
      return;
    }

    isRefreshingRecordingStatus = true;

    try {
      const status = await getRecordingStatus();
      context.setLatestRecordingStatus(status);
      await applyNativeRecordingStatus(status);
    } catch (error) {
      if (
        !suppressErrors &&
        context.getActiveRecordingSession() !== null &&
        context.getRecordingCommandState() === null
      ) {
        stopRecordingStatusPolling();
        context.setActiveRecordingSession(null);
        context.setAppStatus(
          getAppStatusForPhase("error", {
            detail:
              error instanceof Error
                ? error.message
                : "Unable to refresh the native recording status.",
            transcriptPreview:
              "SpeakEx could not refresh the current recording status. No transcription ran automatically.",
            inputLabel: context.getSelectedMicrophoneLabel(),
            recordingTiming: null,
            recordedAudio: context.getRecordedAudio()
          })
        );
      }
    } finally {
      isRefreshingRecordingStatus = false;
    }
  }

  async function applyNativeRecordingStatus(status: RecordingStatus) {
    const isActivePhase =
      status.phase === "starting" ||
      status.phase === "recording" ||
      status.phase === "stopping" ||
      status.phase === "cancelling";

    if (isActivePhase && status.activeSessionId && status.inputDeviceName) {
      context.setActiveRecordingSession({
        id: status.activeSessionId,
        inputDeviceName: status.inputDeviceName
      });
      context.setAppStatus(
        buildActiveRecordingStatus({
          status,
          selectedMicrophoneLabel: context.getSelectedMicrophoneLabel(),
          formatDuration: context.formatDuration
        })
      );
      startRecordingStatusPolling();
      return;
    }

    stopRecordingStatusPolling();

    const activeRecordingSession = context.getActiveRecordingSession();

    if (
      activeRecordingSession !== null &&
      status.lastCompletedSessionId !== null &&
      status.lastCompletedSessionId === activeRecordingSession.id &&
      context.getRecordingCommandState() === null
    ) {
      await finalizeCompletedRecording(status);
      return;
    }

    if (
      status.phase === "idle" &&
      context.getActiveRecordingSession() === null &&
      context.getAppStatusPhase() === "idle"
    ) {
      context.syncIdleStatus();
    }
  }

  async function finalizeCompletedRecording(status: RecordingStatus) {
    const recordingTiming = buildRecordingTimingFromStatus(status);

    context.setRecordingCommandState("stopping");
    context.setAppStatus(
      getAppStatusForPhase("recording", {
        headline: status.limitReached ? "Maximum duration reached." : "Finalizing recorded audio.",
        detail: status.limitReached
          ? `Capture stopped automatically after reaching the ${context.formatDuration(status.maxDurationMs)} limit. Finalizing the local recording now.`
          : "Recording has stopped. Finalizing the local recording now.",
        transcriptPreview: status.limitReached
          ? "The app is preparing the completed recording after the automatic safety stop. No transcription will run automatically."
          : "The app is preparing the completed recording metadata. No transcription will run automatically.",
        inputLabel: status.inputDeviceName ?? context.getSelectedMicrophoneLabel(),
        durationLabel: buildDurationSummaryLabel(recordingTiming, {
          formatDuration: context.formatDuration
        }),
        recordingTiming,
        recordedAudio: null
      })
    );

    try {
      const stoppedRecording = await stopRecording();
      context.setActiveRecordingSession(null);
      context.resetTranscriptionRun();
      const completedStatus = buildCompletedRecordingStatus({
        stoppedRecording,
        maxDurationMs: context.resolveRecordingLimitMs(),
        formatDuration: context.formatDuration,
        formatFileName: context.formatFileName
      });
      context.setLatestCompletedRecordingMetadata(completedStatus.recordedAudio);
      context.setAppStatus(completedStatus);
    } catch (error) {
      context.setActiveRecordingSession(null);
      context.setAppStatus(
        getAppStatusForPhase("error", {
          detail:
            error instanceof Error
              ? error.message
              : "Unable to finalize the completed recording.",
          transcriptPreview: status.limitReached
            ? "The recorder hit the hard time limit, but the UI could not finish loading the completed recording metadata. No transcription ran automatically."
            : "The UI could not finish loading the completed recording metadata. No transcription ran automatically.",
          inputLabel: status.inputDeviceName ?? context.getSelectedMicrophoneLabel(),
          durationLabel: buildDurationSummaryLabel(recordingTiming, {
            formatDuration: context.formatDuration
          }),
          recordingTiming,
          recordedAudio: null
        })
      );
    } finally {
      context.setRecordingCommandState(null);
    }
  }

  async function beginRecording() {
    if (!context.getCanStartRecording()) {
      if (context.getSelectedMicrophoneUnavailable()) {
        context.setAppStatus(
          getAppStatusForPhase("error", {
            detail:
              "The saved microphone is unavailable. Choose one of the loaded inputs before starting a recording.",
            transcriptPreview:
              "The recorder was not started because the current microphone selection does not match any available device.",
            inputLabel: context.getSelectedMicrophoneLabel(),
            recordedAudio: null
          })
        );
      }

      return;
    }

    context.setRecordingCommandState("starting");
    context.resetTranscriptionRun({ clearCompletedRecordingMetadata: true });

    try {
      const session = await startRecording(context.resolveSelectedDeviceName());
      context.setActiveRecordingSession(session);
      context.setAppStatus(
        buildActiveRecordingStatus({
          status: buildFallbackRecordingStatus(session, context.resolveRecordingLimitMs()),
          selectedMicrophoneLabel: context.getSelectedMicrophoneLabel(),
          formatDuration: context.formatDuration
        })
      );
      await syncRecorderFromNative(true);
      startRecordingStatusPolling();
    } catch (error) {
      context.setAppStatus(
        getAppStatusForPhase("error", {
          detail: error instanceof Error ? error.message : "Unable to start the recorder.",
          transcriptPreview:
            "The native start_recording command did not succeed. Check the selected microphone and try again.",
          inputLabel: context.getSelectedMicrophoneLabel(),
          recordedAudio: null
        })
      );
    } finally {
      context.setRecordingCommandState(null);
    }
  }

  async function discardRecording() {
    if (!context.getCanDiscardRecording()) {
      return;
    }

    context.setRecordingCommandState("cancelling");
    stopRecordingStatusPolling();

    try {
      let cancelledSessionId = activeRecordingSession?.id ?? "unknown";
      let deletedAudioPath: string | null = null;

      if (activeRecordingSession !== null) {
        const cancelled = await cancelRecording();
        cancelledSessionId = cancelled.sessionId;
        deletedAudioPath = cancelled.deletedAudioPath;
      }

      context.setLatestRecordingStatus(null);
      context.setActiveRecordingSession(null);
      context.resetTranscriptionRun({ clearCompletedRecordingMetadata: true });
      context.syncIdleStatus(
        deletedAudioPath
          ? `Recording ${cancelledSessionId} was cancelled and the temporary file was deleted from the app cache.`
          : `Recording ${cancelledSessionId} was discarded.`
      );
    } catch (error) {
      context.setActiveRecordingSession(null);
      context.setAppStatus(
        getAppStatusForPhase("error", {
          detail: error instanceof Error ? error.message : "Unable to cancel the recorder.",
          transcriptPreview:
            "The native cancel_recording command did not finish successfully. Retry only after confirming the recorder returned to idle.",
          inputLabel: context.getSelectedMicrophoneLabel(),
          recordedAudio: null
        })
      );
    } finally {
      context.setRecordingCommandState(null);
    }
  }

  async function runTranscriptionForRecordedAudio(recordedAudio: RecordedAudioMetadata) {
    if (!(await hasCompletedRecordingAudio(recordedAudio))) {
      context.resetTranscriptionRun();
      const detail = `The completed recording is no longer available at ${recordedAudio.path}.`;
      context.setLatestManualTranscriptionFailure(detail);
      context.setLatestCompletedRecordingMetadata(recordedAudio);
      context.setAppStatus(
        buildMissingManualRecordingStatus({
          recordedAudio,
          detail,
          transcriptPreview: `The recorded audio file is gone, so transcription cannot start and Retry transcription stays hidden. Record again to create a fresh file before transcribing. ${context.hiddenManualNotificationFailedMessage}`,
          formatDuration: context.formatDuration
        })
      );
      return;
    }

    const manualOutcomeSettings = context.getManualOutcomeSettingsSnapshot();

    context.setActiveTranscriptionCommand("manual");
    context.resetTranscriptionRun();

    context.setAppStatus(
      getAppStatusForPhase("transcribing", {
        headline: "Transcription is running.",
        inputLabel: recordedAudio.inputDeviceName,
        detail:
          "SpeakEx is transcribing the current recorded audio with Gemini. Clipboard, history, default audio cleanup, retryable failure state, and hidden-window notifications follow your saved settings.",
        transcriptTitle: "Transcript incoming…",
        transcriptPreview: `${context.formatFileName(recordedAudio.path)} is being transcribed with Gemini.`,
        durationLabel: context.formatDuration(recordedAudio.durationMs),
        recordingTiming: buildRecordingTimingFromRecordedAudio(recordedAudio),
        recordedAudio
      })
    );

    try {
      const result = await runCompletedRecordingTranscription(recordedAudio);
      const retainedRecordedAudio = buildRetainedRecordedAudio(recordedAudio, result);

      context.setLatestTranscript(result.transcript);
      context.setLatestManualTranscriptionResult(result);
      context.setLatestManualOutcomeSettings(manualOutcomeSettings);
      context.setLatestCompletedRecordingMetadata(recordedAudio);
      context.setAppStatus(
        buildCompletedManualStatus({
          result,
          recordedAudio,
          retainedRecordedAudio,
          settings: manualOutcomeSettings,
          hiddenManualNotificationCompletedMessage: context.hiddenManualNotificationCompletedMessage,
          formatDuration: context.formatDuration
        })
      );

      if (result.historySaved) {
        await context.loadHistoryEntries(result.historyId);
      }
    } catch (error) {
      const manualFailureDetail =
        error instanceof Error ? error.message : "Unable to finish the transcription flow.";
      const retryableRecordedAudio = (await hasCompletedRecordingAudio(recordedAudio))
        ? recordedAudio
        : null;

      context.setLatestTranscript(null);
      context.setLatestManualOutcomeSettings(null);
      context.setLatestManualTranscriptionFailure(manualFailureDetail);
      context.setLatestCompletedRecordingMetadata(recordedAudio);
      context.setAppStatus(
        getAppStatusForPhase("error", {
          inputLabel: recordedAudio.inputDeviceName,
          detail: manualFailureDetail,
          transcriptPreview:
            retryableRecordedAudio === null
              ? `Transcription did not finish, and the recorded audio file is no longer available for recovery. Retry transcription is hidden until you record again. ${context.hiddenManualNotificationFailedMessage}`
              : `Transcription did not finish, but the recorded audio file remains local so you can use Retry transcription to try the same recording again. ${context.hiddenManualNotificationFailedMessage}`,
          durationLabel: context.formatDuration(recordedAudio.durationMs),
          recordingTiming: buildRecordingTimingFromRecordedAudio(recordedAudio),
          recordedAudio: retryableRecordedAudio
        })
      );
    } finally {
      context.setActiveTranscriptionCommand(null);
    }
  }

  async function confirmRecordingAndTranscribe() {
    if (!context.getCanConfirmRecordingAndTranscribe()) {
      return;
    }

    context.setRecordingCommandState("stopping");
    stopRecordingStatusPolling();

    try {
      const stoppedRecording = await stopRecording();
      const completedStatus = buildCompletedRecordingStatus({
        stoppedRecording,
        maxDurationMs: context.resolveRecordingLimitMs(),
        formatDuration: context.formatDuration,
        formatFileName: context.formatFileName
      });
      const recordedAudio = completedStatus.recordedAudio;

      context.setActiveRecordingSession(null);
      context.setLatestCompletedRecordingMetadata(recordedAudio);
      context.setAppStatus(completedStatus);
      context.setRecordingCommandState(null);

      if (recordedAudio !== null) {
        await runTranscriptionForRecordedAudio(recordedAudio);
      }
    } catch (error) {
      context.setActiveRecordingSession(null);
      context.setAppStatus(
        getAppStatusForPhase("error", {
          detail:
            error instanceof Error
              ? error.message
              : "Unable to finish the recorder before transcription.",
          transcriptPreview:
            "SpeakEx could not finish the recording before starting transcription. Retry after confirming the recorder returned to idle.",
          inputLabel: context.getSelectedMicrophoneLabel(),
          recordedAudio: null
        })
      );
    } finally {
      context.setRecordingCommandState(null);
    }
  }

  async function startManualTranscription() {
    const transcribableRecordedAudio = context.getTranscribableRecordedAudio();

    if (!context.getCanRunManualTranscription() || transcribableRecordedAudio === null) {
      return;
    }

    await runTranscriptionForRecordedAudio(transcribableRecordedAudio);
  }

  return {
    beginRecording,
    confirmRecordingAndTranscribe,
    discardRecording,
    loadRecordingDevices,
    startManualTranscription,
    stopRecordingDevicePolling,
    stopRecordingStatusPolling,
    syncRecorderFromNative
  };
}
