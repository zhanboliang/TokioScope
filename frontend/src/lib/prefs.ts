import { Store as PluginStore } from "@tauri-apps/plugin-store";
import { store } from "./store.svelte";
import { locale, setLocale } from "./i18n.svelte";

let backing: PluginStore | null = null;

export const DEFAULTS = {
  codePaneWidth: 420,
  outputHeight: 140,
  timelineHeight: 260,
  zoom: 20,
  speed: 4,
  loop: false,
  follow: true,
};

export async function initPrefs() {
  backing = await PluginStore.load("prefs.json", { autoSave: true, defaults: {} });
  store.theme = ((await backing.get<string>("theme")) as "dark" | "light" | "hc") || "dark";
  store.codePaneWidth = (await backing.get<number>("codePaneWidth")) ?? DEFAULTS.codePaneWidth;
  store.outputHeight = (await backing.get<number>("outputHeight")) ?? DEFAULTS.outputHeight;
  store.timelineHeight = (await backing.get<number>("timelineHeight")) ?? DEFAULTS.timelineHeight;
  store.zoom = (await backing.get<number>("zoom")) ?? DEFAULTS.zoom;
  store.speed = (await backing.get<number>("speed")) ?? DEFAULTS.speed;
  store.loop_ = (await backing.get<boolean>("loop")) ?? DEFAULTS.loop;
  store.follow = (await backing.get<boolean>("follow")) ?? DEFAULTS.follow;

  const savedLang = await backing.get<string>("lang");
  if (savedLang === "zh" || savedLang === "en") setLocale(savedLang);

  document.documentElement.dataset.theme = store.theme;
}

/** Restore layout & playback prefs to their defaults (theme is left untouched). */
export function resetLayout() {
  store.codePaneWidth = DEFAULTS.codePaneWidth;
  store.outputHeight = DEFAULTS.outputHeight;
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
    await backing.set("codePaneWidth", store.codePaneWidth);
    await backing.set("outputHeight", store.outputHeight);
    await backing.set("timelineHeight", store.timelineHeight);
    await backing.set("zoom", store.zoom);
    await backing.set("speed", store.speed);
    await backing.set("loop", store.loop_);
    await backing.set("follow", store.follow);
    await backing.set("lang", locale());
    await backing.save();
  }, 300) as unknown as number;
}
