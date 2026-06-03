import { Store as PluginStore } from "@tauri-apps/plugin-store";
import { store, DEFAULT_FONT, DEFAULT_UI_FONT } from "./store.svelte";
import { ws } from "./workspace.svelte";
import { locale, setLocale } from "./i18n.svelte";
import { applyFonts } from "./font";

let backing: PluginStore | null = null;

export const DEFAULTS = {
  treeWidth: 220,
  editorWidth: 520,
  bottomHeight: 190,
  timelineHeight: 260,
  zoom: 20,
  speed: 4,
  loop: false,
  follow: true,
  editorFontSize: 13,
  terminalFontSize: 12,
  outputFontSize: 11,
  uiFontSize: 13,
};

export async function initPrefs() {
  try {
    backing = await PluginStore.load("prefs.json", { autoSave: true, defaults: {} });
  } catch (e) {
    // No Tauri backend (frontend opened in a plain browser / web preview) — boot
    // with in-memory defaults instead of crashing the whole app.
    console.warn("prefs store unavailable; using defaults", e);
    document.documentElement.dataset.theme = store.theme;
    applyFonts();
    return;
  }
  store.theme = ((await backing.get<string>("theme")) as "dark" | "light" | "hc") || "dark";
  store.timelineHeight = (await backing.get<number>("timelineHeight")) ?? DEFAULTS.timelineHeight;
  store.zoom = (await backing.get<number>("zoom")) ?? DEFAULTS.zoom;
  store.speed = (await backing.get<number>("speed")) ?? DEFAULTS.speed;
  store.loop_ = (await backing.get<boolean>("loop")) ?? DEFAULTS.loop;
  store.follow = (await backing.get<boolean>("follow")) ?? DEFAULTS.follow;

  ws.treeWidth = (await backing.get<number>("treeWidth")) ?? DEFAULTS.treeWidth;
  ws.editorWidth = (await backing.get<number>("editorWidth")) ?? DEFAULTS.editorWidth;
  ws.bottomHeight = (await backing.get<number>("bottomHeight")) ?? DEFAULTS.bottomHeight;
  store.editorFont = (await backing.get<string>("editorFont")) || DEFAULT_FONT;
  store.editorFontSize = (await backing.get<number>("editorFontSize")) ?? DEFAULTS.editorFontSize;
  store.terminalFont = (await backing.get<string>("terminalFont")) || DEFAULT_FONT;
  store.terminalFontSize = (await backing.get<number>("terminalFontSize")) ?? DEFAULTS.terminalFontSize;
  store.outputFont = (await backing.get<string>("outputFont")) || DEFAULT_FONT;
  store.outputFontSize = (await backing.get<number>("outputFontSize")) ?? DEFAULTS.outputFontSize;
  store.uiFont = (await backing.get<string>("uiFont")) || DEFAULT_UI_FONT;
  store.uiFontSize = (await backing.get<number>("uiFontSize")) ?? DEFAULTS.uiFontSize;

  const savedLang = await backing.get<string>("lang");
  if (savedLang === "zh" || savedLang === "en") setLocale(savedLang);

  document.documentElement.dataset.theme = store.theme;
  applyFonts(); // paint output + UI font CSS vars from the loaded prefs
}

/** Restore layout & playback prefs to their defaults (theme is left untouched). */
export function resetLayout() {
  ws.treeWidth = DEFAULTS.treeWidth;
  ws.editorWidth = DEFAULTS.editorWidth;
  ws.bottomHeight = DEFAULTS.bottomHeight;
  ws.bottomCollapsed = false;
  store.timelineHeight = DEFAULTS.timelineHeight;
  store.zoom = DEFAULTS.zoom;
  store.speed = DEFAULTS.speed;
  store.loop_ = DEFAULTS.loop;
  store.follow = DEFAULTS.follow;
  persistPrefs();
}

let debounceTimer: number | undefined;
export function persistPrefs() {
  if (!backing) return;
  if (debounceTimer) window.clearTimeout(debounceTimer);
  debounceTimer = window.setTimeout(async () => {
    if (!backing) return;
    await backing.set("theme", store.theme);
    await backing.set("timelineHeight", store.timelineHeight);
    await backing.set("zoom", store.zoom);
    await backing.set("speed", store.speed);
    await backing.set("loop", store.loop_);
    await backing.set("follow", store.follow);
    await backing.set("treeWidth", ws.treeWidth);
    await backing.set("editorWidth", ws.editorWidth);
    await backing.set("bottomHeight", ws.bottomHeight);
    await backing.set("editorFont", store.editorFont);
    await backing.set("editorFontSize", store.editorFontSize);
    await backing.set("terminalFont", store.terminalFont);
    await backing.set("terminalFontSize", store.terminalFontSize);
    await backing.set("outputFont", store.outputFont);
    await backing.set("outputFontSize", store.outputFontSize);
    await backing.set("uiFont", store.uiFont);
    await backing.set("uiFontSize", store.uiFontSize);
    await backing.set("lang", locale());
    await backing.save();
  }, 300) as unknown as number;
}
