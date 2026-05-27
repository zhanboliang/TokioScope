<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { ipc, onEvent, onStdout, onStderr, onDone, onStatus, type Example } from "./lib/ipc";
  import { store } from "./lib/store.svelte";
  import { bindGlobal } from "./lib/shortcuts";
  import { initPrefs, persistPrefs } from "./lib/prefs";
  import CodeEditor from "./lib/editor/CodeEditor.svelte";
  import Timeline from "./lib/timeline/Timeline.svelte";
  import Minimap from "./lib/timeline/Minimap.svelte";
  import WorkerPool from "./lib/pool/WorkerPool.svelte";
  import BlockingPool from "./lib/pool/BlockingPool.svelte";
  import ReadyQueue from "./lib/pool/ReadyQueue.svelte";
  import EventLog from "./lib/pool/EventLog.svelte";
  import PlaybackBar from "./lib/controls/PlaybackBar.svelte";
  import StatusBar from "./lib/controls/StatusBar.svelte";
  import OutputPanel from "./lib/output/OutputPanel.svelte";
  import Shortcuts from "./lib/help/Shortcuts.svelte";
  import { type } from "@tauri-apps/plugin-os";

  let examples = $state<Example[]>([]);
  let unlisteners: (() => void)[] = [];
  let rafHandle = 0;

  // active drag descriptor (null when idle)
  let drag = $state<null | { kind: "col" | "out" | "tl"; start: number; base: number }>(null);

  function loadExample(idx: number) {
    if (examples[idx]) {
      store.code = examples[idx].code;
      analyze();
    }
  }

  async function analyze() {
    try {
      store.analysis = await ipc.analyzeCode(store.code);
    } catch (e) {
      console.error(e);
    }
  }

  async function runCode() {
    store.reset();
    try {
      await ipc.startRun(store.code);
    } catch (e) {
      store.rawStderr = [...store.rawStderr, `start_run failed: ${e}`];
    }
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

    unlisteners.push(await onEvent((e) => store.ingest(e)));
    unlisteners.push(await onStdout((t) => {
      const tick = store.sim?.frames.length ?? 0;
      store.rawStdout = [...store.rawStdout, { tick, text: t }];
    }));
    unlisteners.push(await onStderr((t) => {
      store.rawStderr = [...store.rawStderr, t];
    }));
    unlisteners.push(await onDone(() => { store.playing = false; }));
    unlisteners.push(await onStatus((s) => { store.status = s; }));

    analyze();
    unlisteners.push(bindGlobal(loadExample));

    let last = performance.now();
    const tick = (t: number) => {
      const dt = (t - last) / 1000;
      last = t;
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
    void store.theme; void store.codePaneWidth; void store.outputHeight;
    void store.timelineHeight; void store.zoom; void store.speed;
    void store.loop_; void store.follow;
    persistPrefs();
  });

  onDestroy(() => {
    for (const u of unlisteners) try { u(); } catch {}
    cancelAnimationFrame(rafHandle);
  });

  function startDrag(kind: "col" | "out" | "tl", base: number) {
    return (e: PointerEvent) => {
      drag = { kind, start: kind === "col" ? e.clientX : e.clientY, base };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      e.preventDefault();
    };
  }
  function onDragMove(e: PointerEvent) {
    if (!drag) return;
    const winH = window.innerHeight;
    if (drag.kind === "col") {
      store.codePaneWidth = Math.max(280, Math.min(820, drag.base + (e.clientX - drag.start)));
    } else if (drag.kind === "out") {
      // dragging the handle UP shrinks editor / grows output → delta = start - current
      const delta = drag.start - e.clientY;
      store.outputHeight = Math.max(64, Math.min(winH - 180, drag.base + delta));
    } else if (drag.kind === "tl") {
      // dragging down grows timeline
      const delta = e.clientY - drag.start;
      store.timelineHeight = Math.max(120, Math.min(winH - 260, drag.base + delta));
    }
  }
  function endDrag(e: PointerEvent) {
    drag = null;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  }
</script>

<div class="app">
  <header class="app-titlebar" data-tauri-drag-region>
    <div class="spacer" data-tauri-drag-region></div>
    <div class="actions" data-tauri-drag-region>
      {#if !store.status.running}
        <!-- idle: hollow play triangle (IDEA "Run") -->
        <button class="icon-btn run-idle" onclick={runCode}
          title="运行 (⌘+Enter)" aria-label="Run">
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"
            fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round">
            <path d="M5 3.2 L12.5 8 L5 12.8 Z"/>
          </svg>
        </button>
      {:else}
        <!-- running: filled play in rounded chip (IDEA "Rerun") -->
        <button class="icon-btn rerun" onclick={runCode}
          title="重新运行 (⌘+Enter)" aria-label="Rerun">
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"
            fill="currentColor">
            <path d="M5 3.2 L12.5 8 L5 12.8 Z"/>
          </svg>
        </button>
        <!-- red stop square (IDEA "Stop") -->
        <button class="icon-btn stop" onclick={() => ipc.cancelRun()}
          title="取消 (⌘+.)" aria-label="Stop">
          <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"
            fill="currentColor"><rect x="3" y="3" width="10" height="10" rx="1"/></svg>
        </button>
      {/if}
    </div>
  </header>

  <div class="shell">
  <aside class="code-pane" style="width: {store.codePaneWidth}px">
    <div class="pane-head">
      <span class="title">source</span>
    </div>

    <div class="editor-slot">
      <CodeEditor on:input={analyze} />
    </div>

    <div class="hsep" role="separator" aria-orientation="horizontal"
      onpointerdown={startDrag("out", store.outputHeight)}
      onpointermove={onDragMove} onpointerup={endDrag}></div>

    <div class="output-slot" style="height: {store.outputHeight}px">
      <OutputPanel />
    </div>
  </aside>

  <div class="vsep" role="separator" aria-orientation="vertical"
    onpointerdown={startDrag("col", store.codePaneWidth)}
    onpointermove={onDragMove} onpointerup={endDrag}></div>

  <main class="viz">
    <div class="tl-slot" style="height: {store.timelineHeight}px">
      <Timeline />
    </div>
    <div class="hsep" role="separator" aria-orientation="horizontal"
      onpointerdown={startDrag("tl", store.timelineHeight)}
      onpointermove={onDragMove} onpointerup={endDrag}></div>

    <Minimap />

    <section class="pools">
      <WorkerPool />
      <BlockingPool />
      <ReadyQueue />
      <EventLog />
    </section>

    <PlaybackBar />
    <StatusBar />
  </main>
  </div>
</div>

{#if store.showShortcuts}
  <Shortcuts on:close={() => store.showShortcuts = false} />
{/if}

<style>
  .app {
    display: grid;
    grid-template-rows: 30px 1fr;     /* short bar — matches macOS's natural traffic-light center (~y=15) */
    height: 100vh;
    overflow: hidden;
    background: var(--ts-bg-1);
  }

  /* ── titlebar — entire bar is drag region; only the action buttons opt out ── */
  .app-titlebar {
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: var(--ts-bg-1);
    border-bottom: 1px solid var(--ts-line);
    overflow: hidden;
    -webkit-user-select: none;
    user-select: none;
  }
  /* reserve room for macOS traffic lights at far left */
  :global(html[data-platform="macos"]) .app-titlebar { padding-left: 80px; }

  .spacer { flex: 1 1 auto; height: 100%; }

  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    height: 100%;
  }

  /* IDEA / VS Code style icon buttons */
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 22px;                /* fit in 30px bar with breathing room */
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background 80ms ease;
    -webkit-app-region: no-drag;
  }
  .icon-btn:active:not(:disabled) { transform: translateY(0.5px); }

  /* idle Run — outlined green triangle */
  .icon-btn.run-idle { color: var(--ts-st-done); }
  .icon-btn.run-idle:hover { background: rgba(106, 135, 89, 0.18); }

  /* running Rerun — filled green triangle on subtle bg "chip" */
  .icon-btn.rerun {
    color: var(--ts-st-done);
    background: rgba(106, 135, 89, 0.12);
  }
  .icon-btn.rerun:hover { background: rgba(106, 135, 89, 0.28); }

  /* Stop — filled red square */
  .icon-btn.stop { color: var(--ts-error, #BC3F3C); }
  .icon-btn.stop:hover { background: rgba(188, 63, 60, 0.18); }

  .shell {
    display: grid;
    grid-template-columns: auto 4px 1fr;
    min-height: 0;
    overflow: hidden;
  }

  /* ── left pane ────────────────────────────────────────────── */
  .code-pane {
    display: flex;
    flex-direction: column;
    min-width: 280px;
    border-right: 1px solid var(--ts-line);
    background: var(--ts-bg-2);
    overflow: hidden;
  }
  .editor-slot { flex: 1 1 auto; min-height: 120px; display: flex; flex-direction: column; }
  .editor-slot > :global(*) { flex: 1 1 auto; min-height: 0; }
  .output-slot { flex-shrink: 0; min-height: 60px; display: flex; }
  .output-slot > :global(*) { flex: 1 1 auto; min-height: 0; max-height: none; }

  .pane-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--ts-line);
    background: var(--ts-bg-1);
    font-family: var(--ts-mono);
    font-size: 11px;
    flex-shrink: 0;
  }
  .pane-head .title {
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--ts-fg-3);
  }

  /* ── separators ───────────────────────────────────────────── */
  .vsep { background: var(--ts-line); cursor: col-resize; }
  .vsep:hover { background: var(--ts-accent); }
  .hsep { height: 4px; background: var(--ts-line); cursor: row-resize; flex-shrink: 0; }
  .hsep:hover { background: var(--ts-accent); }

  /* ── right pane (viz) ─────────────────────────────────────── */
  .viz {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
    background: var(--ts-bg-0);
  }
  .tl-slot { flex-shrink: 0; min-height: 120px; overflow: hidden; display: flex; }
  .tl-slot > :global(*) { flex: 1 1 auto; min-width: 0; }
  .pools {
    flex: 1 1 auto;
    display: grid;
    grid-template-columns: 1fr 1fr 1fr 1.4fr;
    gap: 1px;
    background: var(--ts-line);
    border-top: 1px solid var(--ts-line);
    min-height: 140px;
    overflow: hidden;
  }
</style>
