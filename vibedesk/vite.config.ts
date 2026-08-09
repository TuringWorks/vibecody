import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

// VibeDesk dev server runs on 1422 (vibecoder=1420, vibeaichat=1421).
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  // Settings screens + theme + model-output parsing live in one place so the
  // two shells cannot drift. Consumed as source — no build step, no dist.
  //
  // Its files sit outside this project root and there is no hoisted root
  // node_modules, so bare imports inside them (react, lucide-react, …) cannot
  // be resolved by walking up; they are mapped to this app's copies here, which
  // also guarantees a single React instance in the bundle. Mirrors the `paths`
  // in tsconfig.json — change both together.
  //
  // Regex `find` on purpose: a plain string alias is a prefix match, so "react"
  // would also rewrite "react-dom" into "<path-to-react>-dom".
  resolve: {
    alias: [
      { find: "@vibe/shared", replacement: resolve(__dirname, "../packages/vibe-ui-shared/src") },
      { find: /^react$/, replacement: resolve(__dirname, "node_modules/react") },
      // resolve() drops a trailing slash, so re-append it — otherwise
      // "react/jsx-runtime" concatenates into "reactjsx-runtime".
      { find: /^react\//, replacement: resolve(__dirname, "node_modules/react") + "/" },
      { find: /^react-dom$/, replacement: resolve(__dirname, "node_modules/react-dom") },
      { find: /^react-dom\//, replacement: resolve(__dirname, "node_modules/react-dom") + "/" },
      { find: /^lucide-react$/, replacement: resolve(__dirname, "node_modules/lucide-react") },
      { find: /^@tauri-apps\/api\//, replacement: resolve(__dirname, "node_modules/@tauri-apps/api") + "/" },
    ],
  },
  clearScreen: false,
  server: {
    port: 1422,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1422,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
