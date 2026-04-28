<script lang="ts">
  import { fade } from "svelte/transition";

  import type { Transcript } from "$lib/native/transcription";
  import type { RecordedAudioMetadata } from "$lib/types/app-shell";

  import LiveWaveform from "../ui/LiveWaveform.svelte";

  export let transcriptTitle = "";
  export let transcriptPreview = "";
  export let isRecordingActive = false;
  export let isRunningTranscription = false;
  export let latestTranscript: Transcript | null = null;
  export let recordedAudio: RecordedAudioMetadata | null = null;
  export let canStartRecording = false;
  export let canDiscardRecording = false;
  export let canConfirmRecordingAndTranscribe = false;
  export let hasTranscribableRecordedAudio = false;
  export let canRunManualTranscription = false;
  export let manualTranscriptionActionLabel = "Run transcription";
  export let geminiApiKeyPresence = false;
  export let onBeginRecording: () => void;
  export let onDiscardRecording: () => void;
  export let onConfirmRecordingAndTranscribe: () => void;
  export let onStartManualTranscription: () => void;
  export let onOpenSettings: () => void;
  export let onClearLatestTranscript: () => void;

  let editedTranscriptText = "";

  $: if (latestTranscript) {
    editedTranscriptText = latestTranscript.text;
  }

  function formatFileSize(sizeBytes: number): string {
    if (sizeBytes < 1024) {
      return `${sizeBytes} B`;
    }

    if (sizeBytes < 1024 * 1024) {
      return `${(sizeBytes / 1024).toFixed(1)} KB`;
    }

    return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatAudioPath(path: string): string {
    return path.split(/[/\\\\]/u).pop() ?? path;
  }
</script>

<div class="main-content">
  <div class="composer-stage">
    <div class="composer-anchor">
      <h1>What should we write?</h1>

      <div class="input-container">
        <div class="chat-input-wrapper" class:recording-mode={isRecordingActive || hasTranscribableRecordedAudio || isRunningTranscription}>
          {#if latestTranscript}
            <textarea
              class="chat-input transcript-edit"
              bind:value={editedTranscriptText}
              placeholder="Edit your transcription..."
            ></textarea>
          {:else if isRecordingActive || hasTranscribableRecordedAudio || isRunningTranscription}
            <div class="waveform-container">
              <LiveWaveform 
                active={isRecordingActive} 
                frozen={hasTranscribableRecordedAudio || isRunningTranscription} 
                shimmer={isRunningTranscription} 
              />
            </div>
          {:else}
            <input
              type="text"
              class="chat-input"
              placeholder="Start your transcription..."
              on:focus={onBeginRecording}
              readonly
            />
          {/if}
          <div class="input-actions">
            {#if latestTranscript}
              <div class="recording-actions" in:fade={{ duration: 180 }} out:fade={{ duration: 140 }}>
                <button
                  class="icon-btn destructive-btn"
                  title="Discard transcription"
                  aria-label="Discard transcription"
                  type="button"
                  on:click={onClearLatestTranscript}
                >
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M18 6L6 18M6 6l12 12" />
                  </svg>
                </button>
                <button
                  class="icon-btn confirm-btn"
                  title="Send transcription"
                  aria-label="Send transcription"
                  type="button"
                  on:click={() => {
                    // Logic to "send" could be implemented here
                    onClearLatestTranscript();
                  }}
                >
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z" />
                  </svg>
                </button>
              </div>
            {:else if isRecordingActive}
              <div class="recording-actions" in:fade={{ duration: 180 }} out:fade={{ duration: 140 }}>
                <button
                  class="icon-btn destructive-btn"
                  title="Discard recording"
                  aria-label="Discard recording"
                  type="button"
                  on:click={onDiscardRecording}
                  disabled={!canDiscardRecording}
                >
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M18 6L6 18M6 6l12 12" />
                  </svg>
                </button>
                <button
                  class="icon-btn confirm-btn"
                  title="Confirm recording and transcribe"
                  aria-label="Confirm recording and transcribe"
                  type="button"
                  on:click={onConfirmRecordingAndTranscribe}
                  disabled={!canConfirmRecordingAndTranscribe}
                >
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M20 6L9 17l-5-5" />
                  </svg>
                </button>
              </div>
            {:else if hasTranscribableRecordedAudio}
              <div class="recording-actions" in:fade={{ duration: 180 }} out:fade={{ duration: 140 }}>
                <button
                  class="icon-btn destructive-btn"
                  title="Discard recording"
                  aria-label="Discard recording"
                  type="button"
                  on:click={onDiscardRecording}
                >
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M18 6L6 18M6 6l12 12" />
                  </svg>
                </button>
                <button
                  class="voice-btn"
                  type="button"
                  title={manualTranscriptionActionLabel}
                  aria-label={manualTranscriptionActionLabel}
                  on:click={onStartManualTranscription}
                  disabled={!canRunManualTranscription}
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 1v22M5 8v8M19 8v8M9 11v2M15 11v2" />
                  </svg>
                  {manualTranscriptionActionLabel}
                </button>
              </div>
            {:else}
              <div in:fade={{ duration: 180 }} out:fade={{ duration: 140 }}>
                <button
                  class="voice-btn start-recording-btn"
                  type="button"
                  title="Start recording"
                  aria-label="Start recording"
                  on:click={onBeginRecording}
                  disabled={!canStartRecording}
                >
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
                    <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                    <line x1="12" y1="19" x2="12" y2="23" />
                    <line x1="8" y1="23" x2="16" y2="23" />
                  </svg>
                </button>
              </div>
            {/if}
          </div>
        </div>
      </div>
    </div>
  </div>

  <div class="phase-summary">
    {#if latestTranscript || recordedAudio}
      <div class="status-card">
        {#if latestTranscript}
          <button
            class="icon-btn status-card-close"
            type="button"
            aria-label="Clear latest transcript preview"
            on:click={onClearLatestTranscript}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        {/if}
        <p class="card-eyebrow">{transcriptTitle}</p>
        {#if latestTranscript}
          <pre class="transcript-preview">{latestTranscript.text}</pre>
          <p class="card-copy compact-copy">
            {latestTranscript.provider}{#if latestTranscript.model} · {latestTranscript.model}{/if}
            {#if latestTranscript.language} · {latestTranscript.language}{/if}
          </p>
        {:else}
          <p class="card-copy">{transcriptPreview}</p>
        {/if}
      </div>
    {/if}

    {#if recordedAudio}
      <div class="status-card">
        <p class="card-eyebrow">Recorded audio</p>
        <dl class="summary-grid">
          <div>
            <dt>File</dt>
            <dd>{formatAudioPath(recordedAudio.path)}</dd>
          </div>
          <div>
            <dt>Size</dt>
            <dd>{formatFileSize(recordedAudio.fileSizeBytes)}</dd>
          </div>
          <div>
            <dt>Sample rate</dt>
            <dd>{recordedAudio.sampleRateHz} Hz</dd>
          </div>
          <div>
            <dt>Channels</dt>
            <dd>{recordedAudio.channels}</dd>
          </div>
          <div>
            <dt>MIME type</dt>
            <dd>{recordedAudio.mimeType}</dd>
          </div>
          <div>
            <dt>Limit reached</dt>
            <dd>{recordedAudio.limitReached ? "Yes" : "No"}</dd>
          </div>
        </dl>
        <p class="path-copy">{recordedAudio.path}</p>
      </div>
    {/if}

    {#if !geminiApiKeyPresence}
      <p class="warning-note">
        Gemini API key is missing. Please set it in
        <button class="link-button" type="button" on:click={onOpenSettings}>Settings</button>.
      </p>
    {/if}
  </div>
</div>

<style>
  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    min-height: 100%;
    padding: 2rem;
    text-align: center;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .composer-stage {
    width: 100%;
    flex: 1 0 auto;
    display: grid;
    place-items: center;
  }

  .composer-anchor {
    position: relative;
    width: min(100%, 768px);
  }

  .main-content h1 {
    position: absolute;
    left: 50%;
    bottom: calc(100% + 1.5rem);
    transform: translateX(-50%);
    font-size: 2.25rem;
    font-weight: 600;
    margin: 0;
    color: #fff;
    font-family: Georgia, "Times New Roman", serif;
    font-style: italic;
    width: max-content;
    max-width: 100%;
  }

  .input-container {
    width: 100%;
    position: relative;
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

  .chat-input {
    flex: 1;
    background: transparent;
    border: none;
    color: #fff;
    font-size: 1.1rem;
    padding: 0.75rem 0;
    outline: none;
    width: 100%;
    resize: none;
    min-height: 24px;
    line-height: 1.5;
  }

  .transcript-edit {
    min-height: 60px;
    max-height: 200px;
  }

  .waveform-container {
    flex: 1;
    display: flex;
    align-items: center;
    padding: 0.5rem 0;
  }

  .chat-input-wrapper.recording-mode {
    padding-left: 1.5rem;
  }

  .chat-input::placeholder {
    color: #9b9b9b;
    font-style: italic;
  }

  .chat-input:disabled {
    cursor: default;
    opacity: 1;
  }

  .input-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .recording-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .icon-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    transition: background 0.2s;
    color: #b4b4b4;
  }

  .icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }

  .icon-btn:disabled,
  .voice-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .destructive-btn {
    color: #f3a8a8;
  }

  .voice-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    padding: 0.4rem 0.8rem;
    border-radius: 20px;
    font-size: 0.85rem;
    font-weight: 600;
    transition: background 0.2s;
    color: #fff;
  }

  .start-recording-btn {
    width: 36px;
    height: 36px;
    padding: 0;
    border-radius: 999px;
    justify-content: center;
    background: rgba(255, 255, 255, 0.06);
    color: #c5c5c5;
  }

  .start-recording-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
    color: #d7d7d7;
  }

  .voice-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
  }

  .confirm-btn {
    background: rgba(24, 174, 96, 0.15);
    color: #9ef0ba;
  }

  .confirm-btn:hover:not(:disabled) {
    background: rgba(24, 174, 96, 0.3);
    color: #d3ffe2;
  }

  .phase-summary {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 1.25rem;
    width: min(100%, 600px);
  }

  .status-card {
    max-width: 600px;
    width: 100%;
    margin: 0 auto;
    text-align: left;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    position: relative;
    box-sizing: border-box;
  }

  .status-card-close {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
  }

  .card-eyebrow {
    margin: 0 0 0.4rem;
    color: #9b9b9b;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .card-copy {
    margin: 0.5rem 0 0;
    color: #c9c9cf;
    line-height: 1.5;
  }

  .compact-copy {
    font-size: 0.85rem;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem 1rem;
    margin: 1rem 0 0;
  }

  .summary-grid dt {
    font-size: 0.75rem;
    color: #9b9b9b;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .summary-grid dd {
    margin: 0.2rem 0 0;
  }

  .transcript-preview {
    margin: 0.5rem 0 0;
    white-space: pre-wrap;
    font-family: inherit;
    line-height: 1.5;
  }

  .path-copy {
    margin: 1rem 0 0;
    word-break: break-all;
    color: #9b9b9b;
    font-size: 0.8rem;
  }

  .warning-note {
    font-size: 0.85rem;
    color: #ffab00;
    margin: 0.5rem 0 0;
  }

  .link-button {
    text-decoration: underline;
  }

  @media (max-width: 640px) {
    .summary-grid {
      grid-template-columns: 1fr;
    }
  }

</style>
