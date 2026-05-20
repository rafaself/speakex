<script lang="ts">
  import MenuList from "$lib/components/ui/MenuList.svelte";
  import {
    captureShortcutFromKeyboardEvent,
    getRecordingShortcutDisplayTokens
  } from "$lib/features/shortcut/presenter";
  import type { RecordingInputOption, SettingsDraft } from "$lib/types/app-shell";
  import type { DraftToggleKey } from "$lib/stores/app-shell";

  export let geminiApiKeyDraft = "";
  export let recordingShortcutDraft = "";
  export let settingsDraft: SettingsDraft;
  export let selectedMicrophoneValue = "default";
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
  export let recordingShortcutPrimaryActionLabel = "Save and activate";
  export let canSubmitRecordingShortcut = false;
  export let canReapplyRecordingShortcut = false;
  export let canClearRecordingShortcut = false;
  export let onSubmitGeminiApiKey: () => void;
  export let onRemoveGeminiApiKey: () => void;
  export let onUpdateMicrophone: (value: string) => void;
  export let onUpdateLanguage: (value: string) => void;
  export let onToggleSetting: (key: DraftToggleKey) => void;
  export let onSubmitRecordingShortcut: () => void;
  export let onReapplyRecordingShortcut: () => void;
  export let onClearRecordingShortcut: () => void;

  $: microphoneMenuOptions = recordingInputOptions.map((option) => ({
    value: option.value,
    label: option.label,
    disabled: option.unavailable ?? false
  }));

  $: languageMenuOptions = languageOptions.map((option) => ({
    value: option.value,
    label: option.label
  }));
  $: recordingShortcutTokens = getRecordingShortcutDisplayTokens(recordingShortcutDraft);
  $: recordingShortcutHelperMessage =
    recordingShortcutCaptureFeedback !== ""
      ? recordingShortcutCaptureFeedback
      : isCapturingRecordingShortcut
        ? "Press the full combination now. The same shortcut starts and stops recording."
        : recordingShortcutTokens.length > 0
          ? "Review the captured shortcut, then save it to start and stop recording from anywhere."
          : "Focus this field and press the keys you want to use. Include at least one modifier or choose a function key.";

  let isCapturingRecordingShortcut = false;
  let recordingShortcutCaptureFeedback = "";

  function handleRecordingShortcutFocus() {
    isCapturingRecordingShortcut = true;
    recordingShortcutCaptureFeedback = "";
  }

  function handleRecordingShortcutBlur() {
    isCapturingRecordingShortcut = false;
    recordingShortcutCaptureFeedback = "";
  }

  function handleRecordingShortcutKeydown(event: KeyboardEvent) {
    if (event.key === "Tab" && !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey) {
      return;
    }

    event.preventDefault();

    const result = captureShortcutFromKeyboardEvent(event);

    if (result.kind === "shortcut") {
      recordingShortcutDraft = result.value;
      recordingShortcutCaptureFeedback = "";
      return;
    }

    if (result.kind === "incomplete" || result.kind === "invalid") {
      recordingShortcutCaptureFeedback = result.message;
    }
  }
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
        <MenuList
          label="Microphone"
          value={selectedMicrophoneValue}
          options={microphoneMenuOptions}
          onSelect={onUpdateMicrophone}
        />
        <p class="status-copy muted-copy">{recordingDevicesStatusMessage}</p>
      </section>

      <section class="settings-section">
        <h3>Language</h3>
        <MenuList
          label="Language"
          value={settingsDraft.defaultLanguage}
          options={languageMenuOptions}
          onSelect={onUpdateLanguage}
        />
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
        <p class="section-description">Choose one shortcut to start and finish recording from anywhere.</p>
        <button
          class:shortcut-capture-active={isCapturingRecordingShortcut}
          class="shortcut-capture"
          type="button"
          aria-label="Recording shortcut"
          aria-describedby="recording-shortcut-helper recording-shortcut-status"
          disabled={isRecordingShortcutBusy}
          on:focus={handleRecordingShortcutFocus}
          on:blur={handleRecordingShortcutBlur}
          on:keydown={handleRecordingShortcutKeydown}
        >
          <span class="shortcut-capture-label">
            {#if isCapturingRecordingShortcut}
              Listening for keys…
            {:else}
              Press keys to start/stop recording
            {/if}
          </span>
          {#if recordingShortcutTokens.length > 0}
            <span class="shortcut-token-list" aria-hidden="true">
              {#each recordingShortcutTokens as token}
                <kbd class="shortcut-token">{token}</kbd>
              {/each}
            </span>
          {:else}
            <span class="shortcut-placeholder">Click here, then press a shortcut</span>
          {/if}
        </button>
        <p
          id="recording-shortcut-helper"
          class:status-error={recordingShortcutCaptureFeedback !== ""}
          class="status-copy muted-copy"
        >
          {recordingShortcutHelperMessage}
        </p>
        <div class="preference-row shortcut-preference-row">
          <div>
            <div class="preference-title">Auto-paste outside SpeakEx</div>
            <div class="preference-description">
              When SpeakEx is not focused, stopping a shortcut recording transcribes it, copies it
              to the clipboard, and pastes it into the active text field. Some systems may ask for
              accessibility/input automation permission.
            </div>
          </div>
          <button
            class:toggle-on={settingsDraft.pasteAfterShortcutRecording}
            class="toggle"
            type="button"
            aria-pressed={settingsDraft.pasteAfterShortcutRecording}
            aria-label="Toggle auto-paste outside SpeakEx"
            on:click={() => onToggleSetting("pasteAfterShortcutRecording")}
          >
            <span
              class:thumb-on={settingsDraft.pasteAfterShortcutRecording}
              class="toggle-thumb"
            ></span>
          </button>
        </div>
        <div class="action-bar">
          <div class="action-group">
            <button
              class="secondary-pill"
              type="button"
              on:click={onReapplyRecordingShortcut}
              disabled={!canReapplyRecordingShortcut}
            >
              Apply saved shortcut
            </button>
            <button
              class="secondary-pill"
              type="button"
              on:click={onClearRecordingShortcut}
              disabled={!canClearRecordingShortcut}
            >
              Remove shortcut
            </button>
          </div>
          <div class="action-group action-group-end">
            <button
              class="primary-pill"
              type="button"
              on:click={onSubmitRecordingShortcut}
              disabled={!canSubmitRecordingShortcut}
            >
              {recordingShortcutPrimaryActionLabel}
            </button>
          </div>
        </div>
        <p id="recording-shortcut-status" class="status-copy muted-copy">{recordingShortcutStatusMessage}</p>
      </section>
    </div>
  </div>
</div>

<style>
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    padding: 2rem;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .settings-layout {
    justify-content: flex-start;
    padding: 4rem clamp(1.5rem, 4vw, 3.5rem);
  }

  .content {
    width: 100%;
    max-width: 880px;
    margin-right: auto;
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
    color: #b7b7b7;
    line-height: 1.5;
  }

  .settings-groups {
    display: flex;
    flex-direction: column;
    gap: 2.5rem;
  }

  .settings-section h3 {
    font-size: 0.85rem;
    color: #b4b4b4;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 1rem;
  }

  .section-description {
    margin: 0 0 1rem;
    color: #b7b7b7;
    line-height: 1.5;
  }

  .chat-input-wrapper {
    background: #303030;
    border: 1px solid rgba(255, 255, 255, 0.12);
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
    color: #b2b2b2;
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
    cursor: default;
  }

  .status-copy {
    font-size: 0.85rem;
    margin: 0.75rem 0 0;
    color: #b0b0b0;
  }

  .status-copy.status-ok {
    color: #72e17b;
  }

  .status-copy.status-error {
    color: #ffb3b3;
  }

  .muted-copy {
    color: #b3b3b3;
  }

  .action-bar {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    flex-wrap: wrap;
    width: 100%;
    margin-top: 0.75rem;
    gap: 1rem;
  }

  .action-group {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    align-items: center;
    min-width: 0;
  }

  .action-group-end {
    margin-left: auto;
  }

  .shortcut-capture {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.85rem;
    padding: 1rem 1.1rem;
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    text-align: left;
    transition: border-color 0.2s, background 0.2s, box-shadow 0.2s;
  }

  .shortcut-capture:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .shortcut-capture:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .shortcut-capture:focus-visible,
  .shortcut-capture.shortcut-capture-active {
    outline: none;
    border-color: rgba(255, 255, 255, 0.28);
    background: rgba(255, 255, 255, 0.08);
    box-shadow: 0 0 0 3px rgba(255, 255, 255, 0.08);
  }

  .shortcut-capture-label {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #c7c7c7;
  }

  .shortcut-placeholder {
    color: #f7f7f7;
    font-size: 1rem;
    line-height: 1.5;
  }

  .shortcut-token-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .shortcut-token {
    min-width: 2.25rem;
    padding: 0.45rem 0.7rem;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.16);
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
    font: inherit;
    font-weight: 600;
    text-align: center;
    box-shadow: inset 0 -1px 0 rgba(0, 0, 0, 0.18);
  }

  .secondary-pill {
    background: rgba(255, 255, 255, 0.1);
    color: #f7f7f7;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 8px;
    padding: 0.55rem 0.8rem;
    font-weight: 600;
    transition: background 0.2s, border-color 0.2s;
  }

  .secondary-pill:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.16);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .secondary-pill:disabled {
    opacity: 0.45;
    cursor: default;
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

  .shortcut-preference-row {
    margin-top: 0.5rem;
    padding-top: 0.85rem;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }

  .preference-title {
    font-weight: 500;
  }

  .preference-description {
    font-size: 0.85rem;
    color: #b2b2b2;
    margin-top: 0.15rem;
  }

  .toggle {
    width: 44px;
    height: 24px;
    background: #5b5b5b;
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

  @media (max-width: 768px) {
    .settings-layout {
      padding: 2rem 1rem;
    }

    .action-group-end {
      margin-left: 0;
    }

    .shortcut-capture {
      padding: 0.9rem 1rem;
    }
  }
</style>
