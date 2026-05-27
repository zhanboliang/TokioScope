import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || "127.0.0.1", // bind localhost-only so dev URL isn't network-reachable
    open: false,               // never auto-open the system browser
    hmr: host
      ? { protocol: "ws", host, port: 5174 }
      : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/runner-template/**"] },
  },
  build: {
    target: "es2022",
    sourcemap: true,
  },
}));
