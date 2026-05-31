<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { EditorState, Compartment, RangeSet } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, highlightActiveLine, Decoration, gutters, gutter, GutterMarker, ViewPlugin, type ViewUpdate, type DecorationSet } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { syntaxHighlighting, HighlightStyle, indentUnit } from "@codemirror/language";
  import { rust } from "@codemirror/lang-rust";
  import { tags as t } from "@lezer/highlight";
  import { StateField, StateEffect } from "@codemirror/state";
  import { store } from "../store.svelte";

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

  const dispatch = createEventDispatcher();
  let root: HTMLDivElement;
  let view: EditorView | null = null;
  let themeCompartment = new Compartment();

  const setHighlight = StateEffect.define<{ runningLine: number; awaitingLines: number[] }>();
  const highlightField = StateField.define<DecorationSet>({
    create() { return Decoration.none; },
    update(deco, tr) {
      let next = deco.map(tr.changes);
      for (const e of tr.effects) {
        if (e.is(setHighlight)) {
          const builder: any[] = [];
          const doc = tr.state.doc;
          const safe = (n: number) => n >= 1 && n <= doc.lines;
          if (safe(e.value.runningLine)) {
            const line = doc.line(e.value.runningLine);
            builder.push(Decoration.line({ class: "cm-run-line" }).range(line.from));
          }
          for (const ln of e.value.awaitingLines) {
            if (!safe(ln)) continue;
            const line = doc.line(ln);
            builder.push(Decoration.line({ class: "cm-await-line" }).range(line.from));
          }
          next = Decoration.set(builder, true);
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
        history(),
        rust(),
        syntaxHighlighting(tsHighlight, { fallback: true }),
        indentUnit.of("    "),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        highlightField,
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

  // Highlight running / awaiting lines based on currentFrame
  $effect(() => {
    if (!view) return;
    const frame = store.currentFrame;
    let runningLine = -1;
    const awaitingLines: number[] = [];
    if (frame) {
      for (const t of frame.tasks.values()) {
        if (t.state === "running" && t.currentLine > 0) {
          if (runningLine < 0) runningLine = t.currentLine;
        }
        if ((t.state === "awaiting" || t.state === "blocking") && t.currentLine > 0) {
          awaitingLines.push(t.currentLine);
        }
      }
    }
    view.dispatch({ effects: setHighlight.of({ runningLine, awaitingLines }) });
  });

  // Theme follow
  $effect(() => {
    void store.theme;
    view?.dispatch({ effects: themeCompartment.reconfigure(editorTheme()) });
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
        caretColor: "var(--ts-accent)",
        userSelect: "text",               // override body{user-select:none}
        WebkitUserSelect: "text",
      },
      ".cm-gutters": {
        background: "var(--ts-bg-1)",
        color: "var(--ts-fg-3)",
        borderRight: "1px solid var(--ts-line)",
      },
      ".cm-gutterElement": { padding: "0 8px 0 12px" },
      ".cm-activeLine": { background: dark ? "rgba(255,255,255,0.035)" : "rgba(0,0,0,0.04)" },
      ".cm-activeLineGutter": { background: "transparent", color: "var(--ts-fg-2)" },
      ".cm-line": { transition: "background-color 120ms ease" },
      // currently executing line — bold fill + a left accent bar. The bar uses an
      // inset box-shadow (not border-left) so the code text doesn't shift.
      ".cm-run-line": {
        background: dark ? "rgba(204, 120, 50, 0.26)" : "rgba(219, 125, 0, 0.22)",
        boxShadow: "inset 3px 0 0 var(--ts-st-running)",
      },
      // awaiting / blocking — a clearly-marked "paused" region in a cool hue
      ".cm-await-line": {
        background: dark ? "rgba(152, 118, 170, 0.24)" : "rgba(135, 16, 148, 0.16)",
        boxShadow: "inset 3px 0 0 var(--ts-st-blocking)",
      },
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
      // (scrollbar visuals live in globals.css so the WKWebView `-webkit-appearance: none`
      //  switch applies; setting it inside EditorView.theme didn't override overlay style.)
    }, { dark });
  }

  onDestroy(() => view?.destroy());
</script>

<div id="editor-root" bind:this={root}></div>

<style>
  #editor-root {
    min-height: 0;
    overflow: hidden;
    border-bottom: 1px solid var(--ts-line);
  }
  :global(.cm-editor.cm-focused) { outline: none; }
</style>
