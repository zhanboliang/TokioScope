import type { RunnerEvent } from "./ipc";

export type TaskState = "ready" | "running" | "awaiting" | "blocking" | "done";

export interface TaskSnapshot {
  id: number;
  name: string;
  parent: number | null;
  state: TaskState;
  currentLine: number;
  spawnLine: number;
  eventCount: number;
  bornTick: number;
  doneTick: number | null;
  blocking: boolean;
  durationTicks: number;
  awaitReason?: string;
}

export interface WakeEntry {
  tick: number;
  taskId: number;
  cause: string;
}

export interface PrintlnEntry {
  tick: number;
  taskId: number;
  line: number;
  text: string;
}

export interface BlockingSlot {
  taskId: number | null;
  startTick: number;
  durationTicks: number;
}

export interface Frame {
  tick: number;
  workers: (number | null)[];
  blocking: BlockingSlot[];
  ready: number[];
  tasks: Map<number, TaskSnapshot>;
  wakeLog: WakeEntry[];
  printlns: PrintlnEntry[];
}

export interface SimResult {
  flavor: string;
  workerCount: number;
  blockingSlots: number;
  frames: Frame[];
  printlns: PrintlnEntry[];
}

/**
 * Walk the raw event stream and produce one Frame per tick boundary.
 * Frames carry the entire visualisation state, so playback / scrubbing is O(1).
 */
export function aggregate(events: RunnerEvent[]): SimResult {
  let flavor = "multi_thread";
  let workerCount = 4;
  let blockingSlots = 8;

  const tasks = new Map<number, TaskSnapshot>();
  const ready: number[] = [];
  const wakeLog: WakeEntry[] = [];
  const printlns: PrintlnEntry[] = [];
  const blockingByTask = new Map<number, BlockingSlot>();
  const blocking: BlockingSlot[] = [];
  const frames: Frame[] = [];

  const ensureBlocking = () => {
    if (blocking.length === 0)
      for (let i = 0; i < blockingSlots; i++)
        blocking.push({ taskId: null, startTick: 0, durationTicks: 0 });
  };

  const setState = (id: number, s: TaskState, line?: number) => {
    const t = tasks.get(id);
    if (!t) return;
    t.state = s;
    if (line !== undefined) t.currentLine = line;
    t.eventCount += 1;
  };

  const pushReady = (id: number) => {
    if (!ready.includes(id)) ready.push(id);
  };
  const removeReady = (id: number) => {
    const i = ready.indexOf(id);
    if (i >= 0) ready.splice(i, 1);
  };

  const workersFrom = (running: number[]): (number | null)[] => {
    const arr: (number | null)[] = new Array(workerCount).fill(null);
    for (const id of running) {
      const w = id % Math.max(1, workerCount);
      // place at chosen slot or first free
      if (arr[w] == null) arr[w] = id;
      else {
        const free = arr.findIndex((v) => v == null);
        if (free >= 0) arr[free] = id;
      }
    }
    return arr;
  };

  for (const ev of events) {
    switch (ev.kind) {
      case "boot":
        flavor = ev.flavor;
        workerCount = Math.max(1, ev.worker_threads);
        blockingSlots = ev.blocking_slots;
        ensureBlocking();
        break;
      case "task_spawn":
        tasks.set(ev.id, {
          id: ev.id,
          name: ev.name,
          parent: ev.parent,
          state: "ready",
          currentLine: ev.line,
          spawnLine: ev.line,
          eventCount: 1,
          bornTick: ev.tick,
          doneTick: null,
          blocking: ev.blocking,
          durationTicks: 0,
        });
        pushReady(ev.id);
        break;
      case "task_poll":
        setState(ev.id, "running", ev.line);
        removeReady(ev.id);
        break;
      case "task_yield":
        setState(ev.id, "ready", ev.line);
        pushReady(ev.id);
        break;
      case "task_await": {
        const reason = ev.reason;
        const next: TaskState = reason === "blocking" ? "blocking" : "awaiting";
        setState(ev.id, next, ev.line);
        const t = tasks.get(ev.id);
        if (t) t.awaitReason = reason;
        removeReady(ev.id);
        break;
      }
      case "task_wake":
        setState(ev.id, "ready");
        pushReady(ev.id);
        wakeLog.push({ tick: ev.tick, taskId: ev.id, cause: ev.cause });
        if (wakeLog.length > 16) wakeLog.shift();
        break;
      case "blocking_start": {
        ensureBlocking();
        const idx = ev.slot < blocking.length ? ev.slot : blocking.findIndex((s) => s.taskId == null);
        if (idx >= 0) {
          blocking[idx] = { taskId: ev.id, startTick: ev.tick, durationTicks: ev.duration_ticks };
          blockingByTask.set(ev.id, blocking[idx]);
        }
        setState(ev.id, "blocking");
        break;
      }
      case "blocking_done": {
        ensureBlocking();
        const idx = ev.slot < blocking.length ? ev.slot : -1;
        if (idx >= 0) {
          blocking[idx] = { taskId: null, startTick: 0, durationTicks: 0 };
        }
        blockingByTask.delete(ev.id);
        setState(ev.id, "ready");
        pushReady(ev.id);
        break;
      }
      case "timer_fire":
        setState(ev.id, "ready");
        pushReady(ev.id);
        wakeLog.push({ tick: ev.tick, taskId: ev.id, cause: "timer_fire" });
        if (wakeLog.length > 16) wakeLog.shift();
        break;
      case "task_done": {
        const t = tasks.get(ev.id);
        if (t) {
          t.state = "done";
          t.doneTick = ev.tick;
          t.durationTicks = ev.tick - t.bornTick;
        }
        removeReady(ev.id);
        break;
      }
      case "println":
        printlns.push({ tick: ev.tick, taskId: ev.id, line: ev.line, text: ev.text });
        break;
      case "tick": {
        const running: number[] = [];
        for (const t of tasks.values()) if (t.state === "running") running.push(t.id);
        const frame: Frame = {
          tick: ev.tick,
          workers: workersFrom(running),
          blocking: blocking.map((s) => ({ ...s })),
          ready: ready.slice(),
          tasks: new Map([...tasks].map(([k, v]) => [k, { ...v }])),
          wakeLog: wakeLog.slice(),
          printlns: printlns.slice(),
        };
        frames.push(frame);
        break;
      }
      case "finish":
        // Tail frame already emitted by preceding tick event.
        break;
    }
  }

  return { flavor, workerCount, blockingSlots, frames, printlns };
}
