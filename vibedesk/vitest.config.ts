import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
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
