import { mount } from "svelte";
// IBM Plex Sans (SIL OFL 1.1) for the interface chrome; JetBrains Mono (SIL OFL
// 1.1) for the code editor, terminal and output.
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/600.css";
import "@fontsource/jetbrains-mono/700.css";
import "./styles/globals.css";
import "./styles/theme-dark.css";
import "./styles/theme-light.css";
import "./styles/theme-hc.css";
import App from "./App.svelte";
import { detectCJK } from "./lib/font";

// Set default theme synchronously to avoid FOUC; prefs.ts may override later.
if (!document.documentElement.dataset.theme) {
  document.documentElement.dataset.theme = "dark";
}
detectCJK();

const app = mount(App, { target: document.getElementById("app")! });
export default app;
