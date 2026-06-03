// Shared run entrypoint used by the titlebar Run button and the keyboard
// shortcut. Routes to a whole-project trace when a Cargo project is open,
// otherwise traces the active file / scratch buffer.

import { ipc } from "./ipc";
import { store } from "./store.svelte";
import { ws } from "./workspace.svelte";

export async function runActive() {
  store.reset();
  try {
    if (ws.root && ws.isCargo) {
      await ws.saveAllDirty();
      store.isProjectRun = true;
      store.runFilePath = `${ws.root}/src/main.rs`;
      await ipc.startRun({ projectRoot: ws.root });
    } else {
      const tab = ws.active;
      if (!tab) return; // nothing open to run
      if (tab.path) await ws.saveActive();
      store.isProjectRun = false;
      store.runFilePath = tab.id;
      await ipc.startRun({ source: tab.content });
    }
  } catch (e) {
    store.rawStderr = [...store.rawStderr, `start_run failed: ${e}`];
  }
}
