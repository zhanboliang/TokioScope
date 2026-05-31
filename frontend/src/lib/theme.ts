import { store } from "./store.svelte";

export type ThemeId = "dark" | "light" | "hc";

export const THEMES: { id: ThemeId; label: string }[] = [
  { id: "dark", label: "Dark" },
  { id: "light", label: "Light" },
  { id: "hc", label: "High-Contrast" },
];

const ORDER: ThemeId[] = THEMES.map((t) => t.id);

export function applyTheme(t: ThemeId) {
  document.documentElement.dataset.theme = t;
  store.theme = t;
}

export function cycleTheme() {
  const i = ORDER.indexOf(store.theme);
  applyTheme(ORDER[(i + 1) % ORDER.length]);
}
