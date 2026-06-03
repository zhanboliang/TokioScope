<script lang="ts">
  import { ws } from "../workspace.svelte";
  import { t } from "../i18n.svelte";
</script>

<div class="tabs-row">
  <div class="tabs">
    {#each ws.tabs as tab (tab.id)}
      <div class="tab" class:active={ws.activeId === tab.id}>
        <button class="lab" onclick={() => ws.setActive(tab.id)} title={tab.path ?? t("tab.untitled")}>
          <span class="tt">{tab.path ? tab.title : t("tab.untitled")}</span>
          {#if ws.isDirty(tab)}<span class="dot" aria-hidden="true"></span>{/if}
        </button>
        <button class="x" title={t("tab.close")} aria-label="Close tab" onclick={() => ws.closeTab(tab.id)}>×</button>
      </div>
    {/each}
    <button class="new" title={t("tab.new")} aria-label="New buffer" onclick={() => ws.newUntitled()}>+</button>
  </div>
</div>

<style>
  .tabs-row {
    display: flex; align-items: stretch; flex-shrink: 0; height: 32px;
    background: var(--ts-bg-1); border-bottom: 1px solid var(--ts-line); overflow: hidden;
  }
  .tabs { display: flex; align-items: stretch; flex: 1 1 auto; overflow-x: auto; }
  .tab {
    display: flex; align-items: center; flex-shrink: 0; max-width: 200px;
    border-right: 1px solid var(--ts-line); background: var(--ts-bg-1);
  }
  .tab.active { background: var(--ts-bg-2); box-shadow: inset 0 2px 0 var(--ts-accent); }
  .lab {
    display: flex; align-items: center; gap: 6px; padding: 0 6px 0 12px; height: 100%;
    background: transparent; border: none; cursor: pointer; min-width: 0;
    font-family: var(--ts-mono); font-size: 11.5px; color: var(--ts-fg-2);
  }
  .tab.active .lab { color: var(--ts-fg); }
  .lab .tt { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lab .dot { flex-shrink: 0; width: 6px; height: 6px; border-radius: 50%; background: var(--ts-accent); }
  .x {
    display: inline-flex; align-items: center; justify-content: center; width: 18px; height: 18px;
    margin-right: 5px; padding: 0; background: transparent; border: none; border-radius: 4px;
    color: var(--ts-fg-3); font-size: 14px; line-height: 1; cursor: pointer;
  }
  .x:hover { background: var(--ts-bg-3); color: var(--ts-fg); }

  .new {
    display: inline-flex; align-items: center; justify-content: center;
    width: 28px; flex-shrink: 0; background: transparent; border: none;
    color: var(--ts-fg-3); font-size: 16px; line-height: 1; cursor: pointer;
  }
  .new:hover { color: var(--ts-fg); background: var(--ts-bg-2); }
</style>
