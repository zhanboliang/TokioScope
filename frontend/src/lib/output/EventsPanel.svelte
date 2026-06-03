<script lang="ts">
  import { store } from "../store.svelte";
  import { t } from "../i18n.svelte";

  // Execution-navigation view: only the program's own output (println!), in tick
  // order. No raw stdout/stderr build noise — that lives in the Output tab.
  const rows = $derived.by(() =>
    [...(store.sim?.printlns ?? [])].sort((a, b) => a.tick - b.tick),
  );

  // The tick the playhead is parked on, so we can mark "where we are".
  const curTick = $derived(store.sim ? (store.currentFrame?.tick ?? 0) : -1);

  let body: HTMLDivElement;

  // Keep the current entry in view as the playhead advances.
  $effect(() => {
    void curTick;
    void rows.length;
    if (!body || !store.sim) return;
    const els = body.querySelectorAll<HTMLElement>("[data-tick]");
    let last: HTMLElement | null = null;
    for (const el of els) {
      if (Number(el.dataset.tick) <= curTick) last = el;
    }
    if (last) {
      const target = last.offsetTop - body.clientHeight + last.offsetHeight + 8;
      body.scrollTop = Math.max(0, target);
    }
  });

  function jump(tick: number) {
    if (store.sim) store.playhead = tick;
  }
</script>

<section class="panel">
  <div class="body" bind:this={body}>
    {#each rows as r}
      <div
        class="row"
        class:cur={r.tick === curTick}
        data-tick={r.tick}
        role="button"
        tabindex="0"
        onclick={() => jump(r.tick)}
        onkeydown={(e) => { if (e.key === "Enter") jump(r.tick); }}
      >
        <span class="t">t{r.tick}</span>
        <span class="ln">L{r.line}</span>
        <span class="text">{r.text}</span>
      </div>
    {/each}
    {#if rows.length === 0}
      <div class="empty">{t("events.empty")}</div>
    {/if}
  </div>
</section>

<style>
  .panel { background: var(--ts-bg-1); display: flex; flex-direction: column; height: 100%; overflow: hidden; }
  .body { flex: 1 1 auto; min-height: 0; overflow: auto; padding: 6px 12px; }
  .row { display: grid; grid-template-columns: 44px auto 1fr; gap: 8px; font-family: var(--ts-output-font); font-size: var(--ts-output-size); line-height: 1.5; color: var(--ts-fg); border-radius: 3px; padding: 0 4px; margin: 0 -4px; cursor: pointer; transition: background 140ms ease; }
  .row:hover { background: var(--ts-bg-3); }
  /* the entry on the tick the playhead is parked on */
  .row.cur { background: rgba(204, 120, 50, 0.18); box-shadow: inset 2px 0 0 var(--ts-st-running); }
  .t { color: var(--ts-fg-2); }
  .cur .t { color: var(--ts-accent); }
  .ln { color: var(--ts-accent); }
  .empty { color: var(--ts-fg-3); font-family: var(--ts-output-font); font-size: var(--ts-output-size); font-style: italic; }
</style>
