<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { app, isActive, toast, type View } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
  import iconUrl from "../assets/icon.png";

  const links = [
    { label: "Sitio web", url: "https://iurefficient.com", icon: "globe" },
    { label: "Demo", url: "https://demo.iurefficient.com", icon: "demo" },
    { label: "YouTube", url: "https://youtube.com/@iurefficient", icon: "youtube" },
  ];
  function go(url: string) {
    openUrl(url).catch((e) => toast(`No se pudo abrir ${url}: ${e}`, "error"));
  }
  const items: { id: View; label: string; icon: string }[] = [
    { id: "ocr", label: "Reconocer texto", icon: "scan" },
    { id: "settings", label: "Ajustes", icon: "settings" },
  ];
  let pending = $derived(app.jobs.filter((j) => j.status === "queued" || isActive(j)).length);
</script>

<aside class="sidebar">
  <div class="brand">
    <img class="logo" src={iconUrl} alt="" width="40" height="40" />
    <div>
      <div class="name">IureOCR</div>
      <div class="ver">v{app.sys?.version ?? ""}</div>
    </div>
  </div>

  <nav>
    {#each items as it}
      <button class:active={app.view === it.id} onclick={() => (app.view = it.id)}>
        <Icon name={it.icon} />
        <span>{it.label}</span>
        {#if it.id === "ocr" && pending > 0}<span class="badge">{pending}</span>{/if}
      </button>
    {/each}
  </nav>

  <div class="links">
    <div class="links-title">Iurefficient</div>
    {#each links as l}
      <button onclick={() => go(l.url)} title={l.url}>
        <Icon name={l.icon} size={15} />
        <span>{l.label}</span>
        <Icon name="external" size={12} />
      </button>
    {/each}
  </div>

  {#if app.updateNotice}
    <button class="update" onclick={() => openUrl(app.updateNotice!.url)} title="Abrir la página de descarga">
      <Icon name="download" size={14} />
      <span>Nueva versión {app.updateNotice.version}</span>
    </button>
  {/if}
  <div class="foot">
    <button class="row conn" class:ok={app.iureSession?.loggedIn} onclick={() => (app.view = "settings")} title={app.iureSession?.loggedIn ? "Conectado a Iurefficient" : "Conectar con Iurefficient"}>
      <Icon name="cloud" size={15} />
      <span>{app.iureSession?.loggedIn ? `Iurefficient: ${app.iureSession.name ?? "conectado"}` : "Conectar con Iurefficient"}</span>
    </button>
    <button class="row conn" class:ok={!!app.sys?.tesseract} class:bad={!app.sys?.tesseract} onclick={() => (app.view = "settings")} title={app.sys?.tesseract ? app.sys.tesseract.exe : (app.sys?.tesseractError ?? "")}>
      <Icon name="text" size={15} />
      <span>{app.sys?.tesseract ? `Tesseract ${app.sys.tesseract.version}` : "Tesseract no encontrado"}</span>
    </button>
  </div>
</aside>

<style>
  .sidebar { display: flex; flex-direction: column; background: var(--surface); border-right: 1px solid var(--border); padding: 18px 12px; gap: 16px; }
  .brand { display: flex; align-items: center; gap: 10px; padding: 4px 8px; }
  .logo { width: 40px; height: 40px; border-radius: 10px; flex-shrink: 0; }
  .name { font-weight: 700; font-size: 15px; letter-spacing: -0.01em; }
  .ver { font-size: 11.5px; color: var(--muted); }
  nav { display: flex; flex-direction: column; gap: 2px; }
  nav button { display: flex; align-items: center; gap: 10px; padding: 9px 10px; border-radius: 9px; color: var(--text-2); font-weight: 550; text-align: left; }
  nav button:hover { background: var(--surface-2); }
  nav button.active { background: var(--accent-soft); color: var(--accent); }
  .badge { margin-left: auto; font-size: 11px; background: var(--accent); color: var(--accent-text); border-radius: 999px; padding: 1px 7px; }
  .links { margin-top: auto; display: flex; flex-direction: column; gap: 1px; padding: 6px 0; }
  .links-title { font-size: 11px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--muted); padding: 4px 10px 6px; }
  .links button { display: flex; align-items: center; gap: 8px; padding: 6px 10px; border-radius: 8px; color: var(--text-2); font-size: 13px; text-align: left; }
  .links button:hover { background: var(--surface-2); }
  .links button span { flex: 1; }
  .update { display: flex; align-items: center; gap: 8px; margin: 0 4px 6px; padding: 8px 10px; border-radius: 9px; background: var(--accent); color: var(--accent-text); font-weight: 650; font-size: 13px; }
  .update:hover { background: var(--accent-hover); }
  .foot { display: flex; flex-direction: column; gap: 4px; padding: 10px 8px 0; border-top: 1px solid var(--border); font-size: 12.5px; color: var(--muted); }
  .row { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .row span { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .conn { text-align: left; color: var(--accent); font-weight: 600; padding: 4px 0; }
  .conn.ok { color: var(--success); font-weight: 550; }
  .conn.bad { color: var(--danger); }
  .conn:hover { text-decoration: underline; }
</style>
