<script lang="ts">
  import { store } from "../store.svelte";

  const f = $derived(store.currentFrame);
  const flavor = $derived(store.sim?.flavor ?? store.analysis?.runtime.flavor ?? "—");
  const workersTotal = $derived(store.sim?.workerCount ?? store.analysis?.runtime.worker_threads ?? 0);
  const blockingTotal = $derived(store.sim?.blockingSlots ?? 8);
  const wActive = $derived(f ? f.workers.filter((w) => w != null).length : 0);
  const bActive = $derived(f ? f.blocking.filter((s) => s.taskId != null).length : 0);
  const readyN = $derived(f?.ready.length ?? 0);
  const tick = $derived(Math.round(store.playhead));
</script>

<section class="bar">
  <span class="stat">tick <b>{tick}</b> / {Math.max(0, store.totalTicks - 1)}</span>
  <span class="stat">t <b>{(tick * 0.1).toFixed(1)}s</b></span>
  <span class="stat">W <b>{wActive}/{workersTotal}</b></span>
  <span class="stat">B <b>{bActive}/{blockingTotal}</b></span>
  <span class="stat">Ready <b>{readyN}</b></span>
  <span class="stat flavor">flavor <b>{flavor}</b></span>

  {#if store.status.building}
    <span class="indicator building">building runner crate…</span>
  {:else if store.status.last_error}
    <span class="indicator err" title={store.status.last_error}>runner error</span>
  {:else if store.status.running}
    <span class="indicator run">running</span>
  {:else if store.status.ready}
    <span class="indicator ok">ready</span>
  {:else}
    <span class="indicator wait">init…</span>
  {/if}

  <span class="spacer"></span>
</section>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 12px;
    background: var(--ts-bg-2);
    border-top: 1px solid var(--ts-line);
    font-family: var(--ts-mono);
    font-size: 11px;
    color: var(--ts-fg-2);
  }
  .stat b { color: var(--ts-fg); font-weight: 600; }
  .flavor b { color: var(--ts-accent); }
  .spacer { flex: 1; }
  .indicator { padding: 2px 8px; border-radius: 2px; font-size: 10px; letter-spacing: 0.02em; }
  .building, .run { background: rgba(232, 133, 58, 0.18); color: var(--ts-accent); }
  .ok { background: rgba(91, 174, 122, 0.18); color: var(--ts-st-done); }
  .err { background: rgba(192, 80, 80, 0.18); color: #e07070; }
  .wait { background: rgba(139, 149, 165, 0.18); color: var(--ts-fg-3); }
</style>
