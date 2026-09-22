<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { app, refreshApps, refreshIureSession, refreshSystem, saveSettings, toast } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  let s = $derived(app.settings!);
  let loginPassword = $state("");
  let loginTotp = $state("");
  let totpToken = $state<string | null>(null);
  let loggingIn = $state(false);
  let showIurePass = $state(false);
  let iureTesting = $state(false);
  let iureResult = $state<{ ok: boolean; text: string } | null>(null);
  let checkingUpdates = $state(false);

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  function debounced(patch: Parameters<typeof saveSettings>[0]) {
    Object.assign(s, patch);
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => saveSettings({}), 400);
  }

  const knownLangs: Record<string, string> = { spa: "Español", eng: "Inglés", fra: "Francés", por: "Portugués", deu: "Alemán", ita: "Italiano", cat: "Catalán" };
  let available = $derived(app.sys?.tesseract?.languages.filter((l) => l !== "osd") ?? ["spa", "eng"]);
  let selected = $derived(s.languages.split("+").filter(Boolean));
  function toggleLang(l: string) {
    const next = selected.includes(l) ? selected.filter((x) => x !== l) : [...selected, l];
    if (!next.length) return;
    saveSettings({ languages: next.join("+") });
  }

  async function login() {
    loggingIn = true;
    clearTimeout(saveTimer);
    await saveSettings({});
    try {
      const r = await api.iureLogin(loginPassword, totpToken ? loginTotp : undefined, totpToken ?? undefined);
      if (r.requiresTotp) {
        totpToken = r.totpToken;
        toast("Introduce el código de verificación en dos pasos", "info");
      } else if (r.loggedIn) {
        loginPassword = "";
        loginTotp = "";
        totpToken = null;
        await refreshIureSession();
        toast(`Conectado como ${r.name ?? s.iureEmail}`, "success");
      }
    } catch (e) {
      toast(String(e), "error", 8000);
    } finally {
      loggingIn = false;
    }
  }
  async function logout() {
    await api.iureLogout();
    await refreshIureSession();
  }
  async function testIure() {
    iureTesting = true;
    iureResult = null;
    try {
      const r = await api.iureTestConnection();
      iureResult = { ok: true, text: `Acceso correcto. Carpetas: ${r.rootFolders.join(", ") || "(vacío)"}` };
    } catch (e) {
      iureResult = { ok: false, text: String(e) };
    } finally {
      iureTesting = false;
    }
  }
  async function pickOutputDir() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") await saveSettings({ outputDir: dir, outputMode: "custom" });
  }
  async function checkUpdates() {
    checkingUpdates = true;
    try {
      const m = await import("../lib/updater");
      await m.checkForUpdates(false);
    } finally {
      checkingUpdates = false;
    }
  }
  onMount(() => {
    if (!app.apps || app.apps.some((a) => a.latestVersion === null)) void refreshApps(true);
  });
</script>

<header class="top">
  <div>
    <h1>Ajustes</h1>
    <p class="hint">Los cambios se guardan automáticamente.</p>
  </div>
</header>

<div class="content scroll">
  <section class="card">
    <h2><Icon name="scan" size={17} /> Reconocimiento</h2>
    <div class="field">
      <span class="label">Idiomas del texto</span>
      <div class="chips">
        {#each available as l}
          <button class="chip" class:on={selected.includes(l)} onclick={() => toggleLang(l)}>{knownLangs[l] ?? l}</button>
        {/each}
      </div>
      <p class="hint">Marca los idiomas que aparecen en los documentos. Más idiomas = más lento y algo menos preciso.</p>
    </div>
    <div class="grid2">
      <div class="field">
        <label for="dpi">Resolución de lectura</label>
        <select id="dpi" class="input" value={String(s.dpi)} onchange={(e) => saveSettings({ dpi: Number((e.target as HTMLSelectElement).value) })}>
          <option value="200">200 ppp · rápido</option>
          <option value="300">300 ppp · recomendado</option>
          <option value="400">400 ppp · letra pequeña</option>
        </select>
      </div>
      <div class="field">
        <label for="jpeg">Calidad de las páginas en el PDF</label>
        <select id="jpeg" class="input" value={String(s.jpegQuality)} onchange={(e) => saveSettings({ jpegQuality: Number((e.target as HTMLSelectElement).value) })}>
          <option value="60">Compacta</option>
          <option value="80">Equilibrada</option>
          <option value="92">Alta (archivos grandes)</option>
        </select>
      </div>
    </div>
    <div class="switchrow">
      <div>
        <span class="label">Omitir los PDF que ya tienen texto</span>
        <p class="hint">Volver a rasterizarlos sólo los haría más pesados. Puedes forzar el OCR por archivo.</p>
      </div>
      <button class="switch" class:on={s.skipIfText} aria-label="Omitir con texto" onclick={() => saveSettings({ skipIfText: !s.skipIfText })}></button>
    </div>
  </section>

  <section class="card">
    <h2><Icon name="folder" size={17} /> Dónde guardar el resultado</h2>
    <div class="grid2">
      <div class="field">
        <label for="omode">Carpeta</label>
        <select id="omode" class="input" value={s.outputMode} onchange={(e) => saveSettings({ outputMode: (e.target as HTMLSelectElement).value as "same" | "custom" })}>
          <option value="same">Junto al archivo original</option>
          <option value="custom">Una carpeta fija</option>
        </select>
      </div>
      <div class="field">
        <label for="suffix">Sufijo del archivo</label>
        <input id="suffix" class="input" value={s.suffix} oninput={(e) => debounced({ suffix: (e.target as HTMLInputElement).value })} placeholder=" - OCR" />
      </div>
    </div>
    {#if s.outputMode === "custom"}
      <div class="row">
        <code class="path">{s.outputDir ?? "(sin elegir)"}</code>
        <button class="btn sm" onclick={pickOutputDir}><Icon name="folder" size={14} /> Elegir carpeta</button>
      </div>
      <p class="hint">Consejo: la unidad de IureDav es una buena carpeta fija: lo reconocido queda en Iurefficient sin subir nada a mano.</p>
    {/if}
  </section>

  <section class="card iure">
    <div class="iure-head">
      <h2><Icon name="cloud" size={17} /> Cuenta de Iurefficient</h2>
      {#if app.iureSession?.loggedIn}
        <span class="pill success"><Icon name="check" size={12} stroke={3} /> Conectado como {app.iureSession.name ?? app.iureSession.email}</span>
      {:else}
        <span class="pill">No conectado</span>
      {/if}
    </div>
    <p class="hint">La misma cuenta que usan IureDav, IureTranscribe e IureEditor en este equipo: si ya iniciaste sesión en una, aquí aparece sola. La contraseña no se guarda, sólo la sesión en el llavero del sistema.</p>
    <div class="grid2">
      <div class="field">
        <label for="iure-domain">Dominio de la instancia</label>
        <input id="iure-domain" class="input" placeholder="p. ej. 2.ds.iurefficient.com" value={s.iureDomain} oninput={(e) => debounced({ iureDomain: (e.target as HTMLInputElement).value.trim() })} spellcheck="false" disabled={app.iureSession?.loggedIn} />
      </div>
      <div class="field">
        <label for="iure-email">Correo de usuario</label>
        <input id="iure-email" class="input" type="email" placeholder="tu@despacho.com" value={s.iureEmail} oninput={(e) => debounced({ iureEmail: (e.target as HTMLInputElement).value.trim() })} spellcheck="false" disabled={app.iureSession?.loggedIn} />
      </div>
    </div>
    {#if app.iureSession?.loggedIn}
      <div class="row"><button class="btn sm ghost" onclick={logout}><Icon name="x" size={14} /> Cerrar sesión</button></div>
    {:else}
      <div class="row">
        {#if totpToken}
          <input class="input login-input" placeholder="Código de verificación (6 dígitos)" bind:value={loginTotp} inputmode="numeric" autocomplete="one-time-code" onkeydown={(e) => e.key === "Enter" && login()} />
        {:else}
          <input class="input login-input" type="password" placeholder="Contraseña de Iurefficient" bind:value={loginPassword} autocomplete="current-password" onkeydown={(e) => e.key === "Enter" && login()} />
        {/if}
        <button class="btn primary" onclick={login} disabled={loggingIn || !s.iureDomain || !s.iureEmail || (totpToken ? loginTotp.length < 6 : !loginPassword)}>
          {#if loggingIn}<span class="spin"><Icon name="loader" size={15} /></span>{:else}<Icon name="key" size={15} />{/if} {totpToken ? "Verificar" : "Conectar"}
        </button>
      </div>
      {#if app.iureSession?.error && s.iureDomain && s.iureEmail}<p class="hint errmsg">{app.iureSession.error}</p>{/if}
    {/if}
    <details class="advanced">
      <summary>Opciones avanzadas: acceso WebDAV (destino «Carpeta», compartido con IureDav)</summary>
      <p class="hint">No hace falta con la sesión iniciada. Sirve para guardar en cualquier carpeta del árbol de documentos con una contraseña de aplicación (<code>iurdav_…</code>). Vacía el campo para eliminarla del llavero.</p>
      <div class="row">
        <input class="input login-input" type={showIurePass ? "text" : "password"} placeholder="iurdav_…" value={s.iureAppPassword} oninput={(e) => debounced({ iureAppPassword: (e.target as HTMLInputElement).value.trim() })} autocomplete="off" spellcheck="false" />
        <button class="btn icon ghost" onclick={() => (showIurePass = !showIurePass)} title={showIurePass ? "Ocultar" : "Mostrar"}><Icon name={showIurePass ? "eyeOff" : "eye"} size={16} /></button>
        <button class="btn sm" onclick={testIure} disabled={iureTesting || !s.iureDomain || !s.iureEmail}>
          {#if iureTesting}<span class="spin"><Icon name="loader" size={15} /></span>{:else}<Icon name="check" size={15} />{/if} Probar acceso
        </button>
      </div>
      {#if iureResult}<p class="hint" class:okmsg={iureResult.ok} class:errmsg={!iureResult.ok}>{iureResult.text}</p>{/if}
    </details>
  </section>

  <section class="card">
    <h2><Icon name="apps" size={17} /> Apps de Iurefficient</h2>
    <p class="hint">Las otras herramientas de escritorio de Iurefficient en este equipo.</p>
    <div class="apps">
      {#each app.apps ?? [] as a (a.id)}
        <div class="app-row">
          <div class="app-info">
            <strong>{a.name}</strong>
            <p class="hint">{a.description}</p>
          </div>
          {#if a.installed}
            <button class="btn sm" onclick={() => api.launchApp(a.id).catch((e) => toast(String(e), "error"))}><Icon name="external" size={14} /> Abrir</button>
          {:else}
            <button class="btn sm primary" onclick={() => openUrl(a.downloadUrl)}><Icon name="download" size={14} /> Descargar{a.latestVersion ? ` ${a.latestVersion}` : ""}</button>
          {/if}
        </div>
      {/each}
      <div class="app-row">
        <div class="app-info">
          <strong>OnlyOffice Desktop Editors</strong>
          <p class="hint">Para editar los PDF y documentos. No es de Iurefficient: IureOCR sólo lo detecta y lo abre.</p>
        </div>
        {#if app.onlyoffice?.installed}
          <span class="pill success"><Icon name="check" size={12} stroke={3} /> Instalado</span>
        {:else}
          <button class="btn sm primary" onclick={() => openUrl(app.onlyoffice?.downloadUrl ?? "https://www.onlyoffice.com/es/download-desktop.aspx")}><Icon name="download" size={14} /> Descargar</button>
        {/if}
      </div>
    </div>
  </section>

  <section class="card">
    <h2><Icon name="settings" size={17} /> Aplicación</h2>
    <div class="grid2">
      <div class="field">
        <label for="theme">Tema</label>
        <select id="theme" class="input" value={s.theme} onchange={(e) => saveSettings({ theme: (e.target as HTMLSelectElement).value as "system" | "light" | "dark" })}>
          <option value="system">Como el sistema</option>
          <option value="light">Claro</option>
          <option value="dark">Oscuro</option>
        </select>
      </div>
      <div class="field">
        <span class="label">Actualizaciones</span>
        <div class="row">
          <button class="switch" class:on={s.checkUpdates} aria-label="Avisar de versiones nuevas" onclick={() => saveSettings({ checkUpdates: !s.checkUpdates })}></button>
          <span class="hint">Avisar de versiones nuevas al arrancar</span>
          <button class="btn sm ghost" onclick={checkUpdates} disabled={checkingUpdates}><Icon name="refresh" size={14} /> Buscar ahora</button>
        </div>
      </div>
    </div>
    <div class="sys">
      {#if app.sys?.tesseract}
        <p class="hint"><strong>Tesseract {app.sys.tesseract.version}</strong>{app.sys.tesseract.bundled ? " (incluido en la app)" : ""} en <code>{app.sys.tesseract.exe}</code><br />Idiomas disponibles: {app.sys.tesseract.languages.join(", ")}{app.sys.tesseract.tessdata ? ` · modelos en ${app.sys.tesseract.tessdata}` : ""}</p>
      {:else}
        <p class="hint errmsg">{app.sys?.tesseractError} <button class="btn sm ghost" onclick={refreshSystem}><Icon name="refresh" size={13} /> Volver a buscar</button></p>
      {/if}
      <p class="hint">Ajustes en: <code>{app.sys?.settingsPath}</code>{#if app.sys?.logPath}<br />Registro en: <code>{app.sys.logPath}</code>{/if}</p>
    </div>
  </section>
</div>

<style>
  .top { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; padding: 22px 26px 14px; }
  .content { flex: 1; padding: 0 26px 26px; display: flex; flex-direction: column; gap: 14px; }
  section.card { padding: 16px 18px; display: flex; flex-direction: column; gap: 12px; }
  section h2 { display: flex; align-items: center; gap: 8px; }
  .grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; }
  .chip { padding: 5px 12px; border-radius: 999px; background: var(--surface-2); color: var(--text-2); font-weight: 550; font-size: 13px; border: 1px solid transparent; }
  .chip.on { background: var(--accent-soft); color: var(--accent); border-color: var(--accent); }
  .switchrow { display: flex; align-items: center; justify-content: space-between; gap: 14px; }
  .path { font-family: var(--mono); font-size: 12.5px; background: var(--surface-2); padding: 4px 8px; border-radius: 6px; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .iure-head { display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap; }
  .login-input { max-width: 320px; }
  .errmsg { color: var(--danger); }
  .okmsg { color: var(--success); }
  .advanced summary { cursor: pointer; font-size: 13px; color: var(--text-2); font-weight: 550; }
  .advanced { display: flex; flex-direction: column; gap: 8px; }
  .apps { display: flex; flex-direction: column; gap: 8px; }
  .app-row { display: flex; align-items: center; gap: 12px; padding: 8px 0; border-top: 1px solid var(--border); }
  .app-info { flex: 1; min-width: 0; }
  .sys { display: flex; flex-direction: column; gap: 6px; border-top: 1px solid var(--border); padding-top: 10px; }
  .sys code { font-family: var(--mono); font-size: 12px; user-select: text; }
  .spin { display: inline-flex; animation: spin 1s linear infinite; }
</style>
