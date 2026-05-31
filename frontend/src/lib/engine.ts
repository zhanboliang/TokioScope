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
 * Incremental simulation builder. Each event is processed exactly once via
 * `push`, appending one Frame per tick boundary — so streaming N events is O(N)
 * rather than the O(N²) of re-walking the whole log on every tick. Frames carry
 * the entire visualisation state, so playback / scrubbing stays O(1).
 */
export class Aggregator {
  flavor = "multi_thread";
  workerCount = 4;
  blockingSlots = 8;

  private tasks = new Map<number, TaskSnapshot>();
  private ready: number[] = [];
  private wakeLog: WakeEntry[] = [];
  private printlns: PrintlnEntry[] = [];
  private blockingByTask = new Map<number, BlockingSlot>();
  private blocking: BlockingSlot[] = [];
  frames: Frame[] = [];

  private ensureBlocking() {
    if (this.blocking.length === 0)
      for (let i = 0; i < this.blockingSlots; i++)
        this.blocking.push({ taskId: null, startTick: 0, durationTicks: 0 });
  }

  private setState(id: number, s: TaskState, line?: number) {
    const t = this.tasks.get(id);
    if (!t) return;
    t.state = s;
    if (line !== undefined) t.currentLine = line;
    t.eventCount += 1;
  }

  private pushReady(id: number) {
    if (!this.ready.includes(id)) this.ready.push(id);
  }
  private removeReady(id: number) {
    const i = this.ready.indexOf(id);
    if (i >= 0) this.ready.splice(i, 1);
  }

  private workersFrom(running: number[]): (number | null)[] {
    const arr: (number | null)[] = new Array(this.workerCount).fill(null);
    for (const id of running) {
      const w = id % Math.max(1, this.workerCount);
      // place at chosen slot or first free
      if (arr[w] == null) arr[w] = id;
      else {
        const free = arr.findIndex((v) => v == null);
        if (free >= 0) arr[free] = id;
      }
    }
    return arr;
  }

  push(ev: RunnerEvent) {
    switch (ev.kind) {
      case "boot":
        this.flavor = ev.flavor;
        this.workerCount = Math.max(1, ev.worker_threads);
        this.blockingSlots = ev.blocking_slots;
        this.ensureBlocking();
        break;
      case "task_spawn":
        this.tasks.set(ev.id, {
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
        this.pushReady(ev.id);
        break;
      case "task_poll":
        this.setState(ev.id, "running", ev.line);
        this.removeReady(ev.id);
        break;
      case "task_yield":
        this.setState(ev.id, "ready", ev.line);
        this.pushReady(ev.id);
        break;
      case "task_await": {
        const reason = ev.reason;
        const next: TaskState = reason === "blocking" ? "blocking" : "awaiting";
        this.setState(ev.id, next, ev.line);
        const t = this.tasks.get(ev.id);
        if (t) t.awaitReason = reason;
        this.removeReady(ev.id);
        break;
      }
      case "task_wake":
        this.setState(ev.id, "ready");
        this.pushReady(ev.id);
        this.wakeLog.push({ tick: ev.tick, taskId: ev.id, cause: ev.cause });
        if (this.wakeLog.length > 16) this.wakeLog.shift();
        break;
      case "blocking_start": {
        this.ensureBlocking();
        const idx = ev.slot < this.blocking.length ? ev.slot : this.blocking.findIndex((s) => s.taskId == null);
        if (idx >= 0) {
          this.blocking[idx] = { taskId: ev.id, startTick: ev.tick, durationTicks: ev.duration_ticks };
          this.blockingByTask.set(ev.id, this.blocking[idx]);
        }
        this.setState(ev.id, "blocking");
        break;
      }
      case "blocking_done": {
        this.ensureBlocking();
        const idx = ev.slot < this.blocking.length ? ev.slot : -1;
        if (idx >= 0) {
          this.blocking[idx] = { taskId: null, startTick: 0, durationTicks: 0 };
        }
        this.blockingByTask.delete(ev.id);
        this.setState(ev.id, "ready");
        this.pushReady(ev.id);
        break;
      }
      case "timer_fire":
        this.setState(ev.id, "ready");
        this.pushReady(ev.id);
        this.wakeLog.push({ tick: ev.tick, taskId: ev.id, cause: "timer_fire" });
        if (this.wakeLog.length > 16) this.wakeLog.shift();
        break;
      case "task_done": {
        const t = this.tasks.get(ev.id);
        if (t) {
          t.state = "done";
          t.doneTick = ev.tick;
          t.durationTicks = ev.tick - t.bornTick;
        }
        this.removeReady(ev.id);
        break;
      }
      case "println":
        this.printlns.push({ tick: ev.tick, taskId: ev.id, line: ev.line, text: ev.text });
        break;
      case "tick": {
        const running: number[] = [];
        for (const t of this.tasks.values()) if (t.state === "running") running.push(t.id);
        this.frames.push({
          tick: ev.tick,
          workers: this.workersFrom(running),
          blocking: this.blocking.map((s) => ({ ...s })),
          ready: this.ready.slice(),
          tasks: new Map([...this.tasks].map(([k, v]) => [k, { ...v }])),
          wakeLog: this.wakeLog.slice(),
          printlns: this.printlns.slice(),
        });
        break;
      }
      case "finish":
        // Tail frame already emitted by preceding tick event.
        break;
    }
  }

  result(): SimResult {
    return {
      flavor: this.flavor,
      workerCount: this.workerCount,
      blockingSlots: this.blockingSlots,
      frames: this.frames,
      printlns: this.printlns,
    };
  }
}

/** One-shot convenience wrapper (used by tests / non-streaming callers). */
export function aggregate(events: RunnerEvent[]): SimResult {
  const agg = new Aggregator();
  for (const ev of events) agg.push(ev);
  return agg.result();
}
