<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { store } from "../store.svelte";
  import { THEMES, applyTheme, type ThemeId } from "../theme";
  import { resetLayout } from "../prefs";
  import { t, locale, setLocale, LANGS } from "../i18n.svelte";

  let open = $state(false);
  let showAbout = $state(false);
  let version = $state("0.1.0");
  let btn: HTMLButtonElement;
  let anchor = $state({ top: 34, right: 8 });

  // hover-flyout submenus (theme / language)
  let openSub = $state<null | "theme" | "lang">(null);
  let subTimer: number | undefined;
  function enterSub(k: "theme" | "lang") {
    if (subTimer) clearTimeout(subTimer);
    openSub = k;
  }
  function leaveSub() {
    subTimer = window.setTimeout(() => (openSub = null), 120) as unknown as number;
  }

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
  function close() { open = false; showAbout = false; openSub = null; }

  function pickTheme(id: ThemeId) { applyTheme(id); }
  function openShortcuts() { store.showShortcuts = true; close(); }
  function doReset() { resetLayout(); close(); }
</script>

<button
  bind:this={btn}
  class="kebab"
  class:active={open}
  onclick={toggle}
  title={t("menu.settings")}
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

    <!-- theme (hover to expand) -->
    <div class="sub-wrap" role="presentation"
      onmouseenter={() => enterSub("theme")} onmouseleave={leaveSub}>
      <div class="row sub-parent" class:open={openSub === "theme"}>
        <span class="check"></span>
        <span class="lbl">{t("menu.theme")}</span>
        <span class="cur">{t(`theme.${store.theme}`)}</span>
        <span class="chev">›</span>
      </div>
      {#if openSub === "theme"}
        <div class="submenu" role="menu">
          {#each THEMES as th (th.id)}
            <button class="row" role="menuitemradio" aria-checked={store.theme === th.id}
              onclick={() => pickTheme(th.id)}>
              <span class="check">{store.theme === th.id ? "✓" : ""}</span>
              <span class="lbl">{t(`theme.${th.id}`)}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- language (hover to expand) -->
    <div class="sub-wrap" role="presentation"
      onmouseenter={() => enterSub("lang")} onmouseleave={leaveSub}>
      <div class="row sub-parent" class:open={openSub === "lang"}>
        <span class="check"></span>
        <span class="lbl">{t("menu.language")}</span>
        <span class="cur">{LANGS.find((l) => l.id === locale())?.label}</span>
        <span class="chev">›</span>
      </div>
      {#if openSub === "lang"}
        <div class="submenu" role="menu">
          {#each LANGS as l (l.id)}
            <button class="row" role="menuitemradio" aria-checked={locale() === l.id}
              onclick={() => setLocale(l.id)}>
              <span class="check">{locale() === l.id ? "✓" : ""}</span>
              <span class="lbl">{l.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="div"></div>
    <div class="sect">{t("menu.playback")}</div>
    <button class="row" role="menuitemcheckbox" aria-checked={store.follow}
      onclick={() => (store.follow = !store.follow)}>
      <span class="check">{store.follow ? "✓" : ""}</span>
      <span class="lbl">{t("menu.follow")}</span>
      <span class="hint">Follow</span>
    </button>
    <button class="row" role="menuitemcheckbox" aria-checked={store.loop_}
      onclick={() => (store.loop_ = !store.loop_)}>
      <span class="check">{store.loop_ ? "✓" : ""}</span>
      <span class="lbl">{t("menu.loop")}</span>
      <span class="hint">Loop</span>
    </button>

    <div class="div"></div>
    <button class="row" role="menuitem" onclick={openShortcuts}>
      <span class="check"></span>
      <span class="lbl">{t("menu.shortcuts")}</span>
      <span class="hint">?</span>
    </button>
    <button class="row" role="menuitem" onclick={doReset}>
      <span class="check"></span>
      <span class="lbl">{t("menu.reset")}</span>
    </button>

    <div class="div"></div>
    <button class="row" role="menuitem" aria-expanded={showAbout}
      onclick={() => (showAbout = !showAbout)}>
      <span class="check">{showAbout ? "▾" : "▸"}</span>
      <span class="lbl">{t("menu.about")}</span>
    </button>
    {#if showAbout}
      <div class="about">
        <div class="about-name">TokioScope <span class="about-ver">v{version}</span></div>
        <div class="about-blurb">{t("about.tagline")}</div>
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

  /* collapsible theme/language → hover-flyout submenu */
  .sub-wrap { position: relative; }
  .sub-parent { cursor: default; }
  .sub-parent:hover, .sub-parent.open { background: var(--ts-bg-3); }
  .sub-parent .cur { flex: 0 0 auto; color: var(--ts-fg-3); font-size: 11px; }
  .sub-parent .chev { flex: 0 0 auto; margin-left: 6px; color: var(--ts-fg-3); }
  .submenu {
    position: absolute;
    right: calc(100% + 4px);
    top: -5px;
    min-width: 152px;
    padding: 5px;
    background: var(--ts-bg-1);
    border: 1px solid var(--ts-line-2);
    border-radius: 7px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
    z-index: 92;
  }

  .about {
    padding: 6px 10px 8px 32px;
    color: var(--ts-fg-2);
  }
  .about-name { font-size: 12px; color: var(--ts-fg); }
  .about-ver { color: var(--ts-fg-3); font-family: var(--ts-mono); font-size: 10.5px; }
  .about-blurb { margin-top: 3px; font-size: 11px; line-height: 1.4; color: var(--ts-fg-3); }
</style>
