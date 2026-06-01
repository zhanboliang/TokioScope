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
</script>

<section class="panel">
  <header>{t("output.header")} · {rows.length}</header>
  <div class="body">
    {#each rows as r}
      <div class="row {r.source}">
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
  .row { display: grid; grid-template-columns: 44px auto 1fr; gap: 8px; font-family: var(--ts-mono); font-size: 11px; line-height: 1.5; color: var(--ts-fg); }
  .t { color: var(--ts-fg-2); }
  .ln { color: var(--ts-accent); }
  .stderr .text { color: #e07070; }
  .stdout .text { color: var(--ts-fg-2); font-style: italic; }
  .empty { color: var(--ts-fg-3); font-family: var(--ts-mono); font-size: 11px; font-style: italic; }
</style>
