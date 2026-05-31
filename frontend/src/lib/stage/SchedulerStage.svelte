<script lang="ts">
  import { crossfade, fade } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { store } from "../store.svelte";
  import { STATE_COLOR } from "../colors";
  import type { TaskSnapshot } from "../engine";

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

  // Cross-zone flow: a chip leaving one zone (out:send) and entering another
  // (in:receive), matched by task id, animates from its old spot to its new one.
  const [send, receive] = crossfade({
    duration: 240,
    fallback: (node) => fade(node, { duration: 160 }),
  });
  const FLIP = { duration: 240 };

  function awaitTag(reason?: string): string {
    return reason === "timer" ? "⏱ 计时器"
      : reason === "children" ? "⏳ 子任务"
      : reason === "blocking" ? "🧵 阻塞"
      : "… 挂起";
  }
</script>

{#snippet chip(t: TaskSnapshot)}
  <span class="chip" style="--c: {STATE_COLOR[t.state]}">
    <span class="dot"></span>
    <span class="nm">{t.name}</span>
    <span class="ln">L{t.currentLine}</span>
  </span>
{/snippet}

<section class="stage">
  <div class="lanes">
    <!-- ready queue -->
    <div class="zone ready">
      <header>就绪队列 · {ready.length}</header>
      <div class="list">
        {#each readyTasks as t (t.id)}
          <div class="row" in:receive={{ key: t.id }} out:send={{ key: t.id }} animate:flip={FLIP}>
            {@render chip(t)}
          </div>
        {/each}
        {#if readyTasks.length === 0}<div class="empty">空</div>{/if}
      </div>
    </div>

    <!-- CPU cores -->
    <div class="zone cpu">
      <header>CPU · {cores.length} 核</header>
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
              <span class="idle">空闲</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <!-- await / timer -->
    <div class="zone awaitz">
      <header>等待 / 计时 · {awaiting.length}</header>
      <div class="list">
        {#each awaiting as t (t.id)}
          <div class="row awaitrow" in:receive={{ key: t.id }} out:send={{ key: t.id }} animate:flip={FLIP}>
            <span class="why">{awaitTag(t.awaitReason)}</span>
            {@render chip(t)}
          </div>
        {/each}
        {#if awaiting.length === 0}<div class="empty">空</div>{/if}
      </div>
    </div>
  </div>

  <!-- blocking pool -->
  <div class="blocking">
    <span class="bl-label">阻塞池</span>
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
      <span class="dt-label">已完成</span>
      {#each done as t (t.id)}
        <div class="row" in:receive={{ key: t.id }} out:send={{ key: t.id }} animate:flip={FLIP}>
          {@render chip(t)}
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
    grid-template-columns: 1fr 1.7fr 1.3fr;
    gap: 1px;
    background: var(--ts-line);
    flex: 1 1 auto;
    min-height: 0;
  }
  .zone {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--ts-bg-1);
    overflow: hidden;
  }
  .zone > header {
    flex-shrink: 0;
    padding: 6px 10px;
    font-family: var(--ts-mono);
    font-size: 10px;
    letter-spacing: 0.04em;
    color: var(--ts-fg-2);
    border-bottom: 1px solid var(--ts-line);
  }
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
  .done-tray .chip { opacity: 0.6; }
</style>
