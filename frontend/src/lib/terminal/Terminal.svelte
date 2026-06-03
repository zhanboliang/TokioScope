<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { ipc, onPty, onPtyExit } from "../ipc";
  import { ws } from "../workspace.svelte";
  import { store } from "../store.svelte";

  let host: HTMLDivElement;
  let term: Terminal | null = null;
  let fit: FitAddon;
  let ptyId: number | null = null;
  let unlisteners: (() => void)[] = [];
  let ro: ResizeObserver | undefined;

  function readVar(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }
  function themeColors() {
    return {
      background: readVar("--ts-bg-1") || "#1e1e1e",
      foreground: readVar("--ts-fg") || "#dddddd",
      cursor: readVar("--ts-accent") || "#e8853a",
      selectionBackground: readVar("--ts-bg-3") || "#333333",
    };
  }
  function termFont() {
    const base = readVar("--ts-mono") || "monospace";
    return store.terminalFont ? `${store.terminalFont}, ${base}` : base;
  }

  onMount(async () => {
    term = new Terminal({
      fontFamily: termFont(),
      fontSize: store.terminalFontSize,
      theme: themeColors(),
      cursorBlink: true,
      scrollback: 5000,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    try { fit.fit(); } catch {}

    ptyId = await ipc.ptySpawn(ws.root, term.cols, term.rows);
    term.onData((d) => {
      if (ptyId != null) ipc.ptyWrite(ptyId, d).catch(() => {});
    });

    unlisteners.push(
      await onPty((id, data) => {
        if (id === ptyId) term?.write(data);
      }),
    );
    unlisteners.push(
      await onPtyExit((id) => {
        if (id === ptyId) {
          term?.write("\r\n\x1b[2m[process exited]\x1b[0m\r\n");
          ptyId = null;
        }
      }),
    );

    ro = new ResizeObserver(() => {
      if (!term) return;
      try {
        fit.fit();
        if (ptyId != null) ipc.ptyResize(ptyId, term.cols, term.rows).catch(() => {});
      } catch {}
    });
    ro.observe(host);
  });

  // Follow the app theme + user font. A font change resizes the cell grid, so
  // refit and tell the PTY the new dimensions.
  $effect(() => {
    void store.theme;
    void store.terminalFont;
    void store.terminalFontSize;
    if (!term) return;
    term.options.theme = themeColors();
    term.options.fontFamily = termFont();
    term.options.fontSize = store.terminalFontSize;
    try {
      fit.fit();
      if (ptyId != null) ipc.ptyResize(ptyId, term.cols, term.rows).catch(() => {});
    } catch {}
  });

  onDestroy(() => {
    for (const u of unlisteners) try { u(); } catch {}
    ro?.disconnect();
    if (ptyId != null) ipc.ptyKill(ptyId).catch(() => {});
    term?.dispose();
  });
</script>

<div class="term" bind:this={host}></div>

<style>
  .term { width: 100%; height: 100%; box-sizing: border-box; padding: 4px 0 4px 8px; background: var(--ts-bg-1); }
  .term :global(.xterm) { height: 100%; }
  .term :global(.xterm-viewport) { background: transparent !important; }
</style>
