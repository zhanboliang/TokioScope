import { aggregate, type Frame, type SimResult } from "./engine";
import type { ParseReport, RunnerEvent, RunnerStatus } from "./ipc";

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

  // Live event buffer
  events = $state<RunnerEvent[]>([]);
  sim = $state<SimResult | null>(null);

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
    const idx = Math.min(this.playhead, this.sim.frames.length - 1);
    return this.sim.frames[idx];
  }

  get totalTicks(): number {
    return this.sim?.frames.length ?? 0;
  }

  reset() {
    this.events = [];
    this.sim = null;
    this.playhead = 0;
    this.playing = false;
    this.rawStdout = [];
    this.rawStderr = [];
  }

  ingest(ev: RunnerEvent) {
    this.events.push(ev);
    // Re-aggregate at tick boundary for cheap incremental visualization.
    if (ev.kind === "tick" || ev.kind === "finish") {
      this.sim = aggregate(this.events);
      if (this.follow && this.sim.frames.length > 0) {
        this.playhead = this.sim.frames.length - 1;
      }
    }
  }
}

export const store = new Store();
