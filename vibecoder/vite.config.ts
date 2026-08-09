import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(fileURLToPath(import.meta.url));

export { sharedPackageAliases };

/**
 * Alias table that lets `packages/vibe-ui-shared/src` resolve its bare imports
 * against this app's `node_modules`. Exported so vitest.config.ts uses exactly
 * the same list — a test run that resolves React differently from the dev
 * server produces "invalid hook call" failures that look like hook bugs.
 */
function sharedPackageAliases(base: string) {
  const dep = (name: string) => resolve(base, "node_modules", name);
  return [
    { find: "@vibe/shared", replacement: resolve(base, "../packages/vibe-ui-shared/src") },
    { find: /^react$/, replacement: dep("react") },
    // resolve() drops a trailing slash, so re-append it — otherwise
    // "react/jsx-runtime" concatenates into "reactjsx-runtime".
    { find: /^react\//, replacement: dep("react") + "/" },
    { find: /^react-dom$/, replacement: dep("react-dom") },
    { find: /^react-dom\//, replacement: dep("react-dom") + "/" },
    // No react-markdown/remark-gfm here: VibeCoder renders markdown itself and
    // doesn't install them. Add them alongside the dependency if it ever
    // consumes @vibe/shared/markdown.
    { find: /^lucide-react$/, replacement: dep("lucide-react") },
    { find: /^@tauri-apps\/api\//, replacement: dep("@tauri-apps/api") + "/" },
  ];
}

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // `@vibe/shared` is consumed as source, not as a built artifact — same
  // arrangement as VibeDesk and VibeAIChat (packages/vibe-ui-shared/src/index.ts
  // explains why). VibeCoder joined late, for the voice hook it had previously
  // duplicated twice over.
  //
  // Those files sit outside this project root with no hoisted node_modules, so
  // their bare imports can't be resolved by walking up — map them to this app's
  // copies, which also keeps a single React instance in the bundle. Mirrors the
  // `paths` in tsconfig.json and the aliases in vitest.config.ts; change them
  // together. Regex `find` on purpose: a plain string alias is a prefix match,
  // so "react" would also rewrite "react-dom" into "<path-to-react>-dom".
  resolve: {
    alias: sharedPackageAliases(rootDir),
  },

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
