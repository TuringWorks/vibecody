/**
 * DesignMode — full-screen visual design editor with tabbed layout.
 *
 * Tabs: Preview | Generate | Components | Inspector | Draw.io | Pencil | Penpot | Diagrams
 */
import { lazy, Suspense, useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { VisualEditor, SelectedElement } from "./VisualEditor";
import { parseProviderSelection } from "../hooks/useModelRegistry";
import { getSelectedEffort } from "../utils/effort";
import { takePendingDesignSubTab } from "../lib/panelDeepLink";

// The four editor tabs are lazy and are not mounted until visited.
//
// They used to be rendered eagerly and merely hidden with `display: none`, so
// opening Design paid for all four: DrawioEditorPanel alone fires two Tauri
// calls the moment it mounts, and Penpot, Pencil and the diagram generator
// each carry their own state and effects. Loading the editor the user opened
// is what they asked for; loading four is not.
const DrawioEditorPanel = lazy(() =>
  import("./DrawioEditorPanel").then((m) => ({ default: m.DrawioEditorPanel })),
);
const PencilPanel = lazy(() => import("./PencilPanel").then((m) => ({ default: m.PencilPanel })));
const PenpotPanel = lazy(() => import("./PenpotPanel").then((m) => ({ default: m.PenpotPanel })));
const DiagramGeneratorPanel = lazy(() =>
  import("./DiagramGeneratorPanel").then((m) => ({ default: m.DiagramGeneratorPanel })),
);

interface DesignModeProps {
  workspacePath: string | null;
  provider: string;
  onOpenFile?: (path: string, line?: number) => void;
}

/** One workspace file and the components it declares. */
interface ComponentFile {
  path: string;
  components: string[];
  lines: number;
}

interface ComponentTree {
  files: ComponentFile[];
  file_count: number;
  component_count: number;
  /** True when the scan hit its file cap — the list is a sample, and says so. */
  truncated: boolean;
}

type DesignTab = "preview" | "generate" | "components" | "inspector" | "drawio" | "pencil" | "penpot" | "diagrams";

// Ports and hostnames that serve the VibeCoder app itself — never load in the preview iframe.
const BLOCKED_PATTERNS = [
  /^https?:\/\/localhost:1420/i,   // Tauri dev server
  /^https?:\/\/127\.0\.0\.1:1420/i,
  /^tauri:\/\//i,                  // Tauri internal protocol
  /^https?:\/\/localhost:5173/i,   // Vite default (VibeCoder dev)
  /^https?:\/\/127\.0\.0\.1:5173/i,
];

function isBlockedUrl(url: string): boolean {
  if (!url.trim()) return false;
  return BLOCKED_PATTERNS.some((p) => p.test(url.trim()));
}

// LLM-output helpers live in utils/llmOutput so other panels (Drawio,
// future generators) can share the same fence-extraction + module-stripping
// behavior. Re-exported here under the legacy name for back-compat.
import { extractFencedBlock, stripModuleSyntax } from "../utils/llmOutput";
export { stripModuleSyntax };
export const extractGeneratedCode = extractFencedBlock;

/** Ensure URL has a protocol — bare "example.com" → "https://example.com" */
function normalizeUrl(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed) return trimmed;
  if (/^https?:\/\//i.test(trimmed)) return trimmed;
  // Relative paths (no dots at start, no slash) are not external URLs — reject
  if (!/[./]/.test(trimmed.split("/")[0])) return trimmed;
  return "https://" + trimmed;
}

const tabDefs: { id: DesignTab; label: string }[] = [
  { id: "preview", label: "Preview" },
  { id: "generate", label: "Generate" },
  { id: "components", label: "Components" },
  { id: "inspector", label: "Inspector" },
  { id: "drawio", label: "Draw.io" },
  { id: "pencil", label: "Pencil" },
  { id: "penpot", label: "Penpot" },
  { id: "diagrams", label: "Diagrams" },
  // Figma lives in DesignHubPanel ("Hub" tab) and in the Import tab. The copy
  // that used to sit here was unreachable from the tab bar yet still mounted,
  // with its own duplicate token handling; it is gone rather than hidden.
];

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "7px 16px",
  fontSize: "var(--font-size-base)",
  fontWeight: active ? 600 : 400,
  cursor: "pointer",
  border: "none",
  borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  background: "transparent",
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  transition: "color 0.15s, border-color 0.15s",
  whiteSpace: "nowrap",
});

const panelStyle: React.CSSProperties = {
  flex: 1,
  overflow: "auto",
  padding: 16,
};

export function DesignMode({ workspacePath, provider, onOpenFile }: DesignModeProps) {
  const [activeTab, setActiveTab] = useState<DesignTab>("preview");
  const [previewUrl, setPreviewUrl] = useState("");
  const [blockedError, setBlockedError] = useState(false);
  const [visualEditEnabled, setVisualEditEnabled] = useState(false);
  const [selectedElement, setSelectedElement] = useState<SelectedElement | null>(null);
  const [aiInstruction, setAiInstruction] = useState("");
  // Diffcomplete-into-source. Instruction + element →
  // a CSS/HTML unified diff (never a live-DOM mutation), shown for explicit apply.
  const [diffInstruction, setDiffInstruction] = useState("");
  const [designDiff, setDesignDiff] = useState<string | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [diffError, setDiffError] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [generationResult, setGenerationResult] = useState("");
  /** Why the last generate or edit failed. Kept apart from the result so an
   *  error can never be rendered as generated code. */
  const [generationError, setGenerationError] = useState<string | null>(null);
  const [previewSrcdoc, setPreviewSrcdoc] = useState<string | null>(null);
  const [componentTree, setComponentTree] = useState<ComponentTree | null>(null);
  const [treeError, setTreeError] = useState<string | null>(null);
  const [treeLoading, setTreeLoading] = useState(false);
  const [treeFilter, setTreeFilter] = useState("");
  /** Position of the preview iframe, tracked so the floating editor lands on
   *  the element rather than wherever the iframe was at first paint. */
  const [iframeRect, setIframeRect] = useState<{ top: number; left: number } | null>(null);
  /** Why Visual Edit could not attach, when it could not. */
  const [inspectorError, setInspectorError] = useState<string | null>(null);
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const iframeContainerRef = useRef<HTMLDivElement>(null);
  /** `public/inspector.js`, read once so the generated preview can carry it. */
  const inspectorSourceRef = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    // Same-origin fetch of the app's own asset — this is how the generated
    // preview gets the inspector without a second copy of it in this file.
    fetch("/inspector.js")
      .then((r) => (r.ok ? r.text() : Promise.reject(new Error(`HTTP ${r.status}`))))
      .then((text) => { if (!cancelled) inspectorSourceRef.current = text; })
      .catch(() => { /* Visual Edit reports the absence when it is switched on. */ });
    return () => { cancelled = true; };
  }, []);

  // A tab is mounted once it has been visited, and stays mounted after — the
  // editors hold unsaved work, so switching away must not throw it out.
  const [visited, setVisited] = useState<Set<DesignTab>>(() => new Set<DesignTab>(["preview"]));
  const openTab = useCallback((id: DesignTab) => {
    setActiveTab(id);
    setVisited((prev) => (prev.has(id) ? prev : new Set(prev).add(id)));
  }, []);

  // A deep link from the Hub's Settings tab names an editor in here. Panels
  // are lazy, so the first such link fires before this component exists — the
  // parked request is claimed on mount, the event covers every later one.
  useEffect(() => {
    const parked = takePendingDesignSubTab();
    if (parked && tabDefs.some((t) => t.id === parked)) openTab(parked as DesignTab);
    const handler = (e: Event) => {
      const id = (e as CustomEvent<unknown>).detail;
      if (typeof id === "string" && tabDefs.some((t) => t.id === id)) openTab(id as DesignTab);
    };
    window.addEventListener("vibecoder:design-subtab", handler);
    return () => window.removeEventListener("vibecoder:design-subtab", handler);
  }, [openTab]);

  // Where the preview iframe actually is. Read during render, this was the
  // value from *before* layout on first paint, and never updated when the
  // window resized — so the floating edit toolbar sat away from its element.
  useEffect(() => {
    const node = iframeContainerRef.current;
    if (!node) return;
    const measure = () => {
      const r = node.getBoundingClientRect();
      setIframeRect((prev) =>
        prev && prev.top === r.top && prev.left === r.left ? prev : { top: r.top, left: r.left },
      );
    };
    measure();
    // ResizeObserver is absent in jsdom and in some embedded webviews; the
    // scroll/resize listeners below are the floor, and the observer is the
    // improvement where it exists.
    const observer =
      typeof ResizeObserver === "function" ? new ResizeObserver(measure) : null;
    observer?.observe(node);
    window.addEventListener("scroll", measure, true);
    window.addEventListener("resize", measure);
    return () => {
      observer?.disconnect();
      window.removeEventListener("scroll", measure, true);
      window.removeEventListener("resize", measure);
    };
  }, [activeTab]);

  // Replacing the preview replaces the frame, which takes the inspector with
  // it. Without this the button still read "Exit Edit" over a frame that had
  // no inspector in it.
  useEffect(() => {
    setVisualEditEnabled(false);
    setSelectedElement(null);
    setInspectorError(null);
  }, [previewSrcdoc, previewUrl]);

  const loadComponentTree = useCallback(async () => {
    setTreeLoading(true);
    setTreeError(null);
    try {
      const tree = await invoke<ComponentTree>("design_component_tree", {
        workspacePath,
        workspace_path: workspacePath,
      });
      setComponentTree(tree);
    } catch (e) {
      // An unscannable workspace is not an empty one.
      setComponentTree(null);
      setTreeError(String(e));
    } finally {
      setTreeLoading(false);
    }
  }, [workspacePath]);

  // Build an inline HTML document that renders the generated component
  const buildPreviewSrcdoc = useCallback((code: string) => {
    // Defensive: extractGeneratedCode is also called at result-ingest time,
    // but if a caller passes raw text we still want a working preview.
    let clean = extractGeneratedCode(code);
    // Module-system shims: there's no bundler in the iframe, so imports and
    // re-exports must go. Everything else (TS types, JSX, generics, `as`
    // casts) is handled by Babel's TypeScript preset below — do NOT try to
    // pre-strip TS with regexes; that's what was producing "Script error.
    // Line: 0" because the regexes corrupted the code and Babel's failure
    // was hidden behind cross-origin scrubbing of window.onerror.
    clean = stripModuleSyntax(clean);

    // Find the top-level PascalCase component to mount.
    const nameMatch = clean.match(/(?:const|function|class)\s+([A-Z]\w*)/);
    const componentName = nameMatch?.[1] ?? "App";

    // JSON.stringify safely escapes the source for embedding in a JS string
    // literal (handles backticks, backslashes, newlines, quotes, </script>).
    const codeLiteral = JSON.stringify(clean);
    const nameLiteral = JSON.stringify(componentName);

    // The inspector travels with the preview instead of being injected later:
    // this frame is sandboxed into an opaque origin, so the parent cannot
    // reach into its document at all. It installs on request, so a preview the
    // user is only looking at carries no listeners.
    const inspectorBlock = inspectorSourceRef.current
      ? `<script>
(function(){
  var install = function(){ ${inspectorSourceRef.current} };
  window.addEventListener('message', function(e){
    if (e.data && e.data.type === 'vibe:activate-inspector') install();
  });
  window.parent.postMessage({ type: 'vibe:inspector-available' }, '*');
})();
</script>`
      : "";

    return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8"/>
<script src="https://unpkg.com/react@18/umd/react.development.js" crossorigin="anonymous"></script>
<script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js" crossorigin="anonymous"></script>
<script src="https://unpkg.com/@babel/standalone/babel.min.js" crossorigin="anonymous"></script>
<style>
  *, *::before, *::after { box-sizing: border-box; }
  body { margin: 0; padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #fff; color: #111; }
  #error-display { color: #e53e3e; padding: 16px; font-family: monospace; white-space: pre-wrap; font-size: 13px; }
</style>
</head>
<body>
<div id="root"></div>
<div id="error-display"></div>
<script>
(function(){
  function showError(label, e) {
    var msg = (e && e.message) ? e.message : String(e);
    var stack = (e && e.stack) ? '\\n\\n' + e.stack : '';
    document.getElementById('error-display').textContent = label + ': ' + msg + stack;
    if (window.console) console.error(label, e);
  }
  window.addEventListener('error', function(ev) {
    showError('Runtime error', ev.error || ev.message);
  });
  window.addEventListener('unhandledrejection', function(ev) {
    showError('Unhandled promise rejection', ev.reason);
  });

  if (typeof Babel === 'undefined') {
    showError('Setup error', new Error('Babel failed to load from unpkg.com — check network access'));
    return;
  }

  var source = ${codeLiteral};
  var componentName = ${nameLiteral};

  var transpiled;
  try {
    // Babel handles JSX *and* TypeScript — no regex-based TS stripping.
    transpiled = Babel.transform(source, {
      filename: 'component.tsx',
      presets: [
        ['react', { runtime: 'classic' }],
        ['typescript', { allExtensions: true, isTSX: true, onlyRemoveTypeImports: false }]
      ]
    }).code;
  } catch (e) {
    showError('Compile error', e);
    return;
  }

  var Comp;
  try {
    var body = 'const { useState, useEffect, useRef, useCallback, useMemo, useReducer, useContext, createContext, Fragment } = React;\\n'
      + transpiled
      + '\\nreturn (typeof ' + componentName + " !== 'undefined') ? " + componentName + ' : null;';
    Comp = (new Function('React', 'ReactDOM', body))(React, ReactDOM);
  } catch (e) {
    showError('Evaluation error', e);
    return;
  }

  if (!Comp) {
    showError('Render error', new Error('Component "' + componentName + '" was not defined at the top level. Declare it as: const ' + componentName + ' = (...) => {...}  or  function ' + componentName + '(...) {...}'));
    return;
  }

  // React 18's concurrent renderer queues work — a throw during render
  // happens off the synchronous call stack and would escape any try/catch
  // around .render(). Wrapping in an ErrorBoundary captures it WITH the
  // real message + component stack, instead of letting it surface to
  // window.onerror as a cross-origin-scrubbed "Script error.".
  function PreviewBoundary(props) {
    React.Component.call(this, props);
    this.state = { error: null, info: null };
  }
  PreviewBoundary.prototype = Object.create(React.Component.prototype);
  PreviewBoundary.prototype.constructor = PreviewBoundary;
  PreviewBoundary.getDerivedStateFromError = function(error) { return { error: error }; };
  PreviewBoundary.prototype.componentDidCatch = function(error, info) {
    this.setState({ info: info });
    var stack = (info && info.componentStack) ? '\\n\\nComponent stack:' + info.componentStack : '';
    var msg = (error && error.message) ? error.message : String(error);
    var errStack = (error && error.stack) ? '\\n\\n' + error.stack : '';
    document.getElementById('error-display').textContent = 'Render error: ' + msg + errStack + stack;
    if (window.console) console.error('Render error', error, info);
  };
  PreviewBoundary.prototype.render = function() {
    if (this.state.error) return null;
    return this.props.children;
  };

  try {
    ReactDOM.createRoot(document.getElementById('root')).render(
      React.createElement(PreviewBoundary, null, React.createElement(Comp))
    );
  } catch (e) {
    showError('Render error', e);
  }
})();
</script>
${inspectorBlock}
</body>
</html>`;
  }, []);

  if (!workspacePath) {
    return <div className="empty-state"><p>Open a workspace folder to use the design editor.</p></div>;
  }

  /**
   * Turn the element inspector on inside the preview frame.
   *
   * Two different frames, two different mechanisms:
   *  - a generated preview carries the inspector already (see
   *    `buildPreviewSrcdoc`) and only needs a message, because the frame is
   *    sandboxed into an opaque origin the parent cannot script;
   *  - an external URL has to be injected into, which only works when the page
   *    is same-origin with this app. It usually is not — a dev server on
   *    another port is another origin — and that used to fail into a
   *    `console.warn` nobody sees, leaving a "Visual Edit" button that lit up
   *    and did nothing.
   */
  const activateInspector = (): string | null => {
    const iframe = iframeRef.current;
    if (!iframe?.contentWindow) return "The preview frame is not ready yet.";

    if (previewSrcdoc) {
      if (!inspectorSourceRef.current) {
        return "The inspector script could not be loaded, so Visual Edit is unavailable.";
      }
      iframe.contentWindow.postMessage({ type: "vibe:activate-inspector" }, "*");
      return null;
    }

    try {
      const doc = iframe.contentDocument;
      if (!doc) throw new Error("no document");
      const script = doc.createElement("script");
      script.src = "/inspector.js";
      doc.head?.appendChild(script);
      return null;
    } catch {
      return (
        "Visual Edit cannot attach to this page: it is served from a different " +
        "origin than VibeCoder, so its document cannot be read. Generate a " +
        "component here and edit that preview instead."
      );
    }
  };

  const handleVisualEditToggle = () => {
    if (visualEditEnabled) {
      iframeRef.current?.contentWindow?.postMessage({ type: "vibe:deactivate-inspector" }, "*");
      setSelectedElement(null);
      setInspectorError(null);
      setVisualEditEnabled(false);
      return;
    }
    const failure = activateInspector();
    setInspectorError(failure);
    // Only claim the mode is on when the inspector actually attached.
    setVisualEditEnabled(failure === null);
  };

  const handleElementEdit = async (element: SelectedElement, instruction: string) => {
    setSelectedElement(element);
    setAiInstruction(instruction);
    setIsGenerating(true);
    setGenerationError(null);
    setGenerationResult("");
    openTab("inspector");
    try {
      const result = await invoke<string>("visual_edit_element", {
        workspacePath,
        selector: element.selector,
        instruction,
        currentHtml: element.outerHTML,
        reactComponent: element.reactComponent ?? null,
        provider,
      });
      setGenerationResult(result);
    } catch (e) {
      // The old catch swallowed this and wrote "Edit queued — check agent
      // output." into the result pane: a sentence describing work that had not
      // been queued and output that did not exist.
      setGenerationError(String(e));
    } finally {
      setIsGenerating(false);
    }
  };

  // Emit a CSS/HTML unified diff for the selected element.
  // The backend (`design_emit_diff`) rejects live-DOM-mutation payloads, so the
  // agent can only ever propose a source diff the user explicitly applies.
  const handleEmitDiff = async () => {
    if (!selectedElement || !diffInstruction.trim()) return;
    // `design_emit_diff` builds the provider itself, so it needs the id and the
    // model split out of the toolbar's display name. The sibling calls below
    // stay on the display name: they go through `set_provider_by_name`, which
    // matches on exactly that.
    const selection = parseProviderSelection(provider);
    if (!selection.provider || !selection.model) {
      setDiffError("Select a provider in the toolbar first.");
      return;
    }
    setDiffLoading(true);
    setDiffError(null);
    setDesignDiff(null);
    try {
      const result = await invoke<{ source_file: string; unified_diff: string }>(
        "design_emit_diff",
        {
          provider: selection.provider,
          model: selection.model,
          selector: selectedElement.selector,
          // Best-effort source label: the React component name when known.
          sourceFile: selectedElement.reactComponent ?? selectedElement.selector,
          snippet: selectedElement.outerHTML,
          instruction: diffInstruction,
          effort: getSelectedEffort(),
        },
      );
      setDesignDiff(result.unified_diff || "(no change needed)");
    } catch (e) {
      setDiffError(String(e));
    } finally {
      setDiffLoading(false);
    }
  };

  const handleGenerateComponent = async () => {
    if (!aiInstruction.trim()) return;
    if (!provider) {
      setGenerationError("No provider selected — pick one in the toolbar dropdown.");
      return;
    }
    setIsGenerating(true);
    setGenerationResult("");
    setGenerationError(null);
    setPreviewSrcdoc(null);
    try {
      // No `.catch(e => String(e))` here: that turned a provider error into a
      // string the panel then displayed as generated code and tried to render.
      const raw = await invoke<string>("generate_component", {
        workspacePath,
        description: aiInstruction,
        provider,
      });
      // Strip markdown fences + surrounding prose once, here, so the editor
      // shows clean code and the preview gets the same string the user sees.
      const result = extractGeneratedCode(raw);
      setGenerationResult(result);
      // Try to preview any result that looks like it contains JSX or a component
      const looksLikeCode = result && (
        result.includes("return (") || result.includes("return(") ||
        result.includes("export") || result.includes("function") ||
        result.includes("const ") || result.includes("useState") ||
        result.includes("<div") || result.includes("<>")
      );
      if (looksLikeCode) {
        setPreviewSrcdoc(buildPreviewSrcdoc(result));
        openTab("preview");
      }
    } catch (e) {
      setGenerationError(String(e));
    } finally {
      setIsGenerating(false);
    }
  };

  // ── Tab content renderers ───────────────────────────────────────────

  const renderPreview = () => (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, overflow: "hidden" }}>
      {/* Toolbar */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
        <input
          value={previewUrl}
          onChange={(e) => { setPreviewUrl(e.target.value); setBlockedError(false); }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              const url = normalizeUrl(previewUrl);
              setPreviewUrl(url);
              if (isBlockedUrl(url)) { setBlockedError(true); return; }
              setBlockedError(false);
              setPreviewSrcdoc(null);
              iframeRef.current?.setAttribute("src", url);
            }
          }}
          onBlur={() => {
            if (previewUrl.trim()) setPreviewUrl(normalizeUrl(previewUrl));
          }}
          style={{ flex: 1, background: "var(--bg-tertiary)", border: `1px solid ${blockedError ? "var(--error-color, #e53e3e)" : "var(--border-color)"}`, borderRadius: "var(--radius-xs-plus)", color: "inherit", padding: "4px 8px", fontSize: "var(--font-size-base)" }}
          placeholder={previewSrcdoc ? "Showing generated preview — enter URL to load external" : "https://example.com"}
        />
        <button
          onClick={() => {
            const url = normalizeUrl(previewUrl);
            setPreviewUrl(url);
            if (isBlockedUrl(url)) { setBlockedError(true); return; }
            setBlockedError(false);
            setPreviewSrcdoc(null);
            iframeRef.current?.setAttribute("src", url);
          }}
          style={{ background: "none", border: "none", cursor: "pointer", color: "inherit", fontSize: 16 }}
          title="Reload"
        >
          ↺
        </button>
        <button
          onClick={handleVisualEditToggle}
          style={{
            background: visualEditEnabled ? "var(--accent-color)" : "var(--bg-tertiary)",
            border: "1px solid var(--border-color)",
            borderRadius: "var(--radius-xs-plus)",
            padding: "3px 10px",
            cursor: "pointer",
            color: "inherit",
            fontSize: "var(--font-size-base)",
            fontWeight: 600,
          }}
          title="Toggle visual element selection"
        >
          {visualEditEnabled ? "Exit Edit" : "Visual Edit"}
        </button>
      </div>

      {inspectorError && (
        <div
          role="alert"
          style={{ padding: "6px 12px", background: "color-mix(in srgb, var(--warning-color) 12%, transparent)", color: "var(--warning-color)", fontSize: "var(--font-size-sm)", lineHeight: 1.5, flexShrink: 0 }}
        >
          {inspectorError}
        </div>
      )}

      {/* Iframe */}
      <div ref={iframeContainerRef} style={{ flex: 1, position: "relative", overflow: "auto" }}>
        {blockedError ? (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "100%", gap: 8, color: "var(--text-secondary)", padding: 32, textAlign: "center" }}>
            <div style={{ fontSize: 32 }}>⚠</div>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", color: "var(--text-primary)" }}>URL blocked</div>
            <div style={{ fontSize: "var(--font-size-md)", maxWidth: 360, lineHeight: 1.6 }}>
              This URL serves the VibeCoder editor and cannot be loaded in the preview pane. Enter an external URL to preview.
            </div>
          </div>
        ) : (
          <iframe
            ref={iframeRef}
            {...(previewSrcdoc ? { srcDoc: previewSrcdoc } : { src: previewUrl || "about:blank" })}
            title="Live Preview"
            // A srcdoc frame inherits the parent's origin when it is granted
            // `allow-same-origin`, which would let model-generated code reach
            // this app's document, storage and Tauri bridge. It is withheld
            // there and kept for an external URL, where the frame has its own
            // origin anyway and the page needs it to work at all.
            sandbox={
              previewSrcdoc
                ? "allow-scripts allow-forms allow-modals"
                : "allow-scripts allow-same-origin allow-forms allow-modals"
            }
            style={{ width: "100%", height: "100%", border: "none", background: "var(--bg-elevated)" }}
          />
        )}
        {visualEditEnabled && (
          <div style={{ position: "absolute", top: 0, left: 0, pointerEvents: "none", width: "100%", height: "100%" }}>
            <VisualEditor
              onEdit={handleElementEdit}
              iframeOffset={iframeRect ?? undefined}
            />
          </div>
        )}
      </div>
    </div>
  );

  const renderGenerate = () => (
    <div style={panelStyle}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 12 }}>Generate Component</div>
      <textarea
        value={aiInstruction}
        onChange={(e) => setAiInstruction(e.target.value)}
        placeholder="Describe a component to generate..."
        rows={5}
        style={{ width: "100%", resize: "vertical", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: 12, fontSize: "var(--font-size-md)", boxSizing: "border-box" }}
      />
      {!provider && (
        <div style={{ marginTop: 8, fontSize: "var(--font-size-base)", color: "var(--warning-color)" }}>
          Pick a provider in the toolbar dropdown before generating.
        </div>
      )}
      <button
        aria-label="Generate component"
        onClick={handleGenerateComponent}
        disabled={isGenerating || !aiInstruction.trim() || !provider}
        style={{ width: "100%", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "var(--radius-sm)", padding: "10px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", marginTop: 8, opacity: isGenerating || !aiInstruction.trim() ? 0.5 : 1 }}
      >
        {isGenerating ? "Generating..." : "Generate"}
      </button>

      {generationError && (
        <div
          role="alert"
          style={{ marginTop: 12, padding: 10, borderRadius: "var(--radius-sm)", border: "1px solid var(--error-color)", background: "color-mix(in srgb, var(--error-color) 10%, transparent)", color: "var(--error-color)", fontSize: "var(--font-size-base)", whiteSpace: "pre-wrap" }}
        >
          {generationError}
        </div>
      )}

      {generationResult && (
        <div style={{ marginTop: 16 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6, gap: 8 }}>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Generated Code</div>
            <div style={{ display: "flex", gap: 6 }}>
              <button
                onClick={() => {
                  setPreviewSrcdoc(buildPreviewSrcdoc(generationResult));
                  openTab("preview");
                }}
                style={{ background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "var(--radius-xs-plus)", padding: "4px 10px", cursor: "pointer", fontSize: "var(--font-size-sm)", fontWeight: 600 }}
              >
                {previewSrcdoc ? "Refresh Preview" : "Preview"}
              </button>
            </div>
          </div>
          <textarea
            value={generationResult}
            onChange={(e) => setGenerationResult(e.target.value)}
            spellCheck={false}
            style={{
              width: "100%",
              minHeight: 320,
              maxHeight: 600,
              fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
              fontSize: "var(--font-size-base)",
              color: "var(--text-primary)",
              background: "var(--bg-secondary)",
              borderRadius: "var(--radius-sm)",
              padding: 12,
              border: "1px solid var(--border-color)",
              resize: "vertical",
              boxSizing: "border-box",
              whiteSpace: "pre",
              tabSize: 2,
            }}
          />
        </div>
      )}
    </div>
  );

  const renderComponents = () => {
    const q = treeFilter.trim().toLowerCase();
    const shown = !componentTree
      ? []
      : q === ""
        ? componentTree.files
        : componentTree.files.filter(
            (f) =>
              f.path.toLowerCase().includes(q) ||
              f.components.some((c) => c.toLowerCase().includes(q)),
          );

    return (
      <div style={panelStyle}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)" }}>Components</div>
          <button
            onClick={loadComponentTree}
            disabled={treeLoading}
            style={{ background: "var(--accent-color)", color: "#fff", border: "none", borderRadius: "var(--radius-sm)", padding: "5px 12px", cursor: treeLoading ? "default" : "pointer", fontSize: "var(--font-size-base)", fontWeight: 600, opacity: treeLoading ? 0.6 : 1 }}
          >
            {treeLoading ? "Scanning…" : componentTree ? "Rescan" : "Scan workspace"}
          </button>
          {componentTree && (
            <input
              aria-label="Filter components"
              value={treeFilter}
              onChange={(e) => setTreeFilter(e.target.value)}
              placeholder="Filter by file or component…"
              style={{ flex: 1, minWidth: 160, background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "inherit", padding: "4px 8px", fontSize: "var(--font-size-base)" }}
            />
          )}
        </div>

        {treeError && (
          <div role="alert" style={{ color: "var(--error-color)", fontSize: "var(--font-size-md)", marginBottom: 12 }}>{treeError}</div>
        )}

        {!componentTree && !treeError && (
          <div style={{ fontSize: "var(--font-size-md)", color: "var(--text-secondary)", lineHeight: 1.6 }}>
            Scan the workspace to list the components it declares. This reads the
            source files — it is not a live render tree, so a component that is never
            rendered still appears here. The scan goes 8 directory levels deep and
            skips <code>node_modules</code>, <code>dist</code>, <code>build</code> and
            <code>target</code>.
          </div>
        )}

        {componentTree && (
          <>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 10 }}>
              {componentTree.component_count} component(s) across {componentTree.file_count} file(s)
              {componentTree.truncated && " — scan hit its file limit, so this is a sample"}
              {q !== "" && ` · showing ${shown.length}`}
            </div>
            {shown.length === 0 ? (
              <div style={{ fontSize: "var(--font-size-md)", color: "var(--text-secondary)" }}>
                {componentTree.file_count === 0
                  ? "No .tsx / .jsx / .vue / .svelte file in this workspace declares an exported component."
                  : `Nothing matches "${treeFilter}".`}
              </div>
            ) : (
              shown.map((file) => (
                <div key={file.path} style={{ marginBottom: 6, border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", overflow: "hidden" }}>
                  <button
                    type="button"
                    onClick={() => onOpenFile?.(file.path)}
                    disabled={!onOpenFile}
                    title={onOpenFile ? `Open ${file.path}` : file.path}
                    style={{ display: "flex", alignItems: "center", gap: 8, width: "100%", padding: "6px 10px", background: "var(--bg-secondary)", border: "none", color: "inherit", font: "inherit", textAlign: "left", cursor: onOpenFile ? "pointer" : "default" }}
                  >
                    <span style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {file.path}
                    </span>
                    <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{file.lines} lines</span>
                  </button>
                  <div style={{ padding: "6px 10px", display: "flex", flexWrap: "wrap", gap: 6 }}>
                    {file.components.map((name) => (
                      <span key={name} style={{ fontSize: "var(--font-size-xs)", fontFamily: "var(--font-mono)", padding: "2px 8px", borderRadius: 10, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>
                        {name}
                      </span>
                    ))}
                  </div>
                </div>
              ))
            )}
          </>
        )}
      </div>
    );
  };

  const renderInspector = () => (
    <div style={panelStyle}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 12 }}>Element Inspector</div>
      {selectedElement ? (
        <div>
          <div style={{ padding: 12, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", marginBottom: 12 }}>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Component</div>
            <div style={{ fontSize: "var(--font-size-lg)", fontFamily: "var(--font-mono)", fontWeight: 600 }}>
              {selectedElement.reactComponent ?? `<${selectedElement.tagName}>`}
            </div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 6, wordBreak: "break-all" }}>
              {selectedElement.selector}
            </div>
          </div>
          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Outer HTML</div>
          <pre style={{ fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 200, whiteSpace: "pre", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 10, border: "1px solid var(--border-color)" }}>
            {selectedElement.outerHTML}
          </pre>

          {/* A7 — diffcomplete: instruction → unified diff (no live DOM mutation) */}
          <div style={{ marginTop: 14, paddingTop: 12, borderTop: "1px solid var(--border-color)" }}>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 6 }}>
              Describe a change — get a reviewable diff
            </div>
            <input
              type="text"
              value={diffInstruction}
              onChange={(e) => setDiffInstruction(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleEmitDiff(); }}
              placeholder='e.g. "make the button green and larger"'
              style={{ width: "100%", padding: 8, fontSize: "var(--font-size-md)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", boxSizing: "border-box" }}
            />
            <button
              onClick={handleEmitDiff}
              disabled={diffLoading || !diffInstruction.trim()}
              style={{ marginTop: 8, padding: "6px 14px", fontSize: "var(--font-size-md)", borderRadius: "var(--radius-sm)", border: "none", background: "var(--accent-color)", color: "#fff", cursor: diffLoading ? "default" : "pointer", opacity: diffLoading || !diffInstruction.trim() ? 0.6 : 1 }}
            >
              {diffLoading ? "Generating…" : "Generate diff (⌘.)"}
            </button>
            {diffError && (
              <div style={{ marginTop: 8, fontSize: "var(--font-size-sm)", color: "var(--text-error, #e55)" }}>{diffError}</div>
            )}
            {designDiff && (
              <div style={{ marginTop: 10 }}>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>
                  Proposed diff — review and apply in the diff panel
                </div>
                <pre style={{ fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 320, whiteSpace: "pre", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 10, border: "1px solid var(--border-color)" }}>
                  {designDiff}
                </pre>
              </div>
            )}
          </div>

          {generationError && (
            <div role="alert" style={{ marginTop: 12, fontSize: "var(--font-size-base)", color: "var(--error-color)", whiteSpace: "pre-wrap" }}>
              {generationError}
            </div>
          )}

          {isGenerating && !generationResult && !generationError && (
            <div style={{ marginTop: 12, fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
              Asking {provider || "the selected model"} for an edit…
            </div>
          )}

          {generationResult && (
            <div style={{ marginTop: 12 }}>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Edit Result</div>
              <pre style={{ fontSize: "var(--font-size-sm)", color: "var(--text-success)", overflow: "auto", maxHeight: 300, whiteSpace: "pre", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 10, border: "1px solid var(--border-color)" }}>
                {generationResult}
              </pre>
            </div>
          )}
        </div>
      ) : (
        <div style={{ fontSize: "var(--font-size-md)", color: "var(--text-secondary)", lineHeight: 1.6 }}>
          No element selected. Go to the <button onClick={() => openTab("preview")} style={{ background: "none", border: "none", color: "var(--accent-color)", cursor: "pointer", padding: 0, fontSize: "inherit", textDecoration: "underline" }}>Preview</button> tab, enable <strong>Visual Edit</strong>, and click an element to inspect it.
        </div>
      )}
    </div>
  );

  /**
   * A tab's pane. Mounted the first time it is opened and kept mounted after,
   * so unsaved work in an editor survives a tab switch — but never mounted
   * before the user has asked for it.
   */
  const tabPane = (id: DesignTab, content: () => React.ReactNode) => {
    if (!visited.has(id)) return null;
    return (
      <div
        key={id}
        style={{
          flex: 1,
          overflow: "hidden",
          display: activeTab === id ? "flex" : "none",
          flexDirection: "column",
        }}
      >
        {content()}
      </div>
    );
  };

  /** An editor tab: lazy, so its chunk is fetched on first open. */
  const editorPane = (id: DesignTab, render: () => React.ReactNode) =>
    tabPane(id, () => (
      <Suspense fallback={<div style={panelStyle}>Loading editor…</div>}>{render()}</Suspense>
    ));

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-header" style={{ overflow: "auto", padding: 0 }}>
        {tabDefs.map(({ id, label }) => (
          <button
            key={id}
            onClick={() => openTab(id)}
            style={tabStyle(activeTab === id)}
          >
            {label}
            {id === "inspector" && selectedElement && (
              <span style={{ display: "inline-block", width: 6, height: 6, borderRadius: "50%", background: "var(--accent-color)", marginLeft: 6, verticalAlign: "middle" }} />
            )}
            {id === "generate" && isGenerating && (
              <span style={{ display: "inline-block", width: 6, height: 6, borderRadius: "50%", background: "var(--warning-color)", marginLeft: 6, verticalAlign: "middle" }} />
            )}
          </button>
        ))}
      </div>

      {/* Mounted on first visit, kept alive after — see `tabPane`. */}
      {tabPane("preview", renderPreview)}
      {tabPane("generate", renderGenerate)}
      {tabPane("components", renderComponents)}
      {tabPane("inspector", renderInspector)}
      {editorPane("drawio", () => <DrawioEditorPanel workspacePath={workspacePath} provider={provider} />)}
      {editorPane("pencil", () => <PencilPanel workspacePath={workspacePath} provider={provider} />)}
      {editorPane("penpot", () => <PenpotPanel workspacePath={workspacePath} provider={provider} />)}
      {editorPane("diagrams", () => <DiagramGeneratorPanel workspacePath={workspacePath} provider={provider} />)}
    </div>
  );
}
