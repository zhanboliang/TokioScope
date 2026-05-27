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
