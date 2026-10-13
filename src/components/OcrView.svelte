<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { addFiles, app, cancelJob, clearFinished, isActive, pendingOcr, queuedCount, removeJob, requestOcr, retryJob, startQueue, stopQueue, toast, type Job } from "../lib/state.svelte";
  import { fmtBytes, fmtSecs } from "../lib/format";
  import Icon from "./Icon.svelte";
  import { locale, t, tn } from "../lib/i18n.svelte";
  import IurePicker from "./IurePicker.svelte";
  import PagesView from "./PagesView.svelte";
  import Viewer from "./Viewer.svelte";

  let picking = $state<Job | null>(null);
  let pagesOf = $state<Job | null>(null);
  let viewing = $state<{ job: Job; page: number } | null>(null);
  let preview = $state<{ job: Job; text: string } | null>(null);

  async function pickFiles() {
    const sel = await open({ multiple: true, filters: [{ name: t("ocr.filterName"), extensions: app.sys?.supportedExtensions ?? ["pdf"] }] });
    if (!sel) return;
    await addFiles(Array.isArray(sel) ? sel : [sel]);
  }
  async function openFile(path: string) {
    try {
      await openPath(path);
    } catch (e) {
      toast(t("common.openFailed", { error: String(e) }), "error");
    }
  }
  async function reveal(path: string) {
    try {
      await api.revealPath(path);
    } catch (e) {
      toast(t("ocr.revealFailed", { error: String(e) }), "error");
    }
  }
  async function showText(job: Job) {
    if (!job.result?.txtPath) return;
    try {
      preview = { job, text: await api.readTextFile(job.result.txtPath, 20000) };
    } catch (e) {
      toast(String(e), "error");
    }
  }
  async function editWith(path: string) {
    if (!app.onlyoffice?.installed) {
      toast(t("ocr.onlyofficeMissingToast"), "info");
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(app.onlyoffice?.downloadUrl ?? "https://www.onlyoffice.com/es/download-desktop.aspx");
      return;
    }
    try {
      await api.openWithOnlyoffice(path);
    } catch (e) {
      toast(String(e), "error");
    }
  }
  function statusLabel(j: Job): string {
    switch (j.status) {
      case "queued": return t("ocr.status.queued");
      case "render": return j.total ? t("ocr.status.renderN", { current: j.current, total: j.total }) : t("ocr.status.render");
      case "ocr": return j.total ? t("ocr.status.ocrN", { current: j.current, total: j.total }) : t("ocr.status.ocr");
      case "done": return t("ocr.status.done");
      case "ready": return t("ocr.status.ready");
      case "skipped": return t("ocr.status.skipped");
      case "error": return t("ocr.status.error");
      case "cancelled": return t("ocr.status.cancelled");
    }
  }
  function pillClass(j: Job): string {
    return j.status === "done" ? "success" : j.status === "error" ? "danger" : j.status === "skipped" ? "warn" : isActive(j) ? "accent" : "";
  }
  let percent = $derived((j: Job) => (j.total ? Math.round(((j.status === "ocr" ? j.total : 0) + j.current) / (2 * j.total) * 100) : 0));
  function recognizeAll() {
    for (const j of pendingOcr()) requestOcr(j.id);
  }
  let pending = $derived(app.jobs.filter((j) => j.status === "ready").length);
  let finished = $derived(app.jobs.filter((j) => !isActive(j) && j.status !== "queued").length);
</script>

<header class="top">
  <div>
    <h1>{t("common.documents")}</h1>
    <p class="hint">{t("ocr.hint")}</p>
  </div>
  <div class="actions">
    <button class="btn primary" onclick={pickFiles}><Icon name="plus" size={16} /> {t("ocr.openFiles")}</button>
    {#if pending > 1}
      <button class="btn" onclick={recognizeAll}><Icon name="scan" size={15} /> {t("ocr.recognizeAll", { n: pending })}</button>
    {/if}
    {#if app.running}
      <button class="btn" onclick={stopQueue}><Icon name="stop" size={15} /> {t("ocr.stopAfterCurrent")}</button>
    {:else if queuedCount() > 0}
      <button class="btn" onclick={startQueue}><Icon name="play" size={15} /> {t("ocr.process", { n: queuedCount() })}</button>
    {/if}
    {#if finished > 0}
      <button class="btn ghost" onclick={clearFinished}><Icon name="trash" size={15} /> {t("ocr.clearFinished")}</button>
    {/if}
  </div>
</header>

<div class="content scroll">
  {#if !app.sys?.tesseract}
    <div class="card warn-card">
      <Icon name="alert" size={18} />
      <div>
        <strong>{t("ocr.tesseractUnavailable")}</strong>
        <p class="hint">{app.sys?.tesseractError}</p>
      </div>
    </div>
  {/if}
  {#if app.jobs.length === 0}
    <div class="empty">
      <Icon name="scan" size={40} />
      <h2>{t("ocr.emptyTitle")}</h2>
      <p class="hint">{t("ocr.emptyHint", { mode: app.settings?.autoOcr ? t("ocr.emptyAuto") : t("ocr.emptyManual"), suffix: app.settings?.suffix ?? "" })}</p>
      <button class="btn primary" onclick={pickFiles}><Icon name="plus" size={16} /> {t("ocr.openFiles")}</button>
    </div>
  {:else}
    <div class="jobs">
      {#each app.jobs as job (job.id)}
        <div class="card job" class:active={isActive(job)}>
          <div class="job-head">
            <Icon name={job.path.toLowerCase().endsWith(".pdf") ? "doc" : "image"} size={18} />
            <div class="job-title">
              <div class="name" title={job.path}>{job.name}</div>
              <div class="hint">{fmtBytes(job.sizeBytes)}{job.result && !job.result.skipped ? ` · ${tn("pages.count", job.result.pages)} · ${t("ocr.chars", { n: job.result.chars.toLocaleString(locale()) })} · ${fmtSecs(job.result.elapsedSecs)}` : ""}{job.force ? ` · ${t("ocr.forced")}` : ""}</div>
            </div>
            <span class="pill {pillClass(job)}">{#if isActive(job)}<span class="spin"><Icon name="loader" size={12} /></span>{/if}{statusLabel(job)}</span>
            {#if isActive(job) || job.status === "queued"}
              <button class="btn icon ghost" title={t("common.cancel")} onclick={() => cancelJob(job.id)}><Icon name="x" size={15} /></button>
            {:else}
              <button class="btn icon ghost" title={t("ocr.removeFromList")} onclick={() => removeJob(job.id)}><Icon name="trash" size={15} /></button>
            {/if}
          </div>
          {#if job.pages}
            <div class="strip">
              {#each Array.from({ length: Math.min(job.pages, 6) }, (_, i) => i) as i (i)}
                <button class="thumb" class:current={isActive(job) && job.total && job.current === i + 1} onclick={() => (viewing = { job, page: i })} title={t("ocr.viewPage", { n: i + 1 })}>
                  {#if job.thumbs[i]}<img src={job.thumbs[i]} alt="" />{:else}<span class="ph"></span>{/if}
                </button>
              {/each}
              {#if job.pages > 6}<button class="more" onclick={() => (viewing = { job, page: 6 })}>+{job.pages - 6}</button>{/if}
              <button class="strip-label" onclick={() => (pagesOf = job)} title={t("ocr.gridTitle")}><Icon name="layers" size={13} /> {tn("pages.count", job.pages)}{isActive(job) && job.total ? t("ocr.atPage", { n: job.current }) : ""}</button>
            </div>
          {/if}
          {#if isActive(job)}
            <div class="progress" class:indeterminate={!job.total}><div style="width: {percent(job)}%"></div></div>
          {/if}
          {#if job.error && job.status === "error"}
            <p class="err">{job.error}</p>
            <div class="row"><button class="btn sm" onclick={() => retryJob(job.id)}><Icon name="refresh" size={14} /> {t("ocr.retry")}</button></div>
          {:else if job.status === "cancelled"}
            <div class="row"><button class="btn sm" onclick={() => retryJob(job.id)}><Icon name="refresh" size={14} /> {t("ocr.requeue")}</button></div>
          {:else if job.status === "skipped"}
            <p class="hint">{job.result?.note}</p>
            <div class="row">
              <button class="btn sm" onclick={() => retryJob(job.id, true)}><Icon name="scan" size={14} /> {t("ocr.forceOcr")}</button>
              <button class="btn sm ghost" onclick={() => (viewing = { job, page: 0 })}><Icon name="eye" size={14} /> {t("common.view")}</button>
              <button class="btn sm ghost" onclick={() => (pagesOf = job)}><Icon name="layers" size={14} /> {t("common.pages")}</button>
            </div>
          {:else if job.status === "ready"}
            <div class="row">
              <button class="btn sm primary" onclick={() => (viewing = { job, page: 0 })}><Icon name="eye" size={14} /> {t("common.view")}</button>
              <button class="btn sm" onclick={() => requestOcr(job.id)}><Icon name="scan" size={14} /> {t("common.recognizeText")}</button>
              <button class="btn sm" onclick={() => (pagesOf = job)}><Icon name="layers" size={14} /> {t("common.pages")}</button>
              <button class="btn sm ghost" onclick={() => openFile(job.path)} title={t("common.openWithSystem")}><Icon name="external" size={14} /> {t("common.openOutside")}</button>
              <button class="btn sm ghost" onclick={() => reveal(job.path)}><Icon name="folder" size={14} /> {t("common.showInFolder")}</button>
              <span class="grow"></span>
              {#if job.upload}
                <span class="pill accent"><span class="spin"><Icon name="loader" size={12} /></span> {t("common.uploading")}</span>
              {:else if job.saved}
                <span class="pill success"><Icon name="check" size={12} stroke={3} /> {t("ocr.savedIn", { target: job.saved.target })}</span>
              {:else}
                <button class="btn sm" onclick={() => (picking = job)} disabled={!app.settings?.iureDomain}><Icon name="cloud" size={14} /> {t("common.saveToIure")}</button>
              {/if}
            </div>
          {:else if job.status === "done" && job.result}
            {#if job.result.note}<p class="hint warn-text">{job.result.note}</p>{/if}
            <div class="row">
              <button class="btn sm primary" onclick={() => (viewing = { job, page: 0 })}><Icon name="eye" size={14} /> {t("common.view")}</button>
              <button class="btn sm ghost" onclick={() => openFile(job.result!.pdfPath!)} title={t("common.openWithSystem")}><Icon name="external" size={14} /> {t("common.openOutside")}</button>
              <button class="btn sm" onclick={() => showText(job)}><Icon name="text" size={14} /> {t("ocr.viewText")}</button>
              <button class="btn sm" onclick={() => (pagesOf = job)}><Icon name="layers" size={14} /> {t("common.pages")}</button>
              <button class="btn sm" onclick={() => reveal(job.result!.pdfPath!)}><Icon name="folder" size={14} /> {t("common.showInFolder")}</button>
              <button class="btn sm" onclick={() => editWith(job.result!.pdfPath!)} title={app.onlyoffice?.installed ? t("ocr.openWithOnlyoffice") : t("ocr.onlyofficeMissing")}><Icon name="edit" size={14} /> {t("ocr.editWithOnlyoffice")}</button>
              <span class="grow"></span>
              {#if job.upload}
                <span class="pill accent"><span class="spin"><Icon name="loader" size={12} /></span> {t("ocr.uploadingFile", { file: job.upload.fileName, i: job.upload.index + 1, n: job.upload.totalFiles })}</span>
              {:else if job.saved}
                <span class="pill success" title={t("ocr.savedAt", { time: new Date(job.saved.at).toLocaleTimeString(locale()) })}><Icon name="check" size={12} stroke={3} /> {t("ocr.savedIn", { target: job.saved.target })}</span>
                <button class="btn sm ghost" onclick={() => (picking = job)}><Icon name="upload" size={14} /> {t("ocr.again")}</button>
              {:else}
                <button class="btn sm" onclick={() => (picking = job)} disabled={!app.settings?.iureDomain}><Icon name="cloud" size={14} /> {t("common.saveToIure")}</button>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if picking}
  <IurePicker job={picking} onclose={() => (picking = null)} />
{/if}
{#if viewing}
  <Viewer job={viewing.job} start={viewing.page} onclose={() => (viewing = null)} onpages={() => { pagesOf = viewing!.job; viewing = null; }} />
{/if}
{#if pagesOf}
  <PagesView job={pagesOf} onclose={() => (pagesOf = null)} onview={(page) => { viewing = { job: pagesOf!, page }; pagesOf = null; }} />
{/if}
{#if preview}
  <div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && (preview = null)}>
    <div class="modal card" role="dialog" aria-modal="true">
      <div class="head">
        <h2><Icon name="text" size={17} /> {preview.job.name}</h2>
        <button class="btn icon ghost" onclick={() => (preview = null)} aria-label={t("common.close")}><Icon name="x" size={16} /></button>
      </div>
      <pre class="text scroll">{preview.text}</pre>
      <div class="foot-row">
        <button class="btn sm" onclick={() => navigator.clipboard.writeText(preview!.text).then(() => toast(t("ocr.textCopied"), "success"))}><Icon name="copy" size={14} /> {t("ocr.copy")}</button>
        <button class="btn sm ghost" onclick={() => openFile(preview!.job.result!.txtPath!)}><Icon name="external" size={14} /> {t("ocr.openTxt")}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .top { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; padding: 22px 26px 14px; }
  .actions { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .content { flex: 1; padding: 0 26px 26px; display: flex; flex-direction: column; gap: 12px; }
  .warn-card { display: flex; gap: 12px; padding: 14px 16px; color: var(--warn); border-color: var(--warn); }
  .empty { margin: auto; display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; max-width: 420px; color: var(--muted); padding: 40px 0; }
  .jobs { display: flex; flex-direction: column; gap: 10px; }
  .job { padding: 12px 14px; display: flex; flex-direction: column; gap: 10px; }
  .job.active { border-color: var(--accent); }
  .job-head { display: flex; align-items: center; gap: 10px; }
  .job-title { flex: 1; min-width: 0; }
  .name { font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .grow { flex: 1; }
  .err { color: var(--danger); font-size: 13px; user-select: text; }
  .warn-text { color: var(--warn); }
  .spin { display: inline-flex; animation: spin 1s linear infinite; }
  .strip { display: flex; align-items: center; gap: 6px; padding: 6px 8px; border-radius: 8px; background: var(--surface-2); text-align: left; }
  .thumb { width: 44px; height: 58px; border-radius: 3px; overflow: hidden; background: #fff; border: 2px solid transparent; flex-shrink: 0; display: block; padding: 0; cursor: zoom-in; }
  .thumb:hover { border-color: var(--accent); }
  .thumb.current { border-color: var(--warn); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .thumb .ph { display: block; width: 100%; height: 100%; background: var(--surface-3); }
  .more { font-size: 12px; font-weight: 600; color: var(--text-2); padding: 0 4px; }
  .strip-label:hover { color: var(--accent); }
  .strip-label { margin-left: auto; display: inline-flex; align-items: center; gap: 5px; font-size: 12.5px; color: var(--muted); }
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); display: grid; place-items: center; z-index: 30; }
  .modal { width: min(760px, 92vw); max-height: 88vh; display: flex; flex-direction: column; overflow: hidden; }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px 8px; }
  .head h2 { display: flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .text { margin: 0; padding: 12px 18px; font-family: var(--font); font-size: 13px; white-space: pre-wrap; user-select: text; border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); flex: 1; min-height: 200px; }
  .foot-row { display: flex; gap: 8px; padding: 10px 18px; }
</style>
