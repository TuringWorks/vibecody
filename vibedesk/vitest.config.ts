import { resolve } from "node:path";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // The shared-UI alias is part of how these components resolve; without it
  // here, any test touching a component that imports `@vibe/shared` fails to
  // load — which is why the composer had no rendering test at all.
  resolve: {
    alias: [
      { find: "@vibe/shared", replacement: resolve(__dirname, "../packages/vibe-ui-shared/src") },
      // The shared package declares lucide-react and react as peers, so its
      // files resolve them from *this* app's node_modules, not their own.
      { find: /^lucide-react$/, replacement: resolve(__dirname, "node_modules/lucide-react") },
      { find: /^react$/, replacement: resolve(__dirname, "node_modules/react") },
      { find: /^react\//, replacement: resolve(__dirname, "node_modules/react") + "/" },
      { find: /^react-dom$/, replacement: resolve(__dirname, "node_modules/react-dom") },
      { find: /^react-dom\//, replacement: resolve(__dirname, "node_modules/react-dom") + "/" },
      { find: /^@tauri-apps\/api\//, replacement: resolve(__dirname, "node_modules/@tauri-apps/api") + "/" },
    ],
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    css: true,
    // Matches vibecoder: CI runners are markedly slower than dev macOS at
    // async render, and the default 5s/1s pair flakes on correct tests.
    testTimeout: 15000,
  },
});
