<script lang="ts">
  import { api, type RedactArea } from "../lib/api";
  import { toast } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
  import { t, tn } from "../lib/i18n.svelte";

  /** Testado: se marcan zonas sobre la página y PdfCraft quita lo que hay debajo. */
  let { session, name, total, onclose, ondone }: { session: string; name: string; total: number; onclose: () => void; ondone: (changed: number[]) => void } = $props();

  let page = $state(0);
  let images = $state<Record<number, string>>({});
  let areas = $state<RedactArea[]>([]);
  let working = $state(false);
  let confirming = $state(false);
  let box = $state<HTMLDivElement | null>(null);
  let draft = $state<{ x0: number; y0: number; x1: number; y1: number } | null>(null);

  let onPage = $derived(areas.filter((a) => a.page === page));
  let markedPages = $derived(new Set(areas.map((a) => a.page)).size);

  $effect(() => {
    const p = page;
    if (images[p]) return;
    api
      .pdfThumbnails(session, [p], 1400)
      .then((r) => (images[p] = r[0]))
      .catch((e) => toast(t("viewer.renderFailed", { error: String(e) }), "error"));
  });

  function frac(e: PointerEvent): [number, number] {
    const r = box!.getBoundingClientRect();
    return [Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)), Math.min(1, Math.max(0, (e.clientY - r.top) / r.height))];
  }
  function onpointerdown(e: PointerEvent) {
    if (e.button !== 0 || working || !box) return;
    const [x, y] = frac(e);
    draft = { x0: x, y0: y, x1: x, y1: y };
    box.setPointerCapture(e.pointerId);
  }
  function onpointermove(e: PointerEvent) {
    if (!draft) return;
    [draft.x1, draft.y1] = frac(e);
  }
  function onpointerup() {
    const d = draft;
    draft = null;
    if (!d) return;
    const rect: [number, number, number, number] = [Math.min(d.x0, d.x1), Math.min(d.y0, d.y1), Math.max(d.x0, d.x1), Math.max(d.y0, d.y1)];
    // Un clic sin arrastrar no marca nada.
    if (rect[2] - rect[0] < 0.004 || rect[3] - rect[1] < 0.004) return;
    areas = [...areas, { page, rect }];
    confirming = false;
  }
  function remove(a: RedactArea) {
    areas = areas.filter((x) => x !== a);
    confirming = false;
  }
  function go(delta: number) {
    page = Math.min(total - 1, Math.max(0, page + delta));
  }
  function onkeydown(e: KeyboardEvent) {
    if (working) return;
    if (e.key === "Escape") onclose();
    else if (e.key === "PageDown") go(1);
    else if (e.key === "PageUp") go(-1);
    else if ((e.ctrlKey || e.metaKey) && e.key === "z" && areas.length) areas = areas.slice(0, -1);
  }
  const style = (r: number[]) => `left:${r[0] * 100}%;top:${r[1] * 100}%;width:${(r[2] - r[0]) * 100}%;height:${(r[3] - r[1]) * 100}%`;

  async function apply() {
    if (!confirming) {
      confirming = true;
      return;
    }
    working = true;
    try {
      const r = await api.pdfRedact(session, areas);
      toast(t("redact.done", { areas: tn("redact.areas", r.areas), glyphs: r.glyphs, images: r.images }), "success", 8000);
      ondone([...new Set(areas.map((a) => a.page))]);
    } catch (e) {
      toast(String(e), "error", 10000);
    } finally {
      working = false;
      confirming = false;
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="backdrop" role="presentation">
  <div class="modal card" role="dialog" aria-modal="true" aria-label={t("redact.title")}>
    <div class="head">
      <h2><Icon name="redact" size={17} /> {t("redact.title")} <span class="hint">· {name}</span></h2>
      <button class="btn icon ghost" onclick={onclose} disabled={working} aria-label={t("common.close")}><Icon name="x" size={16} /></button>
    </div>
    <div class="tools">
      <button class="btn sm ghost" onclick={() => go(-1)} disabled={page === 0} aria-label={t("viewer.prev")} title={t("viewer.prev")}>‹</button>
      <span class="hint">{t("page.n", { n: page + 1 })} / {total}</span>
      <button class="btn sm ghost" onclick={() => go(1)} disabled={page >= total - 1} aria-label={t("viewer.next")} title={t("viewer.next")}>›</button>
      <span class="grow"></span>
      <span class="hint">{areas.length ? t("redact.summary", { areas: tn("redact.areas", areas.length), pages: tn("pages.count", markedPages) }) : t("redact.hint")}</span>
      <button class="btn sm ghost" onclick={() => (areas = areas.filter((a) => a.page !== page))} disabled={!onPage.length || working}>{t("redact.clearPage")}</button>
    </div>
    <div class="stage scroll">
      {#if images[page]}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="sheet" bind:this={box} {onpointerdown} {onpointermove} {onpointerup} onlostpointercapture={onpointerup} onpointercancel={() => (draft = null)}>
          <img src={images[page]} alt={t("page.n", { n: page + 1 })} draggable="false" />
          {#each onPage as a}
            <div class="mark" style={style(a.rect)}>
              <button class="del" onpointerdown={(e) => e.stopPropagation()} onclick={() => remove(a)} aria-label={t("redact.remove")} title={t("redact.remove")}><Icon name="x" size={11} stroke={3} /></button>
            </div>
          {/each}
          {#if draft}
            <div class="mark draft" style={style([Math.min(draft.x0, draft.x1), Math.min(draft.y0, draft.y1), Math.max(draft.x0, draft.x1), Math.max(draft.y0, draft.y1)])}></div>
          {/if}
        </div>
      {:else}
        <div class="empty"><span class="spin"><Icon name="loader" size={16} /></span> {t("common.loading")}</div>
      {/if}
    </div>
    <div class="foot-row">
      <span class="hint">{working ? t("redact.working") : confirming ? t("redact.confirmHint") : t("redact.footHint")}</span>
      <span class="grow"></span>
      <button class="btn" onclick={onclose} disabled={working}>{t("common.cancel")}</button>
      <button class="btn primary" class:confirm={confirming} onclick={apply} disabled={!areas.length || working}>
        <Icon name="redact" size={14} /> {confirming ? t("redact.confirm") : t("redact.apply")}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); display: grid; place-items: center; z-index: 40; }
  .modal { width: min(1000px, 96vw); height: min(900px, 94vh); display: flex; flex-direction: column; overflow: hidden; }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px 8px; }
  .head h2 { display: flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .head .hint { font-weight: 500; }
  .tools { display: flex; align-items: center; gap: 6px; padding: 0 18px 10px; flex-wrap: wrap; }
  .grow { flex: 1; }
  .stage { flex: 1; display: flex; justify-content: center; align-items: flex-start; padding: 16px; background: var(--bg); border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); }
  .sheet { position: relative; max-width: 100%; cursor: crosshair; touch-action: none; user-select: none; box-shadow: var(--shadow); line-height: 0; }
  .sheet img { display: block; max-width: 100%; height: auto; background: #fff; pointer-events: none; }
  .mark { position: absolute; background: rgba(0, 0, 0, 0.78); outline: 2px solid var(--danger); }
  .mark.draft { background: rgba(0, 0, 0, 0.35); outline-style: dashed; }
  .del { position: absolute; top: -9px; right: -9px; width: 18px; height: 18px; border-radius: 50%; border: none; background: var(--danger); color: #fff; display: grid; place-items: center; cursor: pointer; padding: 0; }
  .empty { display: flex; align-items: center; gap: 8px; padding: 40px; color: var(--muted); }
  .spin { display: inline-flex; animation: spin 1s linear infinite; }
  .foot-row { display: flex; align-items: center; gap: 8px; padding: 10px 18px 14px; }
  .btn.primary.confirm, .btn.primary.confirm:hover:not(:disabled) { background: var(--danger); color: #fff; }
</style>
