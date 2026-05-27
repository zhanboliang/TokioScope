import { store } from "./store.svelte";

export type ThemeId = "dark" | "light" | "hc";
const ORDER: ThemeId[] = ["dark", "light", "hc"];

export function applyTheme(t: ThemeId) {
  document.documentElement.dataset.theme = t;
  store.theme = t;
}

export function cycleTheme() {
  const i = ORDER.indexOf(store.theme);
  applyTheme(ORDER[(i + 1) % ORDER.length]);
}
