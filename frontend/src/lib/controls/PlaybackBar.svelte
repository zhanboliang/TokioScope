<script lang="ts">
  import { store } from "../store.svelte";
  import { t } from "../i18n.svelte";

  function step(d: number) {
    if (!store.totalTicks) return;
    store.playhead = Math.max(0, Math.min(store.totalTicks - 1, Math.round(store.playhead) + d));
    store.playing = false;
  }
</script>

<section class="bar">
  <!-- to start -->
  <button class="ctrl" onclick={() => { store.playhead = 0; store.playing = false; }}
    title={t("pb.toStart")} aria-label="To start">
    <svg viewBox="0 0 16 16" width="13" height="13" fill="currentColor" aria-hidden="true">
      <rect x="3" y="3.5" width="1.7" height="9" rx="0.6" />
      <path d="M13 3.5 L6.2 8 L13 12.5 Z" />
    </svg>
  </button>

  <!-- step back (reverse step-over) -->
  <button class="ctrl" onclick={() => step(-1)} title={t("pb.stepBack")} aria-label="Step back">
    <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
      <path d="M13 9 C 13 4.6 3 4.6 3 9" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
      <path d="M3 11 L1.4 8 L4.6 8 Z" fill="currentColor" />
      <circle cx="8" cy="11.3" r="1.05" fill="currentColor" />
    </svg>
  </button>

  <!-- resume / pause -->
  <button class="ctrl play" onclick={() => store.playing = !store.playing}
    title={t("pb.playPause")} aria-label="Play / Pause">
    {#if store.playing}
      <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor" aria-hidden="true">
        <rect x="4" y="3.2" width="3" height="9.6" rx="0.6" />
        <rect x="9" y="3.2" width="3" height="9.6" rx="0.6" />
      </svg>
    {:else}
      <svg viewBox="0 0 16 16" width="13" height="13" fill="currentColor" aria-hidden="true">
        <path d="M4.5 3 L13 8 L4.5 13 Z" />
      </svg>
    {/if}
  </button>

  <!-- step forward (step over) -->
  <button class="ctrl" onclick={() => step(1)} title={t("pb.stepFwd")} aria-label="Step over">
    <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
      <path d="M3 9 C 3 4.6 13 4.6 13 9" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
      <path d="M13 11 L11.4 8 L14.6 8 Z" fill="currentColor" />
      <circle cx="8" cy="11.3" r="1.05" fill="currentColor" />
    </svg>
  </button>

  <!-- to end -->
  <button class="ctrl" onclick={() => { store.playhead = Math.max(0, store.totalTicks - 1); store.playing = false; }}
    title={t("pb.toEnd")} aria-label="To end">
    <svg viewBox="0 0 16 16" width="13" height="13" fill="currentColor" aria-hidden="true">
      <path d="M3 3.5 L9.8 8 L3 12.5 Z" />
      <rect x="11.3" y="3.5" width="1.7" height="9" rx="0.6" />
    </svg>
  </button>

  <span class="div"></span>

  <!-- restart -->
  <button class="ctrl" onclick={() => { store.playhead = 0; store.playing = false; }}
    title={t("pb.restart")} aria-label="Restart">
    <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
      <path d="M8 3.4 A4.6 4.6 0 1 0 12.4 6" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" />
      <path d="M8 1 L8 5.6 L5.2 3.3 Z" fill="currentColor" />
    </svg>
  </button>

  <label class="speed">
    <span>{t("pb.speed")}</span>
    <input type="range" min="0.1" max="20" step="0.1" bind:value={store.speed} />
    <output>{store.speed.toFixed(1)}×</output>
  </label>

  <label class="toggle">
    <input type="checkbox" bind:checked={store.loop_} />
    <span>{t("pb.loop")}</span>
  </label>
  <label class="toggle">
    <input type="checkbox" bind:checked={store.follow} />
    <span>{t("pb.follow")}</span>
  </label>
</section>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 12px;
    background: var(--ts-bg-1);
    border-top: 1px solid var(--ts-line);
    font-family: var(--ts-mono);
    font-size: 11px;
  }

  /* IDEA debug-toolbar style: flat, borderless icon buttons */
  .ctrl {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 5px;
    color: var(--ts-fg-2);
    cursor: pointer;
    transition: background 100ms ease, color 100ms ease, transform 60ms ease;
  }
  .ctrl:hover { background: var(--ts-bg-3); color: var(--ts-fg); }
  .ctrl:active { transform: scale(0.9); }
  .ctrl svg { display: block; }
  /* resume / pause — green, like IDEA's Resume */
  .ctrl.play { color: var(--ts-st-done); }
  .ctrl.play:hover { background: rgba(106, 135, 89, 0.22); }

  .div { width: 1px; height: 15px; background: var(--ts-line-2); margin: 0 5px; }

  .speed { display: flex; align-items: center; gap: 6px; margin-left: 14px; color: var(--ts-fg-2); }
  .speed input { width: 140px; accent-color: var(--ts-accent); }
  .speed output { color: var(--ts-fg); min-width: 36px; text-align: right; }
  .toggle { display: flex; align-items: center; gap: 4px; color: var(--ts-fg-2); margin-left: 12px; }
  .toggle input { accent-color: var(--ts-accent); }
</style>
