<script lang="ts">
  import { api, type IureCase, type IureListing } from "../lib/api";
  import { app, iureLoggedIn, term, toast, uploadJob, type Job } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  let { job, onclose }: { job: Job; onclose: () => void } = $props();

  type Mode = "case" | "folder";
  let mode = $state<Mode>(iureLoggedIn() ? "case" : "folder");
  let includeTxt = $state(false);
  let loading = $state(false);
  let error = $state("");

  let listing = $state<IureListing | null>(null);
  let crumbs = $derived((listing?.path ?? "").split("/").filter(Boolean));
  async function go(path: string) {
    loading = true;
    error = "";
    try {
      listing = await api.iureList(path);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  async function openFolderMode() {
    mode = "folder";
    if (listing) return;
    if (iureLoggedIn()) {
      loading = true;
      try {
        const created = await api.iureEnsureWebdavPassword();
        if (created) toast("Se creó una contraseña de aplicación WebDAV con tu sesión", "success", 5000);
      } catch (e) {
        error = `No se pudo preparar el acceso WebDAV: ${e}`;
        loading = false;
        return;
      } finally {
        loading = false;
      }
    }
    const last = app.settings?.iureLastFolder;
    if (last) {
      try {
        listing = await api.iureList(last);
        return;
      } catch {
        /* la carpeta ya no existe */
      }
    }
    await go("");
  }

  let caseQuery = $state("");
  let cases = $state<IureCase[]>([]);
  let selectedCase = $state<IureCase | null>(null);
  let caseTimer: ReturnType<typeof setTimeout> | undefined;
  async function searchCases() {
    loading = true;
    error = "";
    try {
      cases = await api.iureSearchCases(caseQuery);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  function onCaseInput() {
    clearTimeout(caseTimer);
    caseTimer = setTimeout(searchCases, 350);
  }
  $effect(() => {
    if (mode === "case" && !cases.length) searchCases();
    if (mode === "folder" && !listing && !loading) void openFolderMode();
  });

  let canSave = $derived(!job.upload && !loading && ((mode === "folder" && !!listing?.canUpload) || (mode === "case" && !!selectedCase)));
  async function save() {
    const ok = mode === "folder" && listing
      ? await uploadJob(job, { mode: "folder", folder: listing.path }, includeTxt)
      : selectedCase
        ? await uploadJob(job, { mode: "case", caseId: selectedCase.id, caseTitle: `${selectedCase.caseNumber} · ${selectedCase.title}` }, includeTxt)
        : false;
    if (ok) onclose();
  }
</script>

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && !job.upload && onclose()}>
  <div class="modal card" role="dialog" aria-modal="true" aria-label="Guardar en Iurefficient">
    <div class="head">
      <h2><Icon name="upload" size={17} /> Guardar en Iurefficient</h2>
      <button class="btn icon ghost" onclick={onclose} disabled={!!job.upload} aria-label="Cerrar"><Icon name="x" size={16} /></button>
    </div>
    <div class="modes">
      <button class:active={mode === "case"} onclick={() => (mode = "case")} disabled={!iureLoggedIn()} title={iureLoggedIn() ? "" : "Inicia sesión en Ajustes"}><Icon name="layers" size={14} /> {term("case")}</button>
      <button class:active={mode === "folder"} onclick={openFolderMode}><Icon name="folder" size={14} /> Carpeta</button>
    </div>

    {#if mode === "case"}
      <div class="search">
        <input class="input" placeholder="Buscar {term("case").toLowerCase()} por número, título o {term("client").toLowerCase()}…" bind:value={caseQuery} oninput={onCaseInput} />
      </div>
      <div class="list scroll">
        {#if loading}
          <div class="empty"><span class="spin"><Icon name="loader" size={16} /></span> Buscando…</div>
        {:else if error}
          <div class="empty err"><Icon name="alert" size={16} /> {error}</div>
        {:else if !cases.length}
          <div class="empty">Sin resultados</div>
        {:else}
          {#each cases as c (c.id)}
            <button class="row" class:selected={selectedCase?.id === c.id} onclick={() => (selectedCase = c)}>
              <Icon name="layers" size={15} />
              <span class="name"><strong>{c.caseNumber}</strong> · {c.title}</span>
              {#if selectedCase?.id === c.id}<Icon name="check" size={15} />{/if}
            </button>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="crumbs">
        <button class="crumb" onclick={() => go("")} disabled={loading}><Icon name="cloud" size={13} /> Documentos</button>
        {#each crumbs as c, i}
          <span class="sep">/</span>
          <button class="crumb" onclick={() => go(crumbs.slice(0, i + 1).join("/"))} disabled={loading}>{c}</button>
        {/each}
      </div>
      <div class="list scroll">
        {#if loading}
          <div class="empty"><span class="spin"><Icon name="loader" size={16} /></span> Cargando…</div>
        {:else if error}
          <div class="empty err"><Icon name="alert" size={16} /> {error}</div>
        {:else if listing}
          {#each listing.entries.filter((e) => e.isFolder) as e (e.path)}
            <button class="row" onclick={() => go(e.path)}>
              <Icon name="folder" size={15} />
              <span class="name">{e.name}</span>
              <Icon name="chevronRight" size={14} />
            </button>
          {:else}
            <div class="empty">Sin subcarpetas</div>
          {/each}
        {/if}
      </div>
    {/if}

    <div class="files">
      <label class="check"><input type="checkbox" bind:checked={includeTxt} /> Subir también el texto plano (.txt)</label>
      <p class="hint">Se sube el PDF con texto buscable{includeTxt ? " y el .txt" : ""} a {mode === "folder" ? (listing?.path || "la raíz") : selectedCase ? `${selectedCase.caseNumber} · ${selectedCase.title}` : `un ${term("case").toLowerCase()}`}.</p>
    </div>
    <div class="foot-row">
      <span class="grow"></span>
      <button class="btn" onclick={onclose} disabled={!!job.upload}>Cancelar</button>
      <button class="btn primary" onclick={save} disabled={!canSave}>
        {#if job.upload}<span class="spin"><Icon name="loader" size={15} /></span> Subiendo…{:else}<Icon name="upload" size={15} /> Guardar{/if}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); display: grid; place-items: center; z-index: 30; }
  .modal { width: min(680px, 92vw); max-height: 90vh; display: flex; flex-direction: column; overflow: hidden; }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px 8px; }
  .head h2 { display: flex; align-items: center; gap: 8px; }
  .modes { display: flex; gap: 4px; padding: 0 14px 8px; }
  .modes button { display: inline-flex; align-items: center; gap: 6px; padding: 6px 12px; border-radius: 8px; color: var(--text-2); font-weight: 550; }
  .modes button.active { background: var(--accent-soft); color: var(--accent); }
  .modes button:disabled { opacity: 0.45; }
  .crumbs { display: flex; align-items: center; flex-wrap: wrap; gap: 2px; padding: 0 14px 8px; font-size: 13px; }
  .crumb { display: inline-flex; align-items: center; gap: 4px; padding: 3px 7px; border-radius: 6px; color: var(--accent); font-weight: 550; }
  .crumb:hover:not(:disabled) { background: var(--accent-soft); }
  .sep { color: var(--muted); }
  .search { padding: 0 18px 8px; }
  .list { flex: 1; min-height: 160px; max-height: 300px; border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); }
  .row { width: 100%; display: flex; align-items: center; gap: 10px; padding: 9px 18px; text-align: left; color: var(--text); }
  .row:hover { background: var(--surface-2); }
  .row.selected { background: var(--accent-soft); }
  .row .name { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .empty { display: flex; align-items: center; gap: 8px; justify-content: center; padding: 30px; color: var(--muted); text-align: center; }
  .empty.err { color: var(--danger); }
  .spin { display: inline-flex; animation: spin 1s linear infinite; }
  .files { padding: 10px 18px; font-size: 13px; display: flex; flex-direction: column; gap: 6px; }
  .check { display: flex; align-items: center; gap: 7px; cursor: pointer; }
  .check input[type="checkbox"] { accent-color: var(--accent); }
  .foot-row { display: flex; gap: 8px; padding: 10px 18px 14px; }
  .grow { flex: 1; }
</style>
