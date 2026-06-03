<script lang="ts">
  import { crossfade, fade } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { store } from "../store.svelte";
  import { STATE_COLOR } from "../colors";
  import type { TaskSnapshot } from "../engine";
  import { t as tr } from "../i18n.svelte"; // aliased — `t` is the task var in {#each}

  const frame = $derived(store.currentFrame);
  const tasks = $derived(frame?.tasks);
  const cores = $derived(frame?.workers ?? []);
  const ready = $derived(frame?.ready ?? []);
  const readyTasks = $derived(
    ready.map((id) => task(id)).filter((t): t is TaskSnapshot => !!t),
  );
  const slots = $derived(frame?.blocking ?? []);
  const tick = $derived(frame?.tick ?? 0);
  const awaiting = $derived(
    frame ? [...frame.tasks.values()].filter((t) => t.state === "awaiting").sort((a, b) => a.id - b.id) : [],
  );
  const done = $derived(
    frame ? [...frame.tasks.values()].filter((t) => t.state === "done").sort((a, b) => a.id - b.id) : [],
  );

  function task(id: number): TaskSnapshot | undefined {
    return tasks?.get(id);
  }

  // user-resizable Ready | CPU | Awaiting columns, held as fr ratios
  let cols = $state<[number, number, number]>([1, 1.7, 1.3]);
  let lanesEl: HTMLDivElement;
  // drag between cols[gap] and cols[gap+1]; px-per-fr derived from the lanes width
  let drag = $state<null | { gap: 0 | 1; startX: number; a: number; b: number; perFr: number }>(null);

  const MIN_FR = 0.4;

  function startSepDrag(gap: 0 | 1) {
    return (e: PointerEvent) => {
      const total = cols[0] + cols[1] + cols[2];
      const perFr = lanesEl ? lanesEl.clientWidth / total : 0;
      drag = { gap, startX: e.clientX, a: cols[gap], b: cols[gap + 1], perFr };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      e.preventDefault();
    };
  }
  function onSepMove(e: PointerEvent) {
    if (!drag || !drag.perFr) return;
    const dFr = (e.clientX - drag.startX) / drag.perFr;
    const sum = drag.a + drag.b;
    const a = Math.max(MIN_FR, Math.min(sum - MIN_FR, drag.a + dFr));
    const next: [number, number, number] = [cols[0], cols[1], cols[2]];
    next[drag.gap] = a;
    next[drag.gap + 1] = sum - a;
    cols = next;
  }
  function endSepDrag(e: PointerEvent) {
    drag = null;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  }

  // Cross-zone flow: a chip leaving one zone (out:send) and entering another
  // (in:receive), matched by task id, animates from its old spot to its new one.
  const [send, receive] = crossfade({
    duration: 240,
    fallback: (node) => fade(node, { duration: 160 }),
  });
  const FLIP = { duration: 240 };

  // Icon-only await marker (full label kept as a tooltip) — #3 less text.
  function awaitIcon(reason?: string): string {
    return reason === "timer" ? "⏱" : reason === "children" ? "⏳" : reason === "blocking" ? "🧵" : "…";
  }
  function awaitLabel(reason?: string): string {
    return reason === "timer" ? tr("await.timer")
      : reason === "children" ? tr("await.children")
      : reason === "blocking" ? tr("await.blocking")
      : tr("await.generic");
  }
</script>

{#snippet chip(t: TaskSnapshot)}
  <span class="chip" style="--c: {STATE_COLOR[t.state]}">
    <span class="dot"></span>
    <span class="nm">{t.name}</span>
    <span class="ln">L{t.currentLine}</span>
  </span>
{/snippet}

{#snippet zicon(kind: string)}
  <svg class="zi" viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor"
    stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    {#if kind === "ready"}<path d="M2.5 4h11M2.5 8h11M2.5 12h7" />
    {:else if kind === "cpu"}<rect x="3.5" y="3.5" width="9" height="9" rx="1.2" /><path d="M6 1.5v2M10 1.5v2M6 12.5v2M10 12.5v2M1.5 6h2M1.5 10h2M12.5 6h2M12.5 10h2" />
    {:else if kind === "await"}<circle cx="8" cy="8" r="5.6" /><path d="M8 5v3.2l2 1.3" />
    {:else if kind === "blocking"}<path d="M4 2v3.6l4 4 4-4V2M4 14v-3.6l4-4 4 4V14" />
    {:else if kind === "done"}<path d="M3 8.5l3.2 3 6-7" />{/if}
  </svg>
{/snippet}

<section class="stage">
  <div
    class="lanes"
    bind:this={lanesEl}
    style="grid-template-columns: {cols[0]}fr 4px {cols[1]}fr 4px {cols[2]}fr"
  >
    <!-- ready queue -->
    <div class="zone ready">
      <header title={tr("zone.ready")}>{@render zicon("ready")}<b>{ready.length}</b></header>
      <div class="list">
        {#each readyTasks as t (t.id)}
          <div class="row" in:receive={{ key: t.id }} out:send={{ key: t.id }} animate:flip={FLIP}>
            {@render chip(t)}
          </div>
        {/each}
        {#if readyTasks.length === 0}<div class="empty" title={tr("stage.empty")}>—</div>{/if}
      </div>
    </div>

    <!-- Ready | CPU separator -->
    <div class="sep" role="separator" aria-orientation="vertical"
      onpointerdown={startSepDrag(0)} onpointermove={onSepMove} onpointerup={endSepDrag}></div>

    <!-- CPU cores -->
    <div class="zone cpu">
      <header title={tr("zone.cpu", { n: cores.length })}>{@render zicon("cpu")}<b>{cores.length}</b></header>
      <div class="cores">
        {#each cores as id, i (i)}
          <div class="core" class:busy={id != null}>
            <span class="cap">W{i}</span>
            {#if id != null}
              {@const t = task(id)}
              {#if t}
                {#each [id] as cid (cid)}
                  <div class="row" in:receive={{ key: cid }} out:send={{ key: cid }}>
                    {@render chip(t)}
                  </div>
                {/each}
              {/if}
            {:else}
              <span class="idle" title={tr("stage.idle")}>·</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <!-- CPU | Awaiting separator -->
    <div class="sep" role="separator" aria-orientation="vertical"
      onpointerdown={startSepDrag(1)} onpointermove={onSepMove} onpointerup={endSepDrag}></div>

    <!-- await / timer -->
    <div class="zone awaitz">
      <header title={tr("zone.await")}>{@render zicon("await")}<b>{awaiting.length}</b></header>
      <div class="list">
        {#each awaiting as t (t.id)}
          <div class="row awaitrow" in:receive={{ key: t.id }} out:send={{ key: t.id }} animate:flip={FLIP}>
            <span class="why" title={awaitLabel(t.awaitReason)}>{awaitIcon(t.awaitReason)}</span>
            {@render chip(t)}
          </div>
        {/each}
        {#if awaiting.length === 0}<div class="empty" title={tr("stage.empty")}>—</div>{/if}
      </div>
    </div>
  </div>

  <!-- blocking pool -->
  <div class="blocking">
    <span class="bl-label" title={tr("zone.blocking")}>{@render zicon("blocking")}</span>
    <div class="bslots">
      {#each slots as s, i}
        {@const bt = s.taskId != null ? task(s.taskId) : null}
        {@const pct = s.taskId != null && s.durationTicks > 0
          ? Math.min(100, Math.max(0, ((tick - s.startTick) / s.durationTicks) * 100))
          : 0}
        <div class="bslot" class:busy={!!bt}>
          <span class="bcap">B{i}</span>
          {#if bt}
            <div class="bbar"><div class="bfill" style="width: {pct}%"></div></div>
            <span class="bnm">{bt.name}</span>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  <!-- done tray -->
  {#if done.length}
    <div class="done-tray">
      <span class="dt-label" title={tr("zone.done")}>{@render zicon("done")} {done.length}</span>
      {#each done as t (t.id)}
        <div
          class="done-item"
          class:just={t.doneTick === tick}
          title={tr("done.tooltip", { name: t.name, born: t.bornTick, done: t.doneTick ?? 0, life: t.durationTicks })}
          in:receive={{ key: t.id }}
          out:send={{ key: t.id }}
          animate:flip={FLIP}
        >
          {@render chip(t)}
          <span class="life">⏳ {t.durationTicks}t</span>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .stage {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    min-height: 0;
    background: var(--ts-bg-0);
    border-top: 1px solid var(--ts-line);
    overflow: hidden;
  }
  .lanes {
    display: grid;
    /* grid-template-columns is set inline from resizable fr state, with two
       4px handle tracks between the three zones */
    flex: 1 1 auto;
    min-height: 0;
    background: var(--ts-line);
  }
  /* draggable zone separators — mirror the app's .vsep affordance */
  .sep { background: var(--ts-line); cursor: col-resize; }
  .sep:hover { background: var(--ts-line-2); }
  .zone {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--ts-bg-1);
    overflow: hidden;
  }
  .zone > header {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
    padding: 6px 10px;
    font-family: var(--ts-mono);
    font-size: 11px;
    color: var(--ts-fg-2);
    border-bottom: 1px solid var(--ts-line);
  }
  .zone > header b { color: var(--ts-fg); font-weight: 600; }
  .zi { color: var(--ts-fg-3); flex-shrink: 0; }
  .cpu > header .zi { color: var(--ts-st-running); }
  .awaitz > header .zi { color: var(--ts-st-awaiting); }
  .list {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .empty { color: var(--ts-fg-3); font-family: var(--ts-mono); font-size: 11px; }

  /* task chip */
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: 6px;
    background: color-mix(in srgb, var(--c) 16%, var(--ts-bg-2));
    border: 1px solid color-mix(in srgb, var(--c) 50%, transparent);
    font-family: var(--ts-mono);
    font-size: 11px;
    color: var(--ts-fg);
    white-space: nowrap;
  }
  .chip .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--c); flex-shrink: 0; }
  .chip .nm { overflow: hidden; text-overflow: ellipsis; }
  .chip .ln { color: var(--ts-fg-3); font-size: 10px; }
  .row { display: flex; align-items: center; gap: 6px; }

  /* CPU cores */
  .cores {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    padding: 8px;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 8px;
    align-content: start;
  }
  .core {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 40px;
    padding: 6px 8px;
    border-radius: 8px;
    background: var(--ts-bg-2);
    border: 1px solid var(--ts-line-2);
    transition: border-color 160ms ease, box-shadow 160ms ease, background 160ms ease;
  }
  .core.busy {
    border-color: var(--ts-st-running);
    background: color-mix(in srgb, var(--ts-st-running) 10%, var(--ts-bg-2));
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--ts-st-running) 40%, transparent),
                0 0 14px color-mix(in srgb, var(--ts-st-running) 22%, transparent);
  }
  .core .cap {
    font-family: var(--ts-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--ts-fg-2);
    flex-shrink: 0;
  }
  .core.busy .cap { color: var(--ts-st-running); }
  .core .idle { font-family: var(--ts-mono); font-size: 10px; color: var(--ts-fg-3); }

  /* await rows carry a reason tag */
  .awaitrow .why { font-family: var(--ts-mono); font-size: 9.5px; color: var(--ts-fg-3); flex-shrink: 0; }

  /* blocking pool strip */
  .blocking {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    background: var(--ts-bg-1);
    border-top: 1px solid var(--ts-line);
  }
  .bl-label { font-family: var(--ts-mono); font-size: 10px; letter-spacing: 0.04em; color: var(--ts-fg-2); flex-shrink: 0; }
  .bslots { display: flex; gap: 6px; flex-wrap: wrap; flex: 1 1 auto; }
  .bslot {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--ts-bg-2);
    border: 1px solid var(--ts-line-2);
    font-family: var(--ts-mono);
    font-size: 10px;
    min-width: 54px;
  }
  .bslot .bcap { color: var(--ts-fg-3); }
  .bslot.busy { border-color: color-mix(in srgb, var(--ts-st-blocking) 50%, transparent); }
  .bslot.busy .bcap { color: var(--ts-st-blocking); }
  .bbar { width: 44px; height: 6px; border-radius: 3px; background: var(--ts-bg-0); overflow: hidden; }
  .bfill { height: 100%; background: var(--ts-st-blocking); transition: width 80ms linear; }
  .bnm { color: var(--ts-fg-2); max-width: 90px; overflow: hidden; text-overflow: ellipsis; }

  /* done tray */
  .done-tray {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--ts-bg-1);
    border-top: 1px solid var(--ts-line);
    overflow-x: auto;
  }
  .done-tray .dt-label { font-family: var(--ts-mono); font-size: 10px; color: var(--ts-fg-2); flex-shrink: 0; }
  .done-item {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 1px 4px;
    border-radius: 8px;
    flex-shrink: 0;
  }
  .done-item .chip { opacity: 0.7; }
  .done-item .life { font-family: var(--ts-mono); font-size: 9.5px; color: var(--ts-fg-3); white-space: nowrap; }
  /* the task that finished on this exact tick — flag it so "task ended" is obvious */
  .done-item.just .chip { opacity: 1; }
  .done-item.just .life { color: var(--ts-st-done); }
  .done-item.just {
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--ts-st-done) 55%, transparent),
                0 0 12px color-mix(in srgb, var(--ts-st-done) 30%, transparent);
  }
</style>
