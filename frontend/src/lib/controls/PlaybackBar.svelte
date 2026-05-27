<script lang="ts">
  import { store } from "../store.svelte";

  function step(d: number) {
    if (!store.totalTicks) return;
    store.playhead = Math.max(0, Math.min(store.totalTicks - 1, Math.round(store.playhead) + d));
    store.playing = false;
  }
</script>

<section class="bar">
  <button onclick={() => { store.playhead = 0; store.playing = false; }} title="Home">⏮</button>
  <button onclick={() => step(-1)} title="←">◀</button>
  <button class="play" onclick={() => store.playing = !store.playing} title="Space">
    {store.playing ? "❚❚" : "▶"}
  </button>
  <button onclick={() => step(1)} title="→">▶</button>
  <button onclick={() => { store.playhead = Math.max(0, store.totalTicks - 1); store.playing = false; }} title="End">⏭</button>
  <button class="reset" onclick={() => { store.playhead = 0; store.playing = false; }} title="R">↻</button>

  <label class="speed">
    <span>SPEED</span>
    <input type="range" min="0.5" max="20" step="0.5" bind:value={store.speed} />
    <output>{store.speed.toFixed(1)}×</output>
  </label>

  <label class="toggle">
    <input type="checkbox" bind:checked={store.loop_} />
    <span>LOOP</span>
  </label>
  <label class="toggle">
    <input type="checkbox" bind:checked={store.follow} />
    <span>FOLLOW</span>
  </label>
</section>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: var(--ts-bg-1);
    border-top: 1px solid var(--ts-line);
    font-family: var(--ts-mono);
    font-size: 11px;
  }
  button {
    background: transparent;
    border: 1px solid var(--ts-line-2);
    color: var(--ts-fg);
    padding: 4px 8px;
    border-radius: 3px;
    cursor: pointer;
    font-family: var(--ts-mono);
    min-width: 28px;
  }
  button:hover { border-color: var(--ts-accent); color: var(--ts-accent); }
  .play { background: var(--ts-accent); color: var(--ts-bg-1); border-color: var(--ts-accent); min-width: 44px; }
  .speed { display: flex; align-items: center; gap: 6px; margin-left: 16px; color: var(--ts-fg-2); }
  .speed input { width: 140px; accent-color: var(--ts-accent); }
  .speed output { color: var(--ts-fg); min-width: 36px; text-align: right; }
  .toggle { display: flex; align-items: center; gap: 4px; color: var(--ts-fg-2); margin-left: 12px; }
  .toggle input { accent-color: var(--ts-accent); }
</style>
