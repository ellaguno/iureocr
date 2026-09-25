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
    { id: "ocr", label: "Documentos", icon: "doc" },
    { id: "settings", label: "Ajustes", icon: "settings" },
  ];
  let pending = $derived(app.jobs.filter((j) => j.status === "queued" || isActive(j)).length);

  // Colapsado: se recuerda la preferencia en ventanas anchas y se fuerza al estrechar la ventana.
  const narrow = window.matchMedia("(max-width: 1000px)");
  let pref = (() => {
    try {
      return localStorage.getItem("sidebarCollapsed") === "1";
    } catch {
      return false;
    }
  })();
  let collapsed = $state(narrow.matches || pref);
  $effect(() => {
    const onChange = (e: MediaQueryListEvent) => (collapsed = e.matches || pref);
    narrow.addEventListener("change", onChange);
    return () => narrow.removeEventListener("change", onChange);
  });
  function toggle() {
    collapsed = !collapsed;
    if (narrow.matches) return;
    pref = collapsed;
    try {
      localStorage.setItem("sidebarCollapsed", pref ? "1" : "0");
    } catch { /* sin almacenamiento */ }
  }
</script>

<aside class="sidebar" class:collapsed>
  <div class="brand">
    <img class="logo" src={iconUrl} alt="" width="40" height="40" />
    <div class="label">
      <div class="name">IureOCR</div>
      <div class="ver">v{app.sys?.version ?? ""}</div>
    </div>
    <button class="toggle" onclick={toggle} title={collapsed ? "Expandir la barra lateral" : "Colapsar la barra lateral"} aria-expanded={!collapsed}>
      <Icon name="panel" size={16} />
    </button>
  </div>

  <nav>
    {#each items as it}
      <button class:active={app.view === it.id} onclick={() => (app.view = it.id)} title={collapsed ? it.label : undefined}>
        <Icon name={it.icon} />
        <span class="label">{it.label}</span>
        {#if it.id === "ocr" && pending > 0}<span class="badge">{pending}</span>{/if}
      </button>
    {/each}
  </nav>

  <div class="links">
    <div class="links-title label">Iurefficient</div>
    {#each links as l}
      <button onclick={() => go(l.url)} title={collapsed ? `${l.label}: ${l.url}` : l.url}>
        <Icon name={l.icon} size={15} />
        <span class="label">{l.label}</span>
        <span class="label ext"><Icon name="external" size={12} /></span>
      </button>
    {/each}
  </div>

  {#if app.updateNotice}
    <button class="update" onclick={() => openUrl(app.updateNotice!.url)} title={collapsed ? `Nueva versión ${app.updateNotice.version}` : "Abrir la página de descarga"}>
      <Icon name="download" size={14} />
      <span class="label">Nueva versión {app.updateNotice.version}</span>
    </button>
  {/if}
  <div class="foot">
    <button class="row conn" class:ok={app.iureSession?.loggedIn} onclick={() => (app.view = "settings")} title={app.iureSession?.loggedIn ? "Conectado a Iurefficient" : "Conectar con Iurefficient"}>
      <Icon name="cloud" size={15} />
      <span class="label">{app.iureSession?.loggedIn ? `Iurefficient: ${app.iureSession.name ?? "conectado"}` : "Conectar con Iurefficient"}</span>
    </button>
    <button class="row conn" class:ok={!!app.sys?.tesseract} class:bad={!app.sys?.tesseract} onclick={() => (app.view = "settings")} title={app.sys?.tesseract ? app.sys.tesseract.exe : (app.sys?.tesseractError ?? "")}>
      <Icon name="text" size={15} />
      <span class="label">{app.sys?.tesseract ? `Tesseract ${app.sys.tesseract.version}` : "Tesseract no encontrado"}</span>
    </button>
  </div>
</aside>

<style>
  .sidebar { width: 224px; display: flex; flex-direction: column; background: var(--surface); border-right: 1px solid var(--border); padding: 18px 12px; gap: 16px; overflow-x: hidden; overflow-y: auto; transition: width 0.18s ease; }
  .toggle { margin-left: auto; padding: 6px; border-radius: 8px; color: var(--muted); display: grid; place-items: center; }
  .toggle:hover { background: var(--surface-2); color: var(--accent); }
  .sidebar.collapsed { width: 64px; padding: 18px 8px; }
  .collapsed .label { display: none; }
  .collapsed .brand { flex-direction: column; padding: 4px 0; gap: 8px; }
  .collapsed .toggle { margin-left: 0; }
  .collapsed nav button, .collapsed .links button, .collapsed .update { justify-content: center; position: relative; }
  .collapsed .update { margin: 0 0 6px; }
  .collapsed .badge { position: absolute; top: 2px; right: 2px; margin: 0; padding: 0 5px; font-size: 10px; }
  .collapsed .foot { padding: 10px 0 0; align-items: center; }
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
  .links button span:not(.ext) { flex: 1; }
  .links .ext { opacity: 0.5; display: flex; }
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
