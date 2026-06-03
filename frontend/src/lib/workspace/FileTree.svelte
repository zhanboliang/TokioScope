<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { ws, type TreeNode } from "../workspace.svelte";
  import { t } from "../i18n.svelte";

  let newProjOpen = $state(false);
  let newProjName = $state("");
  let newProjParent = $state<string | null>(null);
  let busy = $state(false);

  async function openFolder() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") await ws.openFolder(picked);
  }

  async function openFile() {
    const picked = await open({ multiple: false });
    if (typeof picked === "string") {
      const ok = await ws.openFile(picked);
      if (!ok) alert(t("file.binary"));
    }
  }

  async function chooseParent() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string") newProjParent = picked;
  }

  async function createProject() {
    if (!newProjName.trim() || !newProjParent || busy) return;
    busy = true;
    try {
      const { ipc } = await import("../ipc");
      const root = await ipc.createProject(newProjParent, newProjName.trim());
      await ws.openFolder(root);
      await ws.openFile(`${root}/src/main.rs`);
      newProjOpen = false;
      newProjName = "";
    } catch (e) {
      alert(String(e));
    } finally {
      busy = false;
    }
  }
</script>

<div class="tree-pane">
  <header class="tree-head">
    <div class="acts">
      <button class="t-btn" title={t("ide.newProject")} aria-label="New project"
        onclick={() => { newProjOpen = true; newProjParent = ws.root; }}>
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round">
          <path d="M8 3.5v9M3.5 8h9" />
        </svg>
      </button>
      <button class="t-btn" title={t("ide.openFolder")} aria-label="Open folder" onclick={openFolder}>
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round">
          <path d="M1.8 4.2h4l1.2 1.4h7.2v6.6a.6.6 0 0 1-.6.6H2.4a.6.6 0 0 1-.6-.6z" />
        </svg>
      </button>
      <button class="t-btn" title={t("ide.openFile")} aria-label="Open file" onclick={openFile}>
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round">
          <path d="M4 1.8h5l3 3v8.4a.6.6 0 0 1-.6.6H4a.6.6 0 0 1-.6-.6V2.4A.6.6 0 0 1 4 1.8z" /><path d="M9 1.9v3.1h3" />
        </svg>
      </button>
      {#if ws.root}
        <button class="t-btn" title={t("ide.refresh")} aria-label="Refresh" onclick={() => ws.refreshTree()}>
          <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round">
            <path d="M12.5 6A5 5 0 1 0 13 9" /><path d="M12.8 3v3h-3" />
          </svg>
        </button>
      {/if}
    </div>
    {#if ws.rootName}<span class="ttl">{ws.rootName}</span>{/if}
  </header>

  <div class="tree-body">
    {#if !ws.root}
      <div class="empty">
        <p>{t("ide.noFolder")}</p>
        <button class="cta" onclick={openFolder}>{t("ide.openFolder")}</button>
        <button class="cta ghost" onclick={() => { newProjOpen = true; newProjParent = null; }}>{t("ide.newProject")}</button>
        <p class="drop-hint">{t("ide.dropHint")}</p>
      </div>
    {:else}
      {#each ws.tree as node (node.path)}
        {@render row(node, 0)}
      {/each}
    {/if}
  </div>
</div>

{#snippet row(node: TreeNode, depth: number)}
  {#if node.isDir}
    <button class="node dir" style="padding-left: {6 + depth * 12}px" onclick={() => ws.toggleDir(node)}>
      <svg class="chev" class:open={node.expanded} viewBox="0 0 16 16" width="11" height="11" fill="currentColor" aria-hidden="true">
        <path d="M6 4l4 4-4 4z" />
      </svg>
      <span class="nm">{node.name}</span>
    </button>
    {#if node.expanded}
      {#each node.children as child (child.path)}
        {@render row(child, depth + 1)}
      {/each}
    {/if}
  {:else}
    <button class="node file" class:active={ws.active?.path === node.path}
      style="padding-left: {20 + depth * 12}px"
      onclick={() => ws.openFile(node.path).then((ok) => { if (!ok) alert(t("file.binary")); })}>
      <span class="nm">{node.name}</span>
    </button>
  {/if}
{/snippet}

{#if newProjOpen}
  <div class="modal-scrim" role="presentation" onclick={() => (newProjOpen = false)}></div>
  <div class="modal" role="dialog" aria-modal="true">
    <h3>{t("newproj.title")}</h3>
    <label class="fld">
      <span>{t("newproj.name")}</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={newProjName} autofocus placeholder="my-async-app"
        onkeydown={(e) => { if (e.key === "Enter") createProject(); }} />
    </label>
    <label class="fld">
      <span>{t("newproj.location")}</span>
      <button class="loc" onclick={chooseParent}>{newProjParent ?? t("newproj.choose")}</button>
    </label>
    <div class="modal-acts">
      <button class="btn ghost" onclick={() => (newProjOpen = false)}>{t("newproj.cancel")}</button>
      <button class="btn primary" disabled={!newProjName.trim() || !newProjParent || busy} onclick={createProject}>
        {t("newproj.create")}
      </button>
    </div>
  </div>
{/if}

<style>
  .tree-pane { display: flex; flex-direction: column; height: 100%; background: var(--ts-bg-2); overflow: hidden; }
  .tree-head {
    display: flex; align-items: center; gap: 6px; flex-shrink: 0;
    padding: 6px; border-bottom: 1px solid var(--ts-line); background: var(--ts-bg-1);
  }
  .tree-head .ttl {
    flex: 1 1 auto; min-width: 0; font-family: var(--ts-mono); font-size: 11px;
    color: var(--ts-fg-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .acts { display: flex; gap: 1px; flex-shrink: 0; }
  .t-btn {
    display: inline-flex; align-items: center; justify-content: center; width: 22px; height: 20px;
    padding: 0; background: transparent; border: none; border-radius: 4px; color: var(--ts-fg-2); cursor: pointer;
  }
  .t-btn:hover { background: var(--ts-bg-3); color: var(--ts-fg); }

  .tree-body { flex: 1 1 auto; min-height: 0; overflow: auto; padding: 4px 0; }

  .node {
    display: flex; align-items: center; gap: 5px; width: 100%; padding: 2px 8px 2px 6px;
    background: transparent; border: none; text-align: left; cursor: pointer;
    font-family: var(--ts-mono); font-size: 12px; line-height: 1.7; color: var(--ts-fg);
    white-space: nowrap; overflow: hidden;
  }
  .node:hover { background: var(--ts-bg-3); }
  .node.file.active { background: color-mix(in srgb, var(--ts-accent) 18%, transparent); }
  .node .nm { overflow: hidden; text-overflow: ellipsis; }
  .node.dir .nm { color: var(--ts-fg); }
  .node.file .nm { color: var(--ts-fg-2); }
  .chev { flex-shrink: 0; transition: transform 120ms ease; color: var(--ts-fg-3); }
  .chev.open { transform: rotate(90deg); }

  .empty { padding: 24px 16px; display: flex; flex-direction: column; gap: 10px; align-items: stretch; }
  .empty p { margin: 0; font-size: 12px; color: var(--ts-fg-3); text-align: center; }
  .empty .drop-hint { margin-top: 8px; font-size: 10.5px; opacity: 0.7; }
  .cta {
    padding: 7px 10px; border-radius: 6px; border: 1px solid var(--ts-line-2);
    background: var(--ts-bg-3); color: var(--ts-fg); font-size: 12px; cursor: pointer;
  }
  .cta:hover { border-color: var(--ts-accent); }
  .cta.ghost { background: transparent; }

  .modal-scrim { position: fixed; inset: 0; z-index: 100; background: rgba(0,0,0,0.4); }
  .modal {
    position: fixed; z-index: 101; top: 50%; left: 50%; transform: translate(-50%, -50%);
    width: 360px; padding: 18px; background: var(--ts-bg-1); border: 1px solid var(--ts-line-2);
    border-radius: 10px; box-shadow: 0 18px 50px rgba(0,0,0,0.5); font-family: var(--ts-sans);
  }
  .modal h3 { margin: 0 0 14px; font-size: 14px; color: var(--ts-fg); font-weight: 600; }
  .fld { display: flex; flex-direction: column; gap: 5px; margin-bottom: 12px; }
  .fld span { font-size: 11px; color: var(--ts-fg-3); }
  .fld input, .fld .loc {
    width: 100%; padding: 7px 9px; border-radius: 6px; border: 1px solid var(--ts-line-2);
    background: var(--ts-bg-2); color: var(--ts-fg); font-family: var(--ts-mono); font-size: 12px;
    text-align: left; cursor: text; box-sizing: border-box;
  }
  .fld .loc { cursor: pointer; color: var(--ts-fg-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fld input:focus { outline: none; border-color: var(--ts-accent); }
  .modal-acts { display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px; }
  .btn { padding: 7px 14px; border-radius: 6px; border: 1px solid var(--ts-line-2); background: var(--ts-bg-3); color: var(--ts-fg); font-size: 12px; cursor: pointer; }
  .btn.ghost { background: transparent; }
  .btn.primary { background: var(--ts-accent); border-color: var(--ts-accent); color: #fff; }
  .btn.primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
