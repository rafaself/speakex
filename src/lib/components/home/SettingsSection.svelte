<script lang="ts">
  import type { RecordingInputOption, SettingsDraft } from "$lib/types/app-shell";
  import type { DraftToggleKey } from "$lib/stores/app-shell";

  export let geminiApiKeyDraft = "";
  export let recordingShortcutDraft = "";
  export let settingsDraft: SettingsDraft;
  export let recordingInputOptions: RecordingInputOption[] = [];
  export let languageOptions: Array<{ value: string; label: string }> = [];
  export let geminiApiKeyPresence = false;
  export let geminiApiKeyStatusMessage = "";
  export let settingsStatusMessage = "";
  export let recordingDevicesStatusMessage = "";
  export let isGeminiApiKeyBusy = false;
  export let geminiApiKeyPrimaryActionLabel = "Save key";
  export let canRemoveGeminiApiKey = false;
  export let recordingShortcutStatusMessage = "";
  export let isRecordingShortcutBusy = false;
  export let recordingShortcutPrimaryActionLabel = "Save and apply";
  export let canReapplyRecordingShortcut = false;
  export let canClearRecordingShortcut = false;
  export let onSubmitGeminiApiKey: () => void;
  export let onRemoveGeminiApiKey: () => void;
  export let onUpdateMicrophone: (event: Event) => void;
  export let onUpdateLanguage: (event: Event) => void;
  export let onToggleSetting: (key: DraftToggleKey) => void;
  export let onSubmitRecordingShortcut: () => void;
  export let onReapplyRecordingShortcut: () => void;
  export let onClearRecordingShortcut: () => void;
</script>

<div class="main-content settings-layout">
  <div class="content">
    <h2 class="heading">Settings</h2>
    <p class="section-note">{settingsStatusMessage}</p>

    <div class="settings-groups">
      <section class="settings-section">
        <h3>Gemini API Key</h3>
        <div class="chat-input-wrapper compact-field">
          <input
            type="password"
            class="chat-input"
            bind:value={geminiApiKeyDraft}
            placeholder={geminiApiKeyPresence ? "••••••••••••" : "Enter API Key"}
          />
          {#if geminiApiKeyPresence}
            <span class="field-indicator" role="img" aria-label="Gemini API key saved">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                <path d="M20 6 9 17l-5-5" />
              </svg>
            </span>
          {/if}
        </div>
        <div class="action-bar">
          <div class="action-group">
            <button
              class="secondary-pill"
              type="button"
              on:click={onRemoveGeminiApiKey}
              disabled={!canRemoveGeminiApiKey}
            >
              Remove saved key
            </button>
          </div>
          <div class="action-group action-group-end">
            <button
              class="primary-pill"
              type="button"
              on:click={onSubmitGeminiApiKey}
              disabled={isGeminiApiKeyBusy || geminiApiKeyDraft.trim().length === 0}
            >
              {geminiApiKeyPrimaryActionLabel}
            </button>
          </div>
        </div>
        {#if geminiApiKeyStatusMessage}
          <p class:status-ok={geminiApiKeyPresence} class="status-copy">{geminiApiKeyStatusMessage}</p>
        {/if}
      </section>

      <section class="settings-section">
        <h3>Microphone</h3>
        <select class="settings-select" value={settingsDraft.selectedMicrophone} on:change={onUpdateMicrophone}>
          {#each recordingInputOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
        <p class="status-copy muted-copy">{recordingDevicesStatusMessage}</p>
      </section>

      <section class="settings-section">
        <h3>Language</h3>
        <select class="settings-select" value={settingsDraft.defaultLanguage} on:change={onUpdateLanguage}>
          {#each languageOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </section>

      <section class="settings-section preferences">
        <h3>Preferences</h3>

        <div class="preference-row">
          <div>
            <div class="preference-title">Auto-copy transcript</div>
            <div class="preference-description">Copy to clipboard after transcription</div>
          </div>
          <button
            class:toggle-on={settingsDraft.autoCopy}
            class="toggle"
            type="button"
            aria-pressed={settingsDraft.autoCopy}
            aria-label="Toggle auto-copy transcript"
            on:click={() => onToggleSetting("autoCopy")}
          >
            <span class:thumb-on={settingsDraft.autoCopy} class="toggle-thumb"></span>
          </button>
        </div>

        <div class="preference-row">
          <div>
            <div class="preference-title">Save history</div>
            <div class="preference-description">Keep a local record of all transcripts</div>
          </div>
          <button
            class:toggle-on={settingsDraft.saveTranscriptionHistory}
            class="toggle"
            type="button"
            aria-pressed={settingsDraft.saveTranscriptionHistory}
            aria-label="Toggle save history"
            on:click={() => onToggleSetting("saveTranscriptionHistory")}
          >
            <span class:thumb-on={settingsDraft.saveTranscriptionHistory} class="toggle-thumb"></span>
          </button>
        </div>

        <div class="preference-row">
          <div>
            <div class="preference-title">Save audio files</div>
            <div class="preference-description">Keep recorded audio locally after transcription</div>
          </div>
          <button
            class:toggle-on={settingsDraft.saveAudioFiles}
            class="toggle"
            type="button"
            aria-pressed={settingsDraft.saveAudioFiles}
            aria-label="Toggle save audio files"
            on:click={() => onToggleSetting("saveAudioFiles")}
          >
            <span class:thumb-on={settingsDraft.saveAudioFiles} class="toggle-thumb"></span>
          </button>
        </div>
      </section>

      <section class="settings-section">
        <h3>Recording Shortcut</h3>
        <div class="chat-input-wrapper compact-field">
          <input
            type="text"
            class="chat-input"
            bind:value={recordingShortcutDraft}
            placeholder="e.g. CommandOrControl+Alt+A"
          />
        </div>
        <div class="action-bar">
          <div class="action-group">
            <button
              class="secondary-pill"
              type="button"
              on:click={onReapplyRecordingShortcut}
              disabled={!canReapplyRecordingShortcut}
            >
              Re-apply saved shortcut
            </button>
            <button
              class="secondary-pill"
              type="button"
              on:click={onClearRecordingShortcut}
              disabled={!canClearRecordingShortcut}
            >
              Clear shortcut
            </button>
          </div>
          <div class="action-group action-group-end">
            <button
              class="primary-pill"
              type="button"
              on:click={onSubmitRecordingShortcut}
              disabled={isRecordingShortcutBusy || recordingShortcutDraft.trim().length === 0}
            >
              {recordingShortcutPrimaryActionLabel}
            </button>
          </div>
        </div>
        <p class="status-copy muted-copy">{recordingShortcutStatusMessage}</p>
      </section>
    </div>
  </div>
</div>

<style>
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem;
    overflow-y: auto;
  }

  .settings-layout {
    justify-content: flex-start;
    padding-top: 5rem;
  }

  .content {
    width: 100%;
    max-width: 600px;
    text-align: left;
  }

  .heading {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0 0 0.5rem;
  }

  .section-note {
    margin: 0 0 2rem;
    font-size: 0.9rem;
    color: #9b9b9b;
    line-height: 1.5;
  }

  .settings-groups {
    display: flex;
    flex-direction: column;
    gap: 2.5rem;
  }

  .settings-section h3 {
    font-size: 0.85rem;
    color: #9b9b9b;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 1rem;
  }

  .chat-input-wrapper {
    background: #2f2f2f;
    border-radius: 24px;
    padding: 0.5rem 0.75rem 0.5rem 1.25rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  }

  .compact-field {
    border-radius: 12px;
    padding: 0.25rem 0.5rem 0.25rem 1rem;
  }

  .chat-input {
    flex: 1;
    background: transparent;
    border: none;
    color: #fff;
    font-size: 1rem;
    padding: 0.75rem 0;
    outline: none;
    width: 100%;
  }

  .chat-input::placeholder {
    color: #9b9b9b;
  }

  .chat-input[type="password"] {
    appearance: none;
    -webkit-appearance: none;
    background-image: none;
  }

  .chat-input[type="password"]::-webkit-credentials-auto-fill-button,
  .chat-input[type="password"]::-webkit-caps-lock-indicator,
  .chat-input[type="password"]::-webkit-strong-password-auto-fill-button,
  .chat-input[type="password"]::-webkit-textfield-decoration-container,
  .chat-input[type="password"]::-ms-reveal,
  .chat-input[type="password"]::-ms-clear {
    display: none;
    visibility: hidden;
    pointer-events: none;
  }

  .field-indicator {
    width: 1.5rem;
    height: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #72e17b;
    flex-shrink: 0;
    margin-right: 0.25rem;
  }

  .primary-pill {
    background: #fff;
    color: #000;
    border-radius: 8px;
    padding: 0.6rem 0.9rem;
    font-weight: 600;
  }

  .primary-pill:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .status-copy {
    font-size: 0.85rem;
    margin: 0.75rem 0 0;
    color: #888;
  }

  .status-copy.status-ok {
    color: #72e17b;
  }

  .muted-copy {
    color: #9b9b9b;
  }

  .action-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    margin-top: 0.75rem;
    gap: 1rem;
  }

  .action-group {
    display: flex;
    gap: 0.75rem;
    flex-wrap: nowrap;
    align-items: center;
    min-width: 0;
  }

  .action-group-end {
    margin-left: auto;
  }

  .secondary-pill {
    background: rgba(255, 255, 255, 0.06);
    color: #fff;
    border-radius: 8px;
    padding: 0.55rem 0.8rem;
    font-weight: 600;
    transition: background 0.2s;
  }

  .secondary-pill:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }

  .secondary-pill:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .settings-select {
    width: 100%;
    appearance: none;
    -webkit-appearance: none;
    background-color: #2f2f2f;
    background-image:
      linear-gradient(45deg, transparent 50%, #b4b4b4 50%),
      linear-gradient(135deg, #b4b4b4 50%, transparent 50%);
    background-position:
      calc(100% - 1.15rem) calc(50% - 0.12rem),
      calc(100% - 0.8rem) calc(50% - 0.12rem);
    background-size:
      0.4rem 0.4rem,
      0.4rem 0.4rem;
    background-repeat: no-repeat;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #fff;
    color-scheme: dark;
    padding: 0.75rem 2.75rem 0.75rem 0.75rem;
    border-radius: 12px;
    outline: none;
  }

  .settings-select option {
    background: #2f2f2f;
    color: #fff;
  }

  .preferences {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .preference-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 0;
  }

  .preference-title {
    font-weight: 500;
  }

  .preference-description {
    font-size: 0.85rem;
    color: #9b9b9b;
    margin-top: 0.15rem;
  }

  .toggle {
    width: 44px;
    height: 24px;
    background: #444;
    border-radius: 12px;
    position: relative;
    transition: background 0.2s;
    flex-shrink: 0;
  }

  .toggle.toggle-on {
    background: #10a37f;
  }

  .toggle-thumb {
    width: 20px;
    height: 20px;
    background: #fff;
    border-radius: 50%;
    position: absolute;
    top: 2px;
    left: 2px;
    transition: left 0.2s;
  }

  .toggle-thumb.thumb-on {
    left: 22px;
  }
</style>
