<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { addFiles, app, cancelJob, clearFinished, isActive, queuedCount, removeJob, retryJob, startQueue, stopQueue, toast, type Job } from "../lib/state.svelte";
  import { fmtBytes, fmtSecs } from "../lib/format";
  import Icon from "./Icon.svelte";
  import IurePicker from "./IurePicker.svelte";
  import PagesView from "./PagesView.svelte";

  let picking = $state<Job | null>(null);
  let pagesOf = $state<Job | null>(null);
  let preview = $state<{ job: Job; text: string } | null>(null);

  async function pickFiles() {
    const sel = await open({ multiple: true, filters: [{ name: "PDF e imágenes", extensions: app.sys?.supportedExtensions ?? ["pdf"] }] });
    if (!sel) return;
    await addFiles(Array.isArray(sel) ? sel : [sel]);
  }
  async function openFile(path: string) {
    try {
      await openPath(path);
    } catch (e) {
      toast(`No se pudo abrir: ${e}`, "error");
    }
  }
  async function reveal(path: string) {
    try {
      await api.revealPath(path);
    } catch (e) {
      toast(`No se pudo mostrar en la carpeta: ${e}`, "error");
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
      toast("OnlyOffice no está instalado: se abre la página de descarga", "info");
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
      case "queued": return "En cola";
      case "render": return j.total ? `Leyendo páginas ${j.current}/${j.total}` : "Leyendo páginas…";
      case "ocr": return j.total ? `Reconociendo ${j.current}/${j.total}` : "Reconociendo texto…";
      case "done": return "Listo";
      case "ready": return "Listo (sin OCR)";
      case "skipped": return "Ya tenía texto";
      case "error": return "Error";
      case "cancelled": return "Cancelado";
    }
  }
  function pillClass(j: Job): string {
    return j.status === "done" || j.status === "ready" ? "success" : j.status === "error" ? "danger" : j.status === "skipped" ? "warn" : isActive(j) ? "accent" : "";
  }
  let percent = $derived((j: Job) => (j.total ? Math.round(((j.status === "ocr" ? j.total : 0) + j.current) / (2 * j.total) * 100) : 0));
  let finished = $derived(app.jobs.filter((j) => !isActive(j) && j.status !== "queued").length);
</script>

<header class="top">
  <div>
    <h1>Reconocer texto</h1>
    <p class="hint">Convierte escaneos e imágenes en PDF con texto buscable, en este equipo. Arrastra archivos o agrégalos.</p>
  </div>
  <div class="actions">
    <button class="btn primary" onclick={pickFiles}><Icon name="plus" size={16} /> Agregar archivos</button>
    {#if app.running}
      <button class="btn" onclick={stopQueue}><Icon name="stop" size={15} /> Detener al terminar el actual</button>
    {:else if queuedCount() > 0}
      <button class="btn" onclick={startQueue}><Icon name="play" size={15} /> Procesar {queuedCount()}</button>
    {/if}
    {#if finished > 0}
      <button class="btn ghost" onclick={clearFinished}><Icon name="trash" size={15} /> Limpiar terminados</button>
    {/if}
  </div>
</header>

<div class="content scroll">
  {#if !app.sys?.tesseract}
    <div class="card warn-card">
      <Icon name="alert" size={18} />
      <div>
        <strong>Tesseract no está disponible.</strong>
        <p class="hint">{app.sys?.tesseractError}</p>
      </div>
    </div>
  {/if}
  {#if app.jobs.length === 0}
    <div class="empty">
      <Icon name="scan" size={40} />
      <h2>Sin archivos</h2>
      <p class="hint">Suelta aquí PDF escaneados o fotos de documentos. El resultado se guarda junto al original con el sufijo «{app.settings?.suffix}».</p>
      <button class="btn primary" onclick={pickFiles}><Icon name="plus" size={16} /> Agregar archivos</button>
    </div>
  {:else}
    <div class="jobs">
      {#each app.jobs as job (job.id)}
        <div class="card job" class:active={isActive(job)}>
          <div class="job-head">
            <Icon name={job.path.toLowerCase().endsWith(".pdf") ? "doc" : "image"} size={18} />
            <div class="job-title">
              <div class="name" title={job.path}>{job.name}</div>
              <div class="hint">{fmtBytes(job.sizeBytes)}{job.result && !job.result.skipped ? ` · ${job.result.pages} página(s) · ${job.result.chars.toLocaleString("es")} caracteres · ${fmtSecs(job.result.elapsedSecs)}` : ""}{job.force ? " · forzado" : ""}</div>
            </div>
            <span class="pill {pillClass(job)}">{#if isActive(job)}<span class="spin"><Icon name="loader" size={12} /></span>{/if}{statusLabel(job)}</span>
            {#if isActive(job) || job.status === "queued"}
              <button class="btn icon ghost" title="Cancelar" onclick={() => cancelJob(job.id)}><Icon name="x" size={15} /></button>
            {:else}
              <button class="btn icon ghost" title="Quitar de la lista" onclick={() => removeJob(job.id)}><Icon name="trash" size={15} /></button>
            {/if}
          </div>
          {#if job.pages}
            <button class="strip" onclick={() => (pagesOf = job)} title="Ver todas las páginas">
              {#each Array.from({ length: Math.min(job.pages, 6) }, (_, i) => i) as i (i)}
                <span class="thumb" class:current={isActive(job) && job.total && job.current === i + 1}>
                  {#if job.thumbs[i]}<img src={job.thumbs[i]} alt="" />{:else}<span class="ph"></span>{/if}
                </span>
              {/each}
              {#if job.pages > 6}<span class="more">+{job.pages - 6}</span>{/if}
              <span class="strip-label"><Icon name="layers" size={13} /> {job.pages} página{job.pages === 1 ? "" : "s"}{isActive(job) && job.total ? ` · en la ${job.current}` : ""}</span>
            </button>
          {/if}
          {#if isActive(job)}
            <div class="progress" class:indeterminate={!job.total}><div style="width: {percent(job)}%"></div></div>
          {/if}
          {#if job.error && job.status === "error"}
            <p class="err">{job.error}</p>
            <div class="row"><button class="btn sm" onclick={() => retryJob(job.id)}><Icon name="refresh" size={14} /> Reintentar</button></div>
          {:else if job.status === "cancelled"}
            <div class="row"><button class="btn sm" onclick={() => retryJob(job.id)}><Icon name="refresh" size={14} /> Volver a la cola</button></div>
          {:else if job.status === "skipped"}
            <p class="hint">{job.result?.note}</p>
            <div class="row">
              <button class="btn sm" onclick={() => retryJob(job.id, true)}><Icon name="scan" size={14} /> Forzar OCR de todas formas</button>
              <button class="btn sm ghost" onclick={() => openFile(job.path)}><Icon name="external" size={14} /> Abrir original</button>
            </div>
          {:else if job.status === "ready"}
            <div class="row">
              <button class="btn sm primary" onclick={() => openFile(job.path)}><Icon name="doc" size={14} /> Abrir PDF</button>
              <button class="btn sm" onclick={() => (pagesOf = job)}><Icon name="layers" size={14} /> Páginas</button>
              <button class="btn sm" onclick={() => reveal(job.path)}><Icon name="folder" size={14} /> Mostrar en carpeta</button>
              <button class="btn sm" onclick={() => retryJob(job.id, true)}><Icon name="scan" size={14} /> Reconocer texto</button>
              <span class="grow"></span>
              {#if job.upload}
                <span class="pill accent"><span class="spin"><Icon name="loader" size={12} /></span> Subiendo…</span>
              {:else if job.saved}
                <span class="pill success"><Icon name="check" size={12} stroke={3} /> En {job.saved.target}</span>
              {:else}
                <button class="btn sm" onclick={() => (picking = job)} disabled={!app.settings?.iureDomain}><Icon name="cloud" size={14} /> Guardar en Iurefficient</button>
              {/if}
            </div>
          {:else if job.status === "done" && job.result}
            {#if job.result.note}<p class="hint warn-text">{job.result.note}</p>{/if}
            <div class="row">
              <button class="btn sm primary" onclick={() => openFile(job.result!.pdfPath!)}><Icon name="doc" size={14} /> Abrir PDF</button>
              <button class="btn sm" onclick={() => showText(job)}><Icon name="text" size={14} /> Ver texto</button>
              <button class="btn sm" onclick={() => (pagesOf = job)}><Icon name="layers" size={14} /> Páginas</button>
              <button class="btn sm" onclick={() => reveal(job.result!.pdfPath!)}><Icon name="folder" size={14} /> Mostrar en carpeta</button>
              <button class="btn sm" onclick={() => editWith(job.result!.pdfPath!)} title={app.onlyoffice?.installed ? "Abrir con OnlyOffice" : "OnlyOffice no está instalado"}><Icon name="edit" size={14} /> Editar con OnlyOffice</button>
              <span class="grow"></span>
              {#if job.upload}
                <span class="pill accent"><span class="spin"><Icon name="loader" size={12} /></span> Subiendo {job.upload.fileName} ({job.upload.index + 1}/{job.upload.totalFiles})</span>
              {:else if job.saved}
                <span class="pill success" title={`Guardado ${new Date(job.saved.at).toLocaleTimeString("es")}`}><Icon name="check" size={12} stroke={3} /> En {job.saved.target}</span>
                <button class="btn sm ghost" onclick={() => (picking = job)}><Icon name="upload" size={14} /> Otra vez</button>
              {:else}
                <button class="btn sm" onclick={() => (picking = job)} disabled={!app.settings?.iureDomain}><Icon name="cloud" size={14} /> Guardar en Iurefficient</button>
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
{#if pagesOf}
  <PagesView job={pagesOf} onclose={() => (pagesOf = null)} />
{/if}
{#if preview}
  <div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && (preview = null)}>
    <div class="modal card" role="dialog" aria-modal="true">
      <div class="head">
        <h2><Icon name="text" size={17} /> {preview.job.name}</h2>
        <button class="btn icon ghost" onclick={() => (preview = null)} aria-label="Cerrar"><Icon name="x" size={16} /></button>
      </div>
      <pre class="text scroll">{preview.text}</pre>
      <div class="foot-row">
        <button class="btn sm" onclick={() => navigator.clipboard.writeText(preview!.text).then(() => toast("Texto copiado", "success"))}><Icon name="copy" size={14} /> Copiar</button>
        <button class="btn sm ghost" onclick={() => openFile(preview!.job.result!.txtPath!)}><Icon name="external" size={14} /> Abrir .txt</button>
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
  .strip:hover { background: var(--surface-3); }
  .thumb { width: 44px; height: 58px; border-radius: 3px; overflow: hidden; background: #fff; border: 2px solid transparent; flex-shrink: 0; display: block; }
  .thumb.current { border-color: var(--warn); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .thumb .ph { display: block; width: 100%; height: 100%; background: var(--surface-3); }
  .more { font-size: 12px; font-weight: 600; color: var(--text-2); padding: 0 4px; }
  .strip-label { margin-left: auto; display: inline-flex; align-items: center; gap: 5px; font-size: 12.5px; color: var(--muted); }
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); display: grid; place-items: center; z-index: 30; }
  .modal { width: min(760px, 92vw); max-height: 88vh; display: flex; flex-direction: column; overflow: hidden; }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px 8px; }
  .head h2 { display: flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .text { margin: 0; padding: 12px 18px; font-family: var(--font); font-size: 13px; white-space: pre-wrap; user-select: text; border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); flex: 1; min-height: 200px; }
  .foot-row { display: flex; gap: 8px; padding: 10px 18px; }
</style>
