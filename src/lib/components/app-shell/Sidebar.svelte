<script lang="ts">
  import type { AppSection } from "$lib/types/app-shell";

  export let currentSection: AppSection;
  export let onResetWorkspace: () => void;
  export let onSelectSection: (section: AppSection) => void;

  function handleNewChat() {
    onResetWorkspace();
  }
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    <button class="icon-btn" title="Open sidebar" type="button">
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
        <line x1="9" y1="3" x2="9" y2="21" />
      </svg>
    </button>
    <button class="icon-btn" title="New chat" type="button" on:click={handleNewChat}>
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 5v14M5 12h14" />
      </svg>
    </button>
  </div>

  <button class="new-chat-btn" type="button" on:click={handleNewChat}>
    <div class="new-chat-copy">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
        <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
      </svg>
      <span>New chat</span>
    </div>
  </button>

  <nav class="sidebar-nav" aria-label="Secondary">
    <button
      class:active={currentSection === "history"}
      class="nav-item"
      type="button"
      aria-current={currentSection === "history" ? "page" : undefined}
      on:click={() => onSelectSection("history")}
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 8v4l3 3" />
        <path d="M3.05 11a9 9 0 1 1 .5 4" />
        <path d="M3 4v7h7" />
      </svg>
      <span>History</span>
    </button>
  </nav>

  <footer class="sidebar-footer">
    <button
      class:active={currentSection === "settings"}
      class="nav-item"
      type="button"
      aria-current={currentSection === "settings" ? "page" : undefined}
      on:click={() => onSelectSection("settings")}
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
        <circle cx="12" cy="12" r="3" />
      </svg>
      <span>Settings</span>
    </button>
    <button class="nav-item" type="button">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10" />
        <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
        <line x1="12" y1="17" x2="12.01" y2="17" />
      </svg>
      <span>Help</span>
    </button>

  </footer>
</aside>

<style>
  .sidebar {
    background: #171717;
    display: flex;
    flex-direction: column;
    padding: 1rem 0.75rem;
    height: 100vh;
    box-sizing: border-box;
    color: #fff;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 0.5rem 1rem;
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

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }

  .new-chat-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 0.75rem;
    border-radius: 8px;
    font-weight: 500;
    font-size: 0.9rem;
    transition: background 0.2s;
  }

  .new-chat-btn:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .new-chat-copy {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .sidebar-nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    overflow-y: auto;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 8px;
    font-size: 0.9rem;
    color: #ececf1;
    transition: background 0.2s;
    width: 100%;
    text-align: left;
  }

  .nav-item:hover,
  .nav-item.active {
    background: rgba(255, 255, 255, 0.1);
  }

  .sidebar-footer {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    padding-top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  @media (max-width: 768px) {
    .sidebar {
      display: none;
    }
  }
</style>
