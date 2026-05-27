import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type RunnerEvent =
  | { kind: "boot"; flavor: string; worker_threads: number; blocking_slots: number }
  | { kind: "task_spawn"; tick: number; id: number; parent: number | null; line: number; name: string; blocking: boolean }
  | { kind: "task_poll"; tick: number; id: number; worker: number; line: number }
  | { kind: "task_yield"; tick: number; id: number; line: number }
  | { kind: "task_await"; tick: number; id: number; line: number; reason: "timer" | "blocking" | "children" | "generic" }
  | { kind: "task_wake"; tick: number; id: number; cause: string }
  | { kind: "blocking_start"; tick: number; id: number; slot: number; duration_ticks: number }
  | { kind: "blocking_done"; tick: number; id: number; slot: number }
  | { kind: "timer_fire"; tick: number; id: number }
  | { kind: "task_done"; tick: number; id: number }
  | { kind: "println"; tick: number; id: number; line: number; text: string }
  | { kind: "tick"; tick: number }
  | { kind: "finish"; tick: number };

export type Flavor = "current_thread" | "multi_thread";

export interface RuntimeConfig {
  flavor: Flavor;
  worker_threads: number;
  blocking_slots: number;
}

export interface ParseReport {
  ok: boolean;
  runtime: RuntimeConfig;
  diagnostics: { level: "error" | "warning" | "info"; line: number; col: number; message: string }[];
  primitives: { kind: string; line: number }[];
  functions: { name: string; line: number; is_async: boolean }[];
}

export interface Example {
  id: string;
  title: string;
  blurb: string;
  code: string;
}

export interface RunnerStatus {
  ready: boolean;
  building: boolean;
  running: boolean;
  cache_dir: string | null;
  last_error: string | null;
}

export const ipc = {
  listExamples: () => invoke<Example[]>("list_examples"),
  analyzeCode: (source: string) => invoke<ParseReport>("analyze_code", { source }),
  startRun: (source: string, runtime?: RuntimeConfig) =>
    invoke<void>("start_run", { args: { source, runtime: runtime ?? null } }),
  cancelRun: () => invoke<void>("cancel_run"),
  ensureRunner: () => invoke<void>("ensure_runner"),
  runnerStatus: () => invoke<RunnerStatus>("runner_status"),
};

export async function onEvent(handler: (e: RunnerEvent) => void): Promise<UnlistenFn> {
  return await listen<RunnerEvent>("ts:event", (msg) => handler(msg.payload));
}

export async function onStdout(handler: (text: string) => void): Promise<UnlistenFn> {
  return await listen<string>("ts:stdout", (m) => handler(m.payload));
}
export async function onStderr(handler: (text: string) => void): Promise<UnlistenFn> {
  return await listen<string>("ts:stderr", (m) => handler(m.payload));
}
export async function onDone(handler: (code: number | null) => void): Promise<UnlistenFn> {
  return await listen<number | null>("ts:done", (m) => handler(m.payload));
}
export async function onStatus(handler: (s: RunnerStatus) => void): Promise<UnlistenFn> {
  return await listen<RunnerStatus>("ts:status", (m) => handler(m.payload));
}
