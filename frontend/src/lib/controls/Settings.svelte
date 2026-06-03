<script lang="ts">
  import { onMount, createEventDispatcher } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { store } from "../store.svelte";
  import { THEMES, applyTheme, type ThemeId } from "../theme";
  import { resetLayout } from "../prefs";
  import { t, locale, setLocale, LANGS, type Lang } from "../i18n.svelte";
  import Select from "./Select.svelte";

  const dispatch = createEventDispatcher();
  let version = $state("0.2.0");

  const themeOpts = $derived(THEMES.map((th) => ({ id: th.id, label: t(`theme.${th.id}`) })));
  const langOpts = LANGS.map((l) => ({ id: l.id, label: l.label }));

  onMount(async () => {
    try { version = await getVersion(); } catch { /* web preview */ }
  });

  function close() { dispatch("close"); }
  function openShortcuts() { store.showShortcuts = true; close(); }
</script>

{#snippet fontRow(label: string, famKey: string, sizeKey: string)}
  <div class="field font-row">
    <span class="lbl">{label}</span>
    <input class="fam" value={(store as any)[famKey]} placeholder={t("font.placeholder")}
      oninput={(e) => ((store as any)[famKey] = (e.target as HTMLInputElement).value)} />
    <input class="size" type="number" min="8" max="32" value={(store as any)[sizeKey]}
      aria-label={t("font.size")}
      oninput={(e) => {
        const el = e.target as HTMLInputElement;
        (store as any)[sizeKey] = Math.max(8, Math.min(32, Number(el.value) || (store as any)[sizeKey]));
      }} />
  </div>
{/snippet}

<div class="scrim" role="presentation" onclick={close}></div>
<div class="modal" role="dialog" aria-modal="true" aria-label={t("settings.title")}>
  <header>
    <h2>{t("settings.title")}</h2>
    <button class="x" onclick={close} aria-label="Close">×</button>
  </header>

  <div class="body">
    <section>
      <h3>{t("settings.appearance")}</h3>
      <div class="field">
        <span class="lbl">{t("menu.theme")}</span>
        <Select value={store.theme} options={themeOpts} onChange={(id) => applyTheme(id as ThemeId)} />
      </div>
      <div class="field">
        <span class="lbl">{t("menu.language")}</span>
        <Select value={locale()} options={langOpts} onChange={(id) => setLocale(id as Lang)} />
      </div>
    </section>

    <section>
      <h3>{t("menu.fonts")}</h3>
      {@render fontRow(t("font.code"), "editorFont", "editorFontSize")}
      {@render fontRow(t("font.terminal"), "terminalFont", "terminalFontSize")}
      {@render fontRow(t("font.output"), "outputFont", "outputFontSize")}
      {@render fontRow(t("font.ui"), "uiFont", "uiFontSize")}
    </section>

    <section>
      <h3>{t("menu.playback")}</h3>
      <label class="check">
        <input type="checkbox" checked={store.follow} onchange={() => (store.follow = !store.follow)} />
        <span>{t("menu.follow")}</span>
      </label>
      <label class="check">
        <input type="checkbox" checked={store.loop_} onchange={() => (store.loop_ = !store.loop_)} />
        <span>{t("menu.loop")}</span>
      </label>
    </section>

    <section class="actions">
      <button class="link" onclick={openShortcuts}>{t("menu.shortcuts")}</button>
      <button class="link" onclick={() => resetLayout()}>{t("menu.reset")}</button>
    </section>

    <section class="about">
      <div class="name">TokioScope <span class="ver">v{version}</span></div>
      <div class="tag">{t("about.tagline")}</div>
    </section>
  </div>
</div>

<svelte:window onkeydown={(e) => { if (e.key === "Escape") close(); }} />

<style>
  .scrim { position: fixed; inset: 0; z-index: 110; background: rgba(0, 0, 0, 0.45); }
  .modal {
    position: fixed; z-index: 111; top: 50%; left: 50%; transform: translate(-50%, -50%);
    width: 440px; max-width: calc(100vw - 40px); max-height: calc(100vh - 64px);
    display: flex; flex-direction: column;
    background: var(--ts-bg-1); border: 1px solid var(--ts-line-2); border-radius: 12px;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5); font-family: var(--ts-sans);
  }
  :global(html[data-theme="light"]) .modal { box-shadow: 0 24px 60px rgba(0, 0, 0, 0.2); }

  header {
    display: flex; align-items: center; justify-content: space-between; flex-shrink: 0;
    padding: 14px 16px; border-bottom: 1px solid var(--ts-line);
  }
  header h2 { margin: 0; font-size: 15px; font-weight: 600; color: var(--ts-fg); }
  .x {
    display: inline-flex; align-items: center; justify-content: center; width: 26px; height: 26px;
    background: transparent; border: none; border-radius: 6px; color: var(--ts-fg-3); font-size: 18px; cursor: pointer;
  }
  .x:hover { background: var(--ts-bg-3); color: var(--ts-fg); }

  .body { overflow-y: auto; padding: 8px 16px 16px; }
  section { padding: 14px 0; border-bottom: 1px solid var(--ts-line); }
  section:last-child { border-bottom: none; }
  h3 { margin: 0 0 12px; font-size: 12px; font-weight: 600; color: var(--ts-fg-2); }

  .field { display: flex; align-items: center; gap: 12px; margin-bottom: 10px; }
  .field:last-child { margin-bottom: 0; }
  .field .lbl { flex: 0 0 88px; font-size: 12.5px; color: var(--ts-fg-2); }
  .field input {
    flex: 1 1 auto; min-width: 0; padding: 6px 9px; border-radius: 6px;
    border: 1px solid var(--ts-line-2); background: var(--ts-bg-2); color: var(--ts-fg);
    font-family: var(--ts-mono); font-size: 12px;
  }
  .field input:focus { outline: none; border-color: var(--ts-accent); }
  .field input::placeholder { color: var(--ts-fg-3); }

  .font-row { gap: 8px; }
  .font-row .fam { flex: 1 1 auto; min-width: 0; }
  .field input.size { flex: 0 0 52px; width: 52px; padding: 6px 6px; text-align: center; }

  .check { display: flex; align-items: center; gap: 9px; padding: 5px 0; font-size: 12.5px; color: var(--ts-fg); cursor: pointer; }
  .check input { accent-color: var(--ts-accent); width: 14px; height: 14px; }

  .actions { display: flex; gap: 18px; }
  .link { background: transparent; border: none; padding: 0; color: var(--ts-accent); font-size: 12.5px; cursor: pointer; }
  .link:hover { text-decoration: underline; }

  .about .name { font-size: 13px; color: var(--ts-fg); }
  .about .ver { font-family: var(--ts-mono); font-size: 11px; color: var(--ts-fg-3); }
  .about .tag { margin-top: 4px; font-size: 12px; line-height: 1.5; color: var(--ts-fg-3); }
</style>
