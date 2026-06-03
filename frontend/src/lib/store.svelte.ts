import { Aggregator, type Frame, type SimResult } from "./engine";
import type { ParseReport, RunnerEvent, RunnerStatus } from "./ipc";
import { t } from "./i18n.svelte";

// Bound runaway programs (e.g. infinite loops) so the UI stays responsive.
const MAX_FRAMES = 1500;   // tick-frames kept; once hit, collection stops + run is cancelled
const MAX_OUTPUT = 1000;   // stdout / stderr scrollback lines

// Bundled default families (both SIL OFL): JetBrains Mono for code / terminal /
// output, IBM Plex Sans (proportional) for the UI. Overridable in Settings.
export const DEFAULT_FONT = "JetBrains Mono";
export const DEFAULT_UI_FONT = "IBM Plex Sans";

class Store {
  analysis = $state<ParseReport | null>(null);
  // Identity of the buffer whose source produced the current sim — editor
  // run-line decorations only apply when the active tab matches this. For a
  // project run it's the entry file path; otherwise it's the active tab's id.
  runFilePath = $state<string | null>(null);
  // True when the current run targeted a whole Cargo project (drives the
  // best-effort "experimental tracing" banner).
  isProjectRun = $state(false);
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

  // Layout (editor/tree/bottom-panel layout lives in workspace.svelte.ts)
  theme = $state<"dark" | "light" | "hc">("dark");
  timelineHeight = $state(260);   // px; resizable inside viz column

  // Four font areas — code editor, terminal, output log, and UI chrome — each a
  // family + size (VSCode-style). Default family is the bundled JetBrains Mono;
  // the user can replace any with a system font name (it falls back to the stack).
  editorFont = $state(DEFAULT_FONT);
  editorFontSize = $state(13);
  terminalFont = $state(DEFAULT_FONT);
  terminalFontSize = $state(12);
  outputFont = $state(DEFAULT_FONT);
  outputFontSize = $state(11);
  uiFont = $state(DEFAULT_UI_FONT);
  uiFontSize = $state(13);
  zoom = $state(20); // pixels per tick
  panTick = $state(0);
  inlineEdit = $state(false);
  showShortcuts = $state(false);
  showSettings = $state(false);

  // Stdout / stderr feed (raw, in addition to engine printlns)
  rawStdout = $state<{ tick: number; text: string }[]>([]);
  rawStderr = $state<string[]>([]);

  get currentFrame(): Frame | null {
    const frames = this.sim?.frames;
    if (!frames || frames.length === 0) return null;
    // playhead is a TICK on the time axis. The virtual clock can skip ticks
    // (sleeps fast-forward it), so frames are sparse — find the last frame whose
    // tick is <= the floored playhead instead of indexing by array position.
    const tick = Math.floor(this.playhead);
    let lo = 0, hi = frames.length - 1, ans = 0;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (frames[mid].tick <= tick) { ans = mid; lo = mid + 1; }
      else hi = mid - 1;
    }
    return frames[ans];
  }

  // The time axis spans tick 0 .. last-frame's tick (NOT the frame count, which
  // is smaller whenever the virtual clock skips ticks).
  get totalTicks(): number {
    const frames = this.sim?.frames;
    return frames && frames.length ? frames[frames.length - 1].tick + 1 : 0;
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
          this.#append(this.rawStderr, [t("cap.notice", { n: MAX_FRAMES })], (v) => (this.rawStderr = v));
          break;
        }
      }
      this.sim = this.#agg.result();
      if (this.follow && this.sim.frames.length > 0) {
        this.playhead = this.sim.frames[this.sim.frames.length - 1].tick;
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
