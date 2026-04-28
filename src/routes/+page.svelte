<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";

  import Sidebar from "$lib/components/app-shell/Sidebar.svelte";
  import TopBar from "$lib/components/app-shell/TopBar.svelte";
  import HistorySection from "$lib/components/home/HistorySection.svelte";
  import RecordingSection from "$lib/components/home/RecordingSection.svelte";
  import SettingsSection from "$lib/components/home/SettingsSection.svelte";
  import type { HistoryEntryViewModel } from "$lib/features/history/types";
  import {
    createRecordingController,
    type LatestManualOutcomeSettings
  } from "$lib/features/recording/controller";
  import {
    collectOutcomeWarnings,
    describeManualAudioOutcome,
    describeManualClipboardOutcome,
    describeManualHistoryOutcome,
    describeStoredAudioOutcome,
    describeStoredClipboardOutcome
  } from "$lib/features/recording/status";
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
    type ActiveRecordingSession,
    type RecordingInputDevice,
    type RecordingStatus
  } from "$lib/native/recording";
  import {
    type RunCompletedRecordingTranscriptionResult,
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
    AppSection,
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
  type TranscriptionCommandState = "manual" | null;

  const providerLabels = new Map(providerOptions.map((provider) => [provider.id, provider.label]));
  const fallbackRecordingLimitMs = 15 * 60 * 1000;
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
  let latestRecordingStatus: RecordingStatus | null = null;

  $: selectedProviderLabel = providerLabels.get($providerSelection) ?? "Unknown provider";
  $: selectedMicrophoneOption =
    $recordingInputOptions.find((option) => option.value === $settingsDraft.selectedMicrophone) ??
    defaultRecordingInputOption;
  $: selectedMicrophoneLabel = selectedMicrophoneOption.label;
  $: selectedMicrophoneUnavailable = selectedMicrophoneOption.unavailable ?? false;
  $: isRunningManualTranscription = activeTranscriptionCommand === "manual";
  $: isRunningTranscription = activeTranscriptionCommand !== null;
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
  $: latestTranscriptHistoryLabel = manualHistoryLabel;
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
    await recordingController.loadRecordingDevices();
    await recordingController.syncRecorderFromNative(true);

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

  function resetTranscriptionRun(options: { clearCompletedRecordingMetadata?: boolean } = {}) {
    latestTranscript = null;
    latestManualTranscriptionResult = null;
    latestManualOutcomeSettings = null;
    latestManualTranscriptionFailure = null;

    if (options.clearCompletedRecordingMetadata) {
      latestCompletedRecordingMetadata = null;
    }
  }

  function resetWorkspace() {
    recordingController.stopRecordingStatusPolling();
    latestRecordingStatus = null;
    activeRecordingSession = null;
    resetTranscriptionRun({ clearCompletedRecordingMetadata: true });
    syncIdleStatus();
  }

  onMount(() => {
    void runPing();
    void initializeWorkspace();
    void loadHistoryEntries();
  });

  onDestroy(() => {
    recordingController.stopRecordingStatusPolling();
  });

  function showSection(section: AppSection) {
    activeSection.set(section);
  }

  function resetWorkspaceView() {
    resetWorkspace();
    activeSection.set("recording");
  }

  function clearLatestTranscriptPreview() {
    latestTranscript = null;
  }

  function syncIdleStatus(detail = buildIdleDetail()) {
    appStatus.setStatus(
      getAppStatusForPhase("idle", {
        detail,
        transcriptPreview:
          recordingDevicesState === "error"
            ? "Microphone loading failed. Retry device loading while recording is unavailable."
            : selectedMicrophoneUnavailable
              ? "Choose an available microphone before starting a recording. Your saved selection stays in place until you update it."
              : "Start a recording to capture a temporary audio file locally, then run Gemini when you are ready.",
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

    return `Ready to record from ${selectedMicrophoneLabel}. Stop keeps the audio file so you can run Gemini when you are ready.`;
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

  const recordingController = createRecordingController({
    getSettingsDraft: () => get(settingsDraft),
    setAvailableRecordingDevices: (devices) => {
      availableRecordingDevices = devices;
    },
    setRecordingDevicesState: (state) => {
      recordingDevicesState = state;
    },
    setRecordingDevicesError: (message) => {
      recordingDevicesError = message;
    },
    setRecordingInputOptions: (devices, selectedMicrophone) => {
      recordingInputOptions.set(createRecordingInputOptions(devices, selectedMicrophone));
    },
    getActiveRecordingSession: () => activeRecordingSession,
    setActiveRecordingSession: (session) => {
      activeRecordingSession = session;
    },
    getAppStatusPhase: () => get(appStatus).phase,
    getRecordedAudio: () => get(appStatus).recordedAudio,
    setAppStatus: (status) => {
      appStatus.setStatus(status);
    },
    syncIdleStatus,
    getSelectedMicrophoneLabel: () => selectedMicrophoneLabel,
    getSelectedMicrophoneUnavailable: () => selectedMicrophoneUnavailable,
    getRecordingCommandState: () => recordingCommandState,
    setRecordingCommandState: (state) => {
      recordingCommandState = state;
    },
    getCanStartRecording: () => canStartRecording,
    getCanStopRecording: () => canStopRecording,
    getCanCancelRecording: () => canCancelRecording,
    getCanRunManualTranscription: () => canRunManualTranscription,
    resolveSelectedDeviceName,
    resolveRecordingLimitMs,
    setLatestRecordingStatus: (status) => {
      latestRecordingStatus = status;
    },
    resetTranscriptionRun,
    setLatestCompletedRecordingMetadata: (value) => {
      latestCompletedRecordingMetadata = value;
    },
    setLatestTranscript: (value) => {
      latestTranscript = value;
    },
    setLatestManualTranscriptionResult: (value) => {
      latestManualTranscriptionResult = value;
    },
    setLatestManualOutcomeSettings: (value) => {
      latestManualOutcomeSettings = value;
    },
    setLatestManualTranscriptionFailure: (value) => {
      latestManualTranscriptionFailure = value;
    },
    setActiveTranscriptionCommand: (value) => {
      activeTranscriptionCommand = value;
    },
    getTranscribableRecordedAudio: () => transcribableRecordedAudio,
    getManualOutcomeSettingsSnapshot: () => {
      const currentSettings = get(settingsDraft);

      return {
        autoCopy: currentSettings.autoCopy,
        saveAudioFiles: currentSettings.saveAudioFiles,
        saveTranscriptionHistory: currentSettings.saveTranscriptionHistory
      };
    },
    loadHistoryEntries,
    formatDuration,
    formatFileName,
    hiddenManualNotificationCompletedMessage,
    hiddenManualNotificationFailedMessage
  });

  async function beginRecording() {
    await recordingController.beginRecording();
  }

  async function finishRecording() {
    await recordingController.finishRecording();
  }

  async function discardRecording() {
    await recordingController.discardRecording();
  }

  async function startManualTranscription() {
    await recordingController.startManualTranscription();
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
    content="SpeakEx desktop app with local recording, Gemini transcription, retry guidance, privacy-safe hidden-window notifications, history, and settings."
  />
</svelte:head>

<main class="app-shell">
  <Sidebar
    currentSection={$activeSection}
    onResetWorkspace={resetWorkspaceView}
    onSelectSection={showSection}
  />

  <section class="workspace">
    <TopBar />

    {#if $activeSection === "recording"}
      <RecordingSection
        bind:pingResponse
        phaseLabel={$appStatus.phaseLabel}
        isRecordingActive={activeRecordingSession !== null}
        elapsedTimeLabel={elapsedTimeLabel}
        latestTranscript={latestTranscript}
        canStartRecording={canStartRecording}
        canStopRecording={canStopRecording}
        canRunManualTranscription={canRunManualTranscription}
        geminiApiKeyPresence={geminiApiKeyPresence}
        onBeginRecording={beginRecording}
        onFinishRecording={finishRecording}
        onStartManualTranscription={startManualTranscription}
        onOpenSettings={() => showSection("settings")}
        onClearLatestTranscript={clearLatestTranscriptPreview}
      />
    {:else if $activeSection === "history"}
      <HistorySection
        historyState={historyState}
        historyEntries={historyEntries}
        isClearingHistory={isClearingHistory}
        historyBusyEntryId={historyBusyEntryId}
        selectedHistoryEntryId={selectedHistoryEntryId}
        selectedHistoryEntry={selectedHistoryEntry}
        onClearAllHistory={clearAllHistory}
        onSelectHistoryEntry={selectHistoryEntry}
        onRemoveHistoryEntry={removeHistoryEntry}
      />
    {:else if $activeSection === "settings"}
      <SettingsSection
        bind:geminiApiKeyDraft
        bind:recordingShortcutDraft
        settingsDraft={$settingsDraft}
        recordingInputOptions={$recordingInputOptions}
        languageOptions={languageOptions}
        geminiApiKeyPresence={geminiApiKeyPresence}
        geminiApiKeyStatusMessage={geminiApiKeyStatusMessage}
        settingsStatusMessage={settingsStatusMessage}
        recordingDevicesStatusMessage={recordingDevicesStatusMessage}
        isGeminiApiKeyBusy={isGeminiApiKeyBusy}
        geminiApiKeyPrimaryActionLabel={geminiApiKeyPrimaryActionLabel}
        recordingShortcutStatusMessage={recordingShortcutStatusMessage}
        isRecordingShortcutBusy={isRecordingShortcutBusy}
        recordingShortcutPrimaryActionLabel={recordingShortcutPrimaryActionLabel}
        onSubmitGeminiApiKey={submitGeminiApiKey}
        onUpdateMicrophone={updateMicrophone}
        onUpdateLanguage={updateLanguage}
        onToggleSetting={toggleSetting}
        onSubmitRecordingShortcut={submitRecordingShortcut}
      />
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
    background: #212121;
    color: #ececf1;
  }

  :global(button) {
    font: inherit;
    border: none;
    cursor: pointer;
    background: transparent;
    color: inherit;
  }

  .app-shell {
    height: 100vh;
    display: grid;
    grid-template-columns: 260px 1fr;
    box-sizing: border-box;
    overflow: hidden;
  }

  .workspace {
    display: flex;
    flex-direction: column;
    height: 100vh;
    position: relative;
    background: #212121;
  }

  @media (max-width: 768px) {
    .app-shell {
      grid-template-columns: 1fr;
    }
  }
</style>
