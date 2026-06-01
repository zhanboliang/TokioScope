import { store } from "./store.svelte";
import { ipc } from "./ipc";
import { t } from "./i18n.svelte";

export interface ShortcutGroup {
  group: string;
  items: { keys: string; action: string }[];
}

/** Localized shortcut reference (reactive: reads t() / locale()). */
export function getShortcuts(): ShortcutGroup[] {
  return [
    {
      group: t("sc.group.playback"),
      items: [
        { keys: "Space", action: t("sc.playPause") },
        { keys: "→ / ←", action: t("sc.step") },
        { keys: "Shift + → / ←", action: t("sc.fine") },
        { keys: "R", action: t("sc.reset") },
        { keys: "Home / End", action: t("sc.homeEnd") },
        { keys: "L", action: t("sc.toggleLoop") },
      ],
    },
    {
      group: t("sc.group.timeline"),
      items: [
        { keys: "+ / -", action: t("sc.zoom") },
        { keys: t("key.wheel"), action: t("sc.zoomAnchor") },
        { keys: t("key.shiftDrag"), action: t("sc.pan") },
        { keys: t("key.midDrag"), action: t("sc.pan") },
        { keys: t("key.dblClick"), action: t("sc.fit") },
        { keys: "F", action: t("sc.follow") },
      ],
    },
    {
      group: t("sc.group.code"),
      items: [
        { keys: "⌘/Ctrl + Enter", action: t("sc.run") },
        { keys: "⌘/Ctrl + .", action: t("sc.cancel") },
        { keys: "⌘/Ctrl + E", action: t("sc.inline") },
        { keys: "⌘/Ctrl + 1/2/3", action: t("sc.examples") },
        { keys: "?", action: t("sc.help") },
      ],
    },
  ];
}

export function bindGlobal(loadExample: (idx: number) => void) {
  const handler = (e: KeyboardEvent) => {
    // ignore when typing in editor — CodeMirror lives in #editor-root
    const tgt = e.target as HTMLElement | null;
    if (tgt && (tgt.closest("#editor-root") || tgt.isContentEditable)) {
      // allow ⌘/Ctrl+Enter for run even inside editor
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
        e.preventDefault();
        runCode();
        return;
      }
      if ((e.metaKey || e.ctrlKey) && e.key === ".") {
        e.preventDefault();
        ipc.cancelRun();
        return;
      }
      return;
    }

    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key === "Enter") { e.preventDefault(); runCode(); return; }
    if (mod && e.key === ".") { e.preventDefault(); ipc.cancelRun(); return; }
    if (mod && e.key === "e") { e.preventDefault(); store.inlineEdit = !store.inlineEdit; return; }
    if (mod && (e.key === "1" || e.key === "2" || e.key === "3")) {
      e.preventDefault();
      loadExample(parseInt(e.key) - 1);
      return;
    }

    switch (e.key) {
      case " ":
        e.preventDefault();
        store.playing = !store.playing;
        break;
      case "ArrowRight":
        e.preventDefault();
        if (store.totalTicks)
          store.playhead = Math.min(store.totalTicks - 1,
            e.shiftKey ? store.playhead + 0.1 : Math.floor(store.playhead) + 1);
        break;
      case "ArrowLeft":
        e.preventDefault();
        store.playhead = Math.max(0,
          e.shiftKey ? store.playhead - 0.1 : Math.ceil(store.playhead) - 1);
        break;
      case "r":
      case "R":
        store.playhead = 0;
        store.playing = false;
        break;
      case "Home":
        store.playhead = 0;
        break;
      case "End":
        if (store.totalTicks) store.playhead = store.totalTicks - 1;
        break;
      case "f":
      case "F":
        store.follow = !store.follow;
        break;
      case "l":
      case "L":
        store.loop_ = !store.loop_;
        break;
      case "+":
      case "=":
        store.zoom = Math.min(120, store.zoom + 4);
        break;
      case "-":
      case "_":
        store.zoom = Math.max(2, store.zoom - 4);
        break;
      case "?":
      case "/":
        if (e.shiftKey || e.key === "?") {
          store.showShortcuts = !store.showShortcuts;
        }
        break;
    }
  };
  window.addEventListener("keydown", handler);
  return () => window.removeEventListener("keydown", handler);
}

async function runCode() {
  store.reset();
  try {
    await ipc.startRun(store.code);
  } catch (e) {
    store.rawStderr = [...store.rawStderr, `start_run failed: ${e}`];
  }
}
