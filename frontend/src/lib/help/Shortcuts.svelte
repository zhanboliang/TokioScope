<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { SHORTCUTS } from "../shortcuts";
  const dispatch = createEventDispatcher();
</script>

<div class="backdrop" onclick={() => dispatch("close")}
  onkeydown={(e) => { if (e.key === "Escape" || e.key === "Enter") dispatch("close"); }}
  role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog" aria-modal="true" tabindex="-1">
    <header>
      <h2>快捷键</h2>
      <button onclick={() => dispatch("close")}>×</button>
    </header>
    <div class="grid">
      {#each SHORTCUTS as g}
        <section>
          <h3>{g.group}</h3>
          <dl>
            {#each g.items as it}
              <dt><kbd>{it.keys}</kbd></dt>
              <dd>{it.action}</dd>
            {/each}
          </dl>
        </section>
      {/each}
    </div>
    <footer><kbd>?</kbd> 切换此面板 · <kbd>Esc</kbd> 关闭</footer>
  </div>
</div>

<svelte:window onkeydown={(e) => { if (e.key === "Escape") dispatch("close"); }} />

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.45);
    display: grid; place-items: center;
    z-index: 100;
    backdrop-filter: blur(2px);
  }
  :global(html[data-theme="light"]) .backdrop { background: rgba(0,0,0,0.25); }
  .modal {
    background: var(--ts-bg-1);
    border: 1px solid var(--ts-line-2);
    border-radius: 6px;
    width: min(880px, 92vw);
    max-height: 86vh;
    overflow: hidden;
    display: grid;
    grid-template-rows: auto 1fr auto;
    box-shadow: 0 24px 80px rgba(0,0,0,0.6);
  }
  header { display: flex; justify-content: space-between; align-items: center; padding: 14px 20px; border-bottom: 1px solid var(--ts-line); }
  header h2 { margin: 0; font-family: var(--ts-sans); font-size: 14px; font-weight: 600; letter-spacing: 0.04em; color: var(--ts-fg); }
  header button { background: transparent; border: none; color: var(--ts-fg-2); font-size: 20px; cursor: pointer; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 24px; padding: 20px; overflow: auto; }
  section h3 { margin: 0 0 10px; font-family: var(--ts-mono); font-size: 10px; letter-spacing: 0.1em; text-transform: uppercase; color: var(--ts-accent); }
  dl { margin: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 6px 12px; }
  dt { font-family: var(--ts-mono); }
  dd { margin: 0; color: var(--ts-fg-2); font-family: var(--ts-sans); font-size: 12px; }
  kbd {
    display: inline-block;
    font-family: var(--ts-mono);
    font-size: 11px;
    padding: 2px 6px;
    background: var(--ts-bg-0);
    border: 1px solid var(--ts-line-2);
    border-radius: 3px;
    color: var(--ts-fg);
  }
  footer { padding: 10px 20px; border-top: 1px solid var(--ts-line); font-family: var(--ts-sans); font-size: 11px; color: var(--ts-fg-3); }
</style>
