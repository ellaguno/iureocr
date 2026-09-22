<script lang="ts">
  import { app } from "../lib/state.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="toasts">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}">
      <Icon name={t.kind === "success" ? "check" : t.kind === "error" ? "alert" : "info"} size={16} />
      <span>{t.text}</span>
    </div>
  {/each}
</div>

<style>
  .toasts { position: fixed; right: 18px; bottom: 18px; display: flex; flex-direction: column; gap: 8px; z-index: 50; max-width: 420px; }
  .toast { display: flex; gap: 10px; align-items: flex-start; padding: 10px 14px; border-radius: 10px; background: var(--surface); border: 1px solid var(--border); box-shadow: var(--shadow); font-size: 13px; animation: in 0.2s ease; }
  .toast.success { border-color: var(--success); color: var(--success); }
  .toast.error { border-color: var(--danger); color: var(--danger); }
  .toast span { color: var(--text); }
  @keyframes in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: none; } }
</style>
