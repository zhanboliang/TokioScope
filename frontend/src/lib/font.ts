import { store } from "./store.svelte";

/**
 * Push the user's font prefs into root CSS custom properties. The code editor
 * and terminal read their own family/size directly from the store (they own
 * EditorView / xterm instances); this handles the Output log and the UI chrome,
 * which are driven purely through CSS vars.
 *
 * The UI family is composed *before* `--ts-sans-base` (the CJK-aware stack) so a
 * custom interface font never clobbers the CJK fallback.
 */
export function applyFonts() {
  const root = document.documentElement.style;
  root.setProperty(
    "--ts-output-font",
    store.outputFont ? `"${store.outputFont}", var(--ts-mono)` : "var(--ts-mono)",
  );
  root.setProperty("--ts-output-size", `${store.outputFontSize}px`);
  root.setProperty(
    "--ts-sans",
    store.uiFont ? `"${store.uiFont}", var(--ts-sans-base)` : "var(--ts-sans-base)",
  );
  root.setProperty("--ts-ui-size", `${store.uiFontSize}px`);
}

/**
 * Best-effort CJK font detection. Adds a body class so the dark/HC stylesheets
 * can substitute platform-appropriate CJK families ahead of the latin stack.
 */
export function detectCJK() {
  const ua = navigator.userAgent.toLowerCase();
  const lang = (navigator.language || "").toLowerCase();
  const langs = (navigator.languages || []).map((l) => l.toLowerCase());
  const all = [lang, ...langs].join(" ");

  let bucket: "zh" | "ja" | "ko" | null = null;
  if (/zh/.test(all)) bucket = "zh";
  else if (/ja/.test(all)) bucket = "ja";
  else if (/ko/.test(all)) bucket = "ko";
  else if (/mac|windows|linux/.test(ua)) bucket = "zh"; // sensible default for east asia ui

  if (bucket) document.documentElement.dataset.cjk = bucket;
}
