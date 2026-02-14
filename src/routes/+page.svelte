<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  let serverUrl = $state("");
  let error = $state("");
  let loading = $state(true);
  let connecting = $state(false);
  let showSettings = $state(false);
  let notificationsEnabled = $state(true);
  let autostartEnabled = $state(false);
  let autostartAvailable = $state(false);

  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    const params = new URLSearchParams(window.location.search);
    const forceSettings = params.has("settings");

    unlisten = await listen("open-settings", () => {
      showSettings = true;
      connecting = false;
      loadPreferences();
    });

    try {
      const url = await invoke<string | null>("get_server_url");
      if (url && !forceSettings) {
        serverUrl = url;
        loading = false;
      } else {
        serverUrl = url ?? "";
        showSettings = true;
        loading = false;
        await loadPreferences();
      }
    } catch {
      showSettings = true;
      loading = false;
    }
  });

  onDestroy(() => {
    unlisten?.();
  });

  async function loadPreferences() {
    try {
      notificationsEnabled = await invoke<boolean>("get_notifications_enabled");
    } catch {
      // defaults are fine
    }
    try {
      autostartEnabled = await invoke<boolean>("get_autostart_enabled");
      autostartAvailable = true;
    } catch {
      // autostart not available (mobile)
      autostartAvailable = false;
    }
  }

  async function connect(event: Event) {
    event.preventDefault();
    error = "";

    let url = serverUrl.trim() || "https://dev.chatto.run";

    // Add https:// if no protocol specified
    if (!/^https?:\/\//i.test(url)) {
      url = "https://" + url;
    }

    try {
      new URL(url);
    } catch {
      error = "Invalid URL format.";
      return;
    }

    connecting = true;

    try {
      await invoke("set_server_url", { url });
      // The webview will navigate to the server URL — this UI disappears
    } catch (e) {
      error = `${e}`;
      connecting = false;
    }
  }

  async function toggleNotifications() {
    notificationsEnabled = !notificationsEnabled;
    try {
      await invoke("set_notifications_enabled", { enabled: notificationsEnabled });
    } catch (e) {
      notificationsEnabled = !notificationsEnabled;
      error = `Failed to update notifications: ${e}`;
    }
  }

  async function toggleAutostart() {
    autostartEnabled = !autostartEnabled;
    try {
      await invoke("set_autostart_enabled", { enabled: autostartEnabled });
    } catch (e) {
      autostartEnabled = !autostartEnabled;
      error = `Failed to update autostart: ${e}`;
    }
  }
</script>

{#if loading}
  <main class="container">
    <img src="/icon.png" alt="Chatto" class="icon icon-pulse" width="96" height="96" />
  </main>
{:else if showSettings || connecting}
  <main class="container">
    <img src="/icon.png" alt="Chatto" class="icon" width="80" height="80" />
    <h1>Chatto</h1>
    <p class="subtitle">Desktop Settings</p>

    <div class="settings">
      <section>
        <h2>Server</h2>
        <form onsubmit={connect}>
          <input
            type="url"
            bind:value={serverUrl}
            placeholder="https://dev.chatto.run"
            spellcheck="false"
            autocomplete="off"
            autocapitalize="off"
            disabled={connecting}
          />
          <button type="submit" disabled={connecting}>
            {connecting ? "Connecting…" : "Connect"}
          </button>
        </form>
      </section>

      <section>
        <h2>Preferences</h2>
        <label class="toggle-row">
          <span>Notifications</span>
          <button
            class="toggle"
            class:active={notificationsEnabled}
            onclick={toggleNotifications}
            role="switch"
            aria-checked={notificationsEnabled}
            aria-label="Toggle notifications"
          >
            <span class="toggle-knob"></span>
          </button>
        </label>
        {#if autostartAvailable}
        <label class="toggle-row">
          <span>Start at Login</span>
          <button
            class="toggle"
            class:active={autostartEnabled}
            onclick={toggleAutostart}
            role="switch"
            aria-checked={autostartEnabled}
            aria-label="Toggle start at login"
          >
            <span class="toggle-knob"></span>
          </button>
        </label>
        {/if}
      </section>
      {#if error}
        <p class="error">{error}</p>
      {/if}
    </div>
  </main>
{/if}

<style>
  :root {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 16px;
    color: #1a1a1a;
    background: #fafafa;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #e8e8e8;
      background: #1a1a1a;
    }
    input {
      color: #e8e8e8;
      background: #2a2a2a;
      border-color: #444;
    }
    input:focus {
      border-color: #6366f1;
    }
    h2 {
      color: #999;
    }
    .toggle-row {
      border-color: #333;
    }
  }

  .container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem;
  }

  .icon {
    border-radius: 20px;
    margin-bottom: 1.5rem;
  }

  .icon-pulse {
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.7; transform: scale(0.96); }
  }

  h1 {
    font-size: 2rem;
    margin: 0 0 0.25rem;
  }

  h2 {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #666;
    margin: 0 0 0.75rem;
  }

  .subtitle {
    color: #666;
    margin: 0 0 2rem;
  }

  .settings {
    width: 100%;
    max-width: 480px;
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  section {
    display: flex;
    flex-direction: column;
  }

  form {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  input {
    flex: 1 1 100%;
    min-width: 0;
    padding: 0.75rem 1rem;
    border: 1px solid #ccc;
    border-radius: 8px;
    font-size: 1rem;
    outline: none;
    transition: border-color 0.2s;
  }

  input:focus {
    border-color: #6366f1;
  }

  button[type="submit"] {
    width: 100%;
    padding: 0.75rem 1.5rem;
    background: #6366f1;
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  button[type="submit"]:hover:not(:disabled) {
    background: #4f46e5;
  }

  button[type="submit"]:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .error {
    color: #ef4444;
    margin-top: 0.5rem;
    font-size: 0.875rem;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.625rem 0;
    border-bottom: 1px solid #eee;
    cursor: pointer;
  }

  .toggle-row:last-child {
    border-bottom: none;
  }

  .toggle {
    position: relative;
    width: 44px;
    height: 24px;
    border-radius: 12px;
    border: none;
    background: #ccc;
    cursor: pointer;
    padding: 0;
    transition: background 0.2s;
  }

  .toggle.active {
    background: #6366f1;
  }

  .toggle-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: white;
    transition: transform 0.2s;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .toggle.active .toggle-knob {
    transform: translateX(20px);
  }
</style>
