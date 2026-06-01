<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { EditorState, Compartment, RangeSet } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, highlightActiveLine, drawSelection, Decoration, gutters, gutter, GutterMarker, ViewPlugin, placeholder, type ViewUpdate, type DecorationSet } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { syntaxHighlighting, HighlightStyle, indentUnit } from "@codemirror/language";
  import { rust } from "@codemirror/lang-rust";
  import { tags as t } from "@lezer/highlight";
  import { StateField, StateEffect } from "@codemirror/state";
  import { autocompletion, completionKeymap, snippetCompletion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
  import { linter, lintGutter, forceLinting, type Diagnostic } from "@codemirror/lint";
  import { store } from "../store.svelte";
  import { t as tr } from "../i18n.svelte"; // aliased — `t` is the lezer tags import

  // Syntax theme mapped to the IDEA-Darcula design tokens. Colours are CSS vars,
  // so the highlight follows the active theme (dark / light / hc) automatically.
  const tsHighlight = HighlightStyle.define([
    { tag: [t.keyword, t.modifier, t.controlKeyword, t.operatorKeyword, t.definitionKeyword, t.moduleKeyword, t.self], color: "var(--ts-accent)" },
    { tag: [t.string, t.special(t.string), t.character, t.escape], color: "var(--ts-st-done)" },
    { tag: [t.number, t.integer, t.float, t.bool, t.null], color: "var(--ts-st-ready)" },
    { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: "var(--ts-st-awaiting)", fontStyle: "italic" },
    { tag: [t.function(t.variableName), t.function(t.definition(t.variableName)), t.function(t.propertyName), t.macroName], color: "var(--ts-accent-2)" },
    { tag: [t.constant(t.variableName), t.standard(t.name), t.atom], color: "var(--ts-st-blocking)" },
    { tag: [t.typeName, t.className, t.namespace, t.definition(t.typeName)], color: "var(--ts-fg)" },
    { tag: [t.propertyName, t.attributeName, t.variableName, t.labelName], color: "var(--ts-fg)" },
    { tag: [t.operator, t.derefOperator, t.punctuation, t.separator, t.bracket, t.angleBracket, t.squareBracket, t.paren, t.brace], color: "var(--ts-fg-2)" },
    { tag: [t.meta, t.annotation], color: "var(--ts-fg-3)" },
    { tag: t.invalid, color: "var(--ts-error)" },
  ]);

  // ── Lightweight completion: Rust keywords + common tokio snippets ──────────
  const RUST_KEYWORDS = [
    "async", "await", "fn", "let", "mut", "move", "match", "if", "else", "for",
    "while", "loop", "impl", "struct", "enum", "trait", "pub", "use", "mod",
    "return", "ref", "as", "dyn", "const", "static", "where", "Some", "None",
    "Ok", "Err", "Result", "Option", "Vec", "String", "Box",
  ];
  const keywordOptions = RUST_KEYWORDS.map((label) => ({ label, type: "keyword" }));
  const tokioSnippets = [
    snippetCompletion("tokio::spawn(async move {\n    ${}\n});", { label: "tokio::spawn", type: "function", detail: "spawn a task" }),
    snippetCompletion("tokio::time::sleep(std::time::Duration::from_millis(${ms})).await;", { label: "tokio::time::sleep", type: "function", detail: "async sleep" }),
    snippetCompletion("tokio::task::spawn_blocking(move || {\n    ${}\n});", { label: "tokio::task::spawn_blocking", type: "function", detail: "blocking task" }),
    snippetCompletion("tokio::task::yield_now().await;", { label: "tokio::task::yield_now", type: "function" }),
    snippetCompletion("tokio::join!(${})", { label: "tokio::join!", type: "function" }),
    snippetCompletion("#[tokio::main(flavor = \"multi_thread\", worker_threads = ${4})]", { label: "tokio::main", type: "keyword", detail: "runtime entry" }),
    snippetCompletion("println!(\"${}\");", { label: "println!", type: "function" }),
  ];
  function rustComplete(ctx: CompletionContext): CompletionResult | null {
    const word = ctx.matchBefore(/[\w:!]+/);
    if (!word || (word.from === word.to && !ctx.explicit)) return null;
    return { from: word.from, options: [...keywordOptions, ...tokioSnippets], validFor: /^[\w:!]*$/ };
  }

  // ── Diagnostics: surface the existing `syn` analysis (store.analysis) as
  //    inline squiggles. col is 0-based, line is 1-based (see parser.rs). ──────
  function lintFromAnalysis(view: EditorView): Diagnostic[] {
    const report = store.analysis;
    if (!report?.diagnostics) return [];
    const doc = view.state.doc;
    const out: Diagnostic[] = [];
    for (const d of report.diagnostics) {
      if (d.line < 1 || d.line > doc.lines) continue;
      const line = doc.line(d.line);
      const from = Math.min(line.to, line.from + Math.max(0, d.col));
      out.push({
        from,
        to: line.to,
        severity: d.level === "error" ? "error" : d.level === "warning" ? "warning" : "info",
        message: d.message,
      });
    }
    return out;
  }

  const dispatch = createEventDispatcher();
  let root: HTMLDivElement;
  let view: EditorView | null = null;
  let themeCompartment = new Compartment();

  // One-shot "about to run" blink timers + the previous frame's per-task state,
  // used to detect the awaiting/blocking → ready (wait-completed) transition.
  let blinkTimers = new Set<ReturnType<typeof setTimeout>>();
  let prevTaskState = new Map<number, string>();

  // Per-line task state drives the background colour so the editor matches the
  // stage palette. One colour per line; precedence running > ready > blocking >
  // awaiting (done is not highlighted).
  type LineState = "running" | "ready" | "awaiting" | "blocking";
  const RANK: Record<LineState, number> = { running: 0, ready: 1, blocking: 2, awaiting: 3 };

  const setHighlight = StateEffect.define<{ runningLine: number; lineStates: Map<number, LineState> }>();
  const highlightField = StateField.define<DecorationSet>({
    create() { return Decoration.none; },
    update(deco, tr) {
      let next = deco.map(tr.changes);
      for (const e of tr.effects) {
        if (e.is(setHighlight)) {
          const builder: any[] = [];
          const doc = tr.state.doc;
          const safe = (n: number) => n >= 1 && n <= doc.lines;
          const lines = [...e.value.lineStates.keys()].filter(safe).sort((a, b) => a - b);
          for (const ln of lines) {
            builder.push(Decoration.line({ class: `cm-st-${e.value.lineStates.get(ln)!}` }).range(doc.line(ln).from));
          }
          next = Decoration.set(builder, true);
        }
      }
      return next;
    },
    provide: (f) => EditorView.decorations.from(f),
  });

  // The "about to run" blink is a one-shot overlay decoration, decoupled from
  // the per-tick rebuild of highlightField so the 3-iteration CSS animation is
  // never restarted while the task lingers in the ready queue.
  const addBlink = StateEffect.define<number[]>();
  const clearBlink = StateEffect.define<number[]>();
  const blinkField = StateField.define<DecorationSet>({
    create() { return Decoration.none; },
    update(deco, tr) {
      let next = deco.map(tr.changes);
      const doc = tr.state.doc;
      for (const e of tr.effects) {
        if (e.is(addBlink)) {
          const add = e.value
            .filter((n) => n >= 1 && n <= doc.lines)
            .map((n) => Decoration.line({ class: "cm-st-ready-blink" }).range(doc.line(n).from));
          if (add.length) next = next.update({ add, sort: true });
        }
        if (e.is(clearBlink)) {
          const drop = new Set(e.value.filter((n) => n >= 1 && n <= doc.lines).map((n) => doc.line(n).from));
          next = next.update({ filter: (from) => !drop.has(from) });
        }
      }
      return next;
    },
    provide: (f) => EditorView.decorations.from(f),
  });

  // Error lines (from the syn analysis) get a full-line tint so the mistake is
  // obvious — not just a lone gutter dot.
  const setErrorLines = StateEffect.define<number[]>();
  const errorLineField = StateField.define<DecorationSet>({
    create() { return Decoration.none; },
    update(deco, tr) {
      let next = deco.map(tr.changes);
      for (const e of tr.effects) {
        if (e.is(setErrorLines)) {
          const doc = tr.state.doc;
          const b = e.value
            .filter((n) => n >= 1 && n <= doc.lines)
            .map((n) => Decoration.line({ class: "cm-error-line" }).range(doc.line(n).from));
          next = Decoration.set(b, true);
        }
      }
      return next;
    },
    provide: (f) => EditorView.decorations.from(f),
  });

  // Tracks the single currently-running line (1-based, or -1) for the gutter
  // arrow and the sliding execution band.
  const runLineField = StateField.define<number>({
    create() { return -1; },
    update(v, tr) {
      for (const e of tr.effects) if (e.is(setHighlight)) return e.value.runningLine;
      return v;
    },
  });

  // IDEA-debugger ▶ marker in its own gutter, to the right of line numbers.
  class RunArrow extends GutterMarker {
    toDOM() {
      const s = document.createElement("span");
      s.className = "cm-run-arrow";
      s.textContent = "▶";
      return s;
    }
  }
  const runArrow = new RunArrow();
  const runGutter = gutter({
    class: "cm-run-gutter",
    markers: (v) => {
      const ln = v.state.field(runLineField);
      const doc = v.state.doc;
      if (ln >= 1 && ln <= doc.lines) return RangeSet.of([runArrow.range(doc.line(ln).from)]);
      return RangeSet.empty;
    },
  });

  // A single element that slides to the running line's geometry — the smooth
  // "current statement" band. Sits behind the text (opaque gutter bg hides the
  // part under the gutters), updated on every view change so it tracks scroll.
  const execBand = ViewPlugin.fromClass(
    class {
      band: HTMLDivElement;
      constructor(v: EditorView) {
        this.band = document.createElement("div");
        this.band.className = "cm-exec-band";
        v.scrollDOM.insertBefore(this.band, v.scrollDOM.firstChild);
        this.draw(v);
      }
      update(u: ViewUpdate) { this.draw(u.view); }
      draw(v: EditorView) {
        const ln = v.state.field(runLineField);
        const doc = v.state.doc;
        try {
          if (ln >= 1 && ln <= doc.lines) {
            const block = v.lineBlockAt(doc.line(ln).from);
            this.band.style.display = "block";
            this.band.style.height = block.height + "px";
            this.band.style.transform = `translateY(${block.top}px)`;
          } else {
            this.band.style.display = "none";
          }
        } catch {
          this.band.style.display = "none";
        }
      }
      destroy() { this.band.remove(); }
    }
  );

  onMount(() => {
    const state = EditorState.create({
      doc: store.code,
      extensions: [
        gutters({ fixed: true }),
        lineNumbers(),
        runGutter,
        highlightActiveLine(),
        drawSelection(),
        history(),
        rust(),
        syntaxHighlighting(tsHighlight, { fallback: true }),
        placeholder(tr("editor.placeholder")),
        autocompletion({ override: [rustComplete] }),
        linter(lintFromAnalysis, { delay: 200 }),
        lintGutter(),
        indentUnit.of("    "),
        keymap.of([...completionKeymap, ...defaultKeymap, ...historyKeymap, indentWithTab]),
        highlightField,
        blinkField,
        errorLineField,
        runLineField,
        execBand,
        themeCompartment.of(editorTheme()),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) {
            store.code = u.state.doc.toString();
            dispatch("input");
          }
        }),
        EditorView.editable.of(true),
      ],
    });
    view = new EditorView({ state, parent: root });
  });

  // Sync external code changes (example loads) back into editor.
  $effect(() => {
    if (!view) return;
    const cur = view.state.doc.toString();
    if (cur !== store.code) {
      view.dispatch({ changes: { from: 0, to: cur.length, insert: store.code } });
    }
  });

  // Colour each active line by its task state (matching the stage palette) and
  // blink lines whose task just finished waiting and is about to run.
  $effect(() => {
    if (!view) return;
    const frame = store.currentFrame;
    let runningLine = -1;
    const lineStates = new Map<number, LineState>();
    const blinkLines: number[] = [];
    const nextTaskState = new Map<number, string>();
    if (frame) {
      for (const t of frame.tasks.values()) {
        nextTaskState.set(t.id, t.state);
        if (t.state === "done" || t.currentLine <= 0) continue;
        const st = t.state as LineState;
        if (st === "running" && runningLine < 0) runningLine = t.currentLine;
        const prev = lineStates.get(t.currentLine);
        if (prev === undefined || RANK[st] < RANK[prev]) lineStates.set(t.currentLine, st);
        // wait-completed → ready: only blink when the task was awaiting/blocking
        // last frame (not a yield-induced ready, not a freshly spawned task).
        if (st === "ready") {
          const was = prevTaskState.get(t.id);
          if (was === "awaiting" || was === "blocking") blinkLines.push(t.currentLine);
        }
      }
    }
    prevTaskState = nextTaskState;

    const effects: StateEffect<any>[] = [setHighlight.of({ runningLine, lineStates })];
    if (blinkLines.length) {
      effects.push(addBlink.of(blinkLines));
      const tid = setTimeout(() => view?.dispatch({ effects: clearBlink.of(blinkLines) }), 1400);
      blinkTimers.add(tid);
    }
    view.dispatch({ effects });
  });

  // Theme follow
  $effect(() => {
    void store.theme;
    view?.dispatch({ effects: themeCompartment.reconfigure(editorTheme()) });
  });

  // Re-run diagnostics + mark error lines whenever the syn analysis updates.
  $effect(() => {
    const report = store.analysis;
    if (!view) return;
    forceLinting(view);
    const lines = report?.diagnostics?.filter((d) => d.level === "error").map((d) => d.line) ?? [];
    view.dispatch({ effects: setErrorLines.of(lines) });
  });

  function editorTheme() {
    const dark = store.theme !== "light";
    return EditorView.theme({
      "&": {
        height: "100%",
        background: "var(--ts-bg-2)",
        color: "var(--ts-fg)",
        fontFamily: "var(--ts-mono)",
        fontSize: "13px",
      },
      ".cm-scroller": {
        fontFamily: "var(--ts-mono)",
        lineHeight: "1.55",
        position: "relative",          // positioning context for the exec band
      },
      ".cm-content": {
        userSelect: "text",               // override body{user-select:none}
        WebkitUserSelect: "text",
      },
      // drawSelection() renders its own caret; native one is hidden. Theme it.
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--ts-accent)" },
      ".cm-gutters": {
        background: "var(--ts-bg-1)",
        color: "var(--ts-fg-3)",
        borderRight: "1px solid var(--ts-line)",
      },
      ".cm-gutterElement": { padding: "0 8px 0 12px" },
      ".cm-activeLine": { background: dark ? "rgba(255,255,255,0.035)" : "rgba(0,0,0,0.04)" },
      ".cm-activeLineGutter": { background: "transparent", color: "var(--ts-fg-2)" },
      ".cm-line": { transition: "background-color 120ms ease" },
      // per-state line fills (.cm-st-*) live in the <style> block so a single
      // color-mix definition tracks the --ts-st-* vars across all themes.
      // ▶ execution arrow gutter (sits between line numbers and the code)
      ".cm-run-gutter": { width: "14px" },
      ".cm-run-gutter .cm-gutterElement": { padding: "0", textAlign: "center" },
      ".cm-run-arrow": { color: "var(--ts-st-running)", fontSize: "10px", fontWeight: "bold", lineHeight: "1.55" },
      // the sliding "current statement" band reinforces the run line as it moves
      ".cm-exec-band": {
        position: "absolute",
        left: "0",
        right: "0",
        display: "none",
        pointerEvents: "none",
        background: dark ? "rgba(204, 120, 50, 0.16)" : "rgba(219, 125, 0, 0.13)",
        transition: "transform 140ms cubic-bezier(0.2, 0.7, 0.2, 1), height 140ms ease",
        willChange: "transform",
      },
      // autocomplete popup — match the theme (CM default is a light box)
      ".cm-tooltip.cm-tooltip-autocomplete": {
        background: "var(--ts-bg-1)",
        border: "1px solid var(--ts-line-2)",
        borderRadius: "6px",
        boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
      },
      ".cm-tooltip-autocomplete > ul > li": {
        fontFamily: "var(--ts-mono)",
        fontSize: "12px",
        color: "var(--ts-fg-2)",
        padding: "2px 8px",
      },
      ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
        background: "var(--ts-bg-3)",
        color: "var(--ts-fg)",
      },
      ".cm-completionIcon": { color: "var(--ts-fg-3)" },
      ".cm-completionDetail": { color: "var(--ts-fg-3)", fontStyle: "normal" },
      // lint diagnostics tooltip
      ".cm-tooltip.cm-tooltip-lint": {
        background: "var(--ts-bg-1)",
        border: "1px solid var(--ts-line-2)",
        borderRadius: "6px",
      },
      ".cm-diagnostic": {
        fontFamily: "var(--ts-sans)",
        fontSize: "12px",
        color: "var(--ts-fg)",
        padding: "4px 8px",
      },
      ".cm-diagnostic-error": { borderLeftColor: "var(--ts-error)" },
      // full-line error highlight (faint red + left bar, like the run/await lines)
      ".cm-error-line": {
        background: dark ? "rgba(188, 63, 60, 0.13)" : "rgba(199, 34, 45, 0.09)",
        boxShadow: "inset 3px 0 0 var(--ts-error)",
      },
      // soften / theme the lint gutter marker + the wavy underline
      ".cm-lint-marker-error": { color: "var(--ts-error)", opacity: "0.9" },
      ".cm-lintRange-error": { backgroundImage: "none", textDecoration: "underline wavy var(--ts-error)" },
      // (scrollbar visuals live in globals.css so the WKWebView `-webkit-appearance: none`
      //  switch applies; setting it inside EditorView.theme didn't override overlay style.)
    }, { dark });
  }

  onDestroy(() => {
    for (const tid of blinkTimers) clearTimeout(tid);
    blinkTimers.clear();
    view?.destroy();
  });
</script>

<div id="editor-root" bind:this={root}></div>

<style>
  #editor-root {
    min-height: 0;
    overflow: hidden;
    border-bottom: 1px solid var(--ts-line);
  }
  :global(.cm-editor.cm-focused) { outline: none; }

  /* gentle "breathing" pulse on the active statement — the exec band tint and
     the ▶ gutter arrow fade in sync so the eye is drawn to the running line
     without the flicker being distracting. */
  :global(.cm-exec-band) { animation: ts-exec-pulse 1.15s ease-in-out infinite; }
  :global(.cm-run-arrow) { animation: ts-exec-pulse 1.15s ease-in-out infinite; }
  @keyframes ts-exec-pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.4; }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.cm-exec-band), :global(.cm-run-arrow) { animation: none; }
  }

  /* per-state line fills — one color-mix definition tracks the --ts-st-* vars
     across dark / light / hc, so the editor matches the stage palette. */
  :global(.cm-st-running)  { background: color-mix(in srgb, var(--ts-st-running) 24%, transparent);  box-shadow: inset 3px 0 0 var(--ts-st-running); }
  :global(.cm-st-ready)    { background: color-mix(in srgb, var(--ts-st-ready) 20%, transparent);    box-shadow: inset 3px 0 0 var(--ts-st-ready); }
  :global(.cm-st-awaiting) { background: color-mix(in srgb, var(--ts-st-awaiting) 20%, transparent); box-shadow: inset 3px 0 0 var(--ts-st-awaiting); }
  :global(.cm-st-blocking) { background: color-mix(in srgb, var(--ts-st-blocking) 22%, transparent); box-shadow: inset 3px 0 0 var(--ts-st-blocking); }

  /* "about to run" — a task whose wait just completed blinks 3× then settles
     onto the static ready fill above. */
  :global(.cm-st-ready-blink) { animation: ts-ready-blink 0.45s ease-in-out 3; }
  @keyframes ts-ready-blink {
    0%, 100% { background: color-mix(in srgb, var(--ts-st-ready) 20%, transparent); }
    50%      { background: color-mix(in srgb, var(--ts-st-ready) 55%, transparent); }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.cm-st-ready-blink) { animation: none; }
  }
</style>
