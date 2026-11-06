import { defineConfig } from "vitest/config";

// Kept apart from vite.config.ts, which is tuned for `tauri dev` (fixed port, Rust-side watch
// exclusions).
export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
    // The scaffold has no logic to test yet, and `vitest run` exits 1 when no file matches. Drop
    // this with the first test file, so a broken `include` glob fails instead of passing silently.
    passWithNoTests: true,
  },
});
