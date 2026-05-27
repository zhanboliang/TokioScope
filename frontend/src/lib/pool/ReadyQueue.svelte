<script lang="ts">
  import { store } from "../store.svelte";

  const f = $derived(store.currentFrame);
  const ids = $derived(f?.ready ?? []);
  const tasks = $derived(f?.tasks);
</script>

<section class="panel">
  <header>READY · {ids.length}</header>
  <ul>
    {#each ids.slice(0, 12) as id}
      {@const t = tasks?.get(id)}
      <li>
        <span class="dot" style="background: var(--ts-st-ready)"></span>
        <span class="name">{t?.name ?? `#${id}`}</span>
        <span class="line">L{t?.currentLine ?? "?"}</span>
      </li>
    {/each}
    {#if ids.length > 12}
      <li class="more">… +{ids.length - 12}</li>
    {/if}
    {#if ids.length === 0}
      <li class="empty">(empty)</li>
    {/if}
  </ul>
</section>

<style>
  .panel { background: var(--ts-bg-1); padding: 8px 10px; overflow: auto; }
  header { font-family: var(--ts-mono); font-size: 10px; letter-spacing: 0.1em; color: var(--ts-fg-2); margin-bottom: 6px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  li { display: grid; grid-template-columns: 8px 1fr auto; align-items: center; gap: 6px; font-family: var(--ts-mono); font-size: 11px; }
  .dot { width: 8px; height: 8px; border-radius: 50%; }
  .name { color: var(--ts-fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .line { color: var(--ts-fg-2); font-size: 10px; }
  .empty, .more { color: var(--ts-fg-3); font-style: italic; }
</style>
