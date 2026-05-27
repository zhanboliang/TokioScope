<script lang="ts">
  import { store } from "../store.svelte";

  const f = $derived(store.currentFrame);
  const log = $derived(f?.wakeLog ?? []);
</script>

<section class="panel">
  <header>WAKES · 最近 {log.length}/16</header>
  <ul>
    {#each [...log].reverse() as w}
      <li>
        <span class="t">t{w.tick}</span>
        <span class="id">#{w.taskId}</span>
        <span class="cause">{w.cause}</span>
      </li>
    {/each}
    {#if log.length === 0}<li class="empty">(no wakes yet)</li>{/if}
  </ul>
</section>

<style>
  .panel { background: var(--ts-bg-1); padding: 8px 10px; overflow: auto; }
  header { font-family: var(--ts-mono); font-size: 10px; letter-spacing: 0.1em; color: var(--ts-fg-2); margin-bottom: 6px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  li { display: grid; grid-template-columns: 40px 36px 1fr; gap: 6px; font-family: var(--ts-mono); font-size: 11px; }
  .t { color: var(--ts-fg-2); }
  .id { color: var(--ts-fg); }
  .cause { color: var(--ts-accent); }
  .empty { color: var(--ts-fg-3); font-style: italic; grid-template-columns: 1fr; }
</style>
