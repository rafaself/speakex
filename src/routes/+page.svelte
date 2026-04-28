<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";

  import {
    clearHistory,
    deleteTranscription,
    getHistory,
    getTranscription,
    type HistoryTranscription,
    type HistoryTranscriptionSummary
  } from "$lib/native/history";
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
  import { ping } from "$lib/native/ping";
  import {
    cancelRecording,
    getRecordingStatus,
    listRecordingInputDevices,
    startRecording,
    stopRecording,
    type ActiveRecordingSession,
    type RecordingInputDevice,
    type RecordingStatus,
    type StoppedRecording
  } from "$lib/native/recording";
  import {
    hasCompletedRecordingAudio,
    runCompletedRecordingTranscription,
    runMockTranscription,
    type RunCompletedRecordingTranscriptionResult,
    type RunMockTranscriptionResult,
    type Transcript
  } from "$lib/native/transcription";
  import {
    activeSection,
    appStatus,
    createRecordingInputOptions,
    defaultRecordingInputOption,
    getAppStatusForPhase,
    languageOptions,
    navigationSections,
    providerOptions,
    providerSelection,
    recordingInputOptions,
    recordingPlanSteps,
    settingsDraft,
    type DraftToggleKey
  } from "$lib/stores/app-shell";
  import type { ProviderId } from "$lib/settings/schema";
  import type {
    GeminiApiKeyActionState,
    GeminiApiKeyPresenceState,
    RecordedAudioMetadata,
    RecordingTiming,
    SettingsDraft
  } from "$lib/types/app-shell";

  type PingState = "idle" | "loading" | "success" | "error";
  type SettingsState = "idle" | "loading" | "saving" | "error";
  type HistoryState = "loading" | "ready" | "error";
  type HistoryDetailState = "idle" | "loading" | "ready" | "error";
  type RecordingDevicesState = "loading" | "ready" | "error";
  type RecordingCommandState = "starting" | "stopping" | "cancelling" | null;
  type ShortcutActionState = "idle" | "loading" | "applying" | "reapplying" | "clearing" | "error";
  type TranscriptionCommandState = "mock" | "manual" | null;

  type HistoryEntryStatus = "saved" | "attention";

  interface HistoryEntryViewModel {
    id: string;
    title: string;
    excerpt: string;
    providerLabel: string;
    createdAtLabel: string;
    durationLabel: string;
    languageLabel: string;
    clipboardLabel: string;
    audioLabel: string;
    status: HistoryEntryStatus;
    statusLabel: string;
  }

  interface LatestManualOutcomeSettings {
    autoCopy: boolean;
    saveAudioFiles: boolean;
    saveTranscriptionHistory: boolean;
  }

  const providerLabels = new Map(providerOptions.map((provider) => [provider.id, provider.label]));
  const fallbackRecordingLimitMs = 15 * 60 * 1000;
  const recordingStatusPollIntervalMs = 1000;
  const hiddenManualNotificationReadyMessage =
    "If SpeakEx is hidden when a manual transcription finishes, the app can also send a desktop notification. Hidden failure notifications stay generic and point you back to SpeakEx for details.";
  const hiddenManualNotificationPendingMessage =
    "If SpeakEx is hidden before the manual run finishes, the app can also send a desktop notification. Completion can confirm success, while failure stays generic and points you back to SpeakEx for details.";
  const hiddenManualNotificationCompletedMessage =
    "If SpeakEx was hidden when this manual transcription finished, you may also have seen a desktop notification.";
  const hiddenManualNotificationFailedMessage =
    "If SpeakEx was hidden when this manual transcription failed, you may also have seen a generic desktop notification. The detailed error stays in SpeakEx.";

  let pingState: PingState = "idle";
  let pingResponse = "";
  let pingError = "";
  let settingsState: SettingsState = "loading";
  let settingsError = "";
  let lastSavedSettings: SettingsDraft | null = null;
  let settingsSaveQueue = Promise.resolve();
  let latestSettingsRequest = 0;
  let geminiApiKeyDraft = "";
  let geminiApiKeyPresence = false;
  let geminiApiKeyPresenceState: GeminiApiKeyPresenceState = "loading";
  let geminiApiKeyActionState: GeminiApiKeyActionState = "checking";
  let geminiApiKeyStatusDetail = "";
  let recordingShortcutDraft = "";
  let recordingShortcutStatus: RecordingShortcutStatus | null = null;
  let recordingShortcutActionState: ShortcutActionState = "loading";
  let recordingShortcutError = "";
  let historyState: HistoryState = "loading";
  let historyEntries: HistoryEntryViewModel[] = [];
  let historyError = "";
  let historyDetailState: HistoryDetailState = "idle";
  let historyDetailError = "";
  let historyBusyEntryId: string | null = null;
  let isClearingHistory = false;
  let selectedHistoryEntryId: string | null = null;
  let selectedHistoryEntry: HistoryTranscription | null = null;
  let latestHistoryDetailRequest = 0;
  let activeTranscriptionCommand: TranscriptionCommandState = null;
  let latestMockTranscriptionResult: RunMockTranscriptionResult | null = null;
  let latestTranscript: Transcript | null = null;
  let latestManualTranscriptionResult: RunCompletedRecordingTranscriptionResult | null = null;
  let latestManualOutcomeSettings: LatestManualOutcomeSettings | null = null;
  let latestManualTranscriptionFailure: string | null = null;
  let latestCompletedRecordingMetadata: RecordedAudioMetadata | null = null;
  let recordingDevicesState: RecordingDevicesState = "loading";
  let recordingDevicesError = "";
  let availableRecordingDevices: RecordingInputDevice[] = [];
  let recordingCommandState: RecordingCommandState = null;
  let activeRecordingSession: ActiveRecordingSession | null = null;
  let recordingStatusPoller: ReturnType<typeof window.setInterval> | null = null;
  let isRefreshingRecordingStatus = false;
  let latestRecordingStatus: RecordingStatus | null = null;

  $: selectedProviderLabel = providerLabels.get($providerSelection) ?? "Unknown provider";
  $: selectedMicrophoneOption =
    $recordingInputOptions.find((option) => option.value === $settingsDraft.selectedMicrophone) ??
    defaultRecordingInputOption;
  $: selectedMicrophoneLabel = selectedMicrophoneOption.label;
  $: selectedMicrophoneUnavailable = selectedMicrophoneOption.unavailable ?? false;
  $: isRunningMockTranscription = activeTranscriptionCommand === "mock";
  $: isRunningManualTranscription = activeTranscriptionCommand === "manual";
  $: isRunningTranscription = activeTranscriptionCommand !== null;
  $: primaryMockActionLabel = isRunningMockTranscription ? "Transcribing…" : "Run mock transcription";
  $: hasRecoverableManualFailure =
    latestManualTranscriptionFailure !== null && transcribableRecordedAudio !== null;
  $: hasUnrecoverableManualFailure =
    latestManualTranscriptionFailure !== null &&
    transcribableRecordedAudio === null &&
    latestCompletedRecordingMetadata !== null;
  $: showManualTranscriptionAction = !hasUnrecoverableManualFailure;
  $: primaryManualActionLabel = isRunningManualTranscription
    ? "Transcribing…"
    : hasRecoverableManualFailure
      ? "Retry transcription"
      : "Transcribe recording";
  $: recordingDevicesStatusMessage =
    recordingDevicesState === "loading"
      ? "Loading available microphones…"
      : recordingDevicesState === "error"
        ? recordingDevicesError
        : availableRecordingDevices.length === 0
          ? "No microphones are available right now."
          : `${availableRecordingDevices.length} microphone${availableRecordingDevices.length === 1 ? "" : "s"} ready to use.`;
  $: mockHistoryModeLabel = $settingsDraft.saveTranscriptionHistory
    ? "Mock transcripts will also be saved to local history."
    : "Mock transcripts will stay out of local history because Save transcription history is off.";
  $: settingsStatusMessage =
    settingsState === "loading"
      ? "Loading saved preferences…"
      : settingsState === "saving"
        ? "Saving changes locally…"
        : settingsState === "error"
          ? settingsError
          : "Preferences are stored locally and secrets stay in the OS keychain.";
  $: geminiApiKeyDraftValue = geminiApiKeyDraft.trim();
  $: isGeminiApiKeyBusy =
    geminiApiKeyActionState === "checking" ||
    geminiApiKeyActionState === "saving" ||
    geminiApiKeyActionState === "clearing";
  $: geminiApiKeyPrimaryActionLabel =
    geminiApiKeyActionState === "saving"
      ? geminiApiKeyPresence
        ? "Replacing…"
        : "Saving…"
      : geminiApiKeyPresence
        ? "Replace saved key"
        : "Save key";
  $: geminiApiKeyStatusMessage =
    geminiApiKeyActionState === "checking"
      ? "Checking the OS keychain for a saved Gemini API key…"
      : geminiApiKeyActionState === "saving"
        ? "Saving the Gemini API key to the OS keychain…"
        : geminiApiKeyActionState === "clearing"
          ? "Clearing the Gemini API key from the OS keychain…"
          : geminiApiKeyPresenceState === "error"
            ? geminiApiKeyStatusDetail
            : geminiApiKeyStatusDetail !== ""
              ? geminiApiKeyStatusDetail
              : geminiApiKeyPresenceState === "loading"
                ? "Checking the OS keychain for a saved Gemini API key…"
                : geminiApiKeyPresence
                  ? "Gemini API key is saved in the OS keychain."
                  : "No Gemini API key is saved in the OS keychain.";
  $: geminiApiKeySavedLabel =
    geminiApiKeyPresenceState === "loading"
      ? "Checking…"
      : geminiApiKeyPresenceState === "error"
        ? "Status unavailable"
        : geminiApiKeyPresence
          ? "Saved in OS keychain"
          : "Not saved";
  $: savedRecordingShortcut = $settingsDraft.shortcut;
  $: savedRecordingShortcutLabel = formatShortcutValue(
    savedRecordingShortcut,
    "No saved shortcut override"
  );
  $: currentRecordingShortcutValueLabel = formatShortcutValue(
    recordingShortcutStatus?.activeShortcut ?? recordingShortcutStatus?.requestedShortcut,
    "No runtime shortcut"
  );
  $: recordingShortcutStateLabel = describeRecordingShortcutState(recordingShortcutStatus);
  $: recordingShortcutSourceLabel = describeRecordingShortcutSource(recordingShortcutStatus);
  $: isRecordingShortcutBusy =
    recordingShortcutActionState === "loading" ||
    recordingShortcutActionState === "applying" ||
    recordingShortcutActionState === "reapplying" ||
    recordingShortcutActionState === "clearing";
  $: canClearRecordingShortcut =
    !isRecordingShortcutBusy &&
    (savedRecordingShortcut !== null ||
      recordingShortcutDraft.trim().length > 0 ||
      (recordingShortcutStatus?.activeShortcut ?? recordingShortcutStatus?.requestedShortcut) !==
        null);
  $: recordingShortcutPrimaryActionLabel =
    recordingShortcutActionState === "applying" ? "Saving and applying…" : "Save and apply";
  $: recordingShortcutReapplyLabel =
    recordingShortcutActionState === "reapplying" ? "Re-applying…" : "Re-apply saved shortcut";
  $: recordingShortcutClearLabel =
    recordingShortcutActionState === "clearing" ? "Clearing…" : "Clear saved shortcut";
  $: recordingShortcutRefreshLabel =
    recordingShortcutActionState === "loading" ? "Checking…" : "Refresh status";
  $: recordingShortcutStatusMessage = buildRecordingShortcutStatusMessage(
    recordingShortcutActionState,
    recordingShortcutStatus,
    savedRecordingShortcut,
    recordingShortcutError
  );
  $: historyStatusMessage =
    historyState === "loading"
      ? "Loading transcript history from the local database…"
      : historyState === "error" || historyError !== ""
        ? historyError
        : historyEntries.length === 0
          ? "No saved transcripts yet."
          : `${historyEntries.length} saved transcript${historyEntries.length === 1 ? " is" : "s are"} available locally. Select one to review the full details.`;
  $: historyCountLabel =
    historyState === "loading"
      ? "Loading…"
      : historyState === "error"
        ? "Unavailable"
        : `${historyEntries.length} saved item${historyEntries.length === 1 ? "" : "s"}`;
  $: selectedHistorySummary =
    selectedHistoryEntryId === null
      ? null
      : historyEntries.find((entry) => entry.id === selectedHistoryEntryId) ?? null;
  $: selectedHistoryStoredIssue = selectedHistoryEntry?.error?.trim() ?? "";
  $: historyDetailTitle =
    selectedHistorySummary?.title ??
    (selectedHistoryEntry
      ? createHistoryTitle(selectedHistoryEntry.text)
      : "Select a saved transcript");
  $: historyDetailStatusLabel =
    historyDetailState === "loading"
      ? "Loading"
      : historyDetailState === "error"
        ? "Unavailable"
        : selectedHistoryStoredIssue !== ""
          ? "Saved with warnings"
          : selectedHistoryEntry
            ? "Saved"
            : "Select one";
  $: selectedHistoryClipboardLabel = selectedHistoryEntry
    ? describeStoredClipboardOutcome(
        selectedHistoryEntry.copiedToClipboard,
        selectedHistoryStoredIssue
      )
    : "—";
  $: selectedHistoryAudioStatusLabel = selectedHistoryEntry
    ? describeStoredAudioOutcome(selectedHistoryEntry.audioDeleted, selectedHistoryEntry.audioPath)
    : "—";
  $: selectedHistoryAudioPathLabel = selectedHistoryEntry
    ? selectedHistoryEntry.audioPath ?? "No retained audio path."
    : "—";
  $: transcribableRecordedAudio = $appStatus.recordedAudio;
  $: displayedRecordedAudio = transcribableRecordedAudio ?? latestCompletedRecordingMetadata;
  $: currentRecordingTiming = $appStatus.recordingTiming;
  $: elapsedTimeLabel = formatDuration(
    currentRecordingTiming?.elapsedMs ?? displayedRecordedAudio?.durationMs ?? null
  );
  $: remainingTimeLabel = formatDuration(currentRecordingTiming?.remainingMs ?? null);
  $: maxDurationLabel = formatDuration(resolveRecordingLimitMs());
  $: recordingLimitLabel = `Automatic stop at ${maxDurationLabel}`;
  $: stopReasonLabel =
    displayedRecordedAudio?.limitReached || currentRecordingTiming?.limitReached
      ? `Stopped automatically at the ${maxDurationLabel} limit`
      : displayedRecordedAudio
        ? "Stopped manually and kept locally"
        : "No completed recording yet";
  $: statusPollingLabel = activeRecordingSession === null ? "Inactive" : "Polling every second";
  $: manualTranscriptionWarnings = latestManualTranscriptionResult
    ? collectOutcomeWarnings(latestManualTranscriptionResult)
    : [];
  $: manualHistoryLabel = latestManualTranscriptionResult
    ? describeManualHistoryOutcome(
        latestManualTranscriptionResult,
        latestManualOutcomeSettings?.saveTranscriptionHistory ??
          $settingsDraft.saveTranscriptionHistory
      )
    : null;
  $: manualClipboardLabel = latestManualTranscriptionResult
    ? describeManualClipboardOutcome(
        latestManualTranscriptionResult,
        latestManualOutcomeSettings?.autoCopy ?? $settingsDraft.autoCopy
      )
    : null;
  $: manualAudioLabel = latestManualTranscriptionResult
    ? describeManualAudioOutcome(
        latestManualTranscriptionResult,
        latestManualOutcomeSettings?.saveAudioFiles ?? $settingsDraft.saveAudioFiles
      )
    : null;
  $: latestTranscriptHistoryLabel =
    latestManualTranscriptionResult !== null
      ? manualHistoryLabel
      : latestMockTranscriptionResult !== null
        ? describeMockHistoryOutcome(latestMockTranscriptionResult.savedToHistory)
        : null;
  $: manualTranscriptionStatusMessage =
    latestManualTranscriptionResult !== null
      ? manualTranscriptionWarnings.length === 0
        ? `Transcription finished. Review the clipboard, history, and audio results below. ${hiddenManualNotificationCompletedMessage}`
        : `Transcription finished with warnings. Review the clipboard, history, and audio results below. ${hiddenManualNotificationCompletedMessage}`
      : latestManualTranscriptionFailure !== null
        ? hasRecoverableManualFailure
          ? geminiApiKeyPresenceState === "loading" || geminiApiKeyActionState === "checking"
            ? "Transcription failed, but the recorded audio file is still available. Checking Settings before enabling Retry transcription…"
            : geminiApiKeyPresenceState === "error"
              ? "Transcription failed. The recorded audio file is still available, but SpeakEx could not verify the Gemini API key. Check Settings before using Retry transcription."
              : !geminiApiKeyPresence
                ? "Transcription failed. The recorded audio file is still available, but Retry transcription stays unavailable until you save a Gemini API key in Settings."
                : `Transcription failed. The recorded audio file is still available, so use Retry transcription to try the same recording again. ${hiddenManualNotificationFailedMessage}`
          : `Transcription failed. Retry transcription is hidden because the recorded audio file is no longer available. Record again to create a new file before trying again. ${hiddenManualNotificationFailedMessage}`
      : transcribableRecordedAudio === null
        ? "Complete a local recording first, then run transcription manually from this screen."
      : geminiApiKeyPresenceState === "loading" || geminiApiKeyActionState === "checking"
          ? "Checking the OS keychain before enabling Gemini transcription…"
          : geminiApiKeyPresenceState === "error"
            ? "Unable to verify the Gemini API key right now. Recheck key status in Settings before running Gemini."
            : !geminiApiKeyPresence
              ? "Save a Gemini API key in Settings before running Gemini on the current recording."
              : isRunningManualTranscription
                ? `Gemini is transcribing the current recording. Clipboard, history, and audio cleanup follow your saved settings. ${hiddenManualNotificationPendingMessage}`
                : `Gemini is ready to transcribe the current recording on demand. ${hiddenManualNotificationReadyMessage}`;
  $: manualRecoveryGuidanceMessage =
    latestManualTranscriptionFailure === null
      ? null
      : hasRecoverableManualFailure
        ? geminiApiKeyPresenceState === "loading" || geminiApiKeyActionState === "checking"
          ? "Retry will reuse the recorded audio shown below as soon as SpeakEx finishes checking the Gemini key status."
          : geminiApiKeyPresenceState === "error"
            ? "The recorded audio file is still available, but Retry transcription stays blocked until Gemini key status can be checked again in Settings."
            : !geminiApiKeyPresence
              ? "The recorded audio file is still available. Save a Gemini API key in Settings, then use Retry transcription to try the same recording again."
              : "Retry transcription will reuse the recorded audio shown below and run Gemini again with the same file."
        : latestCompletedRecordingMetadata
          ? `Retry transcription is hidden because SpeakEx can no longer find the recorded audio file at ${latestCompletedRecordingMetadata.path}. Record again to create a fresh file before transcribing.`
          : "Retry transcription is hidden because the recorded audio file is no longer available. Record again to create a fresh file before transcribing.";
  $: canStartRecording =
    recordingDevicesState === "ready" &&
    recordingCommandState === null &&
    activeRecordingSession === null &&
    !isRunningTranscription &&
    !selectedMicrophoneUnavailable;
  $: canStopRecording =
    activeRecordingSession !== null &&
    recordingCommandState === null &&
    !isRunningTranscription;
  $: canCancelRecording =
    activeRecordingSession !== null &&
    recordingCommandState === null &&
    !isRunningTranscription;
  $: canRunMockTranscription =
    activeRecordingSession === null &&
    recordingCommandState === null &&
    !isRunningTranscription;
  $: canRunManualTranscription =
    transcribableRecordedAudio !== null &&
    activeRecordingSession === null &&
    recordingCommandState === null &&
    !isRunningTranscription &&
    !isGeminiApiKeyBusy &&
    geminiApiKeyPresenceState !== "error" &&
    geminiApiKeyPresence;
  $: resetActionLabel = displayedRecordedAudio || latestTranscript ? "Clear preview" : "Reset to idle";
  $: recordingActionLabel =
    recordingCommandState === "starting"
      ? "Starting…"
      : activeRecordingSession === null
        ? "Start recording"
        : "Recording active";

  async function runPing() {
    pingState = "loading";
    pingError = "";

    try {
      pingResponse = await ping();
      pingState = "success";
    } catch (error) {
      pingResponse = "";
      pingError = error instanceof Error ? error.message : "Unknown ping failure";
      pingState = "error";
    }
  }

  async function initializeWorkspace() {
    await Promise.all([
      hydrateSettings(),
      refreshGeminiApiKeyPresence(false),
      refreshRecordingShortcutStatus(false)
    ]);
    await loadRecordingDevices();
    await syncRecorderFromNative(true);

    if (activeRecordingSession === null && get(appStatus).phase === "idle") {
      syncIdleStatus();
    }
  }

  async function hydrateSettings() {
    settingsState = "loading";
    settingsError = "";

    try {
      const persistedSettings = await loadAppSettings();

      settingsDraft.patch(persistedSettings);
      lastSavedSettings = persistedSettings;
      recordingShortcutDraft = persistedSettings.shortcut ?? "";
      settingsState = "idle";
    } catch (error) {
      settingsError = error instanceof Error ? error.message : "Unable to load saved preferences";
      settingsState = "error";
    }
  }

  async function refreshRecordingShortcutStatus(showFeedback = true) {
    if (
      recordingShortcutActionState === "applying" ||
      recordingShortcutActionState === "reapplying" ||
      recordingShortcutActionState === "clearing"
    ) {
      return;
    }

    recordingShortcutActionState = "loading";

    if (showFeedback) {
      recordingShortcutError = "";
    }

    try {
      recordingShortcutStatus = await getRecordingShortcutStatus();
      recordingShortcutActionState = "idle";
    } catch (error) {
      recordingShortcutActionState = "error";
      recordingShortcutError =
        error instanceof Error ? error.message : "Unable to load the recording shortcut status.";
    }
  }

  async function refreshGeminiApiKeyPresence(showFeedback = true) {
    if (geminiApiKeyActionState === "saving" || geminiApiKeyActionState === "clearing") {
      return;
    }

    geminiApiKeyActionState = "checking";
    geminiApiKeyPresenceState = "loading";

    if (showFeedback) {
      geminiApiKeyStatusDetail = "";
    }

    try {
      geminiApiKeyPresence = await hasGeminiApiKey();
      geminiApiKeyPresenceState = geminiApiKeyPresence ? "present" : "missing";
      geminiApiKeyActionState = "idle";

      if (showFeedback) {
        geminiApiKeyStatusDetail = geminiApiKeyPresence
          ? "Gemini API key is available in the OS keychain."
          : "No Gemini API key is saved in the OS keychain.";
      }
    } catch (error) {
      geminiApiKeyPresenceState = "error";
      geminiApiKeyActionState = "error";
      geminiApiKeyStatusDetail =
        error instanceof Error ? error.message : "Unable to check the Gemini API key status.";
    }
  }

  async function persistSettings(
    nextSettings: SettingsDraft,
    options: { rethrow?: boolean } = {}
  ) {
    const requestId = ++latestSettingsRequest;

    settingsDraft.patch(nextSettings);
    settingsState = "saving";
    settingsError = "";

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

      settingsDraft.patch(persistedSettings);
      recordingInputOptions.set(
        createRecordingInputOptions(availableRecordingDevices, persistedSettings.selectedMicrophone)
      );

      if (activeRecordingSession === null && get(appStatus).phase === "idle") {
        syncIdleStatus();
      }

      settingsState = "idle";
    } catch (error) {
      const resolvedError =
        error instanceof Error ? error : new Error("Unable to save preferences");

      if (requestId !== latestSettingsRequest) {
        if (options.rethrow) {
          throw resolvedError;
        }

        return;
      }

      settingsError = resolvedError.message;
      settingsState = "error";

      if (lastSavedSettings) {
        settingsDraft.patch(lastSavedSettings);
        recordingInputOptions.set(
          createRecordingInputOptions(availableRecordingDevices, lastSavedSettings.selectedMicrophone)
        );
      }

      if (options.rethrow) {
        throw resolvedError;
      }
    }
  }

  async function loadRecordingDevices() {
    recordingDevicesState = "loading";
    recordingDevicesError = "";

    try {
      availableRecordingDevices = await listRecordingInputDevices();
      recordingInputOptions.set(
        createRecordingInputOptions(availableRecordingDevices, get(settingsDraft).selectedMicrophone)
      );
      recordingDevicesState = "ready";
    } catch (error) {
      availableRecordingDevices = [];
      recordingDevicesError =
        error instanceof Error ? error.message : "Unable to load recording input devices";
      recordingInputOptions.set(
        createRecordingInputOptions(availableRecordingDevices, get(settingsDraft).selectedMicrophone)
      );
      recordingDevicesState = "error";
    }

    if (activeRecordingSession === null && get(appStatus).phase === "idle") {
      syncIdleStatus();
    }
  }

  function startRecordingStatusPolling() {
    if (recordingStatusPoller !== null) {
      return;
    }

    recordingStatusPoller = window.setInterval(() => {
      void syncRecorderFromNative(true);
    }, recordingStatusPollIntervalMs);
  }

  function stopRecordingStatusPolling() {
    if (recordingStatusPoller === null) {
      return;
    }

    window.clearInterval(recordingStatusPoller);
    recordingStatusPoller = null;
  }

  async function syncRecorderFromNative(suppressErrors = false) {
    if (isRefreshingRecordingStatus) {
      return;
    }

    isRefreshingRecordingStatus = true;

    try {
      const status = await getRecordingStatus();

      latestRecordingStatus = status;
      await applyNativeRecordingStatus(status);
    } catch (error) {
      if (!suppressErrors && activeRecordingSession !== null && recordingCommandState === null) {
        stopRecordingStatusPolling();
        activeRecordingSession = null;
        appStatus.setStatus(
          getAppStatusForPhase("error", {
            detail:
              error instanceof Error
                ? error.message
                : "Unable to refresh the native recording status.",
            transcriptPreview:
              "SpeakEx could not refresh the current recording status. No transcription ran automatically.",
            inputLabel: selectedMicrophoneLabel,
            recordingTiming: null,
            recordedAudio: get(appStatus).recordedAudio
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
      activeRecordingSession = {
        id: status.activeSessionId,
        inputDeviceName: status.inputDeviceName
      };
      appStatus.setStatus(buildActiveRecordingStatus(status));
      startRecordingStatusPolling();
      return;
    }

    stopRecordingStatusPolling();

    if (
      activeRecordingSession !== null &&
      status.lastCompletedSessionId !== null &&
      status.lastCompletedSessionId === activeRecordingSession.id &&
      recordingCommandState === null
    ) {
      await finalizeCompletedRecording(status);
      return;
    }

    if (status.phase === "idle" && activeRecordingSession === null && get(appStatus).phase === "idle") {
      syncIdleStatus();
    }
  }

  async function finalizeCompletedRecording(status: RecordingStatus) {
    const recordingTiming = buildRecordingTimingFromStatus(status);

    recordingCommandState = "stopping";
    appStatus.setStatus(
      getAppStatusForPhase("recording", {
        headline: status.limitReached ? "Maximum duration reached." : "Finalizing recorded audio.",
        detail: status.limitReached
          ? `Capture stopped automatically after reaching the ${formatDuration(status.maxDurationMs)} limit. Finalizing the local recording now.`
          : "Recording has stopped. Finalizing the local recording now.",
        transcriptPreview: status.limitReached
          ? "The app is preparing the completed recording after the automatic safety stop. No transcription will run automatically."
          : "The app is preparing the completed recording metadata. No transcription will run automatically.",
        inputLabel: status.inputDeviceName ?? selectedMicrophoneLabel,
        durationLabel: buildDurationSummaryLabel(recordingTiming),
        recordingTiming,
        recordedAudio: null
      })
    );

    try {
      const stoppedRecording = await stopRecording();

      activeRecordingSession = null;
      latestTranscript = null;
      latestManualTranscriptionResult = null;
      latestManualTranscriptionFailure = null;
      const completedStatus = buildCompletedRecordingStatus(stoppedRecording);
      latestCompletedRecordingMetadata = completedStatus.recordedAudio;
      appStatus.setStatus(completedStatus);
    } catch (error) {
      activeRecordingSession = null;
      appStatus.setStatus(
        getAppStatusForPhase("error", {
          detail:
            error instanceof Error
              ? error.message
              : "Unable to finalize the completed recording.",
          transcriptPreview: status.limitReached
            ? "The recorder hit the hard time limit, but the UI could not finish loading the completed recording metadata. No transcription ran automatically."
            : "The UI could not finish loading the completed recording metadata. No transcription ran automatically.",
          inputLabel: status.inputDeviceName ?? selectedMicrophoneLabel,
          durationLabel: buildDurationSummaryLabel(recordingTiming),
          recordingTiming,
          recordedAudio: null
        })
      );
    } finally {
      recordingCommandState = null;
    }
  }

  function clearHistorySelection() {
    latestHistoryDetailRequest += 1;
    selectedHistoryEntryId = null;
    selectedHistoryEntry = null;
    historyDetailError = "";
    historyDetailState = "idle";
  }

  async function selectHistoryEntry(id: string) {
    if (
      selectedHistoryEntryId === id &&
      (historyDetailState === "loading" || (historyDetailState === "ready" && selectedHistoryEntry?.id === id))
    ) {
      return;
    }

    selectedHistoryEntryId = id;
    selectedHistoryEntry = null;
    historyDetailError = "";
    historyDetailState = "loading";

    const requestId = ++latestHistoryDetailRequest;

    try {
      const entry = await getTranscription(id);

      if (requestId !== latestHistoryDetailRequest) {
        return;
      }

      if (entry === null) {
        throw new Error("The selected transcript is no longer available in local history.");
      }

      selectedHistoryEntry = entry;
      historyDetailState = "ready";
    } catch (error) {
      if (requestId !== latestHistoryDetailRequest) {
        return;
      }

      selectedHistoryEntry = null;
      historyDetailError = error instanceof Error ? error.message : "Unable to load the selected transcript";
      historyDetailState = "error";
    }
  }

  async function loadHistoryEntries(preferredSelectionId: string | null = null) {
    historyState = "loading";
    historyError = "";

    try {
      historyEntries = (await getHistory()).map(mapHistoryEntry);
      historyState = "ready";

      const nextSelectedId =
        preferredSelectionId !== null && historyEntries.some((entry) => entry.id === preferredSelectionId)
          ? preferredSelectionId
          : selectedHistoryEntryId !== null &&
              historyEntries.some((entry) => entry.id === selectedHistoryEntryId)
            ? selectedHistoryEntryId
            : historyEntries[0]?.id ?? null;

      if (nextSelectedId === null) {
        clearHistorySelection();
        return;
      }

      await selectHistoryEntry(nextSelectedId);
    } catch (error) {
      historyEntries = [];
      clearHistorySelection();
      historyError = error instanceof Error ? error.message : "Unable to load saved transcripts";
      historyState = "error";
    }
  }

  async function removeHistoryEntry(id: string) {
    if (historyBusyEntryId || isClearingHistory) {
      return;
    }

    historyBusyEntryId = id;
    historyError = "";

    try {
      const result = await deleteTranscription(id);

      if (!result.deleted) {
        throw new Error("The selected transcript was not found in local history.");
      }

      const removedIndex = historyEntries.findIndex((entry) => entry.id === id);
      historyEntries = historyEntries.filter((entry) => entry.id !== id);

      if (selectedHistoryEntryId === id) {
        const fallbackEntry =
          historyEntries[removedIndex] ?? historyEntries[Math.max(removedIndex - 1, 0)] ?? null;

        if (fallbackEntry) {
          await selectHistoryEntry(fallbackEntry.id);
        } else {
          clearHistorySelection();
        }
      }

      historyState = "ready";
    } catch (error) {
      historyError = error instanceof Error ? error.message : "Unable to delete the selected transcript";
    } finally {
      historyBusyEntryId = null;
    }
  }

  async function clearAllHistory() {
    if (isClearingHistory || historyEntries.length === 0 || historyBusyEntryId) {
      return;
    }

    isClearingHistory = true;
    historyError = "";

    try {
      await clearHistory();
      historyEntries = [];
      clearHistorySelection();
      historyState = "ready";
    } catch (error) {
      historyError = error instanceof Error ? error.message : "Unable to clear transcript history";
    } finally {
      isClearingHistory = false;
    }
  }

  function updateProvider(provider: ProviderId) {
    void persistSettings({ ...get(settingsDraft), provider });
  }

  function updateLanguage(event: Event) {
    void persistSettings({
      ...get(settingsDraft),
      defaultLanguage: (event.currentTarget as HTMLSelectElement).value
    });
  }

  function updateMicrophone(event: Event) {
    const selectedMicrophone = (event.currentTarget as HTMLSelectElement).value;

    recordingInputOptions.set(createRecordingInputOptions(availableRecordingDevices, selectedMicrophone));

    void persistSettings({
      ...get(settingsDraft),
      selectedMicrophone
    });
  }

  function toggleSetting(key: DraftToggleKey) {
    const draft = get(settingsDraft);

    void persistSettings({
      ...draft,
      [key]: !draft[key]
    });
  }

  async function submitRecordingShortcut() {
    const nextShortcut = recordingShortcutDraft.trim();

    if (isRecordingShortcutBusy || nextShortcut.length === 0) {
      return;
    }

    await saveAndApplyRecordingShortcut(nextShortcut, "applying");
  }

  async function reapplySavedRecordingShortcut() {
    const savedShortcut = get(settingsDraft).shortcut;

    if (isRecordingShortcutBusy || savedShortcut === null) {
      return;
    }

    recordingShortcutActionState = "reapplying";
    recordingShortcutError = "";

    try {
      recordingShortcutStatus = await applyRecordingShortcut(savedShortcut);
      recordingShortcutDraft = savedShortcut;
      recordingShortcutActionState = "idle";
    } catch (error) {
      recordingShortcutActionState = "error";
      recordingShortcutError =
        error instanceof Error
          ? error.message
          : "Unable to re-apply the saved recording shortcut.";
    }
  }

  async function clearRecordingShortcutSetting() {
    if (!canClearRecordingShortcut) {
      return;
    }

    recordingShortcutDraft = "";
    await saveAndApplyRecordingShortcut(null, "clearing");
  }

  async function saveAndApplyRecordingShortcut(
    shortcut: string | null,
    action: "applying" | "clearing"
  ) {
    recordingShortcutActionState = action;
    recordingShortcutError = "";

    try {
      await persistSettings(
        {
          ...get(settingsDraft),
          shortcut
        },
        { rethrow: true }
      );

      const savedShortcut = get(settingsDraft).shortcut;
      recordingShortcutDraft = savedShortcut ?? "";
      recordingShortcutStatus = await applyRecordingShortcut(savedShortcut);
      recordingShortcutActionState = "idle";
    } catch (error) {
      recordingShortcutDraft = get(settingsDraft).shortcut ?? "";
      recordingShortcutActionState = "error";
      recordingShortcutError =
        error instanceof Error
          ? error.message
          : action === "clearing"
            ? "Unable to clear the saved recording shortcut."
            : "Unable to save and apply the recording shortcut.";
    }
  }

  async function submitGeminiApiKey() {
    if (isGeminiApiKeyBusy || geminiApiKeyDraftValue.length === 0) {
      return;
    }

    const replacingExistingKey = geminiApiKeyPresence;

    geminiApiKeyActionState = "saving";
    geminiApiKeyStatusDetail = "";

    try {
      await saveGeminiApiKey(geminiApiKeyDraftValue);
      geminiApiKeyDraft = "";
      geminiApiKeyPresence = true;
      geminiApiKeyPresenceState = "present";
      geminiApiKeyActionState = "idle";
      geminiApiKeyStatusDetail = replacingExistingKey
        ? "Gemini API key replaced in the OS keychain."
        : "Gemini API key saved to the OS keychain.";
    } catch (error) {
      geminiApiKeyActionState = "error";
      geminiApiKeyPresenceState = geminiApiKeyPresence ? "present" : "missing";
      geminiApiKeyStatusDetail =
        error instanceof Error ? error.message : "Unable to save the Gemini API key.";
    }
  }

  async function removeGeminiApiKey() {
    if (isGeminiApiKeyBusy || !geminiApiKeyPresence) {
      return;
    }

    geminiApiKeyActionState = "clearing";
    geminiApiKeyStatusDetail = "";

    try {
      const cleared = await clearGeminiApiKey();

      geminiApiKeyDraft = "";
      geminiApiKeyPresence = false;
      geminiApiKeyPresenceState = "missing";
      geminiApiKeyActionState = "idle";
      geminiApiKeyStatusDetail = cleared
        ? "Gemini API key cleared from the OS keychain."
        : "No Gemini API key was stored in the OS keychain.";
    } catch (error) {
      geminiApiKeyActionState = "error";
      geminiApiKeyPresenceState = geminiApiKeyPresence ? "present" : "missing";
      geminiApiKeyStatusDetail =
        error instanceof Error ? error.message : "Unable to clear the Gemini API key.";
    }
  }

  async function beginRecording() {
    if (!canStartRecording) {
      if (selectedMicrophoneUnavailable) {
        appStatus.setStatus(
          getAppStatusForPhase("error", {
            detail:
              "The saved microphone is unavailable. Choose one of the loaded inputs before starting a recording.",
            transcriptPreview:
              "The recorder was not started because the current microphone selection does not match any available device.",
            inputLabel: selectedMicrophoneLabel,
            recordedAudio: null
          })
        );
      }

      return;
    }

    recordingCommandState = "starting";
    latestTranscript = null;
    latestMockTranscriptionResult = null;
    latestManualTranscriptionResult = null;
    latestManualOutcomeSettings = null;
    latestManualTranscriptionFailure = null;
    latestCompletedRecordingMetadata = null;

    try {
      const session = await startRecording(resolveSelectedDeviceName());

      activeRecordingSession = session;
      appStatus.setStatus(buildActiveRecordingStatus(buildFallbackRecordingStatus(session)));
      await syncRecorderFromNative(true);
      startRecordingStatusPolling();
    } catch (error) {
      appStatus.setStatus(
        getAppStatusForPhase("error", {
          detail: error instanceof Error ? error.message : "Unable to start the recorder.",
          transcriptPreview:
            "The native start_recording command did not succeed. Check the selected microphone and try again.",
          inputLabel: selectedMicrophoneLabel,
          recordedAudio: null
        })
      );
    } finally {
      recordingCommandState = null;
    }
  }

  async function finishRecording() {
    if (!canStopRecording) {
      return;
    }

    recordingCommandState = "stopping";
    stopRecordingStatusPolling();

    try {
      const stoppedRecording = await stopRecording();

      activeRecordingSession = null;
      latestTranscript = null;
      latestMockTranscriptionResult = null;
      latestManualTranscriptionResult = null;
      latestManualOutcomeSettings = null;
      latestManualTranscriptionFailure = null;
      const completedStatus = buildCompletedRecordingStatus(stoppedRecording);
      latestCompletedRecordingMetadata = completedStatus.recordedAudio;
      appStatus.setStatus(completedStatus);
    } catch (error) {
      activeRecordingSession = null;
      appStatus.setStatus(
        getAppStatusForPhase("error", {
          detail: error instanceof Error ? error.message : "Unable to stop the recorder.",
          transcriptPreview:
            "The native stop_recording command did not finish successfully. The recording session is no longer marked active in the UI.",
          inputLabel: selectedMicrophoneLabel,
          recordedAudio: null
        })
      );
    } finally {
      recordingCommandState = null;
    }
  }

  async function discardRecording() {
    if (!canCancelRecording) {
      return;
    }

    recordingCommandState = "cancelling";
    stopRecordingStatusPolling();

    try {
      const cancelled = await cancelRecording();

      latestRecordingStatus = null;
      activeRecordingSession = null;
      latestTranscript = null;
      latestMockTranscriptionResult = null;
      latestManualTranscriptionResult = null;
      latestManualOutcomeSettings = null;
      latestManualTranscriptionFailure = null;
      latestCompletedRecordingMetadata = null;
      syncIdleStatus(
        cancelled.deletedAudioPath
          ? `Recording ${cancelled.sessionId} was cancelled and the temporary file was deleted from the app cache.`
          : `Recording ${cancelled.sessionId} was cancelled before any audio file needed to be kept.`
      );
    } catch (error) {
      activeRecordingSession = null;
      appStatus.setStatus(
        getAppStatusForPhase("error", {
          detail: error instanceof Error ? error.message : "Unable to cancel the recorder.",
          transcriptPreview:
            "The native cancel_recording command did not finish successfully. Retry only after confirming the recorder returned to idle.",
          inputLabel: selectedMicrophoneLabel,
          recordedAudio: null
        })
      );
    } finally {
      recordingCommandState = null;
    }
  }

  async function startMockTranscription() {
    if (!canRunMockTranscription) {
      return;
    }

    activeTranscriptionCommand = "mock";
    const previousRecordedAudio = get(appStatus).recordedAudio;
    latestTranscript = null;
    latestMockTranscriptionResult = null;
    latestManualTranscriptionResult = null;
    latestManualOutcomeSettings = null;
    latestManualTranscriptionFailure = null;
    latestCompletedRecordingMetadata = previousRecordedAudio;

    appStatus.setStatus(
      getAppStatusForPhase("transcribing", {
        inputLabel: selectedMicrophoneLabel,
        detail:
          "SpeakEx is running the mock transcript command. Any recorded audio remains separate from this sample result.",
        recordingTiming: previousRecordedAudio ? buildRecordingTimingFromRecordedAudio(previousRecordedAudio) : null,
        recordedAudio: previousRecordedAudio
      })
    );

    try {
      const result = await runMockTranscription();

      latestMockTranscriptionResult = result;
      latestTranscript = result.transcript;
      appStatus.setStatus(buildCompletedMockStatus(result, previousRecordedAudio));

      if (result.savedToHistory) {
        await loadHistoryEntries();
      }
    } catch (error) {
      latestMockTranscriptionResult = null;
      latestTranscript = null;
      appStatus.setStatus(
        getAppStatusForPhase("error", {
          inputLabel: selectedMicrophoneLabel,
          detail: error instanceof Error ? error.message : "Unable to finish the mock transcription flow.",
          transcriptPreview:
            "The mock transcript command did not finish. Any recorded audio remains local and separate from Gemini transcription.",
          recordingTiming: previousRecordedAudio ? buildRecordingTimingFromRecordedAudio(previousRecordedAudio) : null,
          recordedAudio: previousRecordedAudio
        })
      );
    } finally {
      activeTranscriptionCommand = null;
    }
  }

  async function startManualTranscription() {
    if (!canRunManualTranscription || transcribableRecordedAudio === null) {
      return;
    }

    if (!(await hasCompletedRecordingAudio(transcribableRecordedAudio))) {
      latestTranscript = null;
      latestMockTranscriptionResult = null;
      latestManualTranscriptionResult = null;
      latestManualOutcomeSettings = null;
      latestManualTranscriptionFailure = `The completed recording is no longer available at ${transcribableRecordedAudio.path}.`;
      latestCompletedRecordingMetadata = transcribableRecordedAudio;
      appStatus.setStatus(
        buildMissingManualRecordingStatus(
          transcribableRecordedAudio,
          latestManualTranscriptionFailure,
          `The recorded audio file is gone, so manual transcription cannot start and Retry transcription stays hidden. Record again to create a fresh file before transcribing. ${hiddenManualNotificationFailedMessage}`
        )
      );
      return;
    }

    const currentSettings = get(settingsDraft);
    const manualOutcomeSettings: LatestManualOutcomeSettings = {
      autoCopy: currentSettings.autoCopy,
      saveAudioFiles: currentSettings.saveAudioFiles,
      saveTranscriptionHistory: currentSettings.saveTranscriptionHistory
    };

    activeTranscriptionCommand = "manual";
    latestTranscript = null;
    latestMockTranscriptionResult = null;
    latestManualTranscriptionResult = null;
    latestManualOutcomeSettings = null;
    latestManualTranscriptionFailure = null;

    appStatus.setStatus(
      getAppStatusForPhase("transcribing", {
        headline: "Manual transcription is running.",
        inputLabel: transcribableRecordedAudio.inputDeviceName,
        detail:
          "SpeakEx is transcribing the current recorded audio with Gemini. Clipboard, history, default audio cleanup, retryable failure state, and hidden-window notifications follow your saved settings.",
        transcriptTitle: "Transcript incoming…",
        transcriptPreview: `${formatFileName(transcribableRecordedAudio.path)} is being transcribed with Gemini.`,
        durationLabel: formatDuration(transcribableRecordedAudio.durationMs),
        recordingTiming: buildRecordingTimingFromRecordedAudio(transcribableRecordedAudio),
        recordedAudio: transcribableRecordedAudio
      })
    );

    try {
      const result = await runCompletedRecordingTranscription(transcribableRecordedAudio);
      const retainedRecordedAudio = buildRetainedRecordedAudio(transcribableRecordedAudio, result);

      latestTranscript = result.transcript;
      latestManualTranscriptionResult = result;
      latestManualOutcomeSettings = manualOutcomeSettings;
      latestCompletedRecordingMetadata = transcribableRecordedAudio;
      appStatus.setStatus(buildCompletedManualStatus(result, transcribableRecordedAudio, retainedRecordedAudio));

      if (result.historySaved) {
        await loadHistoryEntries(result.historyId);
      }
    } catch (error) {
      const manualFailureDetail =
        error instanceof Error ? error.message : "Unable to finish the manual transcription flow.";
      const retryableRecordedAudio = (await hasCompletedRecordingAudio(transcribableRecordedAudio))
        ? transcribableRecordedAudio
        : null;

      latestTranscript = null;
      latestMockTranscriptionResult = null;
      latestManualOutcomeSettings = null;
      latestManualTranscriptionFailure = manualFailureDetail;
      latestCompletedRecordingMetadata = transcribableRecordedAudio;
      appStatus.setStatus(
        getAppStatusForPhase("error", {
          inputLabel: transcribableRecordedAudio.inputDeviceName,
          detail: manualFailureDetail,
          transcriptPreview:
            retryableRecordedAudio === null
              ? `Transcription did not finish, and the recorded audio file is no longer available for recovery. Retry transcription is hidden until you record again. ${hiddenManualNotificationFailedMessage}`
              : `Transcription did not finish, but the recorded audio file remains local so you can use Retry transcription to try the same recording again. ${hiddenManualNotificationFailedMessage}`,
          durationLabel: formatDuration(transcribableRecordedAudio.durationMs),
          recordingTiming: buildRecordingTimingFromRecordedAudio(transcribableRecordedAudio),
          recordedAudio: retryableRecordedAudio
        })
      );
    } finally {
      activeTranscriptionCommand = null;
    }
  }

  function resetWorkspace() {
    stopRecordingStatusPolling();
    latestRecordingStatus = null;
    activeRecordingSession = null;
    latestTranscript = null;
    latestMockTranscriptionResult = null;
    latestManualTranscriptionResult = null;
    latestManualOutcomeSettings = null;
    latestManualTranscriptionFailure = null;
    latestCompletedRecordingMetadata = null;
    syncIdleStatus();
  }

  onMount(() => {
    void runPing();
    void initializeWorkspace();
    void loadHistoryEntries();
  });

  onDestroy(() => {
    stopRecordingStatusPolling();
  });

  function syncIdleStatus(detail = buildIdleDetail()) {
    appStatus.setStatus(
      getAppStatusForPhase("idle", {
        detail,
        transcriptPreview:
          recordingDevicesState === "error"
            ? "Microphone loading failed. Retry device loading or switch to the mock transcription flow while recording is unavailable."
            : selectedMicrophoneUnavailable
              ? "Choose an available microphone before starting a recording. Your saved selection stays in place until you update it."
              : "Start a recording to capture a temporary audio file locally, then choose either the mock transcript or full Gemini transcription.",
        inputLabel: selectedMicrophoneLabel,
        durationLabel: "—",
        recordingTiming: null,
        recordedAudio: null
      })
    );
  }

  function buildIdleDetail() {
    if (recordingDevicesState === "loading") {
      return "Loading available microphones before recording becomes available.";
    }

    if (recordingDevicesState === "error") {
      return `Unable to load recording inputs: ${recordingDevicesError}`;
    }

    if (selectedMicrophoneUnavailable) {
      return "The saved microphone is not currently available. Pick one of the loaded inputs before starting a recording.";
    }

    return `Ready to record from ${selectedMicrophoneLabel}. Stop keeps the audio file so you can choose what to do next.`;
  }

  function resolveSelectedDeviceName() {
    const selectedMicrophone = get(settingsDraft).selectedMicrophone;

    return selectedMicrophone === "default" ? null : selectedMicrophone;
  }

  function resolveRecordingLimitMs() {
    return (
      latestRecordingStatus?.maxDurationMs ??
      get(appStatus).recordingTiming?.maxDurationMs ??
      get(appStatus).recordedAudio?.maxDurationMs ??
      fallbackRecordingLimitMs
    );
  }

  function buildFallbackRecordingStatus(session: ActiveRecordingSession): RecordingStatus {
    const maxDurationMs = resolveRecordingLimitMs();

    return {
      phase: "recording",
      activeSessionId: session.id,
      inputDeviceName: session.inputDeviceName,
      elapsedMs: 0,
      remainingMs: maxDurationMs,
      maxDurationMs,
      limitReached: false,
      lastCompletedSessionId: null
    };
  }

  function buildRecordingTimingFromStatus(status: RecordingStatus): RecordingTiming {
    return {
      elapsedMs: status.elapsedMs,
      remainingMs: status.remainingMs,
      maxDurationMs: status.maxDurationMs,
      limitReached: status.limitReached
    };
  }

  function buildRecordingTimingFromRecordedAudio(recordedAudio: RecordedAudioMetadata): RecordingTiming {
    return {
      elapsedMs: recordedAudio.durationMs,
      remainingMs:
        recordedAudio.durationMs === null || recordedAudio.maxDurationMs === null
          ? null
          : Math.max(recordedAudio.maxDurationMs - recordedAudio.durationMs, 0),
      maxDurationMs: recordedAudio.maxDurationMs,
      limitReached: recordedAudio.limitReached
    };
  }

  function buildDurationSummaryLabel(recordingTiming: RecordingTiming | null): string {
    if (!recordingTiming) {
      return "—";
    }

    return `${formatDuration(recordingTiming.elapsedMs)} elapsed · ${formatDuration(recordingTiming.remainingMs)} remaining`;
  }

  function buildActiveRecordingStatus(status: RecordingStatus) {
    const recordingTiming = buildRecordingTimingFromStatus(status);

    return getAppStatusForPhase("recording", {
      headline:
        status.phase === "starting"
          ? "Recording is starting."
          : status.phase === "stopping"
            ? "Recording is stopping."
            : status.phase === "cancelling"
              ? "Recording is cancelling."
              : "Recording is in progress.",
      detail: `Audio capture is active and will stop automatically at ${formatDuration(status.maxDurationMs)}. Use Stop to keep the audio file or Cancel to discard it sooner.`,
      transcriptTitle: "Live capture in progress…",
      transcriptPreview: `Recording session ${status.activeSessionId ?? "current"} is saving a temporary audio file locally. No transcription will run automatically, including when the duration limit is reached.`,
      inputLabel: status.inputDeviceName ?? selectedMicrophoneLabel,
      durationLabel: buildDurationSummaryLabel(recordingTiming),
      recordingTiming,
      recordedAudio: null
    });
  }

  function buildCompletedRecordingStatus(stoppedRecording: StoppedRecording) {
    const recordedAudio = mapStoppedRecording(stoppedRecording);
    const recordingTiming = buildRecordingTimingFromRecordedAudio(recordedAudio);

    return getAppStatusForPhase("completed", {
      headline: stoppedRecording.limitReached
        ? `Recording stopped at the ${formatDuration(recordingTiming.maxDurationMs)} limit.`
        : "Recorded audio is ready.",
      detail: stoppedRecording.limitReached
        ? `Capture stopped automatically because the maximum recording duration of ${formatDuration(recordingTiming.maxDurationMs)} was reached. The audio file was kept locally, and no transcription ran automatically.`
        : "Recording stopped successfully and kept the audio file locally. Choose the mock transcript or Gemini transcription when you are ready.",
      transcriptTitle: "Recorded audio metadata",
      transcriptPreview: stoppedRecording.limitReached
        ? `${formatFileName(recordedAudio.path)} was captured locally after the automatic safety stop. Run a mock transcript or Gemini transcription when you are ready.`
        : `${formatFileName(recordedAudio.path)} is available locally and ready for a mock transcript or Gemini transcription.`,
      inputLabel: stoppedRecording.inputDeviceName,
      durationLabel: buildDurationSummaryLabel(recordingTiming),
      recordingTiming,
      recordedAudio
    });
  }

  function buildCompletedMockStatus(
    result: RunMockTranscriptionResult,
    recordedAudio: RecordedAudioMetadata | null
  ) {
    const historyDetail = describeMockHistoryOutcome(result.savedToHistory);

    return getAppStatusForPhase("completed", {
      headline: "Mock transcript ready.",
      detail: `${historyDetail} This sample result is separate from Gemini transcription and does not use the recorded audio file.`,
      transcriptTitle: result.savedToHistory
        ? "Mock transcript saved locally"
        : "Mock transcript kept in memory only",
      transcriptPreview: result.transcript.text,
      inputLabel: selectedMicrophoneLabel,
      durationLabel: formatDuration(result.transcript.durationMs),
      recordingTiming: recordedAudio ? buildRecordingTimingFromRecordedAudio(recordedAudio) : null,
      recordedAudio
    });
  }

  function buildCompletedManualStatus(
    result: RunCompletedRecordingTranscriptionResult,
    recordedAudio: RecordedAudioMetadata,
    retainedRecordedAudio: RecordedAudioMetadata | null
  ) {
    const outcomeSummary = [
      describeManualHistoryOutcome(result, get(settingsDraft).saveTranscriptionHistory),
      describeManualClipboardOutcome(result, get(settingsDraft).autoCopy),
      describeManualAudioOutcome(result, get(settingsDraft).saveAudioFiles)
    ].join(" ");
    const warnings = collectOutcomeWarnings(result);

    return getAppStatusForPhase("completed", {
      headline: warnings.length === 0 ? "Transcript ready." : "Transcript ready with warnings.",
      detail: `${outcomeSummary} ${hiddenManualNotificationCompletedMessage}`,
      transcriptTitle: result.historySaved
        ? warnings.length === 0
          ? "Transcript saved locally"
          : "Transcript saved locally with warnings"
        : result.historyError
          ? "Transcript ready in preview only"
          : "Transcript ready in preview",
      transcriptPreview: result.transcript.text,
      inputLabel: recordedAudio.inputDeviceName,
      durationLabel: formatDuration(result.transcript.durationMs ?? recordedAudio.durationMs),
      recordingTiming: buildRecordingTimingFromRecordedAudio(recordedAudio),
      recordedAudio: retainedRecordedAudio
    });
  }

  function buildRetainedRecordedAudio(
    recordedAudio: RecordedAudioMetadata,
    result: RunCompletedRecordingTranscriptionResult
  ): RecordedAudioMetadata | null {
    if (result.audioDeleted && !result.audioDeleteError) {
      return null;
    }

    return {
      ...recordedAudio,
      path: result.retainedAudioPath ?? recordedAudio.path
    };
  }

  function buildMissingManualRecordingStatus(
    recordedAudio: RecordedAudioMetadata,
    detail: string,
    transcriptPreview: string
  ) {
    return getAppStatusForPhase("error", {
      headline: "Manual transcription needs a new recording.",
      inputLabel: recordedAudio.inputDeviceName,
      detail,
      transcriptPreview,
      durationLabel: formatDuration(recordedAudio.durationMs),
      recordingTiming: buildRecordingTimingFromRecordedAudio(recordedAudio),
      recordedAudio: null
    });
  }

  function collectOutcomeWarnings(
    result: RunCompletedRecordingTranscriptionResult
  ): string[] {
    return [result.historyError, result.clipboardError, result.audioDeleteError].filter(
      (value): value is string => value !== null && value.trim() !== ""
    );
  }

  function describeMockHistoryOutcome(savedToHistory: boolean): string {
    return savedToHistory
      ? "Saved to local history."
      : "Local history was skipped because Save transcription history is turned off.";
  }

  function describeManualHistoryOutcome(
    result: RunCompletedRecordingTranscriptionResult,
    saveTranscriptionHistory: boolean
  ): string {
    if (result.historySaved) {
      return collectOutcomeWarnings(result).length === 0
        ? "Saved to local history."
        : "Saved to local history with warnings recorded on the entry.";
    }

    if (result.historyError) {
      return "Could not save to local history, so this transcript stays in the current preview.";
    }

    return saveTranscriptionHistory
      ? "Local history was not updated."
      : "Local history was skipped because Save transcription history is turned off.";
  }

  function describeManualClipboardOutcome(
    result: RunCompletedRecordingTranscriptionResult,
    autoCopy: boolean
  ): string {
    if (result.copiedToClipboard) {
      return "Copied to the system clipboard.";
    }

    if (result.clipboardError) {
      return "Clipboard copy failed, so the transcript stayed in SpeakEx only.";
    }

    return autoCopy
      ? "Clipboard copy was requested but did not finish."
      : "Clipboard copy was skipped because Auto-copy transcript is turned off.";
  }

  function describeManualAudioOutcome(
    result: RunCompletedRecordingTranscriptionResult,
    saveAudioFiles: boolean
  ): string {
    if (result.audioDeleted && !result.audioDeleteError) {
      return "The local audio file was deleted after transcription.";
    }

    if (result.audioDeleteError) {
      return "Audio cleanup failed, so the local audio file was retained.";
    }

    if (result.retainedAudioPath) {
      return saveAudioFiles
        ? "The local audio file was retained because Save audio files is turned on."
        : "The local audio file was retained locally.";
    }

    return "The local audio file outcome is unavailable.";
  }

  function describeStoredClipboardOutcome(
    copiedToClipboard: boolean,
    storedIssue: string
  ): string {
    if (copiedToClipboard) {
      return "Copied to clipboard";
    }

    return storedIssue === ""
      ? "Not copied to clipboard"
      : "Not copied to clipboard (see warnings)";
  }

  function describeStoredAudioOutcome(
    audioDeleted: boolean,
    audioPath: string | null
  ): string {
    if (audioDeleted) {
      return "Deleted after transcription";
    }

    return audioPath ? "Retained locally" : "No retained audio";
  }

  function mapStoppedRecording(stoppedRecording: StoppedRecording): RecordedAudioMetadata {
    return {
      sessionId: stoppedRecording.sessionId,
      path: stoppedRecording.audioInput.path,
      mimeType: stoppedRecording.audioInput.mimeType,
      durationMs: stoppedRecording.audioInput.durationMs,
      inputDeviceName: stoppedRecording.inputDeviceName,
      sampleRateHz: stoppedRecording.sampleRateHz,
      channels: stoppedRecording.channels,
      fileSizeBytes: stoppedRecording.fileSizeBytes,
      limitReached: stoppedRecording.limitReached,
      maxDurationMs: resolveRecordingLimitMs()
    };
  }

  function mapHistoryEntry(entry: HistoryTranscriptionSummary): HistoryEntryViewModel {
    return {
      id: entry.id,
      title: createHistoryTitle(entry.text),
      excerpt: createHistoryExcerpt(entry.text),
      providerLabel: [entry.provider, entry.model].filter(Boolean).join(" · ") || "Unknown provider",
      createdAtLabel: formatCreatedAt(entry.createdAt),
      durationLabel: formatDuration(entry.durationMs),
      languageLabel: entry.language ?? "Auto / unspecified",
      clipboardLabel: entry.copiedToClipboard ? "Copied to clipboard" : "Not copied to clipboard",
      audioLabel: entry.hasAudioFile ? "Audio retained" : "No retained audio",
      status: entry.hasError ? "attention" : "saved",
      statusLabel: entry.hasError ? "Saved with warnings" : "Saved"
    };
  }

  function createHistoryTitle(text: string): string {
    const trimmedText = text.trim();

    if (!trimmedText) {
      return "Untitled transcript";
    }

    const firstLine = trimmedText.split(/\r?\n/u, 1)[0] ?? trimmedText;

    return firstLine.length > 56 ? `${firstLine.slice(0, 53).trimEnd()}…` : firstLine;
  }

  function createHistoryExcerpt(text: string): string {
    const normalizedText = text.replace(/\s+/gu, " ").trim();

    if (!normalizedText) {
      return "Saved transcript text is empty.";
    }

    return normalizedText.length > 180 ? `${normalizedText.slice(0, 177).trimEnd()}…` : normalizedText;
  }

  function formatCreatedAt(value: string): string {
    const date = new Date(value);

    if (Number.isNaN(date.valueOf())) {
      return value;
    }

    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short"
    }).format(date);
  }

  function formatShortcutValue(
    value: string | null | undefined,
    fallback: string
  ): string {
    const normalized = value?.trim();

    return normalized ? normalized : fallback;
  }

  function describeRecordingShortcutState(
    status: RecordingShortcutStatus | null
  ): string {
    switch (status?.state) {
      case "active":
        return "Active";
      case "invalid":
        return "Invalid";
      case "unavailable":
        return "Unavailable";
      case "unconfigured":
      default:
        return "Unconfigured";
    }
  }

  function describeRecordingShortcutSource(
    status: RecordingShortcutStatus | null
  ): string {
    switch (status?.source) {
      case "saved":
        return "Saved shortcut";
      case "default":
        return "Default shortcut";
      case "custom":
        return "Applied in this session";
      case "none":
      default:
        return "No runtime source";
    }
  }

  function buildRecordingShortcutStatusMessage(
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

  function formatDuration(durationMs: number | null): string {
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

  function formatFileSize(fileSizeBytes: number): string {
    if (fileSizeBytes < 1024) {
      return `${fileSizeBytes} B`;
    }

    if (fileSizeBytes < 1024 * 1024) {
      return `${(fileSizeBytes / 1024).toFixed(1)} KB`;
    }

    return `${(fileSizeBytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatFileName(path: string): string {
    return path.split(/[/\\\\]/u).pop() ?? path;
  }
</script>

<svelte:head>
  <title>SpeakEx — Local-first Transcription</title>
  <meta
    name="description"
    content="SpeakEx desktop app with local recording, mock and manual transcription flows, retry guidance, privacy-safe hidden-window notifications, history, and settings."
  />
</svelte:head>

<main class="app-shell">
  <aside class="sidebar">
    <div class="brand-block">
      <p class="eyebrow">Local-first desktop transcription</p>
      <h1>SpeakEx</h1>
      <p class="brand-copy">
        Local-first transcription for the desktop. Record audio, run a mock transcript for testing,
        or transcribe a completed recording with Gemini when you are ready.
      </p>
    </div>

    <nav class="section-nav" aria-label="Primary">
      {#each navigationSections as section}
        <button
          type="button"
          class:active={$activeSection === section.id}
          on:click={() => activeSection.set(section.id)}
        >
          <span>{section.label}</span>
          <small>{section.blurb}</small>
        </button>
      {/each}
    </nav>

    <section class="bridge-card" aria-live="polite">
      <div>
        <p class="label">App connection</p>
        <h2>Connection check</h2>
      </div>

      {#if pingState === "loading" || pingState === "idle"}
        <p class="bridge-status pending">Checking the app connection…</p>
      {:else if pingState === "success"}
        <p class="bridge-status success">Connected: <strong>{pingResponse}</strong></p>
      {:else}
        <p class="bridge-status error">Unavailable: {pingError}</p>
      {/if}

      <button type="button" class="ghost-button" on:click={runPing} disabled={pingState === "loading"}>
        {pingState === "loading" ? "Checking…" : "Recheck connection"}
      </button>
    </section>
  </aside>

  <section class="workspace">
    <header class="workspace-header">
      <div>
        <p class="eyebrow">Workspace</p>
        <h2>
          {#if $activeSection === "recording"}
            Recording workspace
          {:else if $activeSection === "history"}
            Transcript history
          {:else}
            Settings
          {/if}
        </h2>
      </div>
        <p class="workspace-copy">
          {#if $activeSection === "recording"}
          Record audio, then choose a mock transcript or manual Gemini transcription. Retry only
          appears while the recorded audio file is still available, and hidden-window notifications
          stay privacy-safe.
        {:else if $activeSection === "history"}
          Saved transcripts stay on this device, and the selected detail panel shows the full text
          plus the clipboard and audio outcomes saved with each entry.
        {:else}
          Choose a provider, manage the global shortcut, and store preferences locally. Your
          preferred microphone reflects the devices SpeakEx can use right now.
        {/if}
      </p>
    </header>

    {#if $activeSection === "recording"}
      <div class="view-grid recording-grid">
        <section class="card hero-card">
          <div class="hero-copy">
            <p class="label">Recorder controls</p>
            <h3>{$appStatus.headline}</h3>
            <p>{$appStatus.detail}</p>
            <p class="provider-caption">Selected provider: <strong>{selectedProviderLabel}</strong></p>
            <p class:pending={recordingDevicesState === "loading"} class:success={recordingDevicesState === "ready"} class:error={recordingDevicesState === "error"}>
              {recordingDevicesStatusMessage}
            </p>
            <p class="phase-note">Recording stays separate from transcription. Stop keeps the current recording ready until you choose what happens next.</p>
            <p class="phase-note">After a manual failure, Retry transcription only stays available while the recorded audio file still exists locally.</p>
            <p class="phase-note">Desktop notifications only cover manual transcription outcomes while SpeakEx is hidden. Failure notifications stay generic and keep details in the app.</p>
            <p class="phase-note"><strong>{recordingLimitLabel}</strong> · Elapsed {elapsedTimeLabel} · Remaining {remainingTimeLabel}</p>
            <p class="phase-note">{mockHistoryModeLabel}</p>
          </div>

          <div class="hero-actions">
            <button
              type="button"
              class="primary-button"
              on:click={beginRecording}
              disabled={!canStartRecording}
            >
              {recordingActionLabel}
            </button>
            <button
              type="button"
              class="secondary-button"
              on:click={finishRecording}
              disabled={!canStopRecording}
            >
              {recordingCommandState === "stopping" ? "Stopping…" : "Stop and keep audio"}
            </button>
            <button
              type="button"
              class="ghost-button"
              on:click={discardRecording}
              disabled={!canCancelRecording}
            >
              {recordingCommandState === "cancelling" ? "Cancelling…" : "Cancel and discard"}
            </button>
            <button
              type="button"
              class="secondary-button"
              on:click={startMockTranscription}
              disabled={!canRunMockTranscription}
            >
              {primaryMockActionLabel}
            </button>
            {#if showManualTranscriptionAction}
              <button
                type="button"
                class="secondary-button"
                on:click={startManualTranscription}
                disabled={!canRunManualTranscription}
              >
                {primaryManualActionLabel}
              </button>
            {/if}
            <button
              type="button"
              class="ghost-button"
              on:click={resetWorkspace}
              disabled={activeRecordingSession !== null || ($appStatus.phase === "idle" && displayedRecordedAudio === null && latestTranscript === null)}
            >
              {resetActionLabel}
            </button>
          </div>

          <p
            class:pending={(transcribableRecordedAudio === null && latestManualTranscriptionResult === null && latestManualTranscriptionFailure === null) || isRunningManualTranscription || (latestManualTranscriptionFailure === null && (geminiApiKeyPresenceState === "loading" || geminiApiKeyActionState === "checking"))}
            class:success={latestManualTranscriptionResult !== null || (latestManualTranscriptionFailure === null && transcribableRecordedAudio !== null && geminiApiKeyPresence && !isRunningManualTranscription && geminiApiKeyPresenceState !== "error")}
            class:error={latestManualTranscriptionFailure !== null || (latestManualTranscriptionResult === null && ((transcribableRecordedAudio !== null && (!geminiApiKeyPresence || geminiApiKeyPresenceState === "error")) || geminiApiKeyActionState === "error"))}
          >
            {manualTranscriptionStatusMessage}
          </p>

          {#if manualRecoveryGuidanceMessage}
            <p class="phase-note">{manualRecoveryGuidanceMessage}</p>
          {/if}
        </section>

        <section class="card transcript-card">
          <div class="section-heading">
            <div>
              <p class="label">Preview area</p>
              <h3>Recorder output</h3>
            </div>
            <span
              class="status-pill"
              class:phase-idle={$appStatus.phase === "idle"}
              class:phase-recording={$appStatus.phase === "recording"}
              class:phase-transcribing={$appStatus.phase === "transcribing"}
              class:phase-completed={$appStatus.phase === "completed"}
              class:phase-error={$appStatus.phase === "error"}
            >
              {$appStatus.phaseLabel}
            </span>
          </div>

          <div class="transcript-placeholder">
            <p>{$appStatus.transcriptTitle}</p>
            <p>{$appStatus.transcriptPreview}</p>
          </div>

          {#if latestTranscript}
            <dl class="history-meta draft-meta">
              <div>
                <dt>Provider</dt>
                <dd>{latestTranscript.provider}</dd>
              </div>
              <div>
                <dt>Model</dt>
                <dd>{latestTranscript.model ?? "Default"}</dd>
              </div>
              <div>
                <dt>Language</dt>
                <dd>{latestTranscript.language ?? ($settingsDraft.defaultLanguage === "auto" ? "Auto-detect" : $settingsDraft.defaultLanguage)}</dd>
              </div>
              <div>
                <dt>Duration</dt>
                <dd>{formatDuration(latestTranscript.durationMs ?? displayedRecordedAudio?.durationMs ?? null)}</dd>
              </div>
              <div>
                <dt>History</dt>
                <dd>{latestTranscriptHistoryLabel ?? "History outcome unavailable."}</dd>
              </div>
            </dl>
          {/if}

          {#if latestManualTranscriptionResult}
            <dl class="history-meta draft-meta">
              <div>
                <dt>Clipboard</dt>
                <dd>{manualClipboardLabel}</dd>
              </div>
              <div>
                <dt>Audio file</dt>
                <dd>{manualAudioLabel}</dd>
              </div>
              <div>
                <dt>History ID</dt>
                <dd>{latestManualTranscriptionResult.historyId ?? "Not saved"}</dd>
              </div>
              <div>
                <dt>Retained path</dt>
                <dd>{latestManualTranscriptionResult.retainedAudioPath ?? "Deleted after transcription"}</dd>
              </div>
            </dl>
          {/if}

          {#if manualTranscriptionWarnings.length > 0}
            <div class="current-state-panel phase-error">
              <p class="label">Non-fatal warnings</p>
              <ul class="warning-list">
                {#each manualTranscriptionWarnings as warning}
                  <li>{warning}</li>
                {/each}
              </ul>
            </div>
          {/if}

          {#if displayedRecordedAudio}
            <dl class="history-meta draft-meta">
              <div>
                <dt>Recorded file</dt>
                <dd>{formatFileName(displayedRecordedAudio.path)}</dd>
              </div>
              <div>
                <dt>{transcribableRecordedAudio ? "Stored at" : "Last known path"}</dt>
                <dd>{displayedRecordedAudio.path}</dd>
              </div>
              <div>
                <dt>Format</dt>
                <dd>{displayedRecordedAudio.mimeType}</dd>
              </div>
              <div>
                <dt>Size</dt>
                <dd>{formatFileSize(displayedRecordedAudio.fileSizeBytes)}</dd>
              </div>
              <div>
                <dt>Sample rate</dt>
                <dd>{displayedRecordedAudio.sampleRateHz} Hz</dd>
              </div>
              <div>
                <dt>Channels</dt>
                <dd>{displayedRecordedAudio.channels}</dd>
              </div>
              <div>
                <dt>Max duration</dt>
                <dd>{formatDuration(displayedRecordedAudio.maxDurationMs)}</dd>
              </div>
              <div>
                <dt>Stop reason</dt>
                <dd>{stopReasonLabel}</dd>
              </div>
            </dl>
          {/if}
        </section>

        <section class="card meta-card">
          <div class="section-heading">
            <div>
              <p class="label">Session snapshot</p>
              <h3>Recording overview</h3>
            </div>
          </div>

          <dl class="stats-grid">
            <div>
              <dt>Status</dt>
              <dd>{$appStatus.phaseLabel}</dd>
            </div>
            <div>
              <dt>Provider</dt>
              <dd>{selectedProviderLabel}</dd>
            </div>
            <div>
              <dt>Input</dt>
              <dd>{$appStatus.inputLabel}</dd>
            </div>
            <div>
              <dt>Elapsed</dt>
              <dd>{elapsedTimeLabel}</dd>
            </div>
            <div>
              <dt>Remaining</dt>
              <dd>{remainingTimeLabel}</dd>
            </div>
            <div>
              <dt>Maximum</dt>
              <dd>{maxDurationLabel}</dd>
            </div>
            <div>
              <dt>Summary</dt>
              <dd>{$appStatus.durationLabel}</dd>
            </div>
            <div>
              <dt>Language</dt>
              <dd>{$settingsDraft.defaultLanguage === "auto" ? "Auto-detect" : $settingsDraft.defaultLanguage}</dd>
            </div>
            <div>
              <dt>History</dt>
              <dd>{historyCountLabel}</dd>
            </div>
          </dl>
        </section>

        <section class="card controller-card">
          <div class="section-heading">
            <div>
              <p class="label">Recorder status</p>
              <h3>Current recording state</h3>
            </div>
          </div>

          <div
            class="current-state-panel"
            class:phase-idle={$appStatus.phase === "idle"}
            class:phase-recording={$appStatus.phase === "recording"}
            class:phase-transcribing={$appStatus.phase === "transcribing"}
            class:phase-completed={$appStatus.phase === "completed"}
            class:phase-error={$appStatus.phase === "error"}
          >
            <p class="label">Visible now</p>
            <h3>{$appStatus.phaseLabel}</h3>
            <p>
              This panel shows whether SpeakEx is idle, recording, transcribing, ready, or showing
              an error.
            </p>
          </div>

          <p class:pending={recordingDevicesState === "loading"} class:success={recordingDevicesState === "ready"} class:error={recordingDevicesState === "error"}>
            {recordingDevicesStatusMessage}
          </p>

          <div class="history-card-actions">
            <button
              type="button"
              class="ghost-button"
              on:click={loadRecordingDevices}
              disabled={recordingDevicesState === "loading" || activeRecordingSession !== null}
            >
              {recordingDevicesState === "loading" ? "Loading inputs…" : "Reload microphones"}
            </button>
          </div>

          <dl class="history-meta draft-meta">
            <div>
              <dt>Start command</dt>
              <dd><code>start_recording</code></dd>
            </div>
            <div>
              <dt>Status command</dt>
              <dd><code>get_recording_status</code></dd>
            </div>
            <div>
              <dt>Stop command</dt>
              <dd><code>stop_recording</code></dd>
            </div>
            <div>
              <dt>Cancel command</dt>
              <dd><code>cancel_recording</code></dd>
            </div>
            <div>
              <dt>Mock command</dt>
              <dd><code>run_mock_transcription</code></dd>
            </div>
            <div>
              <dt>Manual flow command</dt>
              <dd><code>run_completed_recording_transcription</code></dd>
            </div>
            <div>
              <dt>Preferred microphone</dt>
              <dd>{selectedMicrophoneLabel}</dd>
            </div>
            <div>
              <dt>Status polling</dt>
              <dd>{statusPollingLabel}</dd>
            </div>
            <div>
              <dt>Loaded devices</dt>
              <dd>{availableRecordingDevices.length}</dd>
            </div>
          </dl>
        </section>

        <section class="card steps-card">
          <div class="section-heading">
            <div>
              <p class="label">How it works</p>
              <h3>Recording and transcription overview</h3>
            </div>
          </div>

          <ol>
            {#each recordingPlanSteps as step}
              <li>{step}</li>
            {/each}
          </ol>
        </section>
      </div>
    {:else if $activeSection === "history"}
      <div class="view-grid history-grid">
        <section class="card list-card history-toolbar-card">
          <div class="section-heading">
            <div>
              <p class="label">History</p>
              <h3>Local transcript storage</h3>
            </div>
            <span class="status-pill" class:errorState={historyError !== "" && historyState !== "loading"}>
              {historyState === "loading" ? "Loading" : historyEntries.length === 0 ? "Empty" : "Ready"}
            </span>
          </div>
          <p>
            This view shows transcripts saved on this device. Each saved card summarizes clipboard
            and audio outcomes, and the detail panel shows the full transcript plus its saved
            metadata without leaving this screen.
          </p>
          <div class="history-toolbar">
            <p class:pending={historyState === "loading"} class:success={historyState === "ready" && historyError === ""} class:error={historyError !== ""}>
              {historyStatusMessage}
            </p>
            <div class="history-actions">
              <button
                type="button"
                class="ghost-button"
                on:click={() => void loadHistoryEntries()}
                disabled={historyState === "loading" || isClearingHistory}
              >
                {historyState === "loading" ? "Loading…" : "Reload"}
              </button>
              <button
                type="button"
                class="secondary-button"
                on:click={clearAllHistory}
                disabled={historyState === "loading" || isClearingHistory || historyEntries.length === 0 || historyBusyEntryId !== null}
              >
                {isClearingHistory ? "Clearing…" : "Clear all"}
              </button>
            </div>
          </div>
        </section>

        {#if historyState === "loading"}
          <section class="card empty-card">
            <p class="label">History</p>
            <h3>Loading saved transcripts</h3>
            <p>SpeakEx is loading the current list of saved transcripts.</p>
          </section>
        {:else if historyState === "error"}
          <section class="card empty-card">
            <p class="label">History</p>
            <h3>History is unavailable</h3>
            <p>{historyError}</p>
            <button type="button" class="ghost-button" on:click={() => void loadHistoryEntries()}>
              Try again
            </button>
          </section>
        {:else if historyEntries.length === 0}
          <section class="card empty-card">
            <p class="label">History</p>
            <h3>No saved transcripts yet</h3>
<p>Your local history is empty for now. Save a transcript to see it here.</p>
          </section>
        {:else}
          {#if historyError !== ""}
            <section class="card empty-card">
              <p class="label">History</p>
              <h3>Last action failed</h3>
              <p>{historyError}</p>
            </section>
          {/if}

          <section class="card list-card history-detail-card">
            <div class="section-heading">
              <div>
                <p class="label">Selected transcript</p>
                <h3>{historyDetailTitle}</h3>
              </div>
              <span
                class="status-pill"
                class:muted={historyDetailState === "ready" && selectedHistoryStoredIssue === ""}
                class:errorState={historyDetailState === "error" || selectedHistoryStoredIssue !== ""}
              >
                {historyDetailStatusLabel}
              </span>
            </div>

            {#if historyDetailState === "loading"}
              <div class="transcript-placeholder">
                <p>Loading transcript details…</p>
<p>SpeakEx is loading the full saved transcript and its metadata.</p>
              </div>
            {:else if historyDetailState === "error"}
              <div class="current-state-panel phase-error">
                <p class="label">History detail</p>
                <p>{historyDetailError}</p>
              </div>
            {:else if selectedHistoryEntry}
              <p>
                Saved locally on {formatCreatedAt(selectedHistoryEntry.createdAt)}. This panel shows the
                full transcript text plus the clipboard, audio, and warning outcomes stored on that
                history entry.
              </p>

              <dl class="history-meta">
                <div>
                  <dt>Provider</dt>
                  <dd>{selectedHistoryEntry.provider}</dd>
                </div>
                <div>
                  <dt>Model</dt>
                  <dd>{selectedHistoryEntry.model ?? "Default"}</dd>
                </div>
                <div>
                  <dt>Language</dt>
                  <dd>{selectedHistoryEntry.language ?? "Auto / unspecified"}</dd>
                </div>
                <div>
                  <dt>Duration</dt>
                  <dd>{formatDuration(selectedHistoryEntry.durationMs)}</dd>
                </div>
                <div>
                  <dt>Clipboard</dt>
                  <dd>{selectedHistoryClipboardLabel}</dd>
                </div>
                <div>
                  <dt>Audio status</dt>
                  <dd>{selectedHistoryAudioStatusLabel}</dd>
                </div>
                <div>
                  <dt>Audio path</dt>
                  <dd>{selectedHistoryAudioPathLabel}</dd>
                </div>
                <div>
                  <dt>Created</dt>
                  <dd>{formatCreatedAt(selectedHistoryEntry.createdAt)}</dd>
                </div>
                <div>
                  <dt>History ID</dt>
                  <dd>{selectedHistoryEntry.id}</dd>
                </div>
              </dl>

              {#if selectedHistoryStoredIssue !== ""}
                <div class="current-state-panel phase-error">
                  <p class="label">Stored error or warning</p>
                  <p>{selectedHistoryStoredIssue}</p>
                </div>
              {/if}

              <div class="history-transcript">
                <p class="label">Transcript text</p>
                <pre class="history-transcript-body">{selectedHistoryEntry.text.trim() === "" ? "Saved transcript text is empty." : selectedHistoryEntry.text}</pre>
              </div>
            {:else}
              <div class="transcript-placeholder">
                <p>Choose a saved transcript</p>
                <p>Select an item below to inspect its full text and saved outcomes.</p>
              </div>
            {/if}
          </section>

          {#each historyEntries as entry}
            <article class="card history-card" class:selected={selectedHistoryEntryId === entry.id}>
              <div class="section-heading">
                <div>
                  <p class="label">{entry.createdAtLabel}</p>
                  <h3>{entry.title}</h3>
                </div>
                <span class:muted={entry.status === "saved"} class:errorState={entry.status === "attention"} class="status-pill">
                  {entry.statusLabel}
                </span>
              </div>

              <p>{entry.excerpt}</p>

              <dl class="history-meta">
                <div>
                  <dt>Provider</dt>
                  <dd>{entry.providerLabel}</dd>
                </div>
                <div>
                  <dt>Duration</dt>
                  <dd>{entry.durationLabel}</dd>
                </div>
                <div>
                  <dt>Language</dt>
                  <dd>{entry.languageLabel}</dd>
                </div>
                <div>
                  <dt>Clipboard</dt>
                  <dd>{entry.clipboardLabel}</dd>
                </div>
                <div>
                  <dt>Audio file</dt>
                  <dd>{entry.audioLabel}</dd>
                </div>
              </dl>

              <div class="history-card-actions">
                <button
                  type="button"
                  class="ghost-button"
                  aria-pressed={selectedHistoryEntryId === entry.id}
                  on:click={() => void selectHistoryEntry(entry.id)}
                  disabled={isClearingHistory || historyBusyEntryId === entry.id}
                >
                  {selectedHistoryEntryId === entry.id
                    ? historyDetailState === "loading"
                      ? "Loading…"
                      : "Viewing"
                    : "Open details"}
                </button>
                <button
                  type="button"
                  class="ghost-button"
                  on:click={() => removeHistoryEntry(entry.id)}
                  disabled={isClearingHistory || historyBusyEntryId !== null}
                >
                  {historyBusyEntryId === entry.id ? "Deleting…" : "Delete"}
                </button>
              </div>
            </article>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="view-grid placeholder-grid settings-grid">
        <section class="card empty-card">
          <p class="label">Settings</p>
          <h3>Local persistence enabled</h3>
          <p>
            Non-sensitive preferences load on startup with safe defaults, while the Gemini API key
            is managed separately in the OS keychain.
          </p>
          <p class:pending={settingsState === "loading" || settingsState === "saving"} class:success={settingsState === "idle"} class:error={settingsState === "error"}>
            {settingsStatusMessage}
          </p>
        </section>

        <section class="card list-card">
          <p class="label">Provider selection</p>
          <h3>Choose a default transcription provider</h3>
          <div class="provider-grid">
            {#each providerOptions as option}
              <button
                type="button"
                class="provider-button"
                class:active={$providerSelection === option.id}
                aria-pressed={$providerSelection === option.id}
                on:click={() => updateProvider(option.id)}
              >
                <span>{option.label}</span>
                <small>{option.blurb}</small>
                <small>{option.note}</small>
              </button>
            {/each}
          </div>
        </section>

        <section class="card list-card">
          <p class="label">Recording shortcut</p>
          <h3>Manage the global recording shortcut</h3>
          <p>
            Save a shortcut override here, apply it right away, or clear it for the current session.
            When no saved shortcut exists, app startup still tries the default
            <code>Ctrl+Alt+A</code>.
          </p>

          <form class="field-grid" on:submit|preventDefault={submitRecordingShortcut}>
            <label class="field-label" for="recording-shortcut">
              <span>Recording shortcut</span>
              <input
                id="recording-shortcut"
                class="text-field"
                type="text"
                bind:value={recordingShortcutDraft}
                placeholder="Example: Ctrl+Alt+A"
                autocomplete="off"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                disabled={isRecordingShortcutBusy}
              />
            </label>
            <p
              class:pending={recordingShortcutActionState === "loading" || recordingShortcutActionState === "applying" || recordingShortcutActionState === "reapplying" || recordingShortcutActionState === "clearing"}
              class:success={recordingShortcutActionState === "idle" && recordingShortcutStatus?.state === "active"}
              class:error={recordingShortcutActionState === "error" || recordingShortcutStatus?.state === "invalid" || recordingShortcutStatus?.state === "unavailable"}
            >
              {recordingShortcutStatusMessage}
            </p>
            <div class="history-card-actions">
              <button
                type="submit"
                class="ghost-button"
                disabled={isRecordingShortcutBusy || recordingShortcutDraft.trim().length === 0}
              >
                {recordingShortcutPrimaryActionLabel}
              </button>
              <button
                type="button"
                class="ghost-button"
                on:click={reapplySavedRecordingShortcut}
                disabled={isRecordingShortcutBusy || savedRecordingShortcut === null}
              >
                {recordingShortcutReapplyLabel}
              </button>
              <button
                type="button"
                class="ghost-button"
                on:click={clearRecordingShortcutSetting}
                disabled={!canClearRecordingShortcut}
              >
                {recordingShortcutClearLabel}
              </button>
              <button
                type="button"
                class="ghost-button"
                on:click={() => void refreshRecordingShortcutStatus()}
                disabled={isRecordingShortcutBusy}
              >
                {recordingShortcutRefreshLabel}
              </button>
            </div>
          </form>

          <dl class="history-meta draft-meta shortcut-meta">
            <div>
              <dt>Saved value</dt>
              <dd>{savedRecordingShortcutLabel}</dd>
            </div>
            <div>
              <dt>Current runtime value</dt>
              <dd>{currentRecordingShortcutValueLabel}</dd>
            </div>
            <div>
              <dt>Registration state</dt>
              <dd>{recordingShortcutStateLabel}</dd>
            </div>
            <div>
              <dt>Runtime source</dt>
              <dd>{recordingShortcutSourceLabel}</dd>
            </div>
          </dl>
        </section>

        <section class="card list-card">
          <p class="label">Gemini credential</p>
          <h3>Manage the Gemini API key securely</h3>
          <p>
            The saved key is never shown again after save. Enter a new value only when you want to
            add or replace the key in the OS keychain.
          </p>

          <form class="field-grid" on:submit|preventDefault={submitGeminiApiKey}>
            <label class="field-label" for="gemini-api-key">
              <span>Gemini API key</span>
              <input
                id="gemini-api-key"
                class="text-field"
                type="password"
                bind:value={geminiApiKeyDraft}
                placeholder={geminiApiKeyPresence ? "Enter a new key to replace the saved one" : "Paste the Gemini API key"}
                autocomplete="off"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                disabled={isGeminiApiKeyBusy}
              />
            </label>
            <p
              class:pending={geminiApiKeyActionState === "checking" || geminiApiKeyActionState === "saving" || geminiApiKeyActionState === "clearing" || geminiApiKeyPresenceState === "loading"}
              class:success={geminiApiKeyActionState === "idle" && geminiApiKeyPresenceState !== "error"}
              class:error={geminiApiKeyPresenceState === "error" || geminiApiKeyActionState === "error"}
            >
              {geminiApiKeyStatusMessage}
            </p>
            <div class="history-card-actions">
              <button type="submit" class="ghost-button" disabled={isGeminiApiKeyBusy || geminiApiKeyDraftValue.length === 0}>
                {geminiApiKeyPrimaryActionLabel}
              </button>
              <button type="button" class="ghost-button" on:click={removeGeminiApiKey} disabled={isGeminiApiKeyBusy || !geminiApiKeyPresence}>
                {geminiApiKeyActionState === "clearing" ? "Clearing…" : "Clear saved key"}
              </button>
              <button type="button" class="ghost-button" on:click={() => void refreshGeminiApiKeyPresence()} disabled={isGeminiApiKeyBusy}>
                {geminiApiKeyActionState === "checking" ? "Checking…" : "Check key status"}
              </button>
            </div>
          </form>
        </section>

        <section class="card list-card">
          <p class="label">Defaults</p>
          <h3>Saved preferences</h3>

          <div class="field-grid">
            <label class="field-label" for="default-language">
              <span>Default language</span>
              <select id="default-language" class="select-field" value={$settingsDraft.defaultLanguage} on:change={updateLanguage}>
                {#each languageOptions as option}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            </label>

            <label class="field-label" for="selected-microphone">
              <span>Preferred microphone</span>
              <select id="selected-microphone" class="select-field" value={$settingsDraft.selectedMicrophone} on:change={updateMicrophone}>
                {#each $recordingInputOptions as option}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            </label>
          </div>

          <p class:pending={recordingDevicesState === "loading"} class:success={recordingDevicesState === "ready"} class:error={recordingDevicesState === "error"}>
            {recordingDevicesStatusMessage}
          </p>
          <div class="history-card-actions">
            <button
              type="button"
              class="ghost-button"
              on:click={loadRecordingDevices}
              disabled={recordingDevicesState === "loading" || activeRecordingSession !== null}
            >
              {recordingDevicesState === "loading" ? "Loading inputs…" : "Reload microphones"}
            </button>
          </div>

          <div class="setting-row">
            <div>
              <h4>Auto-copy transcript</h4>
              <p>When enabled, manual Gemini transcription copies successful transcripts automatically.</p>
            </div>
            <button type="button" class="toggle-button" class:active={$settingsDraft.autoCopy} aria-pressed={$settingsDraft.autoCopy} on:click={() => toggleSetting("autoCopy")}>
              {$settingsDraft.autoCopy ? "On" : "Off"}
            </button>
          </div>

          <div class="setting-row">
            <div>
              <h4>Save transcription history</h4>
              <p>When enabled, manual Gemini transcription saves transcripts to local history.</p>
            </div>
            <button
              type="button"
              class="toggle-button"
              class:active={$settingsDraft.saveTranscriptionHistory}
              aria-pressed={$settingsDraft.saveTranscriptionHistory}
              on:click={() => toggleSetting("saveTranscriptionHistory")}
            >
              {$settingsDraft.saveTranscriptionHistory ? "On" : "Off"}
            </button>
          </div>

          <div class="setting-row">
            <div>
              <h4>Save audio files</h4>
              <p>When disabled, the manual recording flow deletes the completed audio file after transcription.</p>
            </div>
            <button type="button" class="toggle-button" class:active={$settingsDraft.saveAudioFiles} aria-pressed={$settingsDraft.saveAudioFiles} on:click={() => toggleSetting("saveAudioFiles")}>
              {$settingsDraft.saveAudioFiles ? "On" : "Off"}
            </button>
          </div>
        </section>

        <section class="card list-card">
          <p class="label">Persisted snapshot</p>
          <h3>Current settings values</h3>
          <dl class="history-meta draft-meta">
            <div>
              <dt>Provider</dt>
              <dd>{selectedProviderLabel}</dd>
            </div>
            <div>
              <dt>Language</dt>
              <dd>{$settingsDraft.defaultLanguage === "auto" ? "Auto-detect" : $settingsDraft.defaultLanguage}</dd>
            </div>
            <div>
              <dt>Microphone</dt>
              <dd>{selectedMicrophoneLabel}</dd>
            </div>
            <div>
              <dt>Recording shortcut</dt>
              <dd>{savedRecordingShortcutLabel}</dd>
            </div>
            <div>
              <dt>Shortcut status</dt>
              <dd>{recordingShortcutStateLabel}</dd>
            </div>
            <div>
              <dt>Gemini API key</dt>
              <dd>{geminiApiKeySavedLabel}</dd>
            </div>
            <div>
              <dt>Auto-copy</dt>
              <dd>{$settingsDraft.autoCopy ? "Enabled" : "Disabled"}</dd>
            </div>
            <div>
              <dt>History</dt>
              <dd>{$settingsDraft.saveTranscriptionHistory ? "Enabled" : "Disabled"}</dd>
            </div>
            <div>
              <dt>Audio files</dt>
              <dd>{$settingsDraft.saveAudioFiles ? "Enabled" : "Disabled"}</dd>
            </div>
          </dl>
        </section>
      </div>
    {/if}
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    min-width: 320px;
    min-height: 100vh;
    font-family:
      Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    background:
      radial-gradient(circle at top left, rgba(59, 130, 246, 0.18), transparent 28%),
      radial-gradient(circle at top right, rgba(45, 212, 191, 0.12), transparent 32%),
      linear-gradient(180deg, #020617 0%, #0f172a 52%, #111827 100%);
    color: #e2e8f0;
  }

  :global(button) {
    font: inherit;
  }

  :global(select) {
    font: inherit;
  }

  .app-shell {
    min-height: 100vh;
    display: grid;
    grid-template-columns: minmax(260px, 320px) minmax(0, 1fr);
    gap: 1.5rem;
    padding: 1.5rem;
    box-sizing: border-box;
  }

  .sidebar,
  .workspace,
  .card {
    border: 1px solid rgba(148, 163, 184, 0.14);
    background: rgba(15, 23, 42, 0.78);
    box-shadow: 0 24px 60px rgba(2, 6, 23, 0.3);
    backdrop-filter: blur(18px);
  }

  .sidebar,
  .workspace {
    border-radius: 28px;
  }

  .sidebar {
    display: grid;
    grid-template-rows: auto auto 1fr;
    gap: 1.5rem;
    padding: 1.5rem;
  }

  .brand-block h1,
  .workspace-header h2,
  .section-heading h3,
  .empty-card h3,
  .bridge-card h2,
  .hero-card h3,
  .list-card h3,
  .setting-row h4 {
    margin: 0;
  }

  .eyebrow,
  .label {
    margin: 0 0 0.65rem;
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #7dd3fc;
  }

  .brand-copy,
  .workspace-copy,
  .hero-card p,
  .empty-card p,
  .list-card p,
  .transcript-placeholder p,
  .bridge-status,
  .setting-row p,
  ol,
  code,
  small {
    color: #cbd5e1;
  }

  .brand-copy,
  .workspace-copy,
  .hero-card p,
  .empty-card p,
  .list-card p,
  .transcript-placeholder p,
  .bridge-status,
  .setting-row p {
    margin: 0;
    line-height: 1.7;
  }

  .section-nav {
    display: grid;
    gap: 0.85rem;
  }

  .section-nav button,
  .provider-button {
    display: grid;
    gap: 0.2rem;
    width: 100%;
    padding: 1rem 1.05rem;
    border: 1px solid rgba(148, 163, 184, 0.12);
    border-radius: 18px;
    background: rgba(15, 23, 42, 0.42);
    color: #f8fafc;
    text-align: left;
    cursor: pointer;
    transition:
      transform 0.15s ease,
      border-color 0.15s ease,
      background 0.15s ease;
  }

  .section-nav button:hover,
  .section-nav button.active,
  .provider-button:hover,
  .provider-button.active {
    transform: translateY(-1px);
    border-color: rgba(125, 211, 252, 0.5);
    background: rgba(30, 41, 59, 0.9);
  }

  .section-nav span,
  .provider-button span {
    font-size: 1rem;
    font-weight: 700;
  }

  .section-nav small,
  .provider-button small {
    font-size: 0.9rem;
  }

  .bridge-card {
    align-self: end;
    display: grid;
    gap: 1rem;
    padding: 1.2rem;
    border-radius: 22px;
    background: rgba(15, 23, 42, 0.65);
    border: 1px solid rgba(148, 163, 184, 0.12);
  }

  .ghost-button,
  .secondary-button,
  .primary-button,
  .toggle-button {
    border-radius: 999px;
    padding: 0.85rem 1.15rem;
    font-weight: 700;
    transition:
      transform 0.15s ease,
      opacity 0.15s ease,
      border-color 0.15s ease;
  }

  .ghost-button,
  .secondary-button,
  .toggle-button {
    border: 1px solid rgba(148, 163, 184, 0.2);
    color: #e2e8f0;
    background: rgba(15, 23, 42, 0.7);
  }

  .toggle-button.active {
    border-color: rgba(34, 197, 94, 0.45);
    background: rgba(21, 128, 61, 0.22);
    color: #dcfce7;
  }

  .primary-button {
    border: 0;
    color: #0f172a;
    background: linear-gradient(135deg, #7dd3fc 0%, #22d3ee 100%);
    box-shadow: 0 12px 28px rgba(34, 211, 238, 0.22);
  }

  .ghost-button:disabled,
  .secondary-button:disabled,
  .primary-button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .workspace {
    display: grid;
    gap: 1.5rem;
    padding: 1.5rem;
  }

  .workspace-header {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    justify-content: space-between;
    gap: 1rem;
  }

  .workspace-header h2 {
    font-size: clamp(2rem, 4vw, 3rem);
    line-height: 1.05;
  }

  .workspace-copy {
    max-width: 32rem;
  }

  .view-grid {
    display: grid;
    gap: 1rem;
  }

  .recording-grid {
    grid-template-columns: repeat(12, minmax(0, 1fr));
  }

  .history-grid {
    grid-template-columns: repeat(12, minmax(0, 1fr));
    align-items: start;
  }

  .placeholder-grid {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  .settings-grid {
    align-items: start;
  }

  .card {
    border-radius: 24px;
    padding: 1.35rem;
  }

  .hero-card {
    grid-column: span 7;
    display: grid;
    gap: 1.5rem;
    align-content: space-between;
    min-height: 250px;
    background:
      linear-gradient(135deg, rgba(14, 165, 233, 0.18), transparent 60%),
      rgba(15, 23, 42, 0.78);
  }

  .provider-caption {
    color: #e2e8f0;
  }

  .phase-note {
    font-size: 0.95rem;
    color: #93c5fd;
  }

  .hero-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .history-toolbar,
  .history-actions,
  .history-card-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .history-toolbar {
    align-items: center;
    justify-content: space-between;
  }

  .history-actions,
  .history-card-actions {
    justify-content: flex-end;
  }

  .history-toolbar-card,
  .history-detail-card,
  .history-grid > .empty-card {
    grid-column: 1 / -1;
  }

  .transcript-card {
    grid-column: span 5;
    display: grid;
    gap: 1.25rem;
  }

  .meta-card,
  .controller-card,
  .steps-card {
    grid-column: span 6;
  }

  .section-heading {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 1rem;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.35rem 0.7rem;
    border-radius: 999px;
    background: rgba(59, 130, 246, 0.16);
    color: #bfdbfe;
    font-size: 0.85rem;
    font-weight: 700;
  }

  .status-pill.muted {
    background: rgba(148, 163, 184, 0.12);
    color: #cbd5e1;
  }

  .status-pill.phase-idle {
    background: rgba(59, 130, 246, 0.16);
    color: #bfdbfe;
  }

  .status-pill.phase-recording {
    background: rgba(249, 115, 22, 0.16);
    color: #fdba74;
  }

  .status-pill.phase-transcribing {
    background: rgba(168, 85, 247, 0.16);
    color: #d8b4fe;
  }

  .status-pill.phase-completed {
    background: rgba(34, 197, 94, 0.16);
    color: #bbf7d0;
  }

  .status-pill.phase-error,
  .status-pill.errorState {
    background: rgba(239, 68, 68, 0.14);
    color: #fecaca;
  }

  .transcript-placeholder {
    display: grid;
    place-items: center;
    min-height: 185px;
    padding: 1.25rem;
    border: 1px dashed rgba(148, 163, 184, 0.24);
    border-radius: 20px;
    background: rgba(15, 23, 42, 0.45);
    text-align: center;
  }

  .stats-grid,
  .history-meta {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
    margin: 0;
  }

  .stats-grid div,
  .history-meta div {
    padding: 1rem;
    border-radius: 18px;
    background: rgba(30, 41, 59, 0.62);
    border: 1px solid rgba(148, 163, 184, 0.12);
  }

  dt {
    margin: 0 0 0.35rem;
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #94a3b8;
  }

  dd {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 700;
    color: #f8fafc;
  }

  ol {
    display: grid;
    gap: 0.9rem;
    padding-left: 1.2rem;
    margin: 0;
    line-height: 1.6;
  }

  .empty-card {
    min-height: 220px;
    display: grid;
    align-content: center;
    gap: 0.75rem;
  }

  .list-card,
  .history-card {
    display: grid;
    align-content: start;
    gap: 0.85rem;
  }

  .history-card {
    grid-column: span 6;
  }

  .history-card.selected {
    border: 1px solid rgba(125, 211, 252, 0.38);
    background: rgba(30, 64, 175, 0.18);
  }

  .history-detail-card dd {
    word-break: break-word;
  }

  .history-transcript {
    display: grid;
    gap: 0.75rem;
  }

  .history-transcript-body {
    margin: 0;
    padding: 1rem;
    border-radius: 20px;
    border: 1px solid rgba(148, 163, 184, 0.12);
    background: rgba(15, 23, 42, 0.72);
    color: #f8fafc;
    font: inherit;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
    overflow: auto;
    max-height: 420px;
  }

  .controller-card {
    gap: 1rem;
  }

  .current-state-panel {
    display: grid;
    gap: 0.6rem;
    padding: 1rem;
    border-radius: 20px;
    border: 1px solid rgba(148, 163, 184, 0.12);
    background: rgba(30, 41, 59, 0.62);
  }

  .current-state-panel.phase-idle {
    border-color: rgba(59, 130, 246, 0.26);
    background: rgba(30, 64, 175, 0.16);
  }

  .current-state-panel.phase-recording {
    border-color: rgba(249, 115, 22, 0.28);
    background: rgba(154, 52, 18, 0.18);
  }

  .current-state-panel.phase-transcribing {
    border-color: rgba(168, 85, 247, 0.28);
    background: rgba(107, 33, 168, 0.18);
  }

  .current-state-panel.phase-completed {
    border-color: rgba(34, 197, 94, 0.28);
    background: rgba(20, 83, 45, 0.22);
  }

  .current-state-panel.phase-error {
    border-color: rgba(239, 68, 68, 0.28);
    background: rgba(127, 29, 29, 0.22);
  }

  .current-state-panel h3 {
    margin: 0;
  }

  .current-state-panel p {
    margin: 0;
  }

  .warning-list {
    margin: 0;
    padding-left: 1.2rem;
    display: grid;
    gap: 0.5rem;
  }

  .provider-grid,
  .field-grid {
    display: grid;
    gap: 0.85rem;
  }

  .field-label {
    display: grid;
    gap: 0.5rem;
    font-weight: 600;
    color: #e2e8f0;
  }

  .select-field,
  .text-field {
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 14px;
    padding: 0.85rem 1rem;
    color: #f8fafc;
    background: rgba(15, 23, 42, 0.7);
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 1rem 0;
    border-top: 1px solid rgba(148, 163, 184, 0.12);
  }

  .draft-meta {
    grid-template-columns: 1fr;
  }

  .shortcut-meta dd {
    word-break: break-word;
  }

  .pending {
    color: #f8fafc;
  }

  .success {
    color: #d1fae5;
  }

  .error {
    color: #fecaca;
  }

  code {
    padding: 0.2rem 0.45rem;
    border-radius: 999px;
    background: rgba(15, 23, 42, 0.9);
    font-size: 0.95em;
  }

  strong {
    color: #f8fafc;
  }

  @media (max-width: 1024px) {
    .app-shell {
      grid-template-columns: 1fr;
    }

    .sidebar {
      grid-template-rows: auto auto auto;
    }

    .bridge-card {
      align-self: auto;
    }
  }

  @media (max-width: 900px) {
    .recording-grid {
      grid-template-columns: 1fr;
    }

    .history-grid {
      grid-template-columns: 1fr;
    }

    .hero-card,
    .transcript-card,
    .history-card,
    .meta-card,
    .steps-card {
      grid-column: span 1;
    }
  }

  @media (max-width: 640px) {
    .app-shell {
      padding: 1rem;
      gap: 1rem;
    }

    .sidebar,
    .workspace,
    .card {
      border-radius: 22px;
    }

    .sidebar,
    .workspace {
      padding: 1rem;
    }

    .stats-grid,
    .history-meta,
    .setting-row {
      grid-template-columns: 1fr;
    }

    .setting-row {
      display: grid;
    }

    .history-toolbar {
      align-items: stretch;
    }
  }
</style>
