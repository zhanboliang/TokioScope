import type { TaskState } from "./engine";

export const STATE_COLOR: Record<TaskState, string> = {
  ready: "var(--ts-st-ready)",
  running: "var(--ts-st-running)",
  awaiting: "var(--ts-st-awaiting)",
  blocking: "var(--ts-st-blocking)",
  done: "var(--ts-st-done)",
};

export const STATE_LABEL: Record<TaskState, string> = {
  ready: "Ready",
  running: "Running",
  awaiting: "Awaiting",
  blocking: "Blocking",
  done: "Done",
};

export const STATE_LABEL_ZH: Record<TaskState, string> = {
  ready: "就绪",
  running: "运行",
  awaiting: "等待",
  blocking: "阻塞",
  done: "完成",
};

export const STATE_DESC_ZH: Record<TaskState, string> = {
  ready: "可运行，在队列里等待空闲 worker",
  running: "正在某个 worker 上执行",
  awaiting: "在 .await 处挂起（计时器 / 子任务等）",
  blocking: "占用阻塞线程池（spawn_blocking）",
  done: "任务已结束",
};

export function readCssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}
