<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";
  import { getCurrentWindow, type Window } from "@tauri-apps/api/window";

  import Sidebar from "$lib/components/app-shell/Sidebar.svelte";
  import HistorySection from "$lib/components/home/HistorySection.svelte";
  import LogsSection from "$lib/components/home/LogsSection.svelte";
  import RecordingSection from "$lib/components/home/RecordingSection.svelte";
  import SettingsSection from "$lib/components/home/SettingsSection.svelte";
  import { buildIdleAppStatus, buildIdleDetail as buildHomeIdleDetail } from "$lib/features/home/idle-status";
  import {
    buildRecordingShortcutStatusMessage,
    formatDuration,
    formatFileName,
    resolveDetectedLanguageCodeFromHistory,
    resolveLanguageOptionLabel,
    type ShortcutActionState
  } from "$lib/features/home/presenters";
  import { createHistoryController } from "$lib/features/history/controller";
  import { createLogsController } from "$lib/features/logs/controller";
  import { mapHistoryEntry } from "$lib/features/history/presenters";
  import type { HistoryEntryViewModel } from "$lib/features/history/types";
  import {
    createRecordingController,
    type LatestManualOutcomeSettings
  } from "$lib/features/recording/controller";
  import { createSettingsController } from "$lib/features/settings/controller";
  import { type HistoryTranscription } from "$lib/native/history";
  import { createErrorLog, type ErrorLogEntry } from "$lib/native/logs";
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
  type LogsState = "loading" | "ready" | "error";
  type RecordingDevicesState = "loading" | "ready" | "error";
  type RecordingCommandState = "starting" | "stopping" | "cancelling" | null;
  type TranscriptionCommandState = "manual" | null;

  const fallbackRecordingLimitMs = 15 * 60 * 1000;
  const hiddenManualNotificationCompletedMessage =
    "If SpeakEx was hidden when this transcription finished, you may also have seen a desktop notification.";
  const hiddenManualNotificationFailedMessage =
    "If SpeakEx was hidden when this transcription failed, you may also have seen a generic desktop notification. The detailed error stays in SpeakEx.";
  const defaultWindowTitle = "SpeakEx";

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
  let logsState: LogsState = "loading";
  let logsEntries: ErrorLogEntry[] = [];
  let logsError = "";
  let isClearingLogs = false;
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
  let currentWindow: Window | null = null;
  let isWindowMaximized = false;

  $: effectiveSelectedMicrophoneValue =
    $settingsDraft.selectedMicrophone === "default"
      ? ($recordingInputOptions.find((option) => option.isDefault && !(option.unavailable ?? false))?.value ??
          defaultRecordingInputOption.value)
      : $settingsDraft.selectedMicrophone;
  $: detectedLanguageCode =
    latestTranscript?.language ??
    latestManualTranscriptionResult?.transcript.language ??
    selectedHistoryEntry?.language ??
    resolveDetectedLanguageCodeFromHistory(historyEntries[0]?.languageLabel, languageOptions) ??
    null;
  $: detectedLanguageLabel = resolveLanguageOptionLabel(detectedLanguageCode, languageOptions);
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
    (activeRecordingSession !== null || transcribableRecordedAudio !== null) &&
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
    canClearRecordingShortcut: () => canClearRecordingShortcut,
    reportErrorLog
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
    mapHistoryEntry,
    reportErrorLog
  });

  const logsController = createLogsController({
    setLogsState: (state) => {
      logsState = state;
    },
    setLogsEntries: (entries) => {
      logsEntries = entries;
    },
    getLogsEntries: () => logsEntries,
    setLogsError: (message) => {
      logsError = message;
    },
    setIsClearingLogs: (value) => {
      isClearingLogs = value;
    },
    getIsClearingLogs: () => isClearingLogs
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

  async function loadErrorLogs() {
    await logsController.loadErrorLogs();
  }

  async function clearAllErrorLogs() {
    await logsController.clearAllErrorLogs();
  }

  async function reportErrorLog(entry: { scope: string; summary: string; detail: string }) {
    const detail = entry.detail.trim();

    if (detail === "") {
      return;
    }

    try {
      await createErrorLog({
        ...entry,
        source: "frontend"
      });

      if (get(activeSection) === "logs") {
        await loadErrorLogs();
      }
    } catch {
      // Error logging should not break the main user workflow.
    }
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

  function resolveCurrentWindow() {
    try {
      return getCurrentWindow();
    } catch {
      return null;
    }
  }

  onMount(() => {
    currentWindow = resolveCurrentWindow();

    void syncWindowChromeState();
    void initializeWorkspace();
    void loadHistoryEntries();

    if (currentWindow === null) {
      return;
    }

    const unlistenResizePromise = currentWindow.listen("tauri://resize", async () => {
      isWindowMaximized = await currentWindow?.isMaximized() ?? false;
    });

    const unlistenFocusPromise = currentWindow.listen("tauri://focus", async () => {
      isWindowMaximized = await currentWindow?.isMaximized() ?? false;
    });

    return () => {
      void unlistenResizePromise.then((unlisten) => unlisten());
      void unlistenFocusPromise.then((unlisten) => unlisten());
    };
  });

  onDestroy(() => {
    recordingController.stopRecordingDevicePolling();
    recordingController.stopRecordingStatusPolling();
  });

  async function syncWindowChromeState() {
    if (currentWindow === null) {
      return;
    }

    isWindowMaximized = await currentWindow.isMaximized();
  }

  async function startWindowDrag() {
    await currentWindow?.startDragging();
  }

  async function minimizeWindow() {
    await currentWindow?.minimize();
  }

  async function toggleWindowMaximize() {
    await currentWindow?.toggleMaximize();
    await syncWindowChromeState();
  }

  async function closeWindow() {
    await currentWindow?.close();
  }

  function showSection(section: AppSection) {
    activeSection.set(section);

    if (section === "logs") {
      void loadErrorLogs();
    }
  }

  function resetWorkspaceView() {
    resetWorkspace();
    activeSection.set("recording");
  }

  function clearLatestTranscriptPreview() {
    latestTranscript = null;
  }

  function syncIdleStatus(detail = buildHomeIdleDetail({
    recordingDevicesState,
    recordingDevicesError,
    selectedMicrophoneUnavailable,
    selectedMicrophoneLabel
  })) {
    appStatus.setStatus(
      buildIdleAppStatus({
        recordingDevicesState,
        recordingDevicesError,
        selectedMicrophoneUnavailable,
        selectedMicrophoneLabel,
        detail
      })
    );
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
    getAvailableRecordingDevices: () => availableRecordingDevices,
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
    hiddenManualNotificationFailedMessage,
    reportErrorLog
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
</script>

<svelte:head>
  <title>SpeakEx — Local-first Transcription</title>
  <meta
    name="description"
    content="SpeakEx desktop app with local recording, Gemini transcription, retry guidance, privacy-safe hidden-window notifications, history, and settings."
  />
</svelte:head>

<main class="app-shell">
  <header class="window-titlebar">
    <button
      class="window-drag-region"
      type="button"
      aria-label="Move window"
      on:mousedown={startWindowDrag}
      on:dblclick={toggleWindowMaximize}
    >
      <span class="window-title">{defaultWindowTitle}</span>
    </button>

    <div class="window-controls">
      <button class="window-control" type="button" aria-label="Minimize window" on:click={minimizeWindow}>
        <span class="window-control-icon window-control-icon-minimize" aria-hidden="true"></span>
      </button>
      <button class="window-control" type="button" aria-label="Toggle maximize window" on:click={toggleWindowMaximize}>
        <span class:window-control-icon={true} class:window-control-icon-maximize={!isWindowMaximized} class:window-control-icon-restore={isWindowMaximized} aria-hidden="true"></span>
      </button>
      <button class="window-control window-control-close" type="button" aria-label="Close window" on:click={closeWindow}>
        <span class="window-control-icon window-control-icon-close" aria-hidden="true"></span>
      </button>
    </div>
  </header>

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
        isRunningTranscription={isRunningTranscription}
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
    {:else if $activeSection === "logs"}
      <LogsSection
        {logsState}
        {logsError}
        {logsEntries}
        {isClearingLogs}
        onClearAllLogs={clearAllErrorLogs}
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
    appearance: none;
    -webkit-appearance: none;
    font: inherit;
    border: none;
    background: transparent;
    color: inherit;
    box-shadow: none;
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
    grid-template-rows: 48px 1fr;
    box-sizing: border-box;
    overflow: hidden;
  }

  .window-titlebar {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: stretch;
    min-height: 48px;
    background: #171717;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .window-drag-region {
    display: flex;
    align-items: center;
    padding: 0 1rem;
    text-align: left;
    user-select: none;
    cursor: default;
  }

  .window-title {
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    color: #f3f3f5;
  }

  .window-controls {
    display: flex;
    align-items: stretch;
  }

  .window-control {
    width: 46px;
    min-width: 46px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: #d5d5d9;
    cursor: default;
    transition:
      background-color 140ms ease,
      color 140ms ease;
  }

  .window-control:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #ffffff;
  }

  .window-control-close:hover {
    background: #d63b3b;
    color: #ffffff;
  }

  .window-control-icon {
    position: relative;
    display: inline-block;
    width: 12px;
    height: 12px;
  }

  .window-control-icon-minimize::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    bottom: 2px;
    border-top: 1.5px solid currentColor;
  }

  .window-control-icon-maximize::before {
    content: "";
    position: absolute;
    inset: 1px;
    border: 1.5px solid currentColor;
  }

  .window-control-icon-restore::before,
  .window-control-icon-restore::after {
    content: "";
    position: absolute;
    border: 1.5px solid currentColor;
    background: #171717;
  }

  .window-control-icon-restore::before {
    width: 7px;
    height: 7px;
    top: 0;
    right: 0;
  }

  .window-control-icon-restore::after {
    width: 7px;
    height: 7px;
    left: 0;
    bottom: 0;
  }

  .window-control-icon-close::before,
  .window-control-icon-close::after {
    content: "";
    position: absolute;
    top: 5px;
    left: 0;
    width: 12px;
    border-top: 1.5px solid currentColor;
  }

  .window-control-icon-close::before {
    transform: rotate(45deg);
  }

  .window-control-icon-close::after {
    transform: rotate(-45deg);
  }

  .workspace {
    display: flex;
    flex-direction: column;
    min-height: 0;
    position: relative;
    background: #212121;
  }

  :global(.sidebar) {
    min-height: 0;
  }

  @media (max-width: 768px) {
    .app-shell {
      grid-template-columns: 1fr;
      grid-template-rows: 48px auto 1fr;
    }
  }
</style>
