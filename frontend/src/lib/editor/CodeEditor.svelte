<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { EditorState, Compartment } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, highlightActiveLine, Decoration, gutters, type DecorationSet } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { syntaxHighlighting, defaultHighlightStyle, indentUnit } from "@codemirror/language";
  import { rust } from "@codemirror/lang-rust";
  import { StateField, StateEffect } from "@codemirror/state";
  import { store } from "../store.svelte";

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

  onMount(() => {
    const state = EditorState.create({
      doc: store.code,
      extensions: [
        gutters({ fixed: true }),
        lineNumbers(),
        highlightActiveLine(),
        history(),
        rust(),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        indentUnit.of("    "),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        highlightField,
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
      ".cm-run-line": { background: "rgba(204, 120, 50, 0.18)", borderLeft: "2px solid var(--ts-st-running)" },
      ".cm-await-line": { background: dark ? "rgba(152, 118, 170, 0.10)" : "rgba(135, 16, 148, 0.07)" },
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
