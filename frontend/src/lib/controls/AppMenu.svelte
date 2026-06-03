<script lang="ts">
  // The titlebar kebab (⋮) dropdown. Clicking the ⋮ opens this rather than jumping
  // straight into the full Settings modal — quick Theme / Language live here, with
  // the heavier actions below.
  import { store } from "../store.svelte";
  import { THEMES, applyTheme, type ThemeId } from "../theme";
  import { resetLayout } from "../prefs";
  import { t, locale, setLocale, LANGS, type Lang } from "../i18n.svelte";
  import Select from "./Select.svelte";

  let { onClose }: { onClose: () => void } = $props();

  const themeOpts = $derived(THEMES.map((th) => ({ id: th.id, label: t(`theme.${th.id}`) })));
  const langOpts = LANGS.map((l) => ({ id: l.id, label: l.label }));

  function openSettings() { store.showSettings = true; onClose(); }
  function openShortcuts() { store.showShortcuts = true; onClose(); }
  function reset() { resetLayout(); onClose(); }
</script>

<button type="button" class="scrim" aria-label="Close menu" tabindex="-1"
  onpointerdown={onClose}></button>
<div class="menu" role="menu">
  <div class="field">
    <span class="lbl">{t("menu.theme")}</span>
    <Select value={store.theme} options={themeOpts} onChange={(id) => applyTheme(id as ThemeId)} />
  </div>
  <div class="field">
    <span class="lbl">{t("menu.language")}</span>
    <Select value={locale()} options={langOpts} onChange={(id) => setLocale(id as Lang)} />
  </div>
  <div class="div"></div>
  <button type="button" class="item" role="menuitem" onclick={openSettings}>{t("menu.settings")}…</button>
  <button type="button" class="item" role="menuitem" onclick={openShortcuts}>{t("menu.shortcuts")}</button>
  <button type="button" class="item" role="menuitem" onclick={reset}>{t("menu.reset")}</button>
</div>

<svelte:window onkeydown={(e) => { if (e.key === "Escape") onClose(); }} />

<style>
  .scrim { position: fixed; inset: 0; z-index: 120; background: transparent; border: none; cursor: default; }
  .menu {
    position: fixed; z-index: 121; top: 30px; right: 12px; width: 232px; padding: 8px;
    background: var(--ts-bg-1); border: 1px solid var(--ts-line-2); border-radius: 10px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5); font-family: var(--ts-sans);
  }
  :global(html[data-theme="light"]) .menu { box-shadow: 0 16px 40px rgba(0, 0, 0, 0.18); }
  .field { display: flex; align-items: center; gap: 10px; padding: 4px 4px 8px; }
  .field .lbl { flex: 0 0 56px; font-size: 12px; color: var(--ts-fg-2); }
  .div { height: 1px; margin: 4px 0; background: var(--ts-line); }
  .item {
    display: block; width: 100%; text-align: left; padding: 7px 8px; border: none; border-radius: 6px;
    background: transparent; color: var(--ts-fg); font-size: 12.5px; cursor: pointer;
  }
  .item:hover { background: var(--ts-bg-3); }
</style>
