<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../store.svelte";
  import { readCssVar } from "../colors";

  let canvas: HTMLCanvasElement;
  let body: HTMLDivElement;

  function resize() {
    if (!canvas || !body) return;
    const dpr = window.devicePixelRatio || 1;
    const w = body.clientWidth, h = body.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = w + "px";
    canvas.style.height = h + "px";
    const ctx = canvas.getContext("2d");
    ctx?.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw();
  }

  function draw() {
    const ctx = canvas?.getContext("2d");
    if (!ctx || !body || !canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = body.clientWidth, h = body.clientHeight;
    // Re-sync the backing store to the element on every draw (see Timeline).
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      canvas.style.width = w + "px";
      canvas.style.height = h + "px";
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    const sim = store.sim;
    if (!sim || sim.frames.length === 0) return;

    const total = sim.frames.length;
    const colW = w / total;
    const accent = readCssVar("--ts-accent") || "#CC7832";

    // Compressed copy of the timeline: one thin band per task (spawn order),
    // each cell coloured by that task's state — so it maps 1:1 to the lanes above.
    const born = new Map<number, number>();
    for (const f of sim.frames)
      for (const [id, t] of f.tasks) if (!born.has(id)) born.set(id, t.bornTick);
    const taskIds = [...born.entries()].sort((a, b) => a[1] - b[1] || a[0] - b[0]).map(([id]) => id);

    const stateColor: Record<string, string> = {
      ready: readCssVar("--ts-st-ready") || "#6897BB",
      running: readCssVar("--ts-st-running") || "#CC7832",
      awaiting: readCssVar("--ts-st-awaiting") || "#808080",
      blocking: readCssVar("--ts-st-blocking") || "#9876AA",
      done: readCssVar("--ts-st-done") || "#6A8759",
    };

    const padY = 1;
    const rowH = Math.max(2, (h - padY * 2) / Math.max(1, taskIds.length));
    for (let r = 0; r < taskIds.length; r++) {
      const id = taskIds[r];
      const y = padY + r * rowH;
      if (y > h) break;
      for (let i = 0; i < total; i++) {
        const t = sim.frames[i].tasks.get(id);
        if (!t) continue;
        ctx.fillStyle = stateColor[t.state] ?? "#606366";
        ctx.fillRect(i * colW, y, colW + 0.6, Math.max(1, rowH - 0.5));
      }
    }

    // viewport indicator
    const zoom = store.zoom;
    const viewTicks = (body.parentElement?.clientWidth ?? w) / zoom;
    const x0 = (store.panTick / total) * w;
    const x1 = Math.min(w, ((store.panTick + viewTicks) / total) * w);
    ctx.fillStyle = "rgba(255,255,255,0.04)";
    ctx.fillRect(x0, 0, Math.max(2, x1 - x0), h);
    ctx.strokeStyle = accent;
    ctx.lineWidth = 1;
    ctx.strokeRect(x0 + 0.5, 0.5, Math.max(2, x1 - x0 - 1), h - 1);

    // playhead
    const phx = (store.playhead / total) * w;
    ctx.strokeStyle = accent;
    ctx.beginPath();
    ctx.moveTo(phx + 0.5, 0);
    ctx.lineTo(phx + 0.5, h);
    ctx.stroke();
  }

  // Drag the minimap to slide the timeline's view window left/right (pan),
  // centering the visible range on the cursor.
  let panning = false;
  function panFromX(clientX: number) {
    if (!store.sim) return;
    const total = store.sim.frames.length;
    if (total === 0) return;
    const rect = canvas.getBoundingClientRect();
    const cursorTick = ((clientX - rect.left) / rect.width) * total;
    const viewTicks = (body.parentElement?.clientWidth ?? rect.width) / store.zoom;
    store.panTick = Math.max(0, Math.min(Math.max(0, total - viewTicks), cursorTick - viewTicks / 2));
  }
  function down(e: PointerEvent) {
    panning = true;
    (e.target as Element).setPointerCapture(e.pointerId);
    panFromX(e.clientX);
  }
  function move(e: PointerEvent) { if (panning) panFromX(e.clientX); }
  function up(e: PointerEvent) {
    if (panning) { panning = false; (e.target as Element).releasePointerCapture(e.pointerId); }
  }

  onMount(() => {
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(body);
    return () => ro.disconnect();
  });
  $effect(() => { void store.sim; void store.playhead; void store.panTick; void store.zoom; void store.theme; draw(); });
</script>

<section class="mini">
  <span class="label">map</span>
  <div class="body" bind:this={body}>
    <canvas bind:this={canvas} onpointerdown={down} onpointermove={move} onpointerup={up}></canvas>
    {#if !store.sim || store.sim.frames.length === 0}
      <span class="empty">尚无运行数据</span>
    {/if}
  </div>
</section>

<style>
  .mini {
    height: 36px;
    border-top: 1px solid var(--ts-line);
    background: var(--ts-bg-1);
    display: grid;
    grid-template-columns: 44px 1fr;
    align-items: stretch;
    flex-shrink: 0;
  }
  .label {
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--ts-mono);
    font-size: 9px;
    letter-spacing: 0.04em;
    color: var(--ts-fg-3);
    border-right: 1px solid var(--ts-line);
  }
  .body { position: relative; overflow: hidden; }
  canvas { display: block; width: 100%; height: 100%; cursor: grab; }
  canvas:active { cursor: grabbing; }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-family: var(--ts-mono);
    font-size: 10px;
    color: var(--ts-fg-3);
    pointer-events: none;
  }
</style>
