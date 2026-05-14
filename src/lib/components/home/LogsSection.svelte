<script lang="ts">
  import type { ErrorLogEntry } from "$lib/native/logs";

  export let logsState: "loading" | "ready" | "error" = "loading";
  export let logsError = "";
  export let logsEntries: ErrorLogEntry[] = [];
  export let isClearingLogs = false;
  export let onClearAllLogs: () => void;

  function formatTimestamp(value: string) {
    const date = new Date(value);

    if (Number.isNaN(date.getTime())) {
      return value;
    }

    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short"
    }).format(date);
  }

  function formatScope(value: string) {
    return value.charAt(0).toUpperCase() + value.slice(1);
  }
</script>

<div class="main-content logs-layout">
  <div class="content">
    <div class="header">
      <div>
        <h2 class="heading">Logs</h2>
        <p class="muted-copy">Persisted application errors saved locally in SQLite.</p>
      </div>
      <button class="voice-btn" type="button" on:click={onClearAllLogs} disabled={isClearingLogs}>
        {isClearingLogs ? "Clearing..." : "Clear all"}
      </button>
    </div>

    {#if logsState === "loading"}
      <p class="muted-copy">Loading error logs...</p>
    {:else if logsState === "error"}
      <p class="error-copy">{logsError}</p>
    {:else if logsEntries.length === 0}
      <p class="muted-copy">No persisted errors yet.</p>
    {:else}
      <div class="logs-list">
        {#each logsEntries as entry}
          <article class="log-card">
            <div class="log-card-header">
              <div>
                <h3 class="log-summary">{entry.summary}</h3>
                <div class="log-meta">{formatTimestamp(entry.createdAt)}</div>
              </div>
              <div class="log-tags">
                <span class="log-tag">{formatScope(entry.scope)}</span>
                <span class="log-tag">{entry.source}</span>
              </div>
            </div>
            <pre class="log-detail">{entry.detail}</pre>
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

  .logs-layout {
    justify-content: flex-start;
  }

  .content {
    width: 100%;
    max-width: 880px;
    text-align: left;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .heading {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0;
  }

  .muted-copy {
    color: #b0b0b0;
    margin: 0.35rem 0 0;
  }

  .error-copy {
    color: #ff8a80;
    margin: 0;
  }

  .logs-list {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  .log-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    padding: 1rem;
  }

  .log-card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .log-summary {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .log-meta {
    margin-top: 0.3rem;
    color: #a8a8a8;
    font-size: 0.85rem;
  }

  .log-tags {
    display: flex;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .log-tag {
    font-size: 0.75rem;
    color: #f0f0f0;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
  }

  .log-detail {
    margin: 0.9rem 0 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
    font-size: 0.92rem;
    color: #e5e5eb;
    line-height: 1.5;
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
    cursor: default;
  }

  @media (max-width: 768px) {
    .header,
    .log-card-header {
      flex-direction: column;
    }
  }
</style>