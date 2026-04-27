<script lang="ts">
  import { onMount } from "svelte";

  import { ping } from "$lib/native/ping";

  let pingState: "idle" | "loading" | "success" | "error" = "idle";
  let pingResponse = "";
  let pingError = "";

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

  onMount(() => {
    void runPing();
  });
</script>

<main class="shell">
  <section class="panel">
    <p class="eyebrow">Release 0.1</p>
    <h1>SpeakEx</h1>
    <p class="summary">
      Local-first desktop transcription bootstrap. This screen verifies the first narrow frontend-to-Rust
      round-trip before any transcription features are added.
    </p>

    <div class="status-card" aria-live="polite">
      <div>
        <p class="label">Native bridge</p>
        <h2>Rust ping handshake</h2>
      </div>

      {#if pingState === "loading" || pingState === "idle"}
        <p class="status pending">Calling the Rust <code>ping</code> command…</p>
      {:else if pingState === "success"}
        <p class="status success">Frontend received: <strong>{pingResponse}</strong></p>
      {:else}
        <p class="status error">Ping failed: {pingError}</p>
      {/if}

      <button type="button" on:click={runPing} disabled={pingState === "loading"}>
        {pingState === "loading" ? "Pinging…" : "Run ping again"}
      </button>
    </div>
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
      radial-gradient(circle at top, rgba(59, 130, 246, 0.16), transparent 35%),
      linear-gradient(180deg, #0f172a 0%, #111827 100%);
    color: #e5eefb;
  }

  .shell {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 2rem;
    box-sizing: border-box;
  }

  .panel {
    width: min(100%, 860px);
    padding: 2rem;
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 24px;
    background: rgba(15, 23, 42, 0.78);
    box-shadow: 0 24px 60px rgba(15, 23, 42, 0.45);
    backdrop-filter: blur(16px);
  }

  .eyebrow {
    margin: 0 0 0.75rem;
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #7dd3fc;
  }

  h1 {
    margin: 0;
    font-size: clamp(2.5rem, 6vw, 4rem);
    line-height: 1;
  }

  .summary {
    margin: 1rem 0 0;
    max-width: 40rem;
    font-size: 1.05rem;
    line-height: 1.7;
    color: #cbd5e1;
  }

  .status-card {
    display: grid;
    gap: 1.25rem;
    margin-top: 2rem;
    padding: 1.5rem;
    border-radius: 18px;
    background: rgba(30, 41, 59, 0.72);
    border: 1px solid rgba(148, 163, 184, 0.18);
  }

  .label {
    margin: 0 0 0.5rem;
    color: #7dd3fc;
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  h2 {
    margin: 0;
    font-size: 1.4rem;
  }

  .status {
    margin: 0;
    line-height: 1.6;
    color: #cbd5e1;
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

  button {
    justify-self: start;
    border: 0;
    border-radius: 999px;
    padding: 0.8rem 1.1rem;
    font: inherit;
    font-weight: 700;
    color: #0f172a;
    background: linear-gradient(135deg, #7dd3fc 0%, #38bdf8 100%);
    cursor: pointer;
    transition:
      transform 0.15s ease,
      box-shadow 0.15s ease,
      opacity 0.15s ease;
    box-shadow: 0 10px 24px rgba(56, 189, 248, 0.25);
  }

  button:hover:enabled {
    transform: translateY(-1px);
  }

  button:disabled {
    opacity: 0.7;
    cursor: wait;
  }

  @media (max-width: 640px) {
    .shell {
      padding: 1rem;
    }

    .panel {
      padding: 1.5rem;
    }
  }
</style>
