<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { app, refreshApps, refreshIureSession, refreshSystem, saveSettings, toast } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
  import { ocrLangName, t } from "../lib/i18n.svelte";

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
        toast(t("settings.enterTotp"), "info");
      } else if (r.loggedIn) {
        loginPassword = "";
        loginTotp = "";
        totpToken = null;
        await refreshIureSession();
        toast(t("settings.connectedToast", { name: r.name ?? s.iureEmail }), "success");
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
      iureResult = { ok: true, text: t("settings.accessOk", { folders: r.rootFolders.join(", ") || t("settings.empty") }) };
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
    <h1>{t("settings.title")}</h1>
    <p class="hint">{t("settings.autosave")}</p>
  </div>
</header>

<div class="content scroll">
  <section class="card">
    <h2><Icon name="settings" size={17} /> {t("settings.appearance")}</h2>
    <div class="grid2">
      <div class="field">
        <label for="ui-lang">{t("settings.language")}</label>
        <select id="ui-lang" class="input" value={s.uiLanguage} onchange={(e) => saveSettings({ uiLanguage: (e.target as HTMLSelectElement).value as "auto" | "en" | "es" })}>
          <option value="auto">{t("settings.languageAuto")}</option>
          <option value="en">English</option>
          <option value="es">Español</option>
        </select>
      </div>
      <div class="field">
        <label for="theme">{t("settings.theme")}</label>
        <select id="theme" class="input" value={s.theme} onchange={(e) => saveSettings({ theme: (e.target as HTMLSelectElement).value as "system" | "light" | "dark" })}>
          <option value="system">{t("settings.themeSystem")}</option>
          <option value="light">{t("settings.themeLight")}</option>
          <option value="dark">{t("settings.themeDark")}</option>
        </select>
      </div>
    </div>
  </section>

  <section class="card">
    <h2><Icon name="scan" size={17} /> {t("settings.recognition")}</h2>
    <div class="field">
      <span class="label">{t("settings.ocrLanguages")}</span>
      <div class="chips">
        {#each available as l}
          <button class="chip" class:on={selected.includes(l)} onclick={() => toggleLang(l)}>{ocrLangName(l)}</button>
        {/each}
      </div>
      <p class="hint">{t("settings.ocrLanguagesHint")}</p>
    </div>
    <div class="grid2">
      <div class="field">
        <label for="dpi">{t("settings.dpi")}</label>
        <select id="dpi" class="input" value={String(s.dpi)} onchange={(e) => saveSettings({ dpi: Number((e.target as HTMLSelectElement).value) })}>
          <option value="200">{t("settings.dpi200")}</option>
          <option value="300">{t("settings.dpi300")}</option>
          <option value="400">{t("settings.dpi400")}</option>
        </select>
      </div>
      <div class="field">
        <label for="jpeg">{t("settings.jpeg")}</label>
        <select id="jpeg" class="input" value={String(s.jpegQuality)} onchange={(e) => saveSettings({ jpegQuality: Number((e.target as HTMLSelectElement).value) })}>
          <option value="60">{t("settings.jpegCompact")}</option>
          <option value="80">{t("settings.jpegBalanced")}</option>
          <option value="92">{t("settings.jpegHigh")}</option>
        </select>
      </div>
    </div>
    <div class="switchrow">
      <div>
        <span class="label">{t("settings.autoOcr")}</span>
        <p class="hint">{t("settings.autoOcrHint")}</p>
      </div>
      <button class="switch" class:on={s.autoOcr} aria-label={t("settings.autoOcrAria")} onclick={() => saveSettings({ autoOcr: !s.autoOcr })}></button>
    </div>
    <div class="switchrow">
      <div>
        <span class="label">{t("settings.skipIfText")}</span>
        <p class="hint">{t("settings.skipIfTextHint")}</p>
      </div>
      <button class="switch" class:on={s.skipIfText} aria-label={t("settings.skipIfTextAria")} onclick={() => saveSettings({ skipIfText: !s.skipIfText })}></button>
    </div>
  </section>

  <section class="card">
    <h2><Icon name="folder" size={17} /> {t("settings.output")}</h2>
    <div class="grid2">
      <div class="field">
        <label for="omode">{t("settings.folder")}</label>
        <select id="omode" class="input" value={s.outputMode} onchange={(e) => saveSettings({ outputMode: (e.target as HTMLSelectElement).value as "same" | "custom" })}>
          <option value="same">{t("settings.outputSame")}</option>
          <option value="custom">{t("settings.outputCustom")}</option>
        </select>
      </div>
      <div class="field">
        <label for="suffix">{t("settings.suffix")}</label>
        <input id="suffix" class="input" value={s.suffix} oninput={(e) => debounced({ suffix: (e.target as HTMLInputElement).value })} placeholder=" - OCR" />
      </div>
    </div>
    {#if s.outputMode === "custom"}
      <div class="row">
        <code class="path">{s.outputDir ?? t("settings.notChosen")}</code>
        <button class="btn sm" onclick={pickOutputDir}><Icon name="folder" size={14} /> {t("settings.chooseFolder")}</button>
      </div>
      <p class="hint">{t("settings.outputTip")}</p>
    {/if}
  </section>

  <section class="card iure">
    <div class="iure-head">
      <h2><Icon name="cloud" size={17} /> {t("settings.account")}</h2>
      {#if app.iureSession?.loggedIn}
        <span class="pill success"><Icon name="check" size={12} stroke={3} /> {t("settings.connectedAs", { name: app.iureSession.name ?? app.iureSession.email ?? "" })}</span>
      {:else}
        <span class="pill">{t("settings.notConnected")}</span>
      {/if}
    </div>
    <p class="hint">{t("settings.accountHint")}</p>
    <div class="grid2">
      <div class="field">
        <label for="iure-domain">{t("settings.domain")}</label>
        <input id="iure-domain" class="input" placeholder={t("settings.domainPlaceholder")} value={s.iureDomain} oninput={(e) => debounced({ iureDomain: (e.target as HTMLInputElement).value.trim() })} spellcheck="false" disabled={app.iureSession?.loggedIn} />
      </div>
      <div class="field">
        <label for="iure-email">{t("settings.email")}</label>
        <input id="iure-email" class="input" type="email" placeholder={t("settings.emailPlaceholder")} value={s.iureEmail} oninput={(e) => debounced({ iureEmail: (e.target as HTMLInputElement).value.trim() })} spellcheck="false" disabled={app.iureSession?.loggedIn} />
      </div>
    </div>
    {#if app.iureSession?.loggedIn}
      <div class="row"><button class="btn sm ghost" onclick={logout}><Icon name="x" size={14} /> {t("settings.signOut")}</button></div>
    {:else}
      <div class="row">
        {#if totpToken}
          <input class="input login-input" placeholder={t("settings.totpPlaceholder")} bind:value={loginTotp} inputmode="numeric" autocomplete="one-time-code" onkeydown={(e) => e.key === "Enter" && login()} />
        {:else}
          <input class="input login-input" type="password" placeholder={t("settings.passwordPlaceholder")} bind:value={loginPassword} autocomplete="current-password" onkeydown={(e) => e.key === "Enter" && login()} />
        {/if}
        <button class="btn primary" onclick={login} disabled={loggingIn || !s.iureDomain || !s.iureEmail || (totpToken ? loginTotp.length < 6 : !loginPassword)}>
          {#if loggingIn}<span class="spin"><Icon name="loader" size={15} /></span>{:else}<Icon name="key" size={15} />{/if} {totpToken ? t("settings.verify") : t("settings.connect")}
        </button>
      </div>
      {#if app.iureSession?.error && s.iureDomain && s.iureEmail}<p class="hint errmsg">{app.iureSession.error}</p>{/if}
    {/if}
    <details class="advanced">
      <summary>{t("settings.advanced")}</summary>
      <p class="hint">{#each t("settings.advancedHint").split("{code}") as part, i}{#if i > 0}<code>iurdav_…</code>{/if}{part}{/each}</p>
      <div class="row">
        <input class="input login-input" type={showIurePass ? "text" : "password"} placeholder="iurdav_…" value={s.iureAppPassword} oninput={(e) => debounced({ iureAppPassword: (e.target as HTMLInputElement).value.trim() })} autocomplete="off" spellcheck="false" />
        <button class="btn icon ghost" onclick={() => (showIurePass = !showIurePass)} title={showIurePass ? t("settings.hide") : t("settings.show")}><Icon name={showIurePass ? "eyeOff" : "eye"} size={16} /></button>
        <button class="btn sm" onclick={testIure} disabled={iureTesting || !s.iureDomain || !s.iureEmail}>
          {#if iureTesting}<span class="spin"><Icon name="loader" size={15} /></span>{:else}<Icon name="check" size={15} />{/if} {t("settings.testAccess")}
        </button>
      </div>
      {#if iureResult}<p class="hint" class:okmsg={iureResult.ok} class:errmsg={!iureResult.ok}>{iureResult.text}</p>{/if}
    </details>
  </section>

  <section class="card">
    <h2><Icon name="apps" size={17} /> {t("settings.apps")}</h2>
    <p class="hint">{t("settings.appsHint")}</p>
    <div class="apps">
      {#each app.apps ?? [] as a (a.id)}
        <div class="app-row" class:me={a.id === "ocr"}>
          <div class="app-info">
            <strong>{a.name}</strong>
            <p class="hint">{a.description}</p>
          </div>
          {#if a.id === "ocr"}
            <span class="pill success">{t("settings.thisApp")}{app.sys?.version ? ` · ${app.sys.version}` : ""}</span>
          {:else if a.installed}
            <button class="btn sm" onclick={() => api.launchApp(a.id).catch((e) => toast(String(e), "error"))}><Icon name="external" size={14} /> {t("settings.open")}</button>
          {:else}
            <button class="btn sm primary" onclick={() => openUrl(a.downloadUrl)}><Icon name="download" size={14} /> {t("common.download")}{a.latestVersion ? ` ${a.latestVersion}` : ""}</button>
          {/if}
        </div>
      {/each}
      <div class="app-row">
        <div class="app-info">
          <strong>OnlyOffice Desktop Editors</strong>
          <p class="hint">{t("settings.onlyofficeHint")}</p>
        </div>
        {#if app.onlyoffice?.installed}
          <span class="pill success"><Icon name="check" size={12} stroke={3} /> {t("settings.installed")}</span>
        {:else}
          <button class="btn sm primary" onclick={() => openUrl(app.onlyoffice?.downloadUrl ?? "https://www.onlyoffice.com/es/download-desktop.aspx")}><Icon name="download" size={14} /> {t("common.download")}</button>
        {/if}
      </div>
    </div>
  </section>

  <section class="card">
    <h2><Icon name="settings" size={17} /> {t("settings.application")}</h2>
    <div class="grid2">
      <div class="field">
        <span class="label">{t("settings.updates")}</span>
        <div class="row">
          <button class="switch" class:on={s.checkUpdates} aria-label={t("settings.updatesAria")} onclick={() => saveSettings({ checkUpdates: !s.checkUpdates })}></button>
          <span class="hint">{t("settings.updatesHint")}</span>
          <button class="btn sm ghost" onclick={checkUpdates} disabled={checkingUpdates}><Icon name="refresh" size={14} /> {t("settings.checkNow")}</button>
        </div>
      </div>
    </div>
    <div class="sys">
      {#if app.sys?.tesseract}
        <p class="hint"><strong>Tesseract {app.sys.tesseract.version}</strong>{app.sys.tesseract.bundled ? t("settings.tesseractBundled") : ""} {t("settings.tesseractAt")} <code>{app.sys.tesseract.exe}</code><br />{t("settings.tesseractLangs", { langs: app.sys.tesseract.languages.join(", ") })}{app.sys.tesseract.tessdata ? t("settings.tesseractModels", { dir: app.sys.tesseract.tessdata }) : ""}</p>
      {:else}
        <p class="hint errmsg">{app.sys?.tesseractError} <button class="btn sm ghost" onclick={refreshSystem}><Icon name="refresh" size={13} /> {t("settings.searchAgain")}</button></p>
      {/if}
      <p class="hint">{t("settings.settingsAt")} <code>{app.sys?.settingsPath}</code>{#if app.sys?.logPath}<br />{t("settings.logAt")} <code>{app.sys.logPath}</code>{/if}</p>
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
  .app-row.me strong { color: var(--accent); }
  .sys { display: flex; flex-direction: column; gap: 6px; border-top: 1px solid var(--border); padding-top: 10px; }
  .sys code { font-family: var(--mono); font-size: 12px; user-select: text; }
  .spin { display: inline-flex; animation: spin 1s linear infinite; }
</style>
