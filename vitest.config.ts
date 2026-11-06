import { defineConfig } from "vitest/config";

// Kept apart from vite.config.ts, which is tuned for `tauri dev` (fixed port, Rust-side watch
// exclusions).
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
  },
});
