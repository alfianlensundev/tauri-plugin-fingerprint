import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      "tauri-plugin-fingerprint": fileURLToPath(
        new URL("../../guest-js/index.ts", import.meta.url),
      ),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  // prevent Vite from obscuring rust errors
  clearScreen: false,
  // tauri expects a fixed port, fail if that port is not available
  server: {
    fs: {
      allow: ["../.."],
    },
    host: host || false,
    port: 1420,
    strictPort: true,
    hmr: host ? {
      protocol: 'ws',
      host,
      port: 1421
    } : undefined,
  },
})
