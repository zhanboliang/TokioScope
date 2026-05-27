<script lang="ts">
  import { store } from "../store.svelte";

  const f = $derived(store.currentFrame);
  const slots = $derived(f?.blocking ?? []);
  const tick = $derived(f?.tick ?? 0);
  const tasks = $derived(f?.tasks);
</script>

<section class="panel">
  <header>BLOCKING</header>
  <ul>
    {#each slots as s, i}
      {@const t = s.taskId != null ? tasks?.get(s.taskId) : null}
      {@const elapsed = s.taskId != null ? Math.max(0, tick - s.startTick) : 0}
      {@const pct = s.taskId != null && s.durationTicks > 0 ? Math.min(100, (elapsed / s.durationTicks) * 100) : 0}
      <li class:idle={!t}>
        <span class="slot">B{i}</span>
        {#if t}
          <div class="bar"><div class="fill" style="width: {pct}%"></div></div>
          <span class="name" title={`L${t.currentLine}`}>{t.name}</span>
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
  li { display: grid; grid-template-columns: 28px 1fr auto; align-items: center; gap: 6px; font-family: var(--ts-mono); font-size: 11px; }
  .slot { color: var(--ts-fg-2); }
  .bar { background: var(--ts-bg-2); height: 8px; border-radius: 2px; overflow: hidden; }
  .fill { height: 100%; background: var(--ts-st-blocking); transition: width 80ms linear; }
  .name { color: var(--ts-fg); font-size: 10px; max-width: 120px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dim { grid-column: 2 / -1; color: var(--ts-fg-3); }
  li.idle .slot { color: var(--ts-fg-3); }
</style>
