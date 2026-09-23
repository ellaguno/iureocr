<script lang="ts">
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { addReadyFile, isActive, loadThumbs, toast, type Job } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";

  let { job, onclose, onview }: { job: Job; onclose: () => void; onview?: (page: number) => void } = $props();

  let selected = $state<Set<number>>(new Set());
  let working = $state<string | null>(null);
  let total = $derived(job.pages ?? 0);
  /** Página en curso del OCR (1-based), para resaltarla. */
  let current = $derived(isActive(job) && job.total ? job.current : 0);
  let allLoaded = $derived(total > 0 && Object.keys(job.thumbs).length >= total);

  // Carga las miniaturas por tandas mientras el modal está abierto.
  $effect(() => {
    let cancelled = false;
    (async () => {
      for (let from = 0; from < total && !cancelled; from += 12) await loadThumbs(job, from, from + 12);
    })();
    return () => {
      cancelled = true;
    };
  });

  function toggle(i: number) {
    const next = new Set(selected);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    selected = next;
  }
  function selectAll() {
    selected = new Set(Array.from({ length: total }, (_, i) => i));
  }
  function selectNone() {
    selected = new Set();
  }
  function invert() {
    selected = new Set(Array.from({ length: total }, (_, i) => i).filter((i) => !selected.has(i)));
  }
  let pages1 = $derived([...selected].sort((a, b) => a - b).map((i) => i + 1));
  let sourcePdf = $derived(job.result?.pdfPath ?? job.path);
  let isPdf = $derived(sourcePdf.toLowerCase().endsWith(".pdf"));

  async function edit(op: "delete" | "keep" | "rotate", degrees?: number) {
    if (!pages1.length) return;
    working = op;
    try {
      const r = await api.pdfEditPages(sourcePdf, op, pages1, degrees);
      const made = addReadyFile(r.path);
      toast(`Listo: ${made.name} (${r.pages} página${r.pages === 1 ? "" : "s"})`, "success", 6000);
      onclose();
    } catch (e) {
      toast(String(e), "error", 9000);
    } finally {
      working = null;
    }
  }
  async function openPdf() {
    try {
      await openPath(sourcePdf);
    } catch (e) {
      toast(`No se pudo abrir: ${e}`, "error");
    }
  }
</script>

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && !working && onclose()}>
  <div class="modal card" role="dialog" aria-modal="true" aria-label="Páginas">
    <div class="head">
      <h2><Icon name="layers" size={17} /> {job.name} <span class="hint">· {total} página{total === 1 ? "" : "s"}{job.result?.pdfPath ? " · con OCR" : ""}</span></h2>
      <button class="btn icon ghost" onclick={onclose} disabled={!!working} aria-label="Cerrar"><Icon name="x" size={16} /></button>
    </div>
    {#if isPdf}
      <div class="tools">
        <span class="hint">{selected.size ? `${selected.size} seleccionada${selected.size === 1 ? "" : "s"}` : "Marca páginas para editarlas"}</span>
        <button class="btn sm ghost" onclick={selectAll}>Todas</button>
        <button class="btn sm ghost" onclick={selectNone} disabled={!selected.size}>Ninguna</button>
        <button class="btn sm ghost" onclick={invert}>Invertir</button>
        <span class="grow"></span>
        <button class="btn sm" onclick={() => edit("rotate", 90)} disabled={!selected.size || !!working} title="Girar 90° a la derecha"><Icon name="refresh" size={14} /> Rotar</button>
        <button class="btn sm" onclick={() => edit("keep")} disabled={!selected.size || selected.size === total || !!working} title="Nuevo PDF sólo con las seleccionadas"><Icon name="copy" size={14} /> Sólo estas</button>
        <button class="btn sm danger" onclick={() => edit("delete")} disabled={!selected.size || selected.size === total || !!working} title="Nuevo PDF sin las seleccionadas"><Icon name="trash" size={14} /> Quitar</button>
      </div>
    {/if}
    <div class="grid scroll">
      {#each Array.from({ length: total }, (_, i) => i) as i (i)}
        <button class="page" class:selected={selected.has(i)} class:current={current === i + 1} onclick={() => isPdf && toggle(i)} ondblclick={() => onview?.(i)} title="Página {i + 1} · doble clic para verla">
          {#if job.thumbs[i]}
            <img src={job.thumbs[i]} alt="Página {i + 1}" loading="lazy" />
          {:else}
            <div class="ph"><span class="spin"><Icon name="loader" size={16} /></span></div>
          {/if}
          <span class="num">{i + 1}</span>
          {#if selected.has(i)}<span class="tick"><Icon name="check" size={12} stroke={3} /></span>{/if}
          {#if current === i + 1}<span class="cur">reconociendo…</span>{/if}
        </button>
      {/each}
      {#if !total}
        <div class="empty"><span class="spin"><Icon name="loader" size={16} /></span> Contando páginas…</div>
      {/if}
    </div>
    <div class="foot-row">
      <span class="hint">{working ? "Escribiendo el PDF nuevo…" : allLoaded ? "Las ediciones crean un PDF nuevo junto al original; el original no se toca." : "Cargando miniaturas…"}</span>
      <span class="grow"></span>
      {#if onview}<button class="btn sm ghost" onclick={() => onview([...selected].sort((a, b) => a - b)[0] ?? 0)}><Icon name="eye" size={14} /> Ver</button>{/if}
      <button class="btn sm ghost" onclick={openPdf} title="Abrir con la aplicación del sistema"><Icon name="external" size={14} /> Abrir fuera</button>
      <button class="btn" onclick={onclose} disabled={!!working}>Cerrar</button>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); display: grid; place-items: center; z-index: 30; }
  .modal { width: min(980px, 94vw); height: min(760px, 92vh); display: flex; flex-direction: column; overflow: hidden; }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px 8px; }
  .head h2 { display: flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .head .hint { font-weight: 500; }
  .tools { display: flex; align-items: center; gap: 6px; padding: 0 18px 10px; flex-wrap: wrap; }
  .grow { flex: 1; }
  .grid { flex: 1; display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 12px; padding: 12px 18px; border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); align-content: start; background: var(--bg); }
  .page { position: relative; display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 8px; border-radius: 10px; border: 2px solid transparent; background: var(--surface); box-shadow: var(--shadow); cursor: pointer; }
  .page:hover { border-color: var(--border-strong); }
  .page.selected { border-color: var(--accent); background: var(--accent-soft); }
  .page.current { border-color: var(--warn); }
  .page img, .ph { width: 100%; aspect-ratio: 0.72; object-fit: contain; border-radius: 4px; background: #fff; }
  .ph { display: grid; place-items: center; color: var(--muted); }
  .num { font-size: 12px; color: var(--text-2); font-weight: 600; }
  .tick { position: absolute; top: 6px; right: 6px; width: 20px; height: 20px; border-radius: 50%; background: var(--accent); color: var(--accent-text); display: grid; place-items: center; }
  .cur { position: absolute; bottom: 30px; left: 50%; transform: translateX(-50%); font-size: 11px; background: var(--warn); color: #fff; padding: 2px 8px; border-radius: 999px; white-space: nowrap; }
  .empty { grid-column: 1 / -1; display: flex; align-items: center; gap: 8px; justify-content: center; padding: 40px; color: var(--muted); }
  .spin { display: inline-flex; animation: spin 1s linear infinite; }
  .foot-row { display: flex; align-items: center; gap: 8px; padding: 10px 18px 14px; }
</style>
