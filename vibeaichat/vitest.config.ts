import { resolve } from "node:path";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Mirrors the alias table in vite.config.ts. Without it, any test touching a
  // component that imports `@vibe/shared` fails to load — and a *partial* copy
  // resolves React twice and fails with "invalid hook call" instead, which
  // looks like a bug in the component rather than in this file.
  resolve: {
    alias: [
      { find: "@vibe/shared", replacement: resolve(import.meta.dirname, "../packages/vibe-ui-shared/src") },
      { find: /^lucide-react$/, replacement: resolve(import.meta.dirname, "node_modules/lucide-react") },
      { find: /^react$/, replacement: resolve(import.meta.dirname, "node_modules/react") },
      { find: /^react\//, replacement: resolve(import.meta.dirname, "node_modules/react") + "/" },
      { find: /^react-dom$/, replacement: resolve(import.meta.dirname, "node_modules/react-dom") },
      { find: /^react-dom\//, replacement: resolve(import.meta.dirname, "node_modules/react-dom") + "/" },
      { find: /^@tauri-apps\/api\//, replacement: resolve(import.meta.dirname, "node_modules/@tauri-apps/api") + "/" },
    ],
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    css: true,
    // Matches the other two shells: CI runners are markedly slower than dev
    // macOS at async render, and the default 5s/1s pair flakes on correct tests.
    testTimeout: 15000,
  },
});
