// Point @monaco-editor/react at the locally-bundled `monaco-editor` instead of
// its default `@monaco-editor/loader`, which injects a <script> from the
// jsdelivr CDN at runtime. The Tauri CSP is `script-src 'self'`, so that CDN
// script is blocked — which left <Editor> stuck on its "Loading..." placeholder
// forever, for every file. Using the bundled instance loads Monaco offline.
//
// Monaco's language services run in web workers; we register them through Vite's
// `?worker` imports so they load under `worker-src 'self' blob:` rather than
// trying to fetch worker scripts from the CDN too.
//
// This module must be imported before anything renders <Editor> (see main.tsx).
import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor";

import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import JsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import CssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import HtmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import TsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

(globalThis as typeof globalThis & { MonacoEnvironment?: monaco.Environment }).MonacoEnvironment = {
  getWorker(_workerId: string, label: string): Worker {
    switch (label) {
      case "json":
        return new JsonWorker();
      case "css":
      case "scss":
      case "less":
        return new CssWorker();
      case "html":
      case "handlebars":
      case "razor":
        return new HtmlWorker();
      case "typescript":
      case "javascript":
        return new TsWorker();
      default:
        return new EditorWorker();
    }
  },
};

// Resolve @monaco-editor/react against the bundled instance — no network fetch.
loader.config({ monaco });
