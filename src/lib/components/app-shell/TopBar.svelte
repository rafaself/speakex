<div class="top-bar">
  <button class="model-selector" type="button">
    SpeakEx
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="m6 9 6 6 6-6" />
    </svg>
  </button>

  <div class="auth-buttons">
    <button class="login-btn" type="button">Log in</button>
    <button class="signup-btn" type="button">Sign up for free</button>
  </div>
</div>

<style>
  .top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1.5rem;
    z-index: 10;
  }

  .model-selector {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-weight: 600;
    font-size: 1.1rem;
    padding: 0.5rem;
    border-radius: 8px;
    transition: background 0.2s;
  }

  .model-selector:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .auth-buttons {
    display: flex;
    gap: 0.75rem;
  }

  .login-btn {
    padding: 0.4rem 0.8rem;
    border-radius: 999px;
    background: #fff;
    color: #000;
    font-weight: 600;
    font-size: 0.85rem;
  }

  .signup-btn {
    padding: 0.4rem 0.8rem;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    font-weight: 600;
    font-size: 0.85rem;
    color: #fff;
  }
</style>
