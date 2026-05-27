<script lang="ts">
  import { store } from "../store.svelte";

  const f = $derived(store.currentFrame);
  const workers = $derived(f?.workers ?? []);
  const tasks = $derived(f?.tasks);
</script>

<section class="panel">
  <header>WORKERS</header>
  <ul>
    {#each workers as wid, i}
      {@const t = wid != null ? tasks?.get(wid) : null}
      <li class:idle={!t}>
        <span class="slot">W{i}</span>
        {#if t}
          <span class="dot" style="background: var(--ts-st-{t.state})"></span>
          <span class="name" title={`line ${t.currentLine} · ${t.eventCount} evts`}>{t.name}</span>
          <span class="line">L{t.currentLine}</span>
        {:else}
          <span class="dim">idle</span>
        {/if}
      </li>
    {/each}
  </ul>
</section>

<style>
  .panel { background: var(--ts-bg-1); padding: 8px 10px; overflow: auto; }
  header { font-family: var(--ts-mono); font-size: 10px; letter-spacing: 0.1em; color: var(--ts-fg-2); margin-bottom: 6px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 3px; }
  li { display: grid; grid-template-columns: 32px 8px 1fr auto; align-items: center; gap: 6px; font-family: var(--ts-mono); font-size: 11px; color: var(--ts-fg); padding: 2px 0; }
  li.idle { color: var(--ts-fg-3); }
  .slot { color: var(--ts-fg-2); }
  .dot { width: 8px; height: 8px; border-radius: 50%; }
  .name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .line { color: var(--ts-fg-2); font-size: 10px; }
  .dim { grid-column: 2 / -1; color: var(--ts-fg-3); }
</style>
