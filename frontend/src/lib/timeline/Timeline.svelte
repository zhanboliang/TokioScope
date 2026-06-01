<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../store.svelte";
  import { readCssVar, STATE_COLOR } from "../colors";
  import { t as tr, locale } from "../i18n.svelte"; // aliased — `t` is a local task var
  import type { TaskState } from "../engine";

  let canvas: HTMLCanvasElement;
  let wrap: HTMLDivElement;
  let hoverInfo = $state<string>("");
  let legendHover = $state(false);
  let legendPinned = $state(false);

  const LEGEND: TaskState[] = ["ready", "running", "awaiting", "blocking", "done"];
  const LANE_HEIGHT = 18;
  const LANE_PAD = 4;

  let dragging = false;
  let scrubbing = false;
  let dragStartX = 0;
  let dragStartPan = 0;

  function resize() {
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = wrap.clientWidth * dpr;
    canvas.height = wrap.clientHeight * dpr;
    canvas.style.width = wrap.clientWidth + "px";
    canvas.style.height = wrap.clientHeight + "px";
    const ctx = canvas.getContext("2d");
    ctx?.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw();
  }

  function draw() {
    if (!canvas || !wrap) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = wrap.clientWidth;
    const h = wrap.clientHeight;
    // Keep the backing store sized to the element on every draw. Relying on a
    // previously-set canvas.width is fragile across first-paint / relayout races
    // and can leave the canvas 0×0 (blank) even when there's data to show.
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      canvas.style.width = w + "px";
      canvas.style.height = h + "px";
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = readCssVar("--ts-bg-0") || "#2B2B2B";
    ctx.fillRect(0, 0, w, h);

    const sim = store.sim;
    if (!sim) {
      ctx.fillStyle = readCssVar("--ts-fg-3") || "#606366";
      ctx.font = `12px ${getMono()}`;
      ctx.fillText(tr("tl.waiting"), 12, 20);
      return;
    }

    const zoom = store.zoom; // px per tick
    const pan = store.panTick;
    const total = sim.frames.length;

    // Grid + tick markers every 10 ticks
    ctx.strokeStyle = readCssVar("--ts-line") || "#323232";
    ctx.lineWidth = 1;
    ctx.fillStyle = readCssVar("--ts-fg-3") || "#606366";
    ctx.font = `10px ${getMono()}`;
    for (let t = Math.floor(pan / 10) * 10; t < pan + w / zoom + 10; t += 10) {
      const x = (t - pan) * zoom;
      if (x < 0 || x > w) continue;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, h);
      ctx.stroke();
      ctx.fillText(`${t}`, x + 2, 10);
    }

    // Build per-task spans from frames
    type Span = { id: number; state: string; start: number; end: number };
    const spans: Map<number, Span[]> = new Map();
    const taskMeta = new Map<number, { name: string; born: number }>();
    for (let i = 0; i < sim.frames.length; i++) {
      const f = sim.frames[i];
      for (const [id, t] of f.tasks) {
        if (!taskMeta.has(id)) taskMeta.set(id, { name: t.name, born: t.bornTick });
        let list = spans.get(id);
        if (!list) { list = []; spans.set(id, list); }
        const last = list[list.length - 1];
        if (last && last.state === t.state && last.end === f.tick - 1) {
          last.end = f.tick;
        } else if (last && last.state === t.state && last.end === f.tick) {
          last.end = f.tick;
        } else {
          list.push({ id, state: t.state, start: f.tick, end: f.tick });
        }
      }
    }

    // Order lanes by spawn order
    const taskIds = [...taskMeta.entries()]
      .sort((a, b) => a[1].born - b[1].born || a[0] - b[0])
      .map(([id]) => id);

    let y = 16;
    ctx.font = `11px ${getMono()}`;
    for (const id of taskIds) {
      const list = spans.get(id) ?? [];
      // Lane background
      ctx.fillStyle = readCssVar("--ts-bg-1") || "#3C3F41";
      ctx.fillRect(0, y, w, LANE_HEIGHT);

      // state spans
      for (const s of list) {
        const x0 = (s.start - pan) * zoom;
        const x1 = (s.end + 1 - pan) * zoom;
        const ww = Math.max(2, x1 - x0);
        ctx.fillStyle = stateColor(s.state);
        ctx.fillRect(x0, y + 2, ww, LANE_HEIGHT - 4);
      }

      // task name on top, with a chip so it stays readable over the spans
      const name = taskMeta.get(id)?.name ?? `#${id}`;
      const tw = ctx.measureText(name).width;
      ctx.fillStyle = readCssVar("--ts-bg-1") || "#3C3F41";
      ctx.fillRect(2, y + 2, tw + 8, LANE_HEIGHT - 4);
      ctx.fillStyle = readCssVar("--ts-fg") || "#A9B7C6";
      ctx.fillText(name, 6, y + 12);

      y += LANE_HEIGHT + LANE_PAD;
      if (y > h - LANE_HEIGHT) break;
    }

    // Playhead
    const phX = (store.playhead - pan) * zoom;
    ctx.strokeStyle = readCssVar("--ts-accent") || "#CC7832";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(phX, 0);
    ctx.lineTo(phX, h);
    ctx.stroke();

    // Follow head — only auto-pan while actually playing, so manual panning /
    // scrubbing while paused isn't immediately undone.
    if (store.follow && store.playing && phX > w * 0.75) {
      store.panTick = Math.max(0, store.playhead - (w / zoom) * 0.75);
    }

    void total;
  }

  function stateColor(s: string): string {
    switch (s) {
      case "running": return readCssVar("--ts-st-running") || "#CC7832";
      case "ready": return readCssVar("--ts-st-ready") || "#6897BB";
      case "awaiting": return readCssVar("--ts-st-awaiting") || "#808080";
      case "blocking": return readCssVar("--ts-st-blocking") || "#9876AA";
      case "done": return readCssVar("--ts-st-done") || "#6A8759";
    }
    return "#606366";
  }

  function getMono() {
    return "JetBrains Mono, ui-monospace, monospace";
  }

  // Clamp pan so the view can't scroll past the content into empty space.
  function clampPan(p: number): number {
    const total = store.sim?.frames.length ?? 0;
    const viewTicks = (wrap?.clientWidth ?? 0) / store.zoom;
    return Math.max(0, Math.min(Math.max(0, total - viewTicks), p));
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    // Shift, or a horizontal (trackpad sideways) swipe = pan left/right
    if (e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY)) {
      const d = e.shiftKey ? e.deltaY : e.deltaX;
      store.panTick = clampPan(store.panTick + d / store.zoom);
      draw();
      return;
    }
    // vertical (and ctrl/meta pinch) = zoom anchored at the cursor
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const anchorTick = store.panTick + mx / store.zoom;
    const factor = e.deltaY < 0 ? 1.15 : 0.87;
    store.zoom = Math.max(2, Math.min(120, store.zoom * factor));
    store.panTick = clampPan(anchorTick - mx / store.zoom);
    draw();
  }

  // Move the playhead to the exact (fractional) position under the cursor, so
  // scrubbing is smooth/pixel-precise rather than snapping tick-to-tick.
  // currentFrame floors the playhead, so the displayed state stays valid.
  function seekFromX(clientX: number) {
    if (!store.totalTicks) return;
    const rect = canvas.getBoundingClientRect();
    const mx = clientX - rect.left;
    const tick = store.panTick + mx / store.zoom;
    store.playhead = Math.max(0, Math.min(store.totalTicks - 1, tick));
    store.playing = false;
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button === 1 || e.shiftKey) {
      // middle / shift drag = pan
      dragging = true;
      dragStartX = e.clientX;
      dragStartPan = store.panTick;
      (e.target as Element).setPointerCapture(e.pointerId);
      e.preventDefault();
    } else if (e.button === 0) {
      // left drag = scrub the playhead (video-scrubber feel)
      scrubbing = true;
      (e.target as Element).setPointerCapture(e.pointerId);
      seekFromX(e.clientX);
    }
  }
  function onPointerMove(e: PointerEvent) {
    if (dragging) {
      const dx = e.clientX - dragStartX;
      store.panTick = clampPan(dragStartPan - dx / store.zoom);
      draw();
      return;
    }
    if (scrubbing) {
      seekFromX(e.clientX);
      return;
    }
    // hover info
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const tick = Math.round(store.panTick + mx / store.zoom);
    const laneIdx = Math.floor((my - 16) / (LANE_HEIGHT + LANE_PAD));
    const f = store.sim?.frames[Math.min(tick, (store.sim?.frames.length ?? 1) - 1)];
    if (f && laneIdx >= 0) {
      const taskIds = [...f.tasks.entries()].sort((a, b) => a[1].bornTick - b[1].bornTick || a[0] - b[0]).map(([id]) => id);
      const id = taskIds[laneIdx];
      const t = id != null ? f.tasks.get(id) : null;
      hoverInfo = t ? `${t.name} · ${t.state} · line ${t.currentLine} · evt ${t.eventCount} · ${t.durationTicks} ticks` : "";
    } else {
      hoverInfo = "";
    }
  }
  function onPointerUp(e: PointerEvent) {
    if (dragging || scrubbing) {
      dragging = false;
      scrubbing = false;
      (e.target as Element).releasePointerCapture(e.pointerId);
    }
  }
  function onDblClick() {
    if (!store.sim || store.sim.frames.length === 0) return;
    const w = canvas.clientWidth;
    store.panTick = 0;
    store.zoom = Math.max(2, Math.min(120, w / Math.max(1, store.sim.frames.length)));
    draw();
  }

  onMount(() => {
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);
    return () => ro.disconnect();
  });

  // Auto-fit on the first frame of a fresh run.
  let lastFrameCount = -1;
  $effect(() => {
    const n = store.sim?.frames.length ?? 0;
    if (n > 0 && lastFrameCount <= 0 && canvas) {
      const w = canvas.clientWidth;
      store.panTick = 0;
      // leave ~15% headroom so newly arriving ticks don't immediately overflow
      store.zoom = Math.max(8, Math.min(120, (w * 0.85) / Math.max(1, n)));
    }
    lastFrameCount = n;
  });

  $effect(() => {
    // Touch reactive deps to schedule redraws
    void store.sim;
    void store.playhead;
    void store.zoom;
    void store.panTick;
    void store.theme;
    void store.timelineHeight;
    void locale();
    draw();
  });
</script>

<div class="wrap" bind:this={wrap}>
  <canvas
    bind:this={canvas}
    onwheel={onWheel}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    ondblclick={onDblClick}
  ></canvas>

  <!-- color legend: hover the ? (or click to pin) -->
  <div class="legend-anchor"
    onmouseenter={() => (legendHover = true)}
    onmouseleave={() => (legendHover = false)}
    role="presentation">
    <button class="legend-toggle" class:on={legendPinned}
      onclick={() => (legendPinned = !legendPinned)} aria-label={tr("tl.legend")}>?</button>
    {#if legendHover || legendPinned}
      <div class="legend">
        {#each LEGEND as s}
          <div class="row">
            <span class="sw" style="background: {STATE_COLOR[s]}"></span>
            <span class="lab">{tr(`state.${s}`)}</span>
            <span class="desc">{tr(`state.${s}.desc`)}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  {#if hoverInfo}<div class="hover">{hoverInfo}</div>{/if}
</div>

<style>
  .wrap {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--ts-bg-0);
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: crosshair;
  }
  .legend-anchor { position: absolute; top: 6px; right: 8px; z-index: 5; }
  .legend-toggle {
    width: 18px;
    height: 18px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: var(--ts-bg-1);
    border: 1px solid var(--ts-line-2);
    color: var(--ts-fg-2);
    font-family: var(--ts-mono);
    font-size: 11px;
    line-height: 1;
    cursor: pointer;
  }
  .legend-toggle:hover, .legend-toggle.on { color: var(--ts-accent); border-color: var(--ts-accent); }
  .legend {
    position: absolute;
    top: 23px;
    right: 0;
    display: grid;
    gap: 5px;
    padding: 8px 10px;
    background: var(--ts-bg-1);
    border: 1px solid var(--ts-line-2);
    border-radius: 6px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
  }
  .legend .row { display: flex; align-items: center; gap: 8px; }
  .legend .sw { width: 11px; height: 11px; border-radius: 2px; flex-shrink: 0; }
  .legend .lab { font-family: var(--ts-sans); font-size: 11px; font-weight: 600; color: var(--ts-fg); white-space: nowrap; min-width: 28px; }
  .legend .desc { font-family: var(--ts-sans); font-size: 10.5px; color: var(--ts-fg-3); white-space: nowrap; }

  .hover {
    position: absolute;
    bottom: 6px;
    left: 8px;
    font-family: var(--ts-mono);
    font-size: 11px;
    color: var(--ts-fg);
    background: var(--ts-bg-1);
    padding: 3px 8px;
    border: 1px solid var(--ts-line-2);
    border-radius: 3px;
    pointer-events: none;
    box-shadow: 0 2px 8px rgba(0,0,0,0.25);
  }
</style>
