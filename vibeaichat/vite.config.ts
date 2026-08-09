import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  // Settings screens + theme + model-output parsing come from the shared
  // package, consumed as source. Its files live outside this project root and
  // there is no hoisted node_modules, so their bare imports are mapped to this
  // app's copies — which also keeps a single React instance in the bundle.
  // Mirrors `paths` in tsconfig.json; change both together.
  //
  // Regex `find` on purpose: a plain string alias is a prefix match, so
  // "react" would also rewrite "react-dom". And resolve() drops a trailing
  // slash, so sub-path replacements re-append it.
  resolve: {
    alias: [
      { find: "@vibe/shared", replacement: resolve(__dirname, "../packages/vibe-ui-shared/src") },
      { find: /^react$/, replacement: resolve(__dirname, "node_modules/react") },
      { find: /^react\//, replacement: resolve(__dirname, "node_modules/react") + "/" },
      { find: /^react-dom$/, replacement: resolve(__dirname, "node_modules/react-dom") },
      { find: /^react-dom\//, replacement: resolve(__dirname, "node_modules/react-dom") + "/" },
      { find: /^lucide-react$/, replacement: resolve(__dirname, "node_modules/lucide-react") },
      { find: /^@tauri-apps\/api\//, replacement: resolve(__dirname, "node_modules/@tauri-apps/api") + "/" },
    ],
  },

  clearScreen: false,
  server: {
    port: 1421,
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
}));
