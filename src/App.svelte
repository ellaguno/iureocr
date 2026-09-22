<script lang="ts">
  import { onMount } from "svelte";
  import { app, init } from "./lib/state.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import OcrView from "./components/OcrView.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import Toasts from "./components/Toasts.svelte";
  import Icon from "./components/Icon.svelte";

  let error = $state("");
  onMount(() => {
    init().catch((e) => (error = String(e)));
  });
</script>

{#if error}
  <div class="boot"><Icon name="alert" size={28} /><p>No se pudo iniciar la aplicación: {error}</p></div>
{:else if !app.ready}
  <div class="boot"><Icon name="loader" size={26} /><p>Cargando…</p></div>
{:else}
  <div class="shell">
    <Sidebar />
    <main>
      {#if app.view === "ocr"}
        <OcrView />
      {:else}
        <SettingsView />
      {/if}
    </main>
  </div>
  {#if app.dragging}
    <div class="drop-overlay">
      <div class="drop-box"><Icon name="scan" size={36} /><p>Suelta PDF o imágenes para reconocer su texto</p></div>
    </div>
  {/if}
{/if}
<Toasts />

<style>
  .shell { display: grid; grid-template-columns: 224px 1fr; height: 100vh; }
  main { min-width: 0; display: flex; flex-direction: column; overflow: hidden; }
  .boot { height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; color: var(--muted); }
  .boot :global(svg) { animation: spin 1.2s linear infinite; }
  .drop-overlay { position: fixed; inset: 0; background: color-mix(in srgb, var(--accent) 12%, transparent); backdrop-filter: blur(2px); display: grid; place-items: center; z-index: 40; pointer-events: none; }
  .drop-box { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 40px 60px; border: 2px dashed var(--accent); border-radius: 20px; background: var(--surface); color: var(--accent); font-weight: 600; box-shadow: var(--shadow); }
</style>
