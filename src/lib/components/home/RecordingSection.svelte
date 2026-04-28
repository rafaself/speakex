<script lang="ts">
  import type { Transcript } from "$lib/native/transcription";

  export let phaseLabel: string;
  export let isRecordingActive = false;
  export let elapsedTimeLabel = "—";
  export let latestTranscript: Transcript | null = null;
  export let pingResponse = "";
  export let canStartRecording = false;
  export let canStopRecording = false;
  export let canRunManualTranscription = false;
  export let geminiApiKeyPresence = false;
  export let onBeginRecording: () => void;
  export let onFinishRecording: () => void;
  export let onStartManualTranscription: () => void;
  export let onOpenSettings: () => void;
  export let onClearLatestTranscript: () => void;
</script>

<div class="main-content">
  <h1>Where should we begin?</h1>

  <div class="input-container">
    <div class="chat-input-wrapper">
      <button class="icon-btn" title="Add attachment" type="button" on:click={onBeginRecording} disabled={!canStartRecording}>
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 5v14M5 12h14" />
        </svg>
      </button>
      <input type="text" class="chat-input" placeholder="Ask anything" bind:value={pingResponse} />
      <div class="input-actions">
        <button class="icon-btn" title="Voice" type="button" on:click={onFinishRecording} disabled={!canStopRecording}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <line x1="12" y1="19" x2="12" y2="23" />
            <line x1="8" y1="23" x2="16" y2="23" />
          </svg>
        </button>
        <button class="voice-btn" type="button" on:click={onStartManualTranscription} disabled={!canRunManualTranscription}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 1v22M5 8v8M19 8v8M9 11v2M15 11v2" />
          </svg>
          Voice
        </button>
      </div>
    </div>
  </div>

  <div class="phase-summary">
    <p class="phase-note">
      Status: <span class="status-pill">{phaseLabel}</span>
      {#if isRecordingActive}
        <span class="status-pill success">Recording: {elapsedTimeLabel}</span>
      {/if}
    </p>

    {#if latestTranscript}
      <div class="status-card">
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
        <strong>Latest Transcript:</strong> {latestTranscript.text}
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
    justify-content: center;
    padding: 2rem;
    text-align: center;
    overflow-y: auto;
  }

  .main-content h1 {
    font-size: 2.25rem;
    font-weight: 600;
    margin-bottom: 2.5rem;
    color: #fff;
  }

  .input-container {
    width: 100%;
    max-width: 768px;
    position: relative;
    margin-bottom: 1rem;
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
    font-size: 1rem;
    padding: 0.75rem 0;
    outline: none;
    width: 100%;
  }

  .chat-input::placeholder {
    color: #9b9b9b;
  }

  .input-actions {
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
    cursor: not-allowed;
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

  .voice-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
  }

  .phase-summary {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 1rem;
    width: min(100%, 600px);
  }

  .phase-note {
    margin: 0;
  }

  .status-pill {
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .status-pill.success {
    margin-left: 0.5rem;
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

  .warning-note {
    font-size: 0.85rem;
    color: #ffab00;
    margin: 0.5rem 0 0;
  }

  .link-button {
    text-decoration: underline;
  }

</style>
