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
    if (!ctx || !body) return;
    const w = body.clientWidth, h = body.clientHeight;
    ctx.clearRect(0, 0, w, h);

    const sim = store.sim;
    if (!sim || sim.frames.length === 0) return;

    const total = sim.frames.length;
    const colW = w / total;
    const accent = readCssVar("--ts-accent") || "#CC7832";

    for (let i = 0; i < total; i++) {
      const f = sim.frames[i];
      let active = 0;
      for (const t of f.tasks.values()) if (t.state === "running" || t.state === "blocking") active++;
      const intensity = Math.min(1, active / 6);
      if (intensity > 0) {
        ctx.fillStyle = `rgba(204, 120, 50, ${0.2 + intensity * 0.7})`;
        const barH = Math.max(2, intensity * (h - 4));
        ctx.fillRect(i * colW, h - barH - 2, colW + 0.5, barH);
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

  function click(e: MouseEvent) {
    if (!store.sim) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const total = store.sim.frames.length;
    const target = Math.round((x / rect.width) * total);
    store.playhead = Math.max(0, Math.min(total - 1, target));
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
  <span class="label">MAP</span>
  <div class="body" bind:this={body}>
    <canvas bind:this={canvas} onclick={click}></canvas>
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
    letter-spacing: 0.12em;
    color: var(--ts-fg-3);
    border-right: 1px solid var(--ts-line);
  }
  .body { position: relative; overflow: hidden; }
  canvas { display: block; width: 100%; height: 100%; cursor: pointer; }
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
