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

// ── Workspace / filesystem ─────────────────────────────────────────────
export interface DirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  is_symlink: boolean;
}
export interface FileRead {
  text: string;
  too_large: boolean;
  binary: boolean;
  byte_len: number;
}
export interface WorkspaceInfo {
  root: string;
  name: string;
  is_cargo: boolean;
}

export interface StartRunArgs {
  source?: string;
  runtime?: RuntimeConfig | null;
  projectRoot?: string;
}

export const ipc = {
  listExamples: () => invoke<Example[]>("list_examples"),
  analyzeCode: (source: string) => invoke<ParseReport>("analyze_code", { source }),
  startRun: (args: StartRunArgs) =>
    invoke<void>("start_run", {
      args: {
        source: args.source ?? null,
        runtime: args.runtime ?? null,
        project_root: args.projectRoot ?? null,
      },
    }),
  cancelRun: () => invoke<void>("cancel_run"),
  ensureRunner: () => invoke<void>("ensure_runner"),
  runnerStatus: () => invoke<RunnerStatus>("runner_status"),

  // filesystem
  readDir: (path: string) => invoke<DirEntry[]>("read_dir", { path }),
  readTextFile: (path: string) => invoke<FileRead>("read_text_file", { path }),
  writeTextFile: (path: string, contents: string) =>
    invoke<void>("write_text_file", { path, contents }),
  workspaceInfo: (path: string) => invoke<WorkspaceInfo>("workspace_info", { path }),
  createProject: (parent: string, name: string) =>
    invoke<string>("create_project", { parent, name }),

  // pty
  ptySpawn: (cwd: string | null, cols: number, rows: number, shell?: string) =>
    invoke<number>("pty_spawn", { cwd, cols, rows, shell: shell ?? null }),
  ptyWrite: (id: number, data: string) => invoke<void>("pty_write", { id, data }),
  ptyResize: (id: number, cols: number, rows: number) =>
    invoke<void>("pty_resize", { id, cols, rows }),
  ptyKill: (id: number) => invoke<void>("pty_kill", { id }),

  // cargo.toml
};

export async function onPty(handler: (id: number, data: Uint8Array) => void): Promise<UnlistenFn> {
  return await listen<{ id: number; data: number[] }>("ts:pty", (m) =>
    handler(m.payload.id, new Uint8Array(m.payload.data)),
  );
}
export async function onPtyExit(handler: (id: number, code: number | null) => void): Promise<UnlistenFn> {
  return await listen<{ id: number; code: number | null }>("ts:pty-exit", (m) =>
    handler(m.payload.id, m.payload.code),
  );
}

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
