import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

// @ts-expect-error process is a Node.js global available at config-eval time
const host = process.env.TAURI_DEV_HOST;

// Tuned for `tauri dev`: devUrl in tauri.conf.json names this fixed port, and Vite must fail
// rather than move to another one.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
