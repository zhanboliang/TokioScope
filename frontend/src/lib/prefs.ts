import { Store as PluginStore } from "@tauri-apps/plugin-store";
import { store } from "./store.svelte";

let backing: PluginStore | null = null;

export async function initPrefs() {
  backing = await PluginStore.load("prefs.json", { autoSave: true, defaults: {} });
  store.theme = ((await backing.get<string>("theme")) as "dark" | "light" | "hc") || "dark";
  store.codePaneWidth = (await backing.get<number>("codePaneWidth")) ?? 420;
  store.outputHeight = (await backing.get<number>("outputHeight")) ?? 140;
  store.timelineHeight = (await backing.get<number>("timelineHeight")) ?? 260;
  store.zoom = (await backing.get<number>("zoom")) ?? 20;
  store.speed = (await backing.get<number>("speed")) ?? 4;
  store.loop_ = (await backing.get<boolean>("loop")) ?? false;
  store.follow = (await backing.get<boolean>("follow")) ?? true;

  document.documentElement.dataset.theme = store.theme;
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
    await backing.save();
  }, 300) as unknown as number;
}
