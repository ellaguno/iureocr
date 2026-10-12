<script lang="ts">
  import { api, type BatesOptions } from "../lib/api";
  import { addReadyFile, toast } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
  import { t } from "../lib/i18n.svelte";

  /** Numeración Bates de todas las páginas con PdfCraft. */
  let { path, name, total, onclose, ondone }: { path: string; name: string; total: number; onclose: () => void; ondone: () => void } = $props();

  let o = $state<BatesOptions>({ prefix: "", suffix: "", digits: 6, start: 1, position: 5, fontSize: 9 });
  let working = $state(false);

  const label = (n: number) => `${o.prefix}${String(n).padStart(Math.min(15, Math.max(1, o.digits || 1)), "0")}${o.suffix}`;
  const num = (v: unknown, min: number, max: number) => typeof v === "number" && Number.isFinite(v) && v >= min && v <= max;
  let invalid = $derived([o.prefix, o.suffix].some((s) => /#|<<|>>/.test(s)) || !num(o.start, 0, 1e12) || !num(o.digits, 1, 15) || !num(o.fontSize, 4, 48));
  const positions = ["headerLeft", "headerCenter", "headerRight", "footerLeft", "footerCenter", "footerRight"] as const;

  async function apply() {
    working = true;
    try {
      const r = await api.pdfBates(path, { ...o, start: Math.floor(o.start), digits: Math.floor(o.digits) });
      const made = addReadyFile(r.path);
      toast(t("bates.done", { name: made.name, first: r.first, last: r.last }), "success", 8000);
      ondone();
    } catch (e) {
      toast(String(e), "error", 10000);
    } finally {
      working = false;
    }
  }
</script>

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && !working && onclose()}>
  <div class="modal card" role="dialog" aria-modal="true" aria-label={t("bates.title")}>
    <div class="head">
      <h2><Icon name="hash" size={17} /> {t("bates.title")}</h2>
      <button class="btn icon ghost" onclick={onclose} disabled={working} aria-label={t("common.close")}><Icon name="x" size={16} /></button>
    </div>
    <p class="hint intro">{t("bates.intro", { name })}</p>
    <div class="form">
      <label>{t("bates.prefix")}<input class="input" bind:value={o.prefix} placeholder="EXP-" /></label>
      <label>{t("bates.start")}<input class="input" type="number" min="0" bind:value={o.start} /></label>
      <label>{t("bates.digits")}<input class="input" type="number" min="1" max="15" bind:value={o.digits} /></label>
      <label>{t("bates.suffix")}<input class="input" bind:value={o.suffix} /></label>
      <label>{t("bates.position")}
        <select class="input" bind:value={o.position}>
          {#each positions as p, i}<option value={i}>{t(`bates.${p}`)}</option>{/each}
        </select>
      </label>
      <label>{t("bates.fontSize")}<input class="input" type="number" min="4" max="48" bind:value={o.fontSize} /></label>
    </div>
    <div class="preview">
      {#if invalid}
        <span class="pill danger">{t("bates.invalid")}</span>
      {:else}
        <span class="hint">{t("bates.preview")}</span> <code>{label(o.start)}</code> … <code>{label(o.start + total - 1)}</code>
      {/if}
    </div>
    <div class="foot-row">
      <span class="hint">{working ? t("pagesView.writing") : t("bates.footHint")}</span>
      <span class="grow"></span>
      <button class="btn" onclick={onclose} disabled={working}>{t("common.cancel")}</button>
      <button class="btn primary" onclick={apply} disabled={invalid || working}><Icon name="hash" size={14} /> {t("bates.apply")}</button>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); display: grid; place-items: center; z-index: 40; }
  .modal { width: min(560px, 94vw); display: flex; flex-direction: column; overflow: hidden; }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px 4px; }
  .head h2 { display: flex; align-items: center; gap: 8px; }
  .intro { padding: 0 18px 10px; margin: 0; }
  .form { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 10px 12px; padding: 0 18px; }
  .form label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; font-weight: 600; color: var(--text-2); }
  .preview { padding: 14px 18px; display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .preview code { font-size: 13px; background: var(--surface-2); padding: 2px 6px; border-radius: 6px; }
  .grow { flex: 1; }
  .foot-row { display: flex; align-items: center; gap: 8px; padding: 10px 18px 14px; border-top: 1px solid var(--border); }
</style>
