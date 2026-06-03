<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { ipc, onEvent, onStdout, onStderr, onDone, onStatus, type Example } from "./lib/ipc";
  import { store } from "./lib/store.svelte";
  import { ws } from "./lib/workspace.svelte";
  import { runActive } from "./lib/run";
  import { bindGlobal } from "./lib/shortcuts";
  import { initPrefs, persistPrefs } from "./lib/prefs";
  import { applyFonts } from "./lib/font";
  import { t, locale } from "./lib/i18n.svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import CodeEditor from "./lib/editor/CodeEditor.svelte";
  import Timeline from "./lib/timeline/Timeline.svelte";
  import Minimap from "./lib/timeline/Minimap.svelte";
  import SchedulerStage from "./lib/stage/SchedulerStage.svelte";
  import PlaybackBar from "./lib/controls/PlaybackBar.svelte";
  import StatusBar from "./lib/controls/StatusBar.svelte";
  import Settings from "./lib/controls/Settings.svelte";
  import AppMenu from "./lib/controls/AppMenu.svelte";
  import OutputPanel from "./lib/output/OutputPanel.svelte";
  import EventsPanel from "./lib/output/EventsPanel.svelte";
  import Shortcuts from "./lib/help/Shortcuts.svelte";
  import FileTree from "./lib/workspace/FileTree.svelte";
  import Tabs from "./lib/workspace/Tabs.svelte";
  import Terminal from "./lib/terminal/Terminal.svelte";
  import { type } from "@tauri-apps/plugin-os";

  let examples = $state<Example[]>([]);
  let unlisteners: (() => void)[] = [];
  let rafHandle = 0;
  let dragOver = $state(false);
  let warnDismissed = $state(false);
  let menuOpen = $state(false);

  const outputCount = $derived(
    store.rawStdout.length + store.rawStderr.length + (store.sim?.printlns.length ?? 0),
  );
  const showProjectWarn = $derived(
    store.isProjectRun && !warnDismissed && (!!store.sim || store.status.running),
  );

  // active drag descriptor (null when idle)
  let drag = $state<null | { kind: "tree" | "editor" | "bottom" | "tl"; start: number; base: number }>(null);

  function loadExample(idx: number) {
    if (examples[idx]) {
      ws.loadExampleCode(examples[idx].code);
      analyze();
    }
  }

  async function analyze() {
    const tab = ws.active;
    if (!tab || tab.language !== "rust") {
      store.analysis = null;
      return;
    }
    try {
      store.analysis = await ipc.analyzeCode(tab.content);
    } catch (e) {
      console.error(e);
    }
  }

  async function openDropped(path: string) {
    // A directory read succeeds; a file read throws — use that to discriminate.
    try {
      await ipc.readDir(path);
      await ws.openFolder(path);
      return;
    } catch {
      /* not a directory */
    }
    if (path.endsWith("Cargo.toml")) {
      const parent = path.replace(/[\\/]Cargo\.toml$/, "");
      if (parent) await ws.openFolder(parent);
    }
    const ok = await ws.openFile(path);
    if (!ok) alert(t("file.binary"));
  }

  onMount(async () => {
    try { document.documentElement.dataset.platform = type(); } catch { /* web preview */ }
    await initPrefs();
    try { examples = await ipc.listExamples(); } catch (e) { console.error(e); }
    try {
      const st = await ipc.runnerStatus();
      store.status = st;
      if (!st.ready) ipc.ensureRunner().catch(console.error);
    } catch (e) {
      console.error("runner status check failed; running in UI-only mode", e);
    }

    try {
      unlisteners.push(await onEvent((e) => store.bufferEvent(e)));
      unlisteners.push(await onStdout((tx) => store.bufferStdout(tx)));
      unlisteners.push(await onStderr((tx) => store.bufferStderr(tx)));
      unlisteners.push(await onDone(() => {
        if (store.drain()) ipc.cancelRun().catch(console.error);
        store.playing = false;
      }));
      unlisteners.push(await onStatus((s) => { store.status = s; }));
    } catch (e) {
      console.error("event listeners unavailable; running in UI-only mode", e);
    }

    // Native file drag-and-drop (folders + files) onto the window.
    try {
      const un = await getCurrentWebview().onDragDropEvent((e) => {
        if (e.payload.type === "over" || e.payload.type === "enter") dragOver = true;
        else if (e.payload.type === "leave") dragOver = false;
        else if (e.payload.type === "drop") {
          dragOver = false;
          for (const p of e.payload.paths) openDropped(p);
        }
      });
      unlisteners.push(un);
    } catch (e) { console.error("drag-drop unavailable", e); }

    analyze();
    unlisteners.push(bindGlobal(loadExample));

    let last = performance.now();
    const tick = (tt: number) => {
      if (store.drain()) ipc.cancelRun().catch(console.error);
      const dt = (tt - last) / 1000;
      last = tt;
      if (store.playing && store.totalTicks) {
        const next = store.playhead + dt * store.speed;
        if (next >= store.totalTicks - 1) {
          if (store.loop_) store.playhead = 0;
          else { store.playhead = store.totalTicks - 1; store.playing = false; }
        } else {
          store.playhead = next;
        }
      }
      rafHandle = requestAnimationFrame(tick);
    };
    rafHandle = requestAnimationFrame(tick);
  });

  $effect(() => {
    void store.theme; void store.timelineHeight; void store.zoom; void store.speed;
    void store.loop_; void store.follow; void locale();
    void store.editorFont; void store.editorFontSize;
    void store.terminalFont; void store.terminalFontSize;
    void store.outputFont; void store.outputFontSize;
    void store.uiFont; void store.uiFontSize;
    void ws.treeWidth; void ws.editorWidth; void ws.bottomHeight;
    applyFonts();
    persistPrefs();
  });

  onDestroy(() => {
    for (const u of unlisteners) try { u(); } catch {}
    cancelAnimationFrame(rafHandle);
  });

  function startDrag(kind: "tree" | "editor" | "bottom" | "tl", base: number) {
    return (e: PointerEvent) => {
      drag = { kind, start: kind === "bottom" || kind === "tl" ? e.clientY : e.clientX, base };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      e.preventDefault();
    };
  }
  function onDragMove(e: PointerEvent) {
    if (!drag) return;
    const winH = window.innerHeight;
    if (drag.kind === "tree") {
      ws.treeWidth = Math.max(160, Math.min(420, drag.base + (e.clientX - drag.start)));
    } else if (drag.kind === "editor") {
      ws.editorWidth = Math.max(320, Math.min(900, drag.base + (e.clientX - drag.start)));
    } else if (drag.kind === "bottom") {
      ws.bottomHeight = Math.max(80, Math.min(winH - 220, drag.base + (drag.start - e.clientY)));
    } else if (drag.kind === "tl") {
      store.timelineHeight = Math.max(120, Math.min(winH - 260, drag.base + (e.clientY - drag.start)));
    }
  }
  function endDrag(e: PointerEvent) {
    drag = null;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div class="app" class:drag-over={dragOver}>
  <header class="app-titlebar" data-tauri-drag-region>
    {#if ws.treeCollapsed}
      <button class="icon-btn ghost" title={t("ide.explorer")} aria-label="Show explorer"
        onclick={() => (ws.treeCollapsed = false)}>
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round">
          <rect x="2" y="2.5" width="12" height="11" rx="1.2" /><path d="M6 2.5v11" />
        </svg>
      </button>
    {/if}
    <div class="spacer" data-tauri-drag-region></div>
    <div class="run-group">
      {#if !store.status.running}
        <button class="icon-btn run-idle" onclick={runActive}
          title={ws.isCargo ? t("run.project") : t("run.title")} aria-label="Run">
          <svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"
            fill="currentColor" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round">
            <path d="M5 3.2 L12.5 8 L5 12.8 Z"/>
          </svg>
        </button>
      {:else}
        <button class="icon-btn rerun" onclick={runActive}
          title={t("rerun.title")} aria-label="Rerun">
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="currentColor">
            <path d="M5 3.2 L12.5 8 L5 12.8 Z"/>
          </svg>
        </button>
        <button class="icon-btn stop" onclick={() => ipc.cancelRun()}
          title={t("stop.title")} aria-label="Stop">
          <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"
            fill="currentColor"><rect x="3" y="3" width="10" height="10" rx="1"/></svg>
        </button>
      {/if}
    </div>
    <span class="bar-sep" aria-hidden="true"></span>
    <button class="icon-btn ghost gear" title={t("menu.settings")} aria-label="Menu"
      aria-haspopup="menu" aria-expanded={menuOpen} onclick={() => (menuOpen = !menuOpen)}>
      <svg viewBox="0 0 16 16" width="15" height="15" fill="currentColor" aria-hidden="true">
        <circle cx="8" cy="3.4" r="1.4" /><circle cx="8" cy="8" r="1.4" /><circle cx="8" cy="12.6" r="1.4" />
      </svg>
    </button>
  </header>

  {#if menuOpen}
    <AppMenu onClose={() => (menuOpen = false)} />
  {/if}

  <div class="shell">
    {#if !ws.treeCollapsed}
      <aside class="tree-col" style="width: {ws.treeWidth}px"><FileTree /></aside>
      <div class="vsep" role="separator" aria-orientation="vertical"
        onpointerdown={startDrag("tree", ws.treeWidth)} onpointermove={onDragMove} onpointerup={endDrag}></div>
    {/if}

    <section class="editor-col" class:full={ws.vizCollapsed}
      style={ws.vizCollapsed ? "" : `flex-basis: ${ws.editorWidth}px`}>
      <Tabs />
      <div class="editor-area">
        {#if ws.active}
          <CodeEditor on:input={analyze} />
        {:else}
          <div class="welcome">
            <svg viewBox="0 0 16 16" width="34" height="34" fill="none" stroke="currentColor" stroke-width="1" stroke-linejoin="round" aria-hidden="true">
              <path d="M4 1.8h5l3 3v8.4a.6.6 0 0 1-.6.6H4a.6.6 0 0 1-.6-.6V2.4A.6.6 0 0 1 4 1.8z" /><path d="M9 1.9v3.1h3" />
            </svg>
            <p>{t("welcome.hint")}</p>
            <button class="w-btn" onclick={() => ws.newUntitled()}>{t("welcome.newBuffer")}</button>
          </div>
        {/if}
      </div>

      {#if !ws.bottomCollapsed}
        <div class="hsep" role="separator" aria-orientation="horizontal"
          onpointerdown={startDrag("bottom", ws.bottomHeight)} onpointermove={onDragMove} onpointerup={endDrag}></div>
      {/if}

      <div class="bottom" class:collapsed={ws.bottomCollapsed}
        style={ws.bottomCollapsed ? "" : `height: ${ws.bottomHeight}px`}>
        <div class="panel-head">
          <div class="ptabs">
            <div class="ptab" class:on={ws.activeBottom === "output"}>
              <button class="ptab-lab" onclick={() => (ws.activeBottom = "output")}>
                {t("panel.output")}{#if outputCount} · {outputCount}{/if}
              </button>
            </div>
            <div class="ptab" class:on={ws.activeBottom === "events"}>
              <button class="ptab-lab" onclick={() => (ws.activeBottom = "events")}>
                {t("panel.events")}
              </button>
            </div>
            {#each ws.terminals as id, i (id)}
              <div class="ptab" class:on={ws.activeBottom === id}>
                <button class="ptab-lab" onclick={() => (ws.activeBottom = id)}>{t("panel.terminal")} {i + 1}</button>
                <button class="ptab-x" aria-label="Close terminal" onclick={() => ws.removeTerminal(id)}>×</button>
              </div>
            {/each}
          </div>
          <div class="pacts">
            <button class="p-btn" title={t("panel.newTerm")} aria-label="New terminal" onclick={() => ws.addTerminal()}>
              <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
                <rect x="1.8" y="2.8" width="12.4" height="10.4" rx="1.2" /><path d="M4.5 6.2 6.8 8l-2.3 1.8M8.4 10h3" />
              </svg>
            </button>
            <button class="p-btn" title={ws.bottomCollapsed ? t("panel.expand") : t("panel.collapse")}
              aria-label="Toggle panel" onclick={() => (ws.bottomCollapsed = !ws.bottomCollapsed)}>
              <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"
                style="transform: rotate({ws.bottomCollapsed ? 0 : 180}deg)">
                <path d="M4 10l4-4 4 4" />
              </svg>
            </button>
          </div>
        </div>
        <!-- Body stays mounted when collapsed (hidden via CSS) so terminal PTYs
             survive a panel collapse instead of being killed + respawned. -->
        <div class="panel-body" class:hidden={ws.bottomCollapsed}>
          <div class="pane" class:hidden={ws.activeBottom !== "output"}><OutputPanel /></div>
          <div class="pane" class:hidden={ws.activeBottom !== "events"}><EventsPanel /></div>
          {#each ws.terminals as id (id)}
            <div class="pane" class:hidden={ws.activeBottom !== id}><Terminal /></div>
          {/each}
        </div>
      </div>
    </section>

    {#if !ws.vizCollapsed}
    <div class="vsep" role="separator" aria-orientation="vertical"
      onpointerdown={startDrag("editor", ws.editorWidth)} onpointermove={onDragMove} onpointerup={endDrag}></div>

    <main class="viz">
      {#if showProjectWarn}
        <div class="warn">
          <span>⚠ {t("project.experimental")}</span>
          <button onclick={() => (warnDismissed = true)} aria-label="Dismiss">×</button>
        </div>
      {/if}
      <div class="tl-slot" style="height: {store.timelineHeight}px"><Timeline /></div>
      <div class="hsep" role="separator" aria-orientation="horizontal"
        onpointerdown={startDrag("tl", store.timelineHeight)} onpointermove={onDragMove} onpointerup={endDrag}></div>
      <Minimap />
      <SchedulerStage />
      <PlaybackBar />
    </main>
    {/if}
  </div>

  <!-- full-width VSCode-style status bar (spans the whole window bottom) -->
  <StatusBar />
</div>

{#if store.showSettings}
  <Settings on:close={() => (store.showSettings = false)} />
{/if}

{#if store.showShortcuts}
  <Shortcuts on:close={() => store.showShortcuts = false} />
{/if}

<style>
  .app {
    display: grid;
    grid-template-rows: 30px 1fr auto;   /* titlebar / shell / VSCode-style status bar */
    height: 100vh;
    overflow: hidden;
    background: var(--ts-bg-1);
  }
  .app.drag-over::after {
    content: "";
    position: fixed; inset: 0; z-index: 200; pointer-events: none;
    box-shadow: inset 0 0 0 2px var(--ts-accent);
    background: color-mix(in srgb, var(--ts-accent) 8%, transparent);
  }

  .app-titlebar {
    display: flex; align-items: center; gap: 2px; padding: 0 12px;
    background: var(--ts-bg-1); border-bottom: 1px solid var(--ts-line);
    overflow: hidden; -webkit-user-select: none; user-select: none;
  }
  :global(html[data-platform="macos"]) .app-titlebar { padding-left: 80px; }
  .spacer { flex: 1 1 auto; height: 100%; }

  .run-group { display: flex; align-items: center; gap: 2px; flex-shrink: 0; -webkit-app-region: no-drag; }
  .run-group .icon-btn { background: transparent; }
  .run-group .run-idle { background: rgba(106, 135, 89, 0.16); }
  .run-group .run-idle:hover { background: rgba(106, 135, 89, 0.32); }
  .run-group .rerun:hover { background: rgba(106, 135, 89, 0.24); }
  .run-group .stop:hover { background: rgba(188, 63, 60, 0.22); }

  .bar-sep { flex-shrink: 0; width: 1px; height: 16px; margin: 0 6px; background: var(--ts-line-2); }

  .icon-btn {
    display: inline-flex; align-items: center; justify-content: center; width: 28px; height: 23px;
    padding: 0; background: transparent; border: none; border-radius: 5px; cursor: pointer;
    transition: background 110ms ease, transform 60ms ease; -webkit-app-region: no-drag; color: var(--ts-fg-2);
  }
  .icon-btn:active:not(:disabled) { transform: scale(0.92); }
  .icon-btn svg { display: block; }
  .icon-btn.ghost:hover { background: var(--ts-bg-3); color: var(--ts-fg); }
  .icon-btn.run-idle { color: var(--ts-st-done); }
  .icon-btn.rerun { color: var(--ts-st-done); background: rgba(106, 135, 89, 0.12); }
  .icon-btn.stop { color: var(--ts-error, #BC3F3C); }

  /* ── 3-column shell ─────────────────────────────────────────── */
  .shell { display: flex; align-items: stretch; min-height: 0; overflow: hidden; }

  .tree-col { flex: 0 0 auto; min-width: 0; overflow: hidden; }

  .editor-col {
    display: flex; flex-direction: column; flex: 0 1 auto; min-width: 320px;
    border-left: 1px solid var(--ts-line); border-right: 1px solid var(--ts-line);
    background: var(--ts-bg-2); overflow: hidden;
  }
  .editor-col.full { flex: 1 1 auto; }
  .editor-area { flex: 1 1 auto; min-height: 80px; display: flex; flex-direction: column; }
  .editor-area > :global(*) { flex: 1 1 auto; min-height: 0; }
  .welcome {
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 14px; padding: 24px; text-align: center; color: var(--ts-fg-3);
  }
  .welcome p { margin: 0; max-width: 280px; font-size: 12.5px; line-height: 1.5; }
  .welcome .w-btn {
    padding: 7px 16px; border-radius: 7px; border: 1px solid var(--ts-line-2);
    background: var(--ts-bg-3); color: var(--ts-fg); font-size: 12.5px; cursor: pointer;
  }
  .welcome .w-btn:hover { border-color: var(--ts-accent); color: var(--ts-accent); }

  .bottom { flex-shrink: 0; display: flex; flex-direction: column; min-height: 0; background: var(--ts-bg-1); }
  .bottom.collapsed { height: auto; }
  .panel-head {
    display: flex; align-items: center; flex-shrink: 0; height: 30px; padding: 0 6px 0 0;
    border-top: 1px solid var(--ts-line); background: var(--ts-bg-1);
  }
  .ptabs { display: flex; align-items: stretch; flex: 1 1 auto; overflow-x: auto; }
  .ptab { position: relative; display: flex; align-items: center; flex-shrink: 0; }
  .ptab-lab {
    padding: 6px 11px 7px; height: 100%; box-sizing: border-box; background: transparent; border: none; cursor: pointer;
    font-family: var(--ts-mono); font-size: 10.5px; letter-spacing: 0.04em; color: var(--ts-fg-3);
  }
  .ptab:hover .ptab-lab { color: var(--ts-fg-2); }
  .ptab.on .ptab-lab { color: var(--ts-fg); }
  /* active underline pinned to the bottom edge; the text-to-underline gap comes
     from the label's bottom padding, so top/bottom spacing stays symmetric */
  .ptab.on::after {
    content: ""; position: absolute; left: 11px; right: 11px; bottom: 0; height: 2px;
    background: var(--ts-accent); border-radius: 1px;
  }
  .ptab-x {
    display: inline-flex; align-items: center; justify-content: center; width: 16px; height: 16px;
    margin-right: 6px; padding: 0; background: transparent; border: none; border-radius: 4px;
    color: var(--ts-fg-3); font-size: 13px; line-height: 1; cursor: pointer;
  }
  .ptab-x:hover { background: var(--ts-bg-3); color: var(--ts-fg); }
  .pacts { display: flex; align-items: center; gap: 1px; flex-shrink: 0; }
  .p-btn {
    display: inline-flex; align-items: center; justify-content: center; width: 24px; height: 22px;
    padding: 0; background: transparent; border: none; border-radius: 4px; color: var(--ts-fg-3); cursor: pointer;
  }
  .p-btn:hover { background: var(--ts-bg-3); color: var(--ts-fg); }
  .p-btn svg { transition: transform 140ms ease; }
  .panel-body { flex: 1 1 auto; min-height: 0; position: relative; }
  .panel-body.hidden { display: none; }
  .pane { position: absolute; inset: 0; display: flex; }
  .pane > :global(*) { flex: 1 1 auto; min-width: 0; }
  .pane.hidden { display: none; }

  .vsep { flex: 0 0 4px; background: var(--ts-line); cursor: col-resize; }
  .vsep:hover { background: var(--ts-line-2); }
  .hsep { height: 4px; background: var(--ts-line); cursor: row-resize; flex-shrink: 0; }
  .hsep:hover { background: var(--ts-line-2); }

  /* ── viz column ─────────────────────────────────────────────── */
  .viz {
    display: flex; flex-direction: column; flex: 1 1 0; min-width: 560px;
    overflow: hidden; background: var(--ts-bg-0);
  }
  .warn {
    display: flex; align-items: center; gap: 8px; flex-shrink: 0; padding: 5px 10px;
    background: rgba(204, 120, 50, 0.14); border-bottom: 1px solid var(--ts-line);
    font-family: var(--ts-mono); font-size: 10.5px; line-height: 1.4; color: var(--ts-accent);
  }
  .warn span { flex: 1 1 auto; }
  .warn button { flex-shrink: 0; background: transparent; border: none; color: var(--ts-accent); font-size: 14px; cursor: pointer; }
  .tl-slot { flex-shrink: 0; min-height: 120px; overflow: hidden; display: flex; }
  .tl-slot > :global(*) { flex: 1 1 auto; min-width: 0; }
</style>
