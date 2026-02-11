<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let serverUrl = $state("");
  let error = $state("");
  let loading = $state(true);
  let showSettings = $state(false);

  onMount(async () => {
    // Listen for settings open from tray menu
    await listen("open-settings", () => {
      showSettings = true;
    });

    // Check if server URL is already configured
    try {
      const url = await invoke<string | null>("get_server_url");
      if (url) {
        serverUrl = url;
        // Server URL exists — Rust side navigates the webview.
        // This Svelte UI is only visible before that navigation completes
        // or when the user opens settings.
        loading = false;
      } else {
        showSettings = true;
        loading = false;
      }
    } catch {
      showSettings = true;
      loading = false;
    }
  });

  async function connect(event: Event) {
    event.preventDefault();
    error = "";

    let url = serverUrl.trim();
    if (!url) {
      error = "Please enter a server URL.";
      return;
    }

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

    try {
      await invoke("set_server_url", { url });
      // The webview will navigate to the server URL — this UI disappears
    } catch (e) {
      error = `Failed to connect: ${e}`;
    }
  }
</script>

{#if loading}
  <main class="container">
    <p>Loading…</p>
  </main>
{:else if showSettings}
  <main class="container">
    <h1>Chatto</h1>
    <p class="subtitle">Connect to your Chatto server</p>

    <form onsubmit={connect}>
      <input
        type="text"
        bind:value={serverUrl}
        placeholder="https://dev.chatto.run"
        spellcheck="false"
        autocomplete="off"
      />
      <button type="submit">Connect</button>
    </form>

    {#if error}
      <p class="error">{error}</p>
    {/if}
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
  }

  .container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem;
  }

  h1 {
    font-size: 2rem;
    margin: 0 0 0.25rem;
  }

  .subtitle {
    color: #666;
    margin: 0 0 2rem;
  }

  form {
    display: flex;
    gap: 0.5rem;
    width: 100%;
    max-width: 480px;
  }

  input {
    flex: 1;
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

  button {
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

  button:hover {
    background: #4f46e5;
  }

  .error {
    color: #ef4444;
    margin-top: 1rem;
    font-size: 0.875rem;
  }
</style>
