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

export function readCssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}
