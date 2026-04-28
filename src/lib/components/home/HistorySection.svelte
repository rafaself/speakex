<script lang="ts">
  import type { HistoryTranscription } from "$lib/native/history";
  import type { HistoryEntryViewModel } from "$lib/features/history/types";

  export let historyState: "loading" | "ready" | "error" = "loading";
  export let historyError = "";
  export let historyEntries: HistoryEntryViewModel[] = [];
  export let historyDetailState: "idle" | "loading" | "ready" | "error" = "idle";
  export let historyDetailError = "";
  export let isClearingHistory = false;
  export let historyBusyEntryId: string | null = null;
  export let selectedHistoryEntryId: string | null = null;
  export let selectedHistoryEntry: HistoryTranscription | null = null;
  export let onClearAllHistory: () => void;
  export let onSelectHistoryEntry: (id: string) => Promise<void>;
  export let onRemoveHistoryEntry: (id: string) => Promise<void>;
</script>

<div class="main-content history-layout">
  <div class="content">
    <div class="header">
      <h2 class="heading">History</h2>
      <button class="voice-btn" type="button" on:click={onClearAllHistory} disabled={isClearingHistory}>
        {isClearingHistory ? "Clearing..." : "Clear all"}
      </button>
    </div>

    {#if historyState === "loading"}
      <p class="muted-copy">Loading history...</p>
    {:else if historyState === "error"}
      <p class="error-copy">{historyError}</p>
    {:else if historyEntries.length === 0}
      <p class="muted-copy">No saved transcripts yet.</p>
    {:else}
      <div class="history-list">
        {#each historyEntries as entry}
          <article class="history-card">
            <div class="history-card-row">
              <button class="history-entry" type="button" on:click={() => void onSelectHistoryEntry(entry.id)}>
                <div class="history-entry-copy">
                  <div class="history-entry-title">{entry.title}</div>
                  <div class="history-entry-meta">
                    {entry.createdAtLabel} · {entry.durationLabel} · {entry.providerLabel}
                  </div>
                  <p class="history-entry-excerpt">{entry.excerpt}</p>
                  <div class="history-tags">
                    <span class="history-tag">{entry.languageLabel}</span>
                    <span class="history-tag">{entry.clipboardLabel}</span>
                    <span class="history-tag">{entry.audioLabel}</span>
                    <span class:attention-tag={entry.status === "attention"} class="history-tag">
                      {entry.statusLabel}
                    </span>
                  </div>
                </div>
              </button>
              <button
                class="icon-btn history-delete"
                type="button"
                aria-label={`Delete ${entry.title}`}
                on:click={() => void onRemoveHistoryEntry(entry.id)}
                disabled={historyBusyEntryId === entry.id}
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                </svg>
              </button>
            </div>

            {#if selectedHistoryEntryId === entry.id && historyDetailState === "loading"}
              <div class="history-detail">
                <p class="muted-copy">Loading transcript details...</p>
              </div>
            {:else if selectedHistoryEntryId === entry.id && historyDetailState === "error"}
              <div class="history-detail">
                <p class="error-copy">{historyDetailError}</p>
              </div>
            {:else if selectedHistoryEntryId === entry.id && selectedHistoryEntry}
              <div class="history-detail">
                <div class="history-detail-meta">
                  <span>{selectedHistoryEntry.language ?? "Auto / unspecified"}</span>
                  <span>{selectedHistoryEntry.copiedToClipboard ? "Copied to clipboard" : "Not copied to clipboard"}</span>
                  <span>{selectedHistoryEntry.audioDeleted ? "Deleted after transcription" : selectedHistoryEntry.audioPath ? "Retained locally" : "No retained audio"}</span>
                </div>
                {#if selectedHistoryEntry.error}
                  <p class="error-copy">{selectedHistoryEntry.error}</p>
                {/if}
                {#if selectedHistoryEntry.audioPath}
                  <p class="detail-path">{selectedHistoryEntry.audioPath}</p>
                {/if}
                <pre>{selectedHistoryEntry.text}</pre>
              </div>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
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

  .history-layout {
    justify-content: flex-start;
    padding-top: 5rem;
  }

  .content {
    width: 100%;
    max-width: 800px;
    text-align: left;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
    gap: 1rem;
  }

  .heading {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0;
  }

  .muted-copy {
    color: #9b9b9b;
  }

  .error-copy {
    color: #ff8a80;
    margin: 0;
  }

  .history-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .history-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    overflow: hidden;
  }

  .history-card-row {
    display: flex;
    align-items: stretch;
  }

  .history-entry {
    flex: 1;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    text-align: left;
  }

  .history-entry-copy {
    flex: 1;
    min-width: 0;
  }

  .history-entry-title {
    font-weight: 600;
    font-size: 1rem;
  }

  .history-entry-meta {
    font-size: 0.85rem;
    color: #9b9b9b;
    margin-top: 0.25rem;
  }

  .history-entry-excerpt {
    margin: 0.5rem 0 0;
    color: #d0d0d7;
    line-height: 1.45;
  }

  .history-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.75rem;
  }

  .history-tag {
    font-size: 0.75rem;
    color: #c9c9cf;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
  }

  .history-tag.attention-tag {
    color: #ffcc80;
  }

  .icon-btn {
    width: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #ff4d4d;
    opacity: 0.6;
    transition: opacity 0.2s, background 0.2s;
  }

  .icon-btn:hover:not(:disabled) {
    opacity: 1;
    background: rgba(255, 255, 255, 0.06);
  }

  .history-delete:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .history-detail {
    padding: 1rem;
    background: rgba(255, 255, 255, 0.01);
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  .history-detail-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    font-size: 0.8rem;
    color: #9b9b9b;
    margin-bottom: 0.75rem;
  }

  .detail-path {
    margin: 0 0 0.75rem;
    font-size: 0.8rem;
    color: #9b9b9b;
    word-break: break-all;
  }

  .history-detail pre {
    white-space: pre-wrap;
    font-family: inherit;
    font-size: 0.95rem;
    color: #ececf1;
    margin: 0;
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

  .voice-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
