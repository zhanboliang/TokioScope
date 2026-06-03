<script lang="ts">
  import { store } from "../store.svelte";
  import { ws } from "../workspace.svelte";
  import { t } from "../i18n.svelte";
  import { ipc } from "../ipc";

  // VSCode/IDEA-style bottom bar: the runner status + background-task progress
  // sit on the right. Click the status to see what the backend is doing.
  const busy = $derived(store.status.building || store.status.running);

  const kind = $derived(
    store.status.building ? "building"
      : store.status.last_error ? "err"
      : store.status.running ? "run"
      : store.status.ready ? "ok"
      : "wait",
  );
  const label = $derived(
    store.status.building ? t("status.building")
      : store.status.last_error ? t("status.error")
      : store.status.running ? t("status.running")
      : store.status.ready ? t("status.ready")
      : t("status.init"),
  );
  // last few backend log lines, so the popup shows "what's running"
  const logTail = $derived(store.rawStderr.slice(-6));

  let open = $state(false);
</script>

<section class="bar">
  <div class="nav">
    <button class="nav-btn" class:on={ws.treeCollapsed} aria-label="Toggle explorer"
      title={ws.treeCollapsed ? t("ide.explorer") : t("ide.collapseTree")}
      onclick={() => (ws.treeCollapsed = !ws.treeCollapsed)}>
      <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
        <rect x="9.5" y="2.5" width="4" height="3.5" rx="0.8" />
        <rect x="9.5" y="10" width="4" height="3.5" rx="0.8" />
        <path d="M4 4.25h5.5M4 11.75h5.5M4 4.25V11.75" />
      </svg>
    </button>
    <button class="nav-btn" class:on={ws.vizCollapsed} aria-label="Toggle visualization"
      title={ws.vizCollapsed ? t("ide.showViz") : t("ide.collapseViz")}
      onclick={() => (ws.vizCollapsed = !ws.vizCollapsed)}>
      <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round">
        <path d="M3.5 13V9.5M8 13V4M12.5 13V7.5" />
      </svg>
    </button>
  </div>

  <span class="spacer"></span>

  {#if busy}
    <div class="progress" aria-hidden="true"><div class="ind"></div></div>
  {/if}
  <button class="status {kind}" onclick={() => (open = !open)}
    aria-haspopup="dialog" aria-expanded={open} title={t("tasks.title")}>
    <span class="sdot"></span>{label}
  </button>

  {#if open}
    <button class="scrim" aria-label="Close" tabindex="-1" onpointerdown={() => (open = false)}></button>
    <div class="popup" role="dialog" aria-label={t("tasks.title")}>
      <header>{t("tasks.title")}</header>

      {#if busy || store.status.last_error || store.status.ready}
        <div class="task">
          <span class="dot {kind}"></span>
          <span class="name">{label}</span>
          {#if store.status.running}
            <button class="cancel" onclick={() => ipc.cancelRun()}>{t("tasks.cancel")}</button>
          {/if}
        </div>
        {#if busy}<div class="progress wide"><div class="ind"></div></div>{/if}
        {#if store.status.last_error}<div class="err-text">{store.status.last_error}</div>{/if}
        {#if store.status.cache_dir}<div class="meta" title={store.status.cache_dir}>{store.status.cache_dir}</div>{/if}
      {:else}
        <div class="idle">{t("tasks.idle")}</div>
      {/if}

      {#if logTail.length}
        <div class="log">
          {#each logTail as line}<div class="log-line">{line}</div>{/each}
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .bar {
    display: flex; align-items: center; gap: 10px; flex-shrink: 0;
    height: 22px; padding: 0 12px; position: relative;
    background: var(--ts-bg-1); border-top: 1px solid var(--ts-line);
    font-family: var(--ts-mono); font-size: 11px; color: var(--ts-fg-2);
  }
  .spacer { flex: 1; }

  /* left: panel-collapse toggles. Coloured (theme accent) when the panel is
     collapsed, so the active/toggled state reads across all three themes. */
  .nav { display: flex; gap: 1px; }
  .nav-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 24px; height: 18px; padding: 0; border: none; border-radius: 3px;
    background: transparent; color: var(--ts-fg-3); cursor: pointer;
  }
  .nav-btn:hover { color: var(--ts-fg); background: var(--ts-bg-3); }
  .nav-btn.on { color: var(--ts-accent); }
  .nav-btn svg { display: block; }

  /* IDEA-style indeterminate progress, shown while a backend task runs */
  .progress { width: 84px; height: 4px; border-radius: 2px; background: var(--ts-bg-3); overflow: hidden; flex-shrink: 0; }
  .progress .ind { width: 38%; height: 100%; border-radius: 2px; background: var(--ts-accent); animation: ind-slide 1.1s ease-in-out infinite; }
  @keyframes ind-slide { 0% { transform: translateX(-110%); } 100% { transform: translateX(290%); } }
  @media (prefers-reduced-motion: reduce) { .progress .ind { animation: none; width: 100%; } }

  /* no background block — just a theme-coloured dot + label, adapts to all themes */
  .status {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 0; border: none; background: transparent; cursor: pointer;
    font-family: var(--ts-mono); font-size: 11px; letter-spacing: 0.02em; color: var(--ts-fg-3);
  }
  .status .sdot { width: 7px; height: 7px; border-radius: 50%; background: currentColor; flex-shrink: 0; }
  .status.building, .status.run { color: var(--ts-accent); }
  .status.ok { color: var(--ts-st-done); }
  .status.err { color: var(--ts-error); }
  .status.wait { color: var(--ts-fg-3); }
  .status:hover { text-decoration: underline; text-underline-offset: 3px; }

  /* click → background-tasks popup, opens upward above the bar */
  .scrim { position: fixed; inset: 0; z-index: 129; background: transparent; border: none; cursor: default; }
  .popup {
    position: fixed; z-index: 130; bottom: 26px; right: 8px;
    width: 340px; max-width: calc(100vw - 24px); padding: 10px 12px;
    background: var(--ts-bg-1); border: 1px solid var(--ts-line-2); border-radius: 8px;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.5); font-family: var(--ts-sans);
  }
  :global(html[data-theme="light"]) .popup { box-shadow: 0 14px 40px rgba(0, 0, 0, 0.18); }
  .popup header { margin-bottom: 8px; font-size: 11px; font-weight: 600; letter-spacing: 0.02em; color: var(--ts-fg-2); }
  .task { display: flex; align-items: center; gap: 8px; }
  .task .dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .dot.building, .dot.run { background: var(--ts-accent); }
  .dot.ok { background: var(--ts-st-done); }
  .dot.err { background: var(--ts-error); }
  .dot.wait { background: var(--ts-fg-3); }
  .task .name { flex: 1 1 auto; min-width: 0; font-size: 12.5px; color: var(--ts-fg); }
  .task .cancel {
    padding: 2px 8px; border-radius: 5px; border: 1px solid var(--ts-line-2);
    background: transparent; color: var(--ts-error); font-size: 11px; cursor: pointer; flex-shrink: 0;
  }
  .task .cancel:hover { background: color-mix(in srgb, var(--ts-error) 15%, transparent); }
  .progress.wide { width: 100%; height: 5px; margin: 8px 0 2px; }
  .idle { font-size: 12px; color: var(--ts-fg-3); }
  .err-text { margin-top: 6px; font-family: var(--ts-mono); font-size: 11px; color: var(--ts-error); white-space: pre-wrap; word-break: break-word; }
  .meta { margin-top: 6px; font-family: var(--ts-mono); font-size: 10px; color: var(--ts-fg-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .log { margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--ts-line); max-height: 120px; overflow: auto; }
  .log-line { font-family: var(--ts-mono); font-size: 10.5px; line-height: 1.5; color: var(--ts-fg-2); white-space: pre-wrap; word-break: break-word; }
</style>
