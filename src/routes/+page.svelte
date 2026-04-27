<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";

  import {
    clearHistory,
    deleteTranscription,
    getHistory,
    type HistoryTranscriptionSummary
  } from "$lib/native/history";
  import { loadAppSettings, saveAppSettings } from "$lib/native/settings";
  import { ping } from "$lib/native/ping";
  import {
    activeSection,
    appStatus,
    languageOptions,
    microphoneOptions,
    mockRecordingPhases,
    navigationSections,
    providerOptions,
    providerSelection,
    recordingPlanSteps,
    settingsDraft,
    type DraftToggleKey
  } from "$lib/stores/app-shell";
  import type { ProviderId } from "$lib/settings/schema";
  import type { RecordingPhase, SettingsDraft } from "$lib/types/app-shell";

  type PingState = "idle" | "loading" | "success" | "error";
  type SettingsState = "idle" | "loading" | "saving" | "error";
  type HistoryState = "loading" | "ready" | "error";

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

  let pingState: PingState = "idle";
  let pingResponse = "";
  let pingError = "";
  let settingsState: SettingsState = "loading";
  let settingsError = "";
  let lastSavedSettings: SettingsDraft | null = null;
  let settingsSaveQueue = Promise.resolve();
  let latestSettingsRequest = 0;
  let historyState: HistoryState = "loading";
  let historyEntries: HistoryEntryViewModel[] = [];
  let historyError = "";
  let historyBusyEntryId: string | null = null;
  let isClearingHistory = false;

  $: selectedProviderLabel = providerLabels.get($providerSelection) ?? "Unknown provider";
  $: primaryMockActionLabel =
    $appStatus.phase === "idle"
      ? "Show recording"
      : $appStatus.phase === "recording"
        ? "Show transcribing"
        : $appStatus.phase === "transcribing"
          ? "Show completed"
          : "Restart mock flow";
  $: settingsStatusMessage =
    settingsState === "loading"
      ? "Loading saved preferences…"
      : settingsState === "saving"
        ? "Saving changes locally…"
      : settingsState === "error"
          ? settingsError
          : "Preferences are stored locally on this device.";
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
      settingsState = "idle";
    } catch (error) {
      if (requestId !== latestSettingsRequest) {
        return;
      }

      settingsError = error instanceof Error ? error.message : "Unable to save preferences";
      settingsState = "error";

      if (lastSavedSettings) {
        settingsDraft.patch(lastSavedSettings);
      }
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
    void persistSettings({
      ...get(settingsDraft),
      selectedMicrophone: (event.currentTarget as HTMLSelectElement).value
    });
  }

  function toggleSetting(key: DraftToggleKey) {
    const draft = get(settingsDraft);

    void persistSettings({
      ...draft,
      [key]: !draft[key]
    });
  }

  function setMockPhase(phase: RecordingPhase) {
    appStatus.setPhase(phase);
  }

  function advanceMockPhase() {
    if ($appStatus.phase === "completed" || $appStatus.phase === "error") {
      appStatus.reset();
      return;
    }

    appStatus.advance();
  }

  function showMockError() {
    appStatus.setPhase("error");
  }

  function resetMockFlow() {
    appStatus.reset();
  }

  onMount(() => {
    void runPing();
    void hydrateSettings();
    void loadHistoryEntries();
  });

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
</script>

<svelte:head>
  <title>SpeakEx — App Shell</title>
  <meta
    name="description"
    content="SpeakEx desktop app shell with recording, history, and settings views."
  />
</svelte:head>

<main class="app-shell">
  <aside class="sidebar">
    <div class="brand-block">
      <p class="eyebrow">Release 0.4</p>
      <h1>SpeakEx</h1>
      <p class="brand-copy">
        Local-first transcription for the desktop. This release keeps the shell in place while loading
        saved transcript history from local SQLite storage.
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
          Mock app state now feeds the primary capture workspace, including status and transcript preview.
        {:else if $activeSection === "history"}
          Saved transcript history now loads from the local database and supports explicit delete actions.
        {:else}
          Provider selection and preferences now hydrate from local storage and save back through the
          settings access layer.
        {/if}
      </p>
    </header>

    {#if $activeSection === "recording"}
      <div class="view-grid recording-grid">
        <section class="card hero-card">
          <div class="hero-copy">
            <p class="label">Main action</p>
            <h3>{$appStatus.headline}</h3>
            <p>{$appStatus.detail}</p>
            <p class="provider-caption">Current draft provider: <strong>{selectedProviderLabel}</strong></p>
            <p class="phase-note">Everything in this panel is a frontend-only mock for Release 0.2.</p>
          </div>

          <div class="hero-actions">
            <button type="button" class="primary-button" on:click={advanceMockPhase}>
              {primaryMockActionLabel}
            </button>
            <button
              type="button"
              class="secondary-button"
              on:click={resetMockFlow}
              disabled={$appStatus.phase === "idle"}
            >
              Reset to idle
            </button>
            <button type="button" class="ghost-button" on:click={showMockError}>
              Show error state
            </button>
          </div>
        </section>

        <section class="card transcript-card">
          <div class="section-heading">
            <div>
              <p class="label">Preview area</p>
              <h3>Transcript output</h3>
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
              <dt>Duration</dt>
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
              <p class="label">Mock state controls</p>
              <h3>Current app state</h3>
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
              The recording workspace is showing the <strong>{$appStatus.phase}</strong> phase from
              the local frontend store.
            </p>
          </div>

          <div class="phase-button-grid">
            {#each mockRecordingPhases as phaseStatus}
              <button
                type="button"
                class="phase-button"
                class:active={$appStatus.phase === phaseStatus.phase}
                aria-pressed={$appStatus.phase === phaseStatus.phase}
                on:click={() => setMockPhase(phaseStatus.phase)}
              >
                <span>{phaseStatus.phaseLabel}</span>
                <small>{phaseStatus.headline}</small>
              </button>
            {/each}
          </div>
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
            This view now reads saved transcripts from SQLite through explicit native commands. It stays
            limited to loading and deletion for Release 0.4.
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
            Non-sensitive preferences load on startup, missing values are initialized with safe
            defaults, and changes save locally on this device.
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
                {#each microphoneOptions as option}
                  <option value={option.value}>{option.label}</option>
                {/each}
              </select>
            </label>
          </div>

          <div class="setting-row">
            <div>
              <h4>Auto-copy transcript</h4>
              <p>Preference is stored now; clipboard execution still arrives in a later release.</p>
            </div>
            <button type="button" class="toggle-button" class:active={$settingsDraft.autoCopy} aria-pressed={$settingsDraft.autoCopy} on:click={() => toggleSetting("autoCopy")}>
              {$settingsDraft.autoCopy ? "On" : "Off"}
            </button>
          </div>

          <div class="setting-row">
            <div>
              <h4>Save transcription history</h4>
              <p>Preference is stored now so later history work can follow the saved app setting.</p>
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
              <p>Preference is stored now so later native storage work has a clear destination.</p>
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
              <dd>{microphoneOptions.find((option) => option.value === $settingsDraft.selectedMicrophone)?.label}</dd>
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
  .provider-button,
  .phase-button {
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
  .provider-button.active,
  .phase-button:hover,
  .phase-button.active {
    transform: translateY(-1px);
    border-color: rgba(125, 211, 252, 0.5);
    background: rgba(30, 41, 59, 0.9);
  }

  .section-nav span,
  .provider-button span,
  .phase-button span {
    font-size: 1rem;
    font-weight: 700;
  }

  .section-nav small,
  .provider-button small,
  .phase-button small {
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

  .phase-button-grid {
    display: grid;
    gap: 0.85rem;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
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

  .select-field {
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
