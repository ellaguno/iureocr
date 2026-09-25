<script lang="ts">
  import { tick, untrack } from "svelte";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { isActive, pdfOf, requestOcr, toast, type Job } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
  import { t } from "../lib/i18n.svelte";

  let { job, start = 0, onclose, onpages }: { job: Job; start?: number; onclose: () => void; onpages: () => void } = $props();

  // Visor virtualizado: sólo existen en el DOM las páginas cercanas a la vista, y sólo
  // esas se piden a pdfium, a la resolución que ocupan en pantalla.
  const GAP = 14;
  const PAD = 18;
  /** Margen (px) por encima y por debajo de la vista que también se dibuja. */
  const AHEAD = 900;
  /** Páginas dibujadas que se conservan en memoria alrededor de la actual. */
  const KEEP = 24;

  let scroller = $state<HTMLDivElement | null>(null);
  let viewW = $state(0);
  let viewH = $state(0);
  let scrollTop = $state(0);
  let zoom = $state(1);
  let imgs = $state<Record<number, { w: number; url: string }>>({});
  let pageInput = $state("");

  let source = $derived(pdfOf(job));
  let sizes = $derived(job.pageSizes);
  let total = $derived(sizes.length);
  /** Ancho en pantalla de una página con zoom 1: el ancho disponible, sin pasar de 900 px. */
  let fitW = $derived(Math.max(200, Math.min(viewW - 2 * PAD, 900)));
  let widths = $derived(sizes.map(() => fitW * zoom));
  let heights = $derived(sizes.map(([w, h], i) => (widths[i] * h) / w));
  let tops = $derived.by(() => {
    const t: number[] = [];
    let y = PAD;
    for (const h of heights) {
      t.push(y);
      y += h + GAP;
    }
    return t;
  });
  let contentH = $derived(total ? tops[total - 1] + heights[total - 1] + PAD : 0);
  let contentW = $derived(Math.max(viewW, ...widths.map((w) => w + 2 * PAD)));

  /** Primera página cuyo fondo queda por debajo de `y`. */
  function pageAt(y: number): number {
    let lo = 0;
    let hi = total - 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (tops[mid] + heights[mid] < y) lo = mid + 1;
      else hi = mid;
    }
    return Math.max(0, lo);
  }
  let range = $derived.by(() => {
    if (!total) return [0, -1];
    return [pageAt(scrollTop - AHEAD), pageAt(scrollTop + viewH + AHEAD)];
  });
  let visible = $derived(Array.from({ length: Math.max(0, range[1] - range[0] + 1) }, (_, k) => range[0] + k));
  let current = $derived(total ? pageAt(scrollTop + viewH / 3) : 0);
  let ocrPage = $derived(isActive(job) && job.total ? job.current - 1 : -1);

  $effect(() => {
    pageInput = String(current + 1);
  });

  /** Ancho a pedir: el de pantalla por la densidad de píxeles, en escalones para reaprovechar. */
  function wantW(i: number): number {
    const px = widths[i] * (window.devicePixelRatio || 1);
    return Math.min(2400, Math.ceil(px / 200) * 200);
  }

  // Cambió el archivo (p. ej. terminó el OCR): se descarta lo dibujado.
  $effect(() => {
    void source;
    imgs = {};
  });

  // Cola de dibujo: una petición a la vez, las páginas más cercanas a la actual primero.
  let busy = false;
  let again = false;
  async function pump() {
    if (busy) {
      again = true;
      return;
    }
    busy = true;
    try {
      do {
        again = false;
        const src = source;
        const missing = visible.filter((i) => imgs[i]?.w !== wantW(i)).sort((a, b) => Math.abs(a - current) - Math.abs(b - current));
        if (!missing.length) break;
        const batch = missing.slice(0, 2);
        const w = wantW(batch[0]);
        const same = batch.filter((i) => wantW(i) === w);
        try {
          const urls = await api.pdfThumbnails(src, same, w);
          if (src !== source) {
            again = true;
            continue;
          }
          same.forEach((i, k) => (imgs[i] = { w, url: urls[k] }));
        } catch (e) {
          toast(t("viewer.renderFailed", { error: String(e) }), "error", 8000);
          break;
        }
        again = true;
      } while (again);
      // Libera memoria de las páginas lejanas.
      for (const k of Object.keys(imgs)) if (Math.abs(Number(k) - current) > KEEP) delete imgs[Number(k)];
    } finally {
      busy = false;
    }
  }
  $effect(() => {
    void visible;
    void widths;
    void source;
    untrack(() => void pump());
  });

  function goTo(i: number) {
    if (!scroller || !total) return;
    const p = Math.max(0, Math.min(total - 1, i));
    scroller.scrollTop = tops[p] - PAD / 2;
  }
  let started = false;
  $effect(() => {
    if (!started && scroller && total && viewH) {
      started = true;
      if (start > 0) tick().then(() => goTo(start));
    }
  });

  /** Cambia el zoom conservando la página que se está leyendo. */
  function setZoom(z: number) {
    const keep = current;
    const offset = scroller ? (scroller.scrollTop - tops[keep]) / (heights[keep] || 1) : 0;
    zoom = Math.max(0.4, Math.min(3, Math.round(z * 10) / 10));
    tick().then(() => {
      if (scroller) scroller.scrollTop = tops[keep] + offset * heights[keep];
    });
  }

  function onkey(e: KeyboardEvent) {
    if ((e.target as HTMLElement)?.tagName === "INPUT") return;
    if (e.key === "Escape") onclose();
    else if (e.key === "PageDown" || e.key === "ArrowRight") goTo(current + 1);
    else if (e.key === "PageUp" || e.key === "ArrowLeft") goTo(current - 1);
    else if (e.key === "Home") goTo(0);
    else if (e.key === "End") goTo(total - 1);
    else if (e.key === "+" || e.key === "=") setZoom(zoom + 0.2);
    else if (e.key === "-") setZoom(zoom - 0.2);
    else if (e.key === "0") setZoom(1);
    else return;
    e.preventDefault();
  }
  function onwheel(e: WheelEvent) {
    if (!e.ctrlKey) return;
    e.preventDefault();
    setZoom(zoom + (e.deltaY < 0 ? 0.1 : -0.1));
  }
  function submitPage(e: Event) {
    e.preventDefault();
    const n = parseInt(pageInput, 10);
    if (Number.isFinite(n) && n - 1 !== current) goTo(n - 1);
    else pageInput = String(current + 1);
  }
  async function openOutside() {
    try {
      await openPath(source);
    } catch (e) {
      toast(t("common.openFailed", { error: String(e) }), "error");
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="viewer" role="dialog" aria-modal="true" aria-label={t("viewer.label")}>
  <div class="bar">
    <div class="title">
      <Icon name={source.toLowerCase().endsWith(".pdf") ? "doc" : "image"} size={17} />
      <span class="name" title={source}>{job.name}</span>
      {#if job.result?.pdfPath}<span class="pill success">{t("common.withOcr")}</span>{/if}
    </div>
    <div class="nav">
      <button class="btn icon ghost" title={t("viewer.prev")} onclick={() => goTo(current - 1)} disabled={current <= 0}><Icon name="left" size={16} /></button>
      <form onsubmit={submitPage}>
        <input class="page-in" bind:value={pageInput} aria-label={t("viewer.page")} inputmode="numeric" onblur={submitPage} />
      </form>
      <span class="hint">/ {total || "…"}</span>
      <button class="btn icon ghost" title={t("viewer.next")} onclick={() => goTo(current + 1)} disabled={current >= total - 1}><Icon name="right" size={16} /></button>
      <span class="sep"></span>
      <button class="btn icon ghost" title={t("viewer.zoomOut")} onclick={() => setZoom(zoom - 0.2)} disabled={zoom <= 0.4}><Icon name="minus" size={16} /></button>
      <button class="btn sm ghost zoom" title={t("viewer.fit")} onclick={() => setZoom(1)}>{Math.round(zoom * 100)}%</button>
      <button class="btn icon ghost" title={t("viewer.zoomIn")} onclick={() => setZoom(zoom + 0.2)} disabled={zoom >= 3}><Icon name="plus" size={16} /></button>
    </div>
    <div class="actions">
      {#if job.status === "ready"}
        <button class="btn sm primary" onclick={() => requestOcr(job.id)}><Icon name="scan" size={14} /> {t("common.recognizeText")}</button>
      {:else if isActive(job) || job.status === "queued"}
        <span class="pill accent">{job.status === "queued" ? t("ocr.status.queued") : t("ocr.status.ocrN", { current: job.current, total: job.total || "…" })}</span>
      {/if}
      <button class="btn sm" onclick={onpages}><Icon name="layers" size={14} /> {t("common.pages")}</button>
      <button class="btn sm ghost" onclick={openOutside} title={t("common.openWithSystem")}><Icon name="external" size={14} /> {t("common.openOutside")}</button>
      <button class="btn icon ghost" onclick={onclose} aria-label={t("viewer.close")} title={t("viewer.close")}><Icon name="x" size={17} /></button>
    </div>
  </div>

  <div class="scroller" bind:this={scroller} bind:clientWidth={viewW} bind:clientHeight={viewH} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)} {onwheel}>
    {#if !total}
      <div class="loading">
        {#if job.pages === 0}
          <Icon name="alert" size={18} /> {t("viewer.readFailed")}
        {:else}
          <span class="spin"><Icon name="loader" size={18} /></span> {t("viewer.opening")}
        {/if}
      </div>
    {:else}
      <div class="content" style="height: {contentH}px; width: {contentW}px">
        {#each visible as i (i)}
          <div class="page" class:ocr={ocrPage === i} style="top: {tops[i]}px; left: {(contentW - widths[i]) / 2}px; width: {widths[i]}px; height: {heights[i]}px">
            {#if imgs[i]}
              <img src={imgs[i].url} alt={t("page.n", { n: i + 1 })} draggable="false" />
            {:else}
              <span class="spin"><Icon name="loader" size={18} /></span>
            {/if}
            <span class="num">{i + 1}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .viewer { position: fixed; inset: 0; z-index: 30; display: flex; flex-direction: column; background: var(--bg); }
  .bar { display: flex; align-items: center; gap: 12px; padding: 8px 14px; background: var(--surface); border-bottom: 1px solid var(--border); flex-wrap: wrap; }
  .title { display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1; }
  .name { font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .nav { display: flex; align-items: center; gap: 4px; }
  .nav form { display: contents; }
  .page-in { width: 52px; text-align: center; padding: 4px 6px; }
  .sep { width: 1px; height: 20px; background: var(--border); margin: 0 6px; }
  .zoom { min-width: 58px; justify-content: center; font-variant-numeric: tabular-nums; }
  .actions { display: flex; align-items: center; gap: 6px; flex: 1; justify-content: flex-end; }
  .scroller { flex: 1; overflow: auto; position: relative; background: var(--surface-3); }
  .content { position: relative; }
  .page { position: absolute; background: #fff; box-shadow: 0 1px 3px rgba(0, 0, 0, 0.18), 0 6px 20px rgba(0, 0, 0, 0.08); display: grid; place-items: center; color: #8d95a6; }
  .page.ocr { outline: 3px solid var(--warn); }
  .page img { width: 100%; height: 100%; display: block; user-select: none; }
  .num { position: absolute; bottom: 6px; right: 8px; font-size: 11px; font-weight: 600; color: #6f7684; background: rgba(255, 255, 255, 0.85); padding: 1px 6px; border-radius: 999px; }
  .loading { height: 100%; display: flex; align-items: center; justify-content: center; gap: 8px; color: var(--muted); }
  .spin { display: inline-flex; animation: spin 1s linear infinite; }
</style>
