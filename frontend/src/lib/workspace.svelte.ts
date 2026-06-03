// Workspace / IDE shell state: the open folder's file tree, editor tabs, and the
// 3-column + bottom-panel layout. Kept separate from `store.svelte.ts`, which
// owns the simulation/playback. The active tab's `content` is the editor buffer.

import { ipc, type DirEntry } from "./ipc";

export interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  children: TreeNode[];
  loaded: boolean;
  expanded: boolean;
}

export type TabLang = "rust" | "toml" | "text";

export interface Tab {
  id: string; // path for files, "untitled-N" for unsaved buffers
  path: string | null; // null = scratch (no file on disk)
  title: string;
  language: TabLang;
  content: string;
  saved: string; // last-persisted content, for the dirty indicator
}

let untitledSeq = 0;
let termSeq = 0;
function newUntitledTab(): Tab {
  return {
    id: `untitled-${++untitledSeq}`,
    path: null,
    title: "Untitled",
    language: "rust",
    content: "",
    saved: "",
  };
}

function basename(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}

function langForPath(p: string): TabLang {
  if (p.endsWith(".rs")) return "rust";
  if (p.endsWith(".toml")) return "toml";
  return "text";
}

class Workspace {
  root = $state<string | null>(null);
  rootName = $state("");
  isCargo = $state(false);
  tree = $state<TreeNode[]>([]);

  tabs = $state<Tab[]>([newUntitledTab()]);
  activeId = $state<string>(this.tabs[0].id);

  // ── layout ───────────────────────────────────────────────────────────
  treeWidth = $state(220);
  treeCollapsed = $state(true);  // collapsed by default; toggled from the status bar
  vizCollapsed = $state(false);  // the right-hand visualization column
  editorWidth = $state(520); // px; the viz column flex-grows past this (#1)
  bottomHeight = $state(190);
  bottomCollapsed = $state(false);
  // The bottom panel shows the output log, the events list, or one of N terminals.
  activeBottom = $state<"output" | "events" | number>("output");
  terminals = $state<number[]>([]);

  get active(): Tab | null {
    return this.tabs.find((t) => t.id === this.activeId) ?? this.tabs[0] ?? null;
  }

  newUntitled() {
    const tab = newUntitledTab();
    this.tabs = [...this.tabs, tab];
    this.activeId = tab.id;
  }

  addTerminal() {
    const id = ++termSeq;
    this.terminals = [...this.terminals, id];
    this.activeBottom = id;
    this.bottomCollapsed = false;
  }

  removeTerminal(id: number) {
    this.terminals = this.terminals.filter((t) => t !== id);
    if (this.activeBottom === id) {
      this.activeBottom = this.terminals[this.terminals.length - 1] ?? "output";
    }
  }

  isDirty(t: Tab): boolean {
    return t.content !== t.saved;
  }

  private nodeFrom(e: DirEntry): TreeNode {
    return { name: e.name, path: e.path, isDir: e.is_dir, children: [], loaded: false, expanded: false };
  }

  async openFolder(path: string) {
    const info = await ipc.workspaceInfo(path);
    this.root = info.root;
    this.rootName = info.name;
    this.isCargo = info.is_cargo;
    this.treeCollapsed = false;
    this.tree = (await ipc.readDir(path)).map((e) => this.nodeFrom(e));
  }

  async refreshTree() {
    if (this.root) this.tree = (await ipc.readDir(this.root)).map((e) => this.nodeFrom(e));
  }

  async toggleDir(node: TreeNode) {
    node.expanded = !node.expanded;
    if (node.expanded && !node.loaded) {
      node.children = (await ipc.readDir(node.path)).map((e) => this.nodeFrom(e));
      node.loaded = true;
    }
  }

  /** Open a file in a tab (or focus it if already open). Returns false if the
   *  file couldn't be shown (binary / too large). */
  async openFile(path: string): Promise<boolean> {
    const existing = this.tabs.find((t) => t.path === path);
    if (existing) {
      this.activeId = existing.id;
      return true;
    }
    const fr = await ipc.readTextFile(path);
    if (fr.too_large || fr.binary) return false;
    const tab: Tab = {
      id: path,
      path,
      title: basename(path),
      language: langForPath(path),
      content: fr.text,
      saved: fr.text,
    };
    this.tabs = [...this.tabs, tab];
    this.activeId = path;
    return true;
  }

  setActive(id: string) {
    this.activeId = id;
  }

  closeTab(id: string) {
    const idx = this.tabs.findIndex((t) => t.id === id);
    if (idx < 0) return;
    this.tabs = this.tabs.filter((t) => t.id !== id);
    if (this.activeId === id) {
      // Focus the neighbor; empty → welcome state (activeId resolves to null).
      this.activeId = this.tabs[Math.min(idx, this.tabs.length - 1)]?.id ?? "";
    }
  }

  updateActiveContent(text: string) {
    const t = this.active;
    if (t) t.content = text;
  }

  async saveActive() {
    const t = this.active;
    if (!t?.path) return;
    await ipc.writeTextFile(t.path, t.content);
    t.saved = t.content;
  }

  /** Persist every dirty file tab — called before a project run. */
  async saveAllDirty() {
    for (const t of this.tabs) {
      if (t.path && this.isDirty(t)) {
        await ipc.writeTextFile(t.path, t.content);
        t.saved = t.content;
      }
    }
  }

  /** Load example / template code: reuse the active untitled buffer if there is
   *  one, otherwise open a fresh untitled buffer. */
  loadExampleCode(code: string) {
    const a = this.active;
    if (a && a.path === null) {
      a.content = code;
      a.saved = code;
    } else {
      const tab = newUntitledTab();
      tab.content = code;
      tab.saved = code;
      this.tabs = [...this.tabs, tab];
      this.activeId = tab.id;
    }
  }
}

export const ws = new Workspace();
