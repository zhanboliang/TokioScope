// Lightweight i18n: a reactive `locale` + flat zh/en dictionary + `t(key)`.
// Reading t()/locale() inside a component's template/$derived makes it reactive,
// so switching language re-renders the UI. No external dependency.

export type Lang = "zh" | "en";

export const LANGS: { id: Lang; label: string }[] = [
  { id: "zh", label: "中文" },
  { id: "en", label: "English" },
];

const dict: Record<Lang, Record<string, string>> = {
  zh: {
    // titlebar
    "run.title": "运行 (⌘+Enter)",
    "rerun.title": "重新运行 (⌘+Enter)",
    "stop.title": "取消 (⌘+.)",
    "pane.source": "源代码",
    // output
    "output.header": "输出",
    "output.empty": "(println! / stdout 在此显示)",
    // menu
    "menu.settings": "设置",
    "menu.theme": "主题",
    "theme.dark": "深色",
    "theme.light": "浅色",
    "theme.hc": "高对比",
    "menu.playback": "播放",
    "menu.follow": "跟随播放头",
    "menu.loop": "循环播放",
    "menu.shortcuts": "键盘快捷键",
    "menu.reset": "重置布局",
    "menu.about": "关于 TokioScope",
    "menu.language": "语言",
    "about.tagline": "看 Tokio 如何调度你的异步代码，一格一格。",
    // playback bar
    "pb.toStart": "回到开始 (Home)",
    "pb.stepBack": "上一步 (←)",
    "pb.playPause": "播放 / 暂停 (Space)",
    "pb.stepFwd": "下一步 (→)",
    "pb.toEnd": "跳到结束 (End)",
    "pb.restart": "重置 (R)",
    "pb.speed": "速度",
    "pb.loop": "循环",
    "pb.follow": "跟随",
    // status bar
    "status.building": "正在编译 runner crate…",
    "status.error": "runner 错误",
    "status.running": "运行中",
    "status.ready": "就绪",
    "status.init": "初始化…",
    // scheduler stage
    "zone.ready": "就绪队列",
    "zone.cpu": "CPU · {n} 核",
    "zone.await": "等待 / 计时",
    "zone.blocking": "阻塞池",
    "zone.done": "已完成",
    "stage.idle": "空闲",
    "stage.empty": "空",
    "await.timer": "⏱ 计时器",
    "await.children": "⏳ 子任务",
    "await.blocking": "🧵 阻塞",
    "await.generic": "… 挂起",
    "done.tooltip": "{name}：第 {born} → {done} tick，存活 {life} ticks",
    // timeline / minimap
    "tl.waiting": "等待运行…",
    "tl.legend": "颜色图例",
    "minimap.map": "概览",
    "minimap.noData": "尚无运行数据",
    // task states (legend labels + descriptions)
    "state.ready": "就绪",
    "state.running": "运行",
    "state.awaiting": "等待",
    "state.blocking": "阻塞",
    "state.done": "完成",
    "state.ready.desc": "可运行，在队列里等待空闲 worker",
    "state.running.desc": "正在某个 worker 上执行",
    "state.awaiting.desc": "在 .await 处挂起（计时器 / 子任务等）",
    "state.blocking.desc": "占用阻塞线程池（spawn_blocking）",
    "state.done.desc": "任务已结束",
    // editor
    "editor.placeholder": "// 在此编写你的 Tokio async 代码…（⌘/Ctrl + 1/2/3 载入示例）",
    "cap.notice": "⚠ 已达可视化上限 {n} 帧，停止采集并自动终止运行",
    // shortcuts panel
    "sc.title": "快捷键",
    "sc.footer": "切换此面板 · Esc 关闭",
    "sc.group.playback": "播放控制",
    "sc.group.timeline": "时间线",
    "sc.group.code": "代码 & 运行",
    "sc.playPause": "播放 / 暂停",
    "sc.step": "单步前进 / 后退（一帧）",
    "sc.fine": "精细移动播放头",
    "sc.reset": "重置（回到 tick 0）",
    "sc.homeEnd": "跳到开始 / 结束",
    "sc.toggleLoop": "切换循环播放",
    "sc.zoom": "缩放",
    "sc.zoomAnchor": "鼠标锚点缩放",
    "sc.pan": "平移",
    "sc.fit": "自适应",
    "sc.follow": "跟随播放头",
    "sc.run": "运行",
    "sc.cancel": "取消运行",
    "sc.inline": "切换行内编辑模式",
    "sc.examples": "加载示例 1 / 2 / 3",
    "sc.help": "切换快捷键速查表",
    "key.wheel": "Ctrl + 滚轮",
    "key.shiftDrag": "Shift + 拖拽",
    "key.midDrag": "中键拖拽",
    "key.dblClick": "双击",
  },
  en: {
    "run.title": "Run (⌘+Enter)",
    "rerun.title": "Rerun (⌘+Enter)",
    "stop.title": "Stop (⌘+.)",
    "pane.source": "source",
    "output.header": "output",
    "output.empty": "(println! / stdout shows here)",
    "menu.settings": "Settings",
    "menu.theme": "Theme",
    "theme.dark": "Dark",
    "theme.light": "Light",
    "theme.hc": "High-Contrast",
    "menu.playback": "Playback",
    "menu.follow": "Follow playhead",
    "menu.loop": "Loop",
    "menu.shortcuts": "Keyboard shortcuts",
    "menu.reset": "Reset layout",
    "menu.about": "About TokioScope",
    "menu.language": "Language",
    "about.tagline": "See how Tokio schedules your async code, tick by tick.",
    "pb.toStart": "To start (Home)",
    "pb.stepBack": "Step back (←)",
    "pb.playPause": "Play / Pause (Space)",
    "pb.stepFwd": "Step forward (→)",
    "pb.toEnd": "To end (End)",
    "pb.restart": "Reset (R)",
    "pb.speed": "speed",
    "pb.loop": "loop",
    "pb.follow": "follow",
    "status.building": "building runner crate…",
    "status.error": "runner error",
    "status.running": "running",
    "status.ready": "ready",
    "status.init": "init…",
    "zone.ready": "Ready queue",
    "zone.cpu": "CPU · {n} cores",
    "zone.await": "Awaiting / timers",
    "zone.blocking": "Blocking pool",
    "zone.done": "Done",
    "stage.idle": "idle",
    "stage.empty": "empty",
    "await.timer": "⏱ timer",
    "await.children": "⏳ children",
    "await.blocking": "🧵 blocking",
    "await.generic": "… parked",
    "done.tooltip": "{name}: tick {born} → {done}, lived {life} ticks",
    "tl.waiting": "waiting for run…",
    "tl.legend": "Color legend",
    "minimap.map": "map",
    "minimap.noData": "no run data yet",
    "state.ready": "Ready",
    "state.running": "Running",
    "state.awaiting": "Awaiting",
    "state.blocking": "Blocking",
    "state.done": "Done",
    "state.ready.desc": "runnable — queued, waiting for a free worker",
    "state.running.desc": "executing on a worker",
    "state.awaiting.desc": "parked at .await (timer / children / etc.)",
    "state.blocking.desc": "holding a blocking-pool slot (spawn_blocking)",
    "state.done.desc": "task has finished",
    "editor.placeholder": "// write your Tokio async code here…  (⌘/Ctrl + 1/2/3 to load an example)",
    "cap.notice": "⚠ Reached the {n}-frame visualization limit — stopped collecting and cancelled the run",
    "sc.title": "Shortcuts",
    "sc.footer": "toggle this panel · Esc to close",
    "sc.group.playback": "Playback",
    "sc.group.timeline": "Timeline",
    "sc.group.code": "Code & run",
    "sc.playPause": "Play / Pause",
    "sc.step": "Step forward / back (one frame)",
    "sc.fine": "Fine-move the playhead",
    "sc.reset": "Reset (to tick 0)",
    "sc.homeEnd": "Jump to start / end",
    "sc.toggleLoop": "Toggle loop",
    "sc.zoom": "Zoom",
    "sc.zoomAnchor": "Cursor-anchored zoom",
    "sc.pan": "Pan",
    "sc.fit": "Fit to content",
    "sc.follow": "Follow playhead",
    "sc.run": "Run",
    "sc.cancel": "Cancel run",
    "sc.inline": "Toggle inline edit",
    "sc.examples": "Load example 1 / 2 / 3",
    "sc.help": "Toggle shortcuts cheatsheet",
    "key.wheel": "Ctrl + wheel",
    "key.shiftDrag": "Shift + drag",
    "key.midDrag": "Middle-drag",
    "key.dblClick": "Double-click",
  },
};

function detect(): Lang {
  try {
    return navigator.language?.toLowerCase().startsWith("zh") ? "zh" : "en";
  } catch {
    return "en";
  }
}

let _lang = $state<Lang>(detect());

/** Reactive current language. */
export function locale(): Lang {
  return _lang;
}
export function setLocale(l: Lang) {
  _lang = l;
}

/** Translate a key, with optional `{name}` interpolation. Reactive on locale(). */
export function t(key: string, params?: Record<string, string | number>): string {
  let s = dict[_lang][key] ?? dict.en[key] ?? key;
  if (params) for (const [k, v] of Object.entries(params)) s = s.replaceAll(`{${k}}`, String(v));
  return s;
}
