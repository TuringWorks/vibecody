import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,

  // Ensure JS output is compatible with the WKWebView used by Tauri on macOS/iOS.
  // Vite 7 defaults to "esnext" which can produce JS that WKWebView doesn't support,
  // causing a blank white screen in the Tauri window.
  build: {
    // Target Safari 16+ (macOS 13+) for production builds
    target: ["es2021", "safari16"],
  },
  esbuild: {
    // Target the same level in dev mode (esbuild transform)
    target: "es2021",
  },

  // 2. tauri expects a fixed port, fail if that port is not available
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
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
