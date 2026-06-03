<script lang="ts">
  // Compact themed dropdown — the reusable "pick one of several options" control.
  // Used wherever a >1-option choice appears (theme, language, …) so the UI never
  // needs a row of standalone segmented buttons.
  let {
    value,
    options,
    onChange,
  }: {
    value: string;
    options: { id: string; label: string }[];
    onChange: (id: string) => void;
  } = $props();

  let open = $state(false);
  const current = $derived(options.find((o) => o.id === value)?.label ?? value);

  function pick(id: string) {
    onChange(id);
    open = false;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") { open = false; return; }
    if (!open) return;
    const i = options.findIndex((o) => o.id === value);
    if (e.key === "ArrowDown") { e.preventDefault(); onChange(options[Math.min(i + 1, options.length - 1)].id); }
    else if (e.key === "ArrowUp") { e.preventDefault(); onChange(options[Math.max(i - 1, 0)].id); }
    else if (e.key === "Enter") { e.preventDefault(); open = false; }
  }
</script>

<div class="sel">
  <button type="button" class="trigger" aria-haspopup="listbox" aria-expanded={open}
    onclick={() => (open = !open)} onkeydown={onKey}>
    <span class="cur">{current}</span>
    <svg viewBox="0 0 16 16" width="11" height="11" fill="none" stroke="currentColor"
      stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M4 6l4 4 4-4" />
    </svg>
  </button>
  {#if open}
    <button type="button" class="scrim" aria-label="Close" tabindex="-1"
      onpointerdown={() => (open = false)}></button>
    <ul class="menu" role="listbox">
      {#each options as o (o.id)}
        <li>
          <button type="button" class="opt" class:on={o.id === value}
            role="option" aria-selected={o.id === value} onclick={() => pick(o.id)}>
            {o.label}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .sel { position: relative; flex: 1 1 auto; min-width: 0; }
  .trigger {
    display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%;
    padding: 6px 9px; border-radius: 6px; border: 1px solid var(--ts-line-2);
    background: var(--ts-bg-2); color: var(--ts-fg); font-size: 12px; cursor: pointer;
    transition: border-color 100ms ease;
  }
  .trigger[aria-expanded="true"] { border-color: var(--ts-accent); }
  .cur { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .trigger svg { flex-shrink: 0; color: var(--ts-fg-3); }

  /* transparent catcher so a click anywhere outside closes the popover */
  .scrim { position: fixed; inset: 0; z-index: 1; background: transparent; border: none; cursor: default; }

  .menu {
    position: absolute; z-index: 2; top: calc(100% + 4px); left: 0; right: 0; margin: 0;
    padding: 4px; list-style: none; max-height: 240px; overflow-y: auto;
    background: var(--ts-bg-1); border: 1px solid var(--ts-line-2); border-radius: 8px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
  }
  .opt {
    display: block; width: 100%; text-align: left; padding: 6px 9px; border: none; border-radius: 5px;
    background: transparent; color: var(--ts-fg-2); font-size: 12px; cursor: pointer;
  }
  .opt:hover { background: var(--ts-bg-3); color: var(--ts-fg); }
  .opt.on { color: var(--ts-accent); }
</style>
