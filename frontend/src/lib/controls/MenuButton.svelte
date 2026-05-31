<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { store } from "../store.svelte";
  import { THEMES, applyTheme, type ThemeId } from "../theme";
  import { resetLayout } from "../prefs";

  let open = $state(false);
  let showAbout = $state(false);
  let version = $state("0.1.0");
  let btn: HTMLButtonElement;
  let anchor = $state({ top: 34, right: 8 });

  onMount(async () => {
    try { version = await getVersion(); } catch { /* web preview — keep default */ }
  });

  function place() {
    if (!btn) return;
    const r = btn.getBoundingClientRect();
    anchor = { top: r.bottom + 4, right: Math.max(8, window.innerWidth - r.right) };
  }
  function toggle() {
    open = !open;
    if (open) place();
    else showAbout = false;
  }
  function close() { open = false; showAbout = false; }

  function pickTheme(id: ThemeId) { applyTheme(id); }
  function openShortcuts() { store.showShortcuts = true; close(); }
  function doReset() { resetLayout(); close(); }
</script>

<button
  bind:this={btn}
  class="kebab"
  class:active={open}
  onclick={toggle}
  title="设置"
  aria-label="Settings"
  aria-haspopup="menu"
  aria-expanded={open}>
  <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" fill="currentColor">
    <circle cx="8" cy="3" r="1.45" />
    <circle cx="8" cy="8" r="1.45" />
    <circle cx="8" cy="13" r="1.45" />
  </svg>
</button>

{#if open}
  <div class="scrim" onclick={close} role="presentation"></div>
  <div class="popover" role="menu" tabindex="-1"
    style="top: {anchor.top}px; right: {anchor.right}px;">

    <div class="sect">主题</div>
    {#each THEMES as t (t.id)}
      <button class="row" role="menuitemradio" aria-checked={store.theme === t.id}
        onclick={() => pickTheme(t.id)}>
        <span class="check">{store.theme === t.id ? "✓" : ""}</span>
        <span class="lbl">{t.label}</span>
      </button>
    {/each}

    <div class="div"></div>
    <div class="sect">播放</div>
    <button class="row" role="menuitemcheckbox" aria-checked={store.follow}
      onclick={() => (store.follow = !store.follow)}>
      <span class="check">{store.follow ? "✓" : ""}</span>
      <span class="lbl">跟随播放头</span>
      <span class="hint">Follow</span>
    </button>
    <button class="row" role="menuitemcheckbox" aria-checked={store.loop_}
      onclick={() => (store.loop_ = !store.loop_)}>
      <span class="check">{store.loop_ ? "✓" : ""}</span>
      <span class="lbl">循环播放</span>
      <span class="hint">Loop</span>
    </button>

    <div class="div"></div>
    <button class="row" role="menuitem" onclick={openShortcuts}>
      <span class="check"></span>
      <span class="lbl">键盘快捷键</span>
      <span class="hint">?</span>
    </button>
    <button class="row" role="menuitem" onclick={doReset}>
      <span class="check"></span>
      <span class="lbl">重置布局</span>
    </button>

    <div class="div"></div>
    <button class="row" role="menuitem" aria-expanded={showAbout}
      onclick={() => (showAbout = !showAbout)}>
      <span class="check">{showAbout ? "▾" : "▸"}</span>
      <span class="lbl">关于 TokioScope</span>
    </button>
    {#if showAbout}
      <div class="about">
        <div class="about-name">TokioScope <span class="about-ver">v{version}</span></div>
        <div class="about-blurb">See how Tokio schedules your async code, tick by tick.</div>
      </div>
    {/if}
  </div>
{/if}

<svelte:window onkeydown={(e) => { if (open && e.key === "Escape") close(); }} />

<style>
  .kebab {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 22px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--ts-fg-2);
    cursor: pointer;
    transition: background 80ms ease, color 80ms ease;
    -webkit-app-region: no-drag;
  }
  .kebab:hover { background: var(--ts-bg-3); color: var(--ts-fg); }
  .kebab.active { background: var(--ts-bg-3); color: var(--ts-accent); }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 90;
    -webkit-app-region: no-drag;
  }
  .popover {
    position: fixed;
    z-index: 91;
    min-width: 232px;
    padding: 5px;
    background: var(--ts-bg-1);
    border: 1px solid var(--ts-line-2);
    border-radius: 7px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
    font-family: var(--ts-sans);
    -webkit-app-region: no-drag;
  }
  :global(html[data-theme="light"]) .popover { box-shadow: 0 12px 36px rgba(0, 0, 0, 0.18); }

  .sect {
    padding: 6px 10px 3px;
    font-family: var(--ts-mono);
    font-size: 9.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ts-fg-3);
  }
  .div { height: 1px; margin: 5px 6px; background: var(--ts-line); }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--ts-fg);
    font-family: var(--ts-sans);
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .row:hover { background: var(--ts-bg-3); }
  .row .check {
    flex: 0 0 14px;
    color: var(--ts-accent);
    font-size: 11px;
    text-align: center;
  }
  .row .lbl { flex: 1 1 auto; }
  .row .hint {
    flex: 0 0 auto;
    font-family: var(--ts-mono);
    font-size: 10px;
    color: var(--ts-fg-3);
  }
  .row[aria-checked="true"] .lbl { color: var(--ts-fg); }

  .about {
    padding: 6px 10px 8px 32px;
    color: var(--ts-fg-2);
  }
  .about-name { font-size: 12px; color: var(--ts-fg); }
  .about-ver { color: var(--ts-fg-3); font-family: var(--ts-mono); font-size: 10.5px; }
  .about-blurb { margin-top: 3px; font-size: 11px; line-height: 1.4; color: var(--ts-fg-3); }
</style>
