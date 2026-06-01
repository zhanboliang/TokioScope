<script lang="ts">
  import { store } from "../store.svelte";
  import { t } from "../i18n.svelte";

  type Row = { tick: number; source: "println" | "stdout" | "stderr"; text: string; line?: number };

  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    for (const p of store.sim?.printlns ?? []) {
      out.push({ tick: p.tick, source: "println", text: p.text, line: p.line });
    }
    for (const s of store.rawStdout) {
      out.push({ tick: s.tick, source: "stdout", text: s.text });
    }
    for (const e of store.rawStderr) {
      out.push({ tick: -1, source: "stderr", text: e });
    }
    out.sort((a, b) => a.tick - b.tick);
    return out;
  });

  // The tick the playhead is currently on. When no sim is loaded (e.g. a build
  // error with only stderr), nothing is "future" — show the full log undimmed.
  const curTick = $derived(store.sim ? (store.currentFrame?.tick ?? 0) : Infinity);

  let body: HTMLDivElement;

  // Keep the latest already-reached line near the bottom, terminal-style, as the
  // playhead advances — so the output streams in sync with the visualization.
  $effect(() => {
    void curTick;
    void rows.length;
    if (!body || !store.sim) return;
    const els = body.querySelectorAll<HTMLElement>("[data-tick]");
    let last: HTMLElement | null = null;
    for (const el of els) {
      const tk = Number(el.dataset.tick);
      if (tk >= 0 && tk <= curTick) last = el;
    }
    if (last) {
      const target = last.offsetTop - body.clientHeight + last.offsetHeight + 8;
      body.scrollTop = Math.max(0, target);
    }
  });

  function jump(tick: number) {
    if (tick >= 0 && store.sim) store.playhead = tick;
  }
</script>

<section class="panel">
  <header>{t("output.header")} · {rows.length}</header>
  <div class="body" bind:this={body}>
    {#each rows as r}
      {@const future = r.tick >= 0 && r.tick > curTick}
      {@const cur = r.tick >= 0 && r.tick === curTick}
      <div
        class="row {r.source}"
        class:future
        class:cur
        data-tick={r.tick}
        role={r.tick >= 0 ? "button" : undefined}
        tabindex={r.tick >= 0 ? 0 : undefined}
        onclick={() => jump(r.tick)}
        onkeydown={(e) => { if (e.key === "Enter") jump(r.tick); }}
      >
        <span class="t">{r.tick < 0 ? "—" : `t${r.tick}`}</span>
        {#if r.line}<span class="ln">L{r.line}</span>{/if}
        <span class="text">{r.text}</span>
      </div>
    {/each}
    {#if rows.length === 0}
      <div class="empty">{t("output.empty")}</div>
    {/if}
  </div>
</section>

<style>
  .panel { background: var(--ts-bg-1); border-top: 1px solid var(--ts-line); display: grid; grid-template-rows: auto 1fr; height: 100%; overflow: hidden; }
  header { padding: 6px 12px; font-family: var(--ts-mono); font-size: 10px; letter-spacing: 0.1em; color: var(--ts-fg-2); border-bottom: 1px solid var(--ts-line); }
  .body { overflow: auto; padding: 6px 12px; }
  .row { display: grid; grid-template-columns: 44px auto 1fr; gap: 8px; font-family: var(--ts-mono); font-size: 11px; line-height: 1.5; color: var(--ts-fg); border-radius: 3px; padding: 0 4px; margin: 0 -4px; transition: opacity 140ms ease, background 140ms ease; }
  .row[role="button"] { cursor: pointer; }
  .row[role="button"]:hover { background: var(--ts-bg-3); }
  /* not-yet-reached output stays visible but recedes */
  .row.future { opacity: 0.38; }
  /* output emitted on the tick the playhead is parked on */
  .row.cur { background: rgba(204, 120, 50, 0.18); box-shadow: inset 2px 0 0 var(--ts-st-running); }
  .t { color: var(--ts-fg-2); }
  .cur .t { color: var(--ts-accent); }
  .ln { color: var(--ts-accent); }
  .stderr .text { color: #e07070; }
  .stdout .text { color: var(--ts-fg-2); font-style: italic; }
  .empty { color: var(--ts-fg-3); font-family: var(--ts-mono); font-size: 11px; font-style: italic; }
</style>
