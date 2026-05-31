import { store } from "./store.svelte";
import { ipc } from "./ipc";

export interface ShortcutGroup {
  group: string;
  items: { keys: string; action: string }[];
}

export const SHORTCUTS: ShortcutGroup[] = [
  {
    group: "播放控制",
    items: [
      { keys: "Space", action: "播放 / 暂停" },
      { keys: "→ / ←", action: "单步前进 / 后退（一帧）" },
      { keys: "Shift + → / ←", action: "精细移动播放头" },
      { keys: "R", action: "重置(回到 tick 0)" },
      { keys: "Home / End", action: "跳到开始 / 结束" },
      { keys: "L", action: "切换循环播放" },
    ],
  },
  {
    group: "时间线",
    items: [
      { keys: "+ / -", action: "缩放" },
      { keys: "Ctrl + 滚轮", action: "鼠标锚点缩放" },
      { keys: "Shift + 拖拽", action: "平移" },
      { keys: "中键拖拽", action: "平移" },
      { keys: "双击", action: "自适应" },
      { keys: "F", action: "跟随播放头" },
    ],
  },
  {
    group: "代码 & 运行",
    items: [
      { keys: "⌘/Ctrl + Enter", action: "运行" },
      { keys: "⌘/Ctrl + .", action: "取消运行" },
      { keys: "⌘/Ctrl + E", action: "切换行内编辑模式" },
      { keys: "⌘/Ctrl + 1/2/3", action: "加载示例 1 / 2 / 3" },
      { keys: "?", action: "切换快捷键速查表" },
    ],
  },
];

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
