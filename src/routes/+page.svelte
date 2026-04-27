<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";

  import {
    clearHistory,
    deleteTranscription,
    getHistory,
    type HistoryTranscriptionSummary
  } from "$lib/native/history";
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
  type RecordingDevicesState = "loading" | "ready" | "error";
  type RecordingCommandState = "starting" | "stopping" | "cancelling" | null;
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

  const providerLabels = new Map(providerOptions.map((provider) => [provider.id, provider.label]));
  const fallbackRecordingLimitMs = 15 * 60 * 1000;
  const recordingStatusPollIntervalMs = 1000;

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
  let historyState: HistoryState = "loading";
  let historyEntries: HistoryEntryViewModel[] = [];
  let historyError = "";
  let historyBusyEntryId: string | null = null;
  let isClearingHistory = false;
  let activeTranscriptionCommand: TranscriptionCommandState = null;
  let latestTranscript: Transcript | null = null;
  let latestManualTranscriptionResult: RunCompletedRecordingTranscriptionResult | null = null;
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
  $: primaryManualActionLabel = isRunningManualTranscription ? "Transcribing…" : "Transcribe recording";
  $: recordingDevicesStatusMessage =
    recordingDevicesState === "loading"
      ? "Loading available microphones from Rust…"
      : recordingDevicesState === "error"
        ? recordingDevicesError
        : availableRecordingDevices.length === 0
          ? "No recording inputs were reported by Rust."
          : `${availableRecordingDevices.length} microphone${availableRecordingDevices.length === 1 ? "" : "s"} available from Rust.`;
  $: mockHistoryModeLabel = $settingsDraft.saveTranscriptionHistory
    ? "Mock transcripts will still be written to local history."
    : "Mock transcripts will stay out of local history.";
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
  $: historyStatusMessage =
    historyState === "loading"
      ? "Loading transcript history from the local database…"
      : historyState === "error" || historyError !== ""
        ? historyError
        : historyEntries.length === 0
          ? "No saved transcripts yet."
          : `${historyEntries.length} saved transcript${historyEntries.length === 1 ? "" : "s"} loaded locally.`;
  $: historyCountLabel =
    historyState === "loading"
      ? "Loading…"
      : historyState === "error"
        ? "Unavailable"
        : `${historyEntries.length} saved item${historyEntries.length === 1 ? "" : "s"}`;
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
        $settingsDraft.saveTranscriptionHistory
      )
    : null;
  $: manualClipboardLabel = latestManualTranscriptionResult
    ? describeManualClipboardOutcome(latestManualTranscriptionResult, $settingsDraft.autoCopy)
    : null;
  $: manualAudioLabel = latestManualTranscriptionResult
    ? describeManualAudioOutcome(latestManualTranscriptionResult, $settingsDraft.saveAudioFiles)
    : null;
  $: manualTranscriptionStatusMessage =
    transcribableRecordedAudio === null
      ? latestManualTranscriptionResult?.audioDeleted && !latestManualTranscriptionResult.audioDeleteError
        ? "The last completed recording was transcribed and deleted using the saved cleanup setting. Record again to transcribe new audio."
        : "Complete a local recording first, then run transcription manually from this screen."
      : geminiApiKeyPresenceState === "loading" || geminiApiKeyActionState === "checking"
        ? "Checking the OS keychain before enabling Gemini transcription…"
        : geminiApiKeyPresenceState === "error"
          ? "Unable to verify the Gemini API key right now. Recheck key status in Settings before running Gemini."
          : !geminiApiKeyPresence
            ? "Save a Gemini API key in Settings before running Gemini on the current recording."
            : isRunningManualTranscription
              ? "Gemini is transcribing the current local recording in Rust. Clipboard, history, and audio cleanup follow the saved settings."
              : "Gemini can transcribe the current completed local recording on demand through the full Rust manual flow.";
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
    await Promise.all([hydrateSettings(), refreshGeminiApiKeyPresence(false)]);
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
      settingsState = "idle";
    } catch (error) {
      settingsError = error instanceof Error ? error.message : "Unable to load saved preferences";
      settingsState = "error";
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

  async function persistSettings(nextSettings: SettingsDraft) {
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
      if (requestId !== latestSettingsRequest) {
        return;
      }

      settingsError = error instanceof Error ? error.message : "Unable to save preferences";
      settingsState = "error";

      if (lastSavedSettings) {
        settingsDraft.patch(lastSavedSettings);
        recordingInputOptions.set(
          createRecordingInputOptions(availableRecordingDevices, lastSavedSettings.selectedMicrophone)
        );
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
              "The UI could not read the explicit recorder status surface. No transcription ran automatically.",
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
          ? `Capture stopped automatically after reaching the ${formatDuration(status.maxDurationMs)} maximum. Finalizing the local WAV file now.`
          : "The recorder has stopped. Finalizing the local WAV file now.",
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

  async function loadHistoryEntries() {
    historyState = "loading";
    historyError = "";

    try {
      historyEntries = (await getHistory()).map(mapHistoryEntry);
      historyState = "ready";
    } catch (error) {
      historyEntries = [];
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

      historyEntries = historyEntries.filter((entry) => entry.id !== id);
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
    latestManualTranscriptionResult = null;
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
      latestManualTranscriptionResult = null;
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
      latestManualTranscriptionResult = null;
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
    latestManualTranscriptionResult = null;
    latestCompletedRecordingMetadata = previousRecordedAudio;

    appStatus.setStatus(
      getAppStatusForPhase("transcribing", {
        inputLabel: selectedMicrophoneLabel,
        detail:
          "The frontend is waiting on the explicit Rust mock transcription command. Any recorded WAV file remains separate from this fake flow.",
        recordingTiming: previousRecordedAudio ? buildRecordingTimingFromRecordedAudio(previousRecordedAudio) : null,
        recordedAudio: previousRecordedAudio
      })
    );

    try {
      const result = await runMockTranscription();

      latestTranscript = result.transcript;
      appStatus.setStatus(buildCompletedMockStatus(result, previousRecordedAudio));

      if (result.savedToHistory) {
        await loadHistoryEntries();
      }
    } catch (error) {
      latestTranscript = null;
      appStatus.setStatus(
        getAppStatusForPhase("error", {
          inputLabel: selectedMicrophoneLabel,
          detail: error instanceof Error ? error.message : "Unable to finish the mock transcription flow.",
          transcriptPreview:
            "The built-in mock transcription command did not finish. Any recorded audio remains local and separate from this fake provider path.",
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

    activeTranscriptionCommand = "manual";
    latestTranscript = null;
    latestManualTranscriptionResult = null;

    appStatus.setStatus(
      getAppStatusForPhase("transcribing", {
        headline: "Manual transcription is running.",
        inputLabel: transcribableRecordedAudio.inputDeviceName,
        detail:
          "The frontend is waiting on the explicit Rust manual transcription command for the current completed WAV file. Rust will also handle clipboard, history, and default audio cleanup.",
        transcriptTitle: "Transcript incoming…",
        transcriptPreview: `${formatFileName(transcribableRecordedAudio.path)} is being transcribed through the explicit Rust manual flow.`,
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
      latestCompletedRecordingMetadata = transcribableRecordedAudio;
      appStatus.setStatus(buildCompletedManualStatus(result, transcribableRecordedAudio, retainedRecordedAudio));

      if (result.historySaved) {
        await loadHistoryEntries();
      }
    } catch (error) {
      latestTranscript = null;
      appStatus.setStatus(
        getAppStatusForPhase("error", {
          inputLabel: transcribableRecordedAudio.inputDeviceName,
          detail: error instanceof Error ? error.message : "Unable to finish the manual transcription flow.",
          transcriptPreview:
            "The explicit manual transcription command did not finish. The current recording remains local and no follow-up side effects were applied.",
          durationLabel: formatDuration(transcribableRecordedAudio.durationMs),
          recordingTiming: buildRecordingTimingFromRecordedAudio(transcribableRecordedAudio),
          recordedAudio: transcribableRecordedAudio
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
    latestManualTranscriptionResult = null;
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
              ? "Choose an available microphone before starting a real recording. The saved selection is preserved so you can update it explicitly."
              : "Start a recording to capture a temporary WAV file locally, then run either the mock path or the full manual transcription flow.",
        inputLabel: selectedMicrophoneLabel,
        durationLabel: "—",
        recordingTiming: null,
        recordedAudio: null
      })
    );
  }

  function buildIdleDetail() {
    if (recordingDevicesState === "loading") {
      return "Loading available microphones from Rust before the recording workflow becomes ready.";
    }

    if (recordingDevicesState === "error") {
      return `Unable to load recording inputs: ${recordingDevicesError}`;
    }

    if (selectedMicrophoneUnavailable) {
      return "The saved microphone is not currently available. Pick one of the loaded inputs before starting a real recording.";
    }

    return `Ready to record from ${selectedMicrophoneLabel}. Stop keeps the temporary WAV file and transcription remains a separate manual step.`;
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
      detail: `Audio capture is active through Rust and will stop automatically at ${formatDuration(status.maxDurationMs)}. Use Stop to keep the temporary WAV file or Cancel to discard it sooner.`,
      transcriptTitle: "Live capture in progress…",
      transcriptPreview: `Recording session ${status.activeSessionId ?? "current"} is writing a temporary WAV file in the app cache. No transcription will run automatically, including when the duration limit is reached.`,
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
        ? `Capture stopped automatically because the maximum recording duration of ${formatDuration(recordingTiming.maxDurationMs)} was reached. The WAV file was kept locally, and no transcription ran automatically.`
        : "The recorder stopped successfully and kept the temporary WAV file locally. Choose mock or the full manual transcription flow when you are ready to transcribe.",
      transcriptTitle: "Recorded audio metadata",
      transcriptPreview: stoppedRecording.limitReached
        ? `${formatFileName(recordedAudio.path)} was captured locally after the automatic safety stop. Run mock or the manual transcription flow when you are ready.`
        : `${formatFileName(recordedAudio.path)} is available locally and ready for explicit mock or manual transcription.`,
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
    const historyDetail = result.savedToHistory
      ? "Saved to the local SQLite history because the persisted setting enables history."
      : "Not saved to SQLite because the persisted setting disables transcription history.";

    return getAppStatusForPhase("completed", {
      headline: "Mock transcript ready.",
      detail: `${historyDetail} The result remains intentionally fake and separate from the full manual Gemini flow in Release 1.0.`,
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
      detail: outcomeSummary,
      transcriptTitle: result.historySaved ? "Transcript saved locally" : "Transcript ready in preview",
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

  function collectOutcomeWarnings(
    result: RunCompletedRecordingTranscriptionResult
  ): string[] {
    return [result.historyError, result.clipboardError, result.audioDeleteError].filter(
      (value): value is string => value !== null && value.trim() !== ""
    );
  }

  function describeManualHistoryOutcome(
    result: RunCompletedRecordingTranscriptionResult,
    saveTranscriptionHistory: boolean
  ): string {
    if (result.historySaved) {
      return "Saved to the local SQLite history.";
    }

    if (result.historyError) {
      return "History saving reported a warning.";
    }

    return saveTranscriptionHistory
      ? "History save was skipped."
      : "History save was skipped by the saved setting.";
  }

  function describeManualClipboardOutcome(
    result: RunCompletedRecordingTranscriptionResult,
    autoCopy: boolean
  ): string {
    if (result.copiedToClipboard) {
      return "Copied to the system clipboard.";
    }

    if (result.clipboardError) {
      return "Clipboard copy reported a warning.";
    }

    return autoCopy
      ? "Clipboard copy did not complete."
      : "Clipboard copy was skipped by the saved setting.";
  }

  function describeManualAudioOutcome(
    result: RunCompletedRecordingTranscriptionResult,
    saveAudioFiles: boolean
  ): string {
    if (result.audioDeleted && !result.audioDeleteError) {
      return "The local audio file was deleted after transcription.";
    }

    if (result.audioDeleteError) {
      return "Audio cleanup reported a warning and the file was retained locally.";
    }

    if (result.retainedAudioPath) {
      return saveAudioFiles
        ? "The local audio file was retained by the saved setting."
        : "The local audio file was retained locally.";
    }

    return "The local audio file state is unchanged.";
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
      clipboardLabel: entry.copiedToClipboard ? "Copied" : "Not copied",
      audioLabel: entry.hasAudioFile ? "Saved" : "None",
      status: entry.hasError ? "attention" : "saved",
      statusLabel: entry.hasError ? "Stored with error" : "Stored"
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
  <title>SpeakEx — Audio Recording</title>
  <meta
    name="description"
    content="SpeakEx desktop app shell with real audio recording, manual transcription flow, history, and settings views."
  />
</svelte:head>

<main class="app-shell">
  <aside class="sidebar">
    <div class="brand-block">
      <p class="eyebrow">Release 1.0</p>
      <h1>SpeakEx</h1>
      <p class="brand-copy">
        Local-first transcription for the desktop. This release keeps recording explicit while the
        Rust manual flow handles Gemini, clipboard, history, and default audio cleanup.
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
        <p class="label">Native bridge</p>
        <h2>Rust ping status</h2>
      </div>

      {#if pingState === "loading" || pingState === "idle"}
        <p class="bridge-status pending">Checking the existing <code>ping</code> command…</p>
      {:else if pingState === "success"}
        <p class="bridge-status success">Connected: <strong>{pingResponse}</strong></p>
      {:else}
        <p class="bridge-status error">Unavailable: {pingError}</p>
      {/if}

      <button type="button" class="ghost-button" on:click={runPing} disabled={pingState === "loading"}>
        {pingState === "loading" ? "Checking…" : "Recheck bridge"}
      </button>
    </section>
  </aside>

  <section class="workspace">
    <header class="workspace-header">
      <div>
        <p class="eyebrow">App shell</p>
        <h2>
          {#if $activeSection === "recording"}
            Recording workspace
          {:else if $activeSection === "history"}
            Transcript history
          {:else}
            Settings draft
          {/if}
        </h2>
      </div>
        <p class="workspace-copy">
          {#if $activeSection === "recording"}
          The recording workspace now polls explicit recorder status, keeps mock transcription
          separate, and runs the full manual MVP flow for completed local recordings.
        {:else if $activeSection === "history"}
          Saved transcript history loads from the local database. The mock path and the manual
          recording flow both write here when history is enabled.
        {:else}
          Provider selection and preferences still hydrate from local storage, and the preferred
          microphone now reflects the real device list exposed by Rust.
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
            <p class="provider-caption">Current draft provider: <strong>{selectedProviderLabel}</strong></p>
            <p class:pending={recordingDevicesState === "loading"} class:success={recordingDevicesState === "ready"} class:error={recordingDevicesState === "error"}>
              {recordingDevicesStatusMessage}
            </p>
            <p class="phase-note">Release 1.0 keeps recording explicit. Stop still only creates a completed local recording until you transcribe manually.</p>
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
            <button
              type="button"
              class="secondary-button"
              on:click={startManualTranscription}
              disabled={!canRunManualTranscription}
            >
              {primaryManualActionLabel}
            </button>
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
            class:pending={(transcribableRecordedAudio === null && latestManualTranscriptionResult === null) || isRunningManualTranscription || geminiApiKeyPresenceState === "loading" || geminiApiKeyActionState === "checking"}
            class:success={latestManualTranscriptionResult !== null || (transcribableRecordedAudio !== null && geminiApiKeyPresence && !isRunningManualTranscription && geminiApiKeyPresenceState !== "error")}
            class:error={latestManualTranscriptionResult === null && ((transcribableRecordedAudio !== null && (!geminiApiKeyPresence || geminiApiKeyPresenceState === "error")) || geminiApiKeyActionState === "error")}
          >
            {manualTranscriptionStatusMessage}
          </p>
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
                <dd>{latestTranscript.provider === "gemini" && manualHistoryLabel ? manualHistoryLabel : $settingsDraft.saveTranscriptionHistory ? "Saved if enabled" : "Not saved"}</dd>
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
              <p class="label">Recorder bridge</p>
              <h3>Current native state</h3>
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
              The UI currently shows the <strong>{$appStatus.phase}</strong> phase while the frontend
              coordinates explicit <code>start_recording</code>, <code>get_recording_status</code>,
              <code>stop_recording</code>, <code>cancel_recording</code>, and
              <code>run_completed_recording_transcription</code> commands.
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
              <p class="label">Planned flow</p>
              <h3>What this shell prepares for</h3>
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
      <div class="view-grid placeholder-grid">
        <section class="card list-card">
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
            This view reads saved transcripts from SQLite through explicit native commands. In Release
            1.0, the separate mock transcription flow and the manual recording flow can both write
            entries here when history is enabled.
          </p>
          <div class="history-toolbar">
            <p class:pending={historyState === "loading"} class:success={historyState === "ready" && historyError === ""} class:error={historyError !== ""}>
              {historyStatusMessage}
            </p>
            <div class="history-actions">
              <button type="button" class="ghost-button" on:click={loadHistoryEntries} disabled={historyState === "loading" || isClearingHistory}>
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
            <p>The app is requesting the current transcript list from the local SQLite history store.</p>
          </section>
        {:else if historyState === "error"}
          <section class="card empty-card">
            <p class="label">History</p>
            <h3>History is unavailable</h3>
            <p>{historyError}</p>
            <button type="button" class="ghost-button" on:click={loadHistoryEntries}>Try again</button>
          </section>
        {:else if historyEntries.length === 0}
          <section class="card empty-card">
            <p class="label">History</p>
            <h3>No saved transcripts yet</h3>
            <p>
              Local history storage is ready, but the database does not contain any transcripts yet.
            </p>
          </section>
        {:else}
          {#if historyError !== ""}
            <section class="card empty-card">
              <p class="label">History</p>
              <h3>Last action failed</h3>
              <p>{historyError}</p>
            </section>
          {/if}

          {#each historyEntries as entry}
            <article class="card history-card">
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
              <p>When enabled, the manual recording flow copies successful transcripts through Rust.</p>
            </div>
            <button type="button" class="toggle-button" class:active={$settingsDraft.autoCopy} aria-pressed={$settingsDraft.autoCopy} on:click={() => toggleSetting("autoCopy")}>
              {$settingsDraft.autoCopy ? "On" : "Off"}
            </button>
          </div>

          <div class="setting-row">
            <div>
              <h4>Save transcription history</h4>
              <p>When enabled, the manual recording flow saves transcripts to the local SQLite history.</p>
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

    .hero-card,
    .transcript-card,
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
