import { Aggregator, type Frame, type SimResult } from "./engine";
import type { ParseReport, RunnerEvent, RunnerStatus } from "./ipc";

// Bound runaway programs (e.g. infinite loops) so the UI stays responsive.
const MAX_FRAMES = 1500;   // tick-frames kept; once hit, collection stops + run is cancelled
const MAX_OUTPUT = 1000;   // stdout / stderr scrollback lines

const DEFAULT_CODE = `use std::time::Duration;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    println!("hello tokioscope");
    let h = tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        println!("inner");
    });
    let _ = h.await;
}
`;

class Store {
  code = $state(DEFAULT_CODE);
  analysis = $state<ParseReport | null>(null);
  status = $state<RunnerStatus>({
    ready: false,
    building: false,
    running: false,
    cache_dir: null,
    last_error: null,
  });

  // Live simulation (raw — not deeply proxied; the frames array is large and is
  // reassigned wholesale on each drain to trigger reactivity).
  sim = $state.raw<SimResult | null>(null);
  capped = $state(false);

  // Incremental builder + buffers drained once per animation frame (see drain()).
  #agg = new Aggregator();
  #pendEv: RunnerEvent[] = [];
  #pendOut: { tick: number; text: string }[] = [];
  #pendErr: string[] = [];

  // Playback
  playhead = $state(0);
  playing = $state(false);
  speed = $state(4);
  loop_ = $state(false);
  follow = $state(true);

  // Layout
  theme = $state<"dark" | "light" | "hc">("dark");
  codePaneWidth = $state(420);
  outputHeight = $state(140);     // px; resizable inside code pane
  timelineHeight = $state(260);   // px; resizable inside viz column
  zoom = $state(20); // pixels per tick
  panTick = $state(0);
  inlineEdit = $state(false);
  showShortcuts = $state(false);

  // Stdout / stderr feed (raw, in addition to engine printlns)
  rawStdout = $state<{ tick: number; text: string }[]>([]);
  rawStderr = $state<string[]>([]);

  get currentFrame(): Frame | null {
    if (!this.sim || this.sim.frames.length === 0) return null;
    // playhead is fractional during playback — floor to the current tick so the
    // pools / editor show a real frame instead of frames[1.7] === undefined.
    const idx = Math.max(0, Math.min(Math.floor(this.playhead), this.sim.frames.length - 1));
    return this.sim.frames[idx];
  }

  get totalTicks(): number {
    return this.sim?.frames.length ?? 0;
  }

  reset() {
    this.#agg = new Aggregator();
    this.#pendEv = [];
    this.#pendOut = [];
    this.#pendErr = [];
    this.sim = null;
    this.capped = false;
    this.playhead = 0;
    this.playing = false;
    this.rawStdout = [];
    this.rawStderr = [];
  }

  // ── Buffered ingestion ─────────────────────────────────────────────
  // Listeners call these (cheap pushes only) so the main thread keeps
  // returning to the event loop even under a flood of events — which is
  // what keeps the Stop button clickable. drain() does the real work,
  // throttled to one call per animation frame.

  bufferEvent(ev: RunnerEvent) {
    if (!this.capped) this.#pendEv.push(ev);
  }
  bufferStdout(text: string) {
    this.#pendOut.push({ tick: this.sim?.frames.length ?? 0, text });
  }
  bufferStderr(text: string) {
    this.#pendErr.push(text);
  }

  /** Process all buffered output. Returns true if the frame cap was just hit
   *  (the caller should then cancel the run). */
  drain(): boolean {
    let justCapped = false;
    if (this.#pendEv.length && !this.capped) {
      const evs = this.#pendEv;
      this.#pendEv = [];
      for (const ev of evs) {
        this.#agg.push(ev);
        if (this.#agg.frames.length >= MAX_FRAMES) {
          this.capped = true;
          justCapped = true;
          this.#pendEv = [];
          this.#append(this.rawStderr, [`⚠ 已达可视化上限 ${MAX_FRAMES} 帧，停止采集并自动终止运行`], (v) => (this.rawStderr = v));
          break;
        }
      }
      this.sim = this.#agg.result();
      if (this.follow && this.sim.frames.length > 0) {
        this.playhead = this.sim.frames.length - 1;
      }
    }
    if (this.#pendOut.length) {
      const b = this.#pendOut;
      this.#pendOut = [];
      this.#append(this.rawStdout, b, (v) => (this.rawStdout = v));
    }
    if (this.#pendErr.length) {
      const b = this.#pendErr;
      this.#pendErr = [];
      this.#append(this.rawStderr, b, (v) => (this.rawStderr = v));
    }
    return justCapped;
  }

  #append<T>(cur: T[], batch: T[], set: (next: T[]) => void) {
    if (!batch.length) return;
    const next = cur.concat(batch);
    set(next.length > MAX_OUTPUT ? next.slice(next.length - MAX_OUTPUT) : next);
  }
}

export const store = new Store();
