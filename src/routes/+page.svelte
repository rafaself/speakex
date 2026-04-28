<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";

  import Sidebar from "$lib/components/app-shell/Sidebar.svelte";
  import HistorySection from "$lib/components/home/HistorySection.svelte";
  import RecordingSection from "$lib/components/home/RecordingSection.svelte";
  import SettingsSection from "$lib/components/home/SettingsSection.svelte";
  import { createHistoryController } from "$lib/features/history/controller";
  import { mapHistoryEntry } from "$lib/features/history/presenters";
  import type { HistoryEntryViewModel } from "$lib/features/history/types";
  import {
    createRecordingController,
    type LatestManualOutcomeSettings
  } from "$lib/features/recording/controller";
  import { createSettingsController } from "$lib/features/settings/controller";
  import { type HistoryTranscription } from "$lib/native/history";
  import { type RecordingShortcutStatus } from "$lib/native/shortcut";
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
    recordingInputOptions,
    settingsDraft,
    type DraftToggleKey
  } from "$lib/stores/app-shell";
  import type {
    AppSection,
    GeminiApiKeyActionState,
    GeminiApiKeyPresenceState,
    RecordedAudioMetadata,
    RecordingTiming
  } from "$lib/types/app-shell";

  type SettingsState = "idle" | "loading" | "saving" | "error";
  type HistoryState = "loading" | "ready" | "error";
  type HistoryDetailState = "idle" | "loading" | "ready" | "error";
  type RecordingDevicesState = "loading" | "ready" | "error";
  type RecordingCommandState = "starting" | "stopping" | "cancelling" | null;
  type ShortcutActionState = "idle" | "loading" | "applying" | "reapplying" | "clearing" | "error";
  type TranscriptionCommandState = "manual" | null;

  const fallbackRecordingLimitMs = 15 * 60 * 1000;
  const hiddenManualNotificationCompletedMessage =
    "If SpeakEx was hidden when this transcription finished, you may also have seen a desktop notification.";
  const hiddenManualNotificationFailedMessage =
    "If SpeakEx was hidden when this transcription failed, you may also have seen a generic desktop notification. The detailed error stays in SpeakEx.";

  let settingsState: SettingsState = "loading";
  let settingsError = "";
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

  $: effectiveSelectedMicrophoneValue =
    $settingsDraft.selectedMicrophone === "default"
      ? ($recordingInputOptions.find((option) => option.isDefault && !(option.unavailable ?? false))?.value ??
          defaultRecordingInputOption.value)
      : $settingsDraft.selectedMicrophone;
  $: detectedLanguageCode =
    latestTranscript?.language ??
    latestManualTranscriptionResult?.transcript.language ??
    selectedHistoryEntry?.language ??
    resolveDetectedLanguageCodeFromHistory(historyEntries[0]?.languageLabel) ??
    null;
  $: detectedLanguageLabel = resolveLanguageOptionLabel(detectedLanguageCode);
  $: settingsLanguageOptions = languageOptions.map((option) =>
    option.value === "auto" && detectedLanguageLabel !== null
      ? { ...option, label: `Auto-detect (${detectedLanguageLabel})` }
      : option
  );
  $: selectedMicrophoneOption =
    $recordingInputOptions.find((option) => option.value === effectiveSelectedMicrophoneValue) ??
    defaultRecordingInputOption;
  $: selectedMicrophoneLabel = selectedMicrophoneOption.label;
  $: selectedMicrophoneUnavailable = selectedMicrophoneOption.unavailable ?? false;
  $: isRunningTranscription = activeTranscriptionCommand !== null;
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
                : "";
  $: savedRecordingShortcut = $settingsDraft.shortcut;
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
  $: recordingShortcutStatusMessage = buildRecordingShortcutStatusMessage(
    recordingShortcutActionState,
    recordingShortcutStatus,
    savedRecordingShortcut,
    recordingShortcutError
  );
  $: transcribableRecordedAudio = $appStatus.recordedAudio;
  $: displayedRecordedAudio = transcribableRecordedAudio ?? latestCompletedRecordingMetadata;
  $: canStartRecording =
    recordingDevicesState === "ready" &&
    recordingCommandState === null &&
    activeRecordingSession === null &&
    !isRunningTranscription &&
    !selectedMicrophoneUnavailable;
  $: canDiscardRecording =
    activeRecordingSession !== null &&
    recordingCommandState === null &&
    !isRunningTranscription;
  $: canConfirmRecordingAndTranscribe =
    activeRecordingSession !== null &&
    recordingCommandState === null &&
    !isRunningTranscription &&
    !isGeminiApiKeyBusy &&
    geminiApiKeyPresenceState !== "error" &&
    geminiApiKeyPresence;
  $: canRunManualTranscription =
    transcribableRecordedAudio !== null &&
    activeRecordingSession === null &&
    recordingCommandState === null &&
    !isRunningTranscription &&
    !isGeminiApiKeyBusy &&
    geminiApiKeyPresenceState !== "error" &&
    geminiApiKeyPresence;
  $: hasTranscribableRecordedAudio = transcribableRecordedAudio !== null;
  $: canRemoveGeminiApiKey = !isGeminiApiKeyBusy && geminiApiKeyPresence;
  $: canReapplyRecordingShortcut = !isRecordingShortcutBusy && savedRecordingShortcut !== null;
  $: manualTranscriptionActionLabel =
    $appStatus.phase === "error" && transcribableRecordedAudio !== null
      ? "Retry transcription"
      : latestTranscript !== null
        ? "Transcribe again"
        : "Run transcription";

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

  const settingsController = createSettingsController({
    getSettingsDraft: () => get(settingsDraft),
    patchSettingsDraft: (value) => {
      settingsDraft.patch(value);
    },
    setSettingsState: (state) => {
      settingsState = state;
    },
    setSettingsError: (message) => {
      settingsError = message;
    },
    getAvailableRecordingDevices: () => availableRecordingDevices,
    setRecordingInputOptions: (devices, selectedMicrophone) => {
      recordingInputOptions.set(createRecordingInputOptions(devices, selectedMicrophone));
    },
    getActiveRecordingSession: () => activeRecordingSession !== null,
    getAppStatusPhase: () => get(appStatus).phase,
    syncIdleStatus,
    getGeminiApiKeyDraft: () => geminiApiKeyDraft,
    setGeminiApiKeyDraft: (value) => {
      geminiApiKeyDraft = value;
    },
    getGeminiApiKeyPresence: () => geminiApiKeyPresence,
    setGeminiApiKeyPresence: (value) => {
      geminiApiKeyPresence = value;
    },
    setGeminiApiKeyPresenceState: (value) => {
      geminiApiKeyPresenceState = value;
    },
    getGeminiApiKeyPresenceState: () => geminiApiKeyPresenceState,
    setGeminiApiKeyActionState: (value) => {
      geminiApiKeyActionState = value;
    },
    getGeminiApiKeyActionState: () => geminiApiKeyActionState,
    setGeminiApiKeyStatusDetail: (value) => {
      geminiApiKeyStatusDetail = value;
    },
    getRecordingShortcutDraft: () => recordingShortcutDraft,
    setRecordingShortcutDraft: (value) => {
      recordingShortcutDraft = value;
    },
    setRecordingShortcutStatus: (value) => {
      recordingShortcutStatus = value;
    },
    getRecordingShortcutActionState: () => recordingShortcutActionState,
    setRecordingShortcutActionState: (value) => {
      recordingShortcutActionState = value;
    },
    setRecordingShortcutError: (value) => {
      recordingShortcutError = value;
    },
    canClearRecordingShortcut: () => canClearRecordingShortcut
  });

  async function hydrateSettings() {
    await settingsController.hydrateSettings();
  }

  async function refreshRecordingShortcutStatus(showFeedback = true) {
    await settingsController.refreshRecordingShortcutStatus(showFeedback);
  }

  async function refreshGeminiApiKeyPresence(showFeedback = true) {
    await settingsController.refreshGeminiApiKeyPresence(showFeedback);
  }

  const historyController = createHistoryController({
    getHistoryState: () => historyState,
    setHistoryState: (state) => {
      historyState = state;
    },
    setHistoryEntries: (entries) => {
      historyEntries = entries;
    },
    getHistoryEntries: () => historyEntries,
    setHistoryError: (message) => {
      historyError = message;
    },
    setHistoryDetailState: (state) => {
      historyDetailState = state;
    },
    getHistoryDetailState: () => historyDetailState,
    setHistoryDetailError: (message) => {
      historyDetailError = message;
    },
    setHistoryBusyEntryId: (id) => {
      historyBusyEntryId = id;
    },
    getHistoryBusyEntryId: () => historyBusyEntryId,
    setIsClearingHistory: (value) => {
      isClearingHistory = value;
    },
    getIsClearingHistory: () => isClearingHistory,
    setSelectedHistoryEntryId: (id) => {
      selectedHistoryEntryId = id;
    },
    getSelectedHistoryEntryId: () => selectedHistoryEntryId,
    setSelectedHistoryEntry: (entry) => {
      selectedHistoryEntry = entry;
    },
    getSelectedHistoryEntry: () => selectedHistoryEntry,
    mapHistoryEntry
  });

  async function selectHistoryEntry(id: string) {
    await historyController.selectHistoryEntry(id);
  }

  async function loadHistoryEntries(preferredSelectionId: string | null = null) {
    await historyController.loadHistoryEntries(preferredSelectionId);
  }

  async function removeHistoryEntry(id: string) {
    await historyController.removeHistoryEntry(id);
  }

  async function clearAllHistory() {
    await historyController.clearAllHistory();
  }

  function updateLanguage(value: string) {
    settingsController.updateLanguage(value);
  }

  function updateMicrophone(value: string) {
    settingsController.updateMicrophone(value);
  }

  function toggleSetting(key: DraftToggleKey) {
    settingsController.toggleSetting(key);
  }

  async function submitRecordingShortcut() {
    await settingsController.submitRecordingShortcut();
  }

  async function submitGeminiApiKey() {
    await settingsController.submitGeminiApiKey();
  }

  async function removeGeminiApiKeySetting() {
    await settingsController.removeGeminiApiKey();
  }

  async function reapplyRecordingShortcutSetting() {
    await settingsController.reapplySavedRecordingShortcut();
  }

  async function clearRecordingShortcutSetting() {
    await settingsController.clearRecordingShortcutSetting();
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

  function resolveLanguageOptionLabel(languageCode: string | null | undefined) {
    if (!languageCode) {
      return null;
    }

    return languageOptions.find((option) => option.value === languageCode)?.label ?? languageCode;
  }

  function resolveDetectedLanguageCodeFromHistory(languageValue: string | null | undefined) {
    if (!languageValue || languageValue === "Auto / unspecified") {
      return null;
    }

    const matchedOption = languageOptions.find(
      (option) => option.value === languageValue || option.label === languageValue
    );

    return matchedOption?.value ?? languageValue;
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
    getCanDiscardRecording: () => canDiscardRecording,
    getCanConfirmRecordingAndTranscribe: () => canConfirmRecordingAndTranscribe,
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

  async function discardRecording() {
    await recordingController.discardRecording();
  }

  async function confirmRecordingAndTranscribe() {
    await recordingController.confirmRecordingAndTranscribe();
  }

  async function startManualTranscription() {
    await recordingController.startManualTranscription();
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
    {#if $activeSection === "recording"}
      <RecordingSection
        transcriptTitle={$appStatus.transcriptTitle}
        transcriptPreview={$appStatus.transcriptPreview}
        isRecordingActive={activeRecordingSession !== null}
        latestTranscript={latestTranscript}
        recordedAudio={displayedRecordedAudio}
        canStartRecording={canStartRecording}
        canDiscardRecording={canDiscardRecording}
        canConfirmRecordingAndTranscribe={canConfirmRecordingAndTranscribe}
        hasTranscribableRecordedAudio={hasTranscribableRecordedAudio}
        canRunManualTranscription={canRunManualTranscription}
        manualTranscriptionActionLabel={manualTranscriptionActionLabel}
        geminiApiKeyPresence={geminiApiKeyPresence}
        onBeginRecording={beginRecording}
        onDiscardRecording={discardRecording}
        onConfirmRecordingAndTranscribe={confirmRecordingAndTranscribe}
        onStartManualTranscription={startManualTranscription}
        onOpenSettings={() => showSection("settings")}
        onClearLatestTranscript={clearLatestTranscriptPreview}
      />
    {:else if $activeSection === "history"}
      <HistorySection
        historyState={historyState}
        historyError={historyError}
        historyEntries={historyEntries}
        historyDetailState={historyDetailState}
        historyDetailError={historyDetailError}
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
        selectedMicrophoneValue={effectiveSelectedMicrophoneValue}
        recordingInputOptions={$recordingInputOptions}
        languageOptions={settingsLanguageOptions}
        geminiApiKeyPresence={geminiApiKeyPresence}
        geminiApiKeyStatusMessage={geminiApiKeyStatusMessage}
        settingsStatusMessage={settingsStatusMessage}
        recordingDevicesStatusMessage={recordingDevicesStatusMessage}
        isGeminiApiKeyBusy={isGeminiApiKeyBusy}
        geminiApiKeyPrimaryActionLabel={geminiApiKeyPrimaryActionLabel}
        canRemoveGeminiApiKey={canRemoveGeminiApiKey}
        recordingShortcutStatusMessage={recordingShortcutStatusMessage}
        isRecordingShortcutBusy={isRecordingShortcutBusy}
        recordingShortcutPrimaryActionLabel={recordingShortcutPrimaryActionLabel}
        canReapplyRecordingShortcut={canReapplyRecordingShortcut}
        canClearRecordingShortcut={canClearRecordingShortcut}
        onSubmitGeminiApiKey={submitGeminiApiKey}
        onRemoveGeminiApiKey={removeGeminiApiKeySetting}
        onUpdateMicrophone={updateMicrophone}
        onUpdateLanguage={updateLanguage}
        onToggleSetting={toggleSetting}
        onSubmitRecordingShortcut={submitRecordingShortcut}
        onReapplyRecordingShortcut={reapplyRecordingShortcutSetting}
        onClearRecordingShortcut={clearRecordingShortcutSetting}
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
    background: transparent;
    color: inherit;
  }

  :global(button:not(:disabled)),
  :global(select:not(:disabled)),
  :global(summary) {
    cursor: pointer;
  }

  :global(button:disabled),
  :global(select:disabled) {
    cursor: default;
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
