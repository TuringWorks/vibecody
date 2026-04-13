/**
 * DesignMode — full-screen visual design editor with tabbed layout.
 *
 * Tabs: Preview | Generate | Components | Inspector | Figma
 */
import { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { VisualEditor, SelectedElement } from "./VisualEditor";
import { DrawioEditorPanel } from "./DrawioEditorPanel";
import { PencilPanel } from "./PencilPanel";
import { PenpotPanel } from "./PenpotPanel";
import { DiagramGeneratorPanel } from "./DiagramGeneratorPanel";

interface DesignModeProps {
  workspacePath: string | null;
  provider: string;
}

interface GeneratedFile {
  path: string;
  content: string;
}

type DesignTab = "preview" | "generate" | "components" | "inspector" | "figma" | "drawio" | "pencil" | "penpot" | "diagrams";

// Ports and hostnames that serve the VibeUI app itself — never load in the preview iframe.
const BLOCKED_PATTERNS = [
  /^https?:\/\/localhost:1420/i,   // Tauri dev server
  /^https?:\/\/127\.0\.0\.1:1420/i,
  /^tauri:\/\//i,                  // Tauri internal protocol
  /^https?:\/\/localhost:5173/i,   // Vite default (VibeUI dev)
  /^https?:\/\/127\.0\.0\.1:5173/i,
];

function isBlockedUrl(url: string): boolean {
  if (!url.trim()) return false;
  return BLOCKED_PATTERNS.some((p) => p.test(url.trim()));
}

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
  { id: "figma", label: "Figma" },
];

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "7px 16px",
  fontSize: 12,
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

export function DesignMode({ workspacePath, provider }: DesignModeProps) {
  const [activeTab, setActiveTab] = useState<DesignTab>("preview");
  const [previewUrl, setPreviewUrl] = useState("");
  const [blockedError, setBlockedError] = useState(false);
  const [visualEditEnabled, setVisualEditEnabled] = useState(false);
  const [selectedElement, setSelectedElement] = useState<SelectedElement | null>(null);
  const [aiInstruction, setAiInstruction] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [generationResult, setGenerationResult] = useState("");
  const [previewSrcdoc, setPreviewSrcdoc] = useState<string | null>(null);
  const [figmaUrl, setFigmaUrl] = useState("");
  const [figmaToken, setFigmaToken] = useState(() => localStorage.getItem("figma_token") ?? "");
  const [figmaSaveToken, setFigmaSaveToken] = useState(() => !!localStorage.getItem("figma_token"));
  const [figmaResult, setFigmaResult] = useState<GeneratedFile[]>([]);
  const [figmaExpandedFile, setFigmaExpandedFile] = useState<string | null>(null);
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const iframeContainerRef = useRef<HTMLDivElement>(null);

  // Build an inline HTML document that renders the generated component
  const buildPreviewSrcdoc = useCallback((code: string) => {
    let clean = code;
    // Strip markdown code fences
    clean = clean.replace(/^```[a-z]*\n?/gim, "").replace(/\n?```$/gm, "").trim();
    // Strip import statements
    clean = clean.replace(/^import\s+.*?['"].*?['"]\s*;?\s*$/gm, "");
    // Strip TypeScript interface/type blocks (multiline)
    clean = clean.replace(/^(export\s+)?(interface|type)\s+\w+[^{]*\{[^}]*\}\s*;?\s*$/gm, "");
    // Strip single-line type aliases
    clean = clean.replace(/^(export\s+)?type\s+\w+\s*=\s*[^;]+;\s*$/gm, "");
    // Strip TS type annotations from function params and return types
    clean = clean.replace(/:\s*React\.\w+(<[^>]*>)?/g, "");
    clean = clean.replace(/:\s*\w+(\[\])?\s*(?=[,)=\n{])/g, "");
    // Strip generic type params from hooks: useState<Foo> -> useState
    clean = clean.replace(/(useState|useRef|useCallback|useMemo|useReducer)<[^>]+>/g, "$1");
    // Strip 'as Type' casts
    clean = clean.replace(/\s+as\s+\w+(\[\])?/g, "");
    // Replace 'export default' with just the declaration
    clean = clean.replace(/export\s+default\s+/g, "");
    // Remove 'export' keyword from named exports
    clean = clean.replace(/^export\s+(?=(?:const|function|class)\s)/gm, "");

    const nameMatch = clean.match(/(?:const|function)\s+([A-Z]\w*)/);
    const componentName = nameMatch?.[1] ?? "App";

    return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8"/>
<script src="https://unpkg.com/react@18/umd/react.development.js" crossorigin><\/script>
<script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js" crossorigin><\/script>
<script src="https://unpkg.com/@babel/standalone/babel.min.js"><\/script>
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
window.onerror = function(msg, src, line, col, err) {
  document.getElementById('error-display').textContent = 'Error: ' + msg + '\\nLine: ' + line;
};
<\/script>
<script type="text/babel">
const { useState, useEffect, useRef, useCallback, useMemo, useReducer, useContext, createContext, Fragment } = React;
${clean}

try {
  const root = ReactDOM.createRoot(document.getElementById('root'));
  root.render(React.createElement(${componentName}));
} catch (e) {
  document.getElementById('error-display').textContent = 'Render error: ' + e.message;
}
<\/script>
</body>
</html>`;
  }, []);

  if (!workspacePath) {
    return <div className="empty-state"><p>Open a workspace folder to use the design editor.</p></div>;
  }

  const injectInspector = () => {
    const iframe = iframeRef.current;
    if (!iframe || !iframe.contentWindow) return;
    try {
      const script = iframe.contentDocument?.createElement("script");
      if (script) {
        script.src = "/inspector.js";
        iframe.contentDocument?.head?.appendChild(script);
      }
    } catch {
      console.warn("Cannot inject inspector into cross-origin iframe");
    }
  };

  const handleVisualEditToggle = () => {
    if (!visualEditEnabled) {
      injectInspector();
    } else {
      iframeRef.current?.contentWindow?.postMessage({ type: "vibe:deactivate-inspector" }, "*");
      setSelectedElement(null);
    }
    setVisualEditEnabled(!visualEditEnabled);
  };

  const handleElementEdit = async (element: SelectedElement, instruction: string) => {
    setSelectedElement(element);
    setAiInstruction(instruction);
    setIsGenerating(true);
    setActiveTab("inspector");
    try {
      const result = await invoke<string>("visual_edit_element", {
        workspacePath,
        selector: element.selector,
        instruction,
        currentHtml: element.outerHTML,
        reactComponent: element.reactComponent ?? null,
      }).catch(() => "Edit queued — check agent output.");
      setGenerationResult(result);
    } finally {
      setIsGenerating(false);
    }
  };

  const handleGenerateComponent = async () => {
    if (!aiInstruction.trim()) return;
    setIsGenerating(true);
    setGenerationResult("");
    setPreviewSrcdoc(null);
    try {
      const result = await invoke<string>("generate_component", {
        workspacePath,
        description: aiInstruction,
        provider,
      }).catch((e: unknown) => String(e));
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
        setActiveTab("preview");
      }
    } finally {
      setIsGenerating(false);
    }
  };

  const handleFigmaImport = async () => {
    if (!figmaUrl.trim() || !figmaToken.trim()) return;
    if (figmaSaveToken) localStorage.setItem("figma_token", figmaToken);
    else localStorage.removeItem("figma_token");
    setIsGenerating(true);
    setFigmaResult([]);
    setFigmaExpandedFile(null);
    try {
      const files = await invoke<GeneratedFile[]>("import_figma", {
        url: figmaUrl,
        token: figmaToken,
        workspacePath,
        provider,
      }).catch(() => [] as GeneratedFile[]);
      setFigmaResult(files);
    } finally {
      setIsGenerating(false);
    }
  };

  const iframeRect = iframeContainerRef.current?.getBoundingClientRect();

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
          style={{ flex: 1, background: "var(--bg-tertiary)", border: `1px solid ${blockedError ? "var(--error-color, #e53e3e)" : "var(--border-color)"}`, borderRadius: 4, color: "inherit", padding: "4px 8px", fontSize: 12 }}
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
            borderRadius: 4,
            padding: "3px 10px",
            cursor: "pointer",
            color: "inherit",
            fontSize: 12,
            fontWeight: 600,
          }}
          title="Toggle visual element selection"
        >
          {visualEditEnabled ? "Exit Edit" : "Visual Edit"}
        </button>
      </div>

      {/* Iframe */}
      <div ref={iframeContainerRef} style={{ flex: 1, position: "relative", overflow: "auto" }}>
        {blockedError ? (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "100%", gap: 8, color: "var(--text-secondary)", padding: 32, textAlign: "center" }}>
            <div style={{ fontSize: 32 }}>⚠</div>
            <div style={{ fontWeight: 600, fontSize: 14, color: "var(--text-primary)" }}>URL blocked</div>
            <div style={{ fontSize: 13, maxWidth: 360, lineHeight: 1.6 }}>
              This URL serves the VibeUI editor and cannot be loaded in the preview pane. Enter an external URL to preview.
            </div>
          </div>
        ) : (
          <iframe
            ref={iframeRef}
            {...(previewSrcdoc ? { srcDoc: previewSrcdoc } : { src: previewUrl || "about:blank" })}
            title="Live Preview"
            sandbox="allow-scripts allow-same-origin allow-forms allow-modals"
            style={{ width: "100%", height: "100%", border: "none", background: "var(--bg-elevated)" }}
          />
        )}
        {visualEditEnabled && (
          <div style={{ position: "absolute", top: 0, left: 0, pointerEvents: "none", width: "100%", height: "100%" }}>
            <VisualEditor
              onEdit={handleElementEdit}
              workspacePath={workspacePath}
              iframeOffset={iframeRect ? { top: iframeRect.top, left: iframeRect.left } : undefined}
            />
          </div>
        )}
      </div>
    </div>
  );

  const renderGenerate = () => (
    <div style={{ ...panelStyle, maxWidth: 600, margin: "0 auto" }}>
      <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 12 }}>Generate Component</div>
      <textarea
        value={aiInstruction}
        onChange={(e) => setAiInstruction(e.target.value)}
        placeholder="Describe a component to generate..."
        rows={5}
        style={{ width: "100%", resize: "vertical", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, color: "inherit", padding: 12, fontSize: 13, boxSizing: "border-box" }}
      />
      <button
        onClick={handleGenerateComponent}
        disabled={isGenerating || !aiInstruction.trim()}
        style={{ width: "100%", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: 6, padding: "10px 0", cursor: "pointer", fontWeight: 600, fontSize: 14, marginTop: 8, opacity: isGenerating || !aiInstruction.trim() ? 0.5 : 1 }}
      >
        {isGenerating ? "Generating..." : "Generate"}
      </button>

      {generationResult && (
        <div style={{ marginTop: 16 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
            <div style={{ fontWeight: 600, fontSize: 13 }}>Generated Code</div>
            {previewSrcdoc && (
              <button
                onClick={() => setActiveTab("preview")}
                style={{ background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: 4, padding: "4px 10px", cursor: "pointer", fontSize: 11, fontWeight: 600 }}
              >
                View Preview
              </button>
            )}
          </div>
          <pre style={{ fontSize: 12, color: "var(--text-success)", overflow: "auto", maxHeight: 500, whiteSpace: "pre", background: "var(--bg-secondary)", borderRadius: 6, padding: 12, border: "1px solid var(--border-color)" }}>
            {generationResult}
          </pre>
        </div>
      )}
    </div>
  );

  const renderComponents = () => (
    <div style={panelStyle}>
      <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 12 }}>Component Tree</div>
      <div style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.6 }}>
        Component tree from your project files will appear here once a live dev server is running at the preview URL.
      </div>
      <div style={{ marginTop: 20, padding: 16, background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)" }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Quick actions</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button
            onClick={() => setActiveTab("generate")}
            style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "8px 14px", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 }}
          >
            Generate Component
          </button>
          <button
            onClick={() => setActiveTab("figma")}
            style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "8px 14px", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 }}
          >
            Import from Figma
          </button>
        </div>
      </div>
    </div>
  );

  const renderInspector = () => (
    <div style={panelStyle}>
      <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 12 }}>Element Inspector</div>
      {selectedElement ? (
        <div>
          <div style={{ padding: 12, background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Component</div>
            <div style={{ fontSize: 14, fontFamily: "var(--font-mono)", fontWeight: 600 }}>
              {selectedElement.reactComponent ?? `<${selectedElement.tagName}>`}
            </div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 6, wordBreak: "break-all" }}>
              {selectedElement.selector}
            </div>
          </div>
          <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Outer HTML</div>
          <pre style={{ fontSize: 11, overflow: "auto", maxHeight: 200, whiteSpace: "pre", background: "var(--bg-secondary)", borderRadius: 6, padding: 10, border: "1px solid var(--border-color)" }}>
            {selectedElement.outerHTML}
          </pre>
          {generationResult && (
            <div style={{ marginTop: 12 }}>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Edit Result</div>
              <pre style={{ fontSize: 11, color: "var(--text-success)", overflow: "auto", maxHeight: 300, whiteSpace: "pre", background: "var(--bg-secondary)", borderRadius: 6, padding: 10, border: "1px solid var(--border-color)" }}>
                {generationResult}
              </pre>
            </div>
          )}
        </div>
      ) : (
        <div style={{ fontSize: 13, color: "var(--text-secondary)", lineHeight: 1.6 }}>
          No element selected. Go to the <button onClick={() => setActiveTab("preview")} style={{ background: "none", border: "none", color: "var(--accent-color)", cursor: "pointer", padding: 0, fontSize: "inherit", textDecoration: "underline" }}>Preview</button> tab, enable <strong>Visual Edit</strong>, and click an element to inspect it.
        </div>
      )}
    </div>
  );

  const renderFigma = () => {
    const steps = ["Connect", "Generate", "Review"];
    const currentStep = figmaResult.length > 0 ? 2 : isGenerating ? 1 : 0;
    const btnDisabled = isGenerating || !figmaUrl.trim() || !figmaToken.trim();
    return (
      <div style={{ ...panelStyle, maxWidth: 480 }}>
        {/* Workflow steps */}
        <div style={{ display: "flex", alignItems: "center", gap: 0, marginBottom: 14 }}>
          {steps.map((s, i) => (
            <div key={s} style={{ display: "flex", alignItems: "center", flex: i < steps.length - 1 ? 1 : undefined }}>
              <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 2 }}>
                <div style={{
                  width: 20, height: 20, borderRadius: "50%", fontSize: 10, fontWeight: 700,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  background: i <= currentStep ? "var(--accent-blue)" : "var(--bg-secondary)",
                  color: i <= currentStep ? "#fff" : "var(--text-secondary)",
                  border: `1px solid ${i <= currentStep ? "var(--accent-blue)" : "var(--border-color)"}`,
                }}>{i + 1}</div>
                <div style={{ fontSize: 9, color: i <= currentStep ? "var(--text-primary)" : "var(--text-secondary)", whiteSpace: "nowrap" }}>{s}</div>
              </div>
              {i < steps.length - 1 && (
                <div style={{ flex: 1, height: 1, background: i < currentStep ? "var(--accent-blue)" : "var(--border-color)", margin: "0 4px", marginBottom: 12 }} />
              )}
            </div>
          ))}
        </div>

        {/* Form */}
        <div style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", padding: "12px 14px", marginBottom: 10 }}>
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 10, lineHeight: 1.5 }}>
            Get your token from <em>Figma → Settings → Personal access tokens</em>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 3 }}>Figma File URL</div>
              <input
                value={figmaUrl}
                onChange={(e) => setFigmaUrl(e.target.value)}
                placeholder="https://www.figma.com/file/…"
                style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 5, color: "inherit", padding: "5px 8px", fontSize: 12, boxSizing: "border-box" }}
              />
            </div>
            <div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 3 }}>Personal Access Token</div>
              <input
                type="password"
                value={figmaToken}
                onChange={(e) => setFigmaToken(e.target.value)}
                placeholder="figd_…"
                style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 5, color: "inherit", padding: "5px 8px", fontSize: 12, boxSizing: "border-box" }}
              />
            </div>
            <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--text-secondary)", cursor: "pointer" }}>
              <input
                type="checkbox"
                checked={figmaSaveToken}
                onChange={(e) => setFigmaSaveToken(e.target.checked)}
              />
              Remember token on this device
            </label>
          </div>
        </div>

        <button
          onClick={handleFigmaImport}
          disabled={btnDisabled}
          style={{ width: "100%", background: "var(--accent-blue)", color: "#fff", border: "none", borderRadius: 6, padding: "8px 0", cursor: btnDisabled ? "not-allowed" : "pointer", fontWeight: 600, fontSize: 13, opacity: btnDisabled ? 0.5 : 1, marginBottom: 14 }}
        >
          {isGenerating ? "Importing…" : "Import & Generate Components"}
        </button>

        {/* Results */}
        {figmaResult.length > 0 && (
          <div>
            <div style={{ fontSize: 12, color: "var(--text-success)", fontWeight: 600, marginBottom: 8 }}>
              {figmaResult.length} component{figmaResult.length > 1 ? "s" : ""} generated — click a file to preview
            </div>
            {figmaResult.map((f) => (
              <div key={f.path} style={{ marginBottom: 6, borderRadius: 6, border: "1px solid var(--border-color)", overflow: "hidden" }}>
                <div
                  onClick={() => setFigmaExpandedFile(figmaExpandedFile === f.path ? null : f.path)}
                  style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 10px", background: "var(--bg-secondary)", cursor: "pointer" }}
                >
                  <span style={{ flex: 1, fontSize: 11, fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.path}</span>
                  <button
                    onClick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(f.content); }}
                    title="Copy code"
                    style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: 10, padding: "2px 4px", borderRadius: 3 }}
                  >
                    Copy
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      setPreviewSrcdoc(buildPreviewSrcdoc(f.content));
                      setActiveTab("preview");
                    }}
                    title="Preview in browser"
                    style={{ background: "none", border: "none", color: "var(--accent-blue)", cursor: "pointer", fontSize: 10, padding: "2px 4px", borderRadius: 3 }}
                  >
                    Preview
                  </button>
                </div>
                {figmaExpandedFile === f.path && (
                  <pre style={{ margin: 0, padding: "10px 12px", fontSize: 10, lineHeight: 1.5, overflow: "auto", maxHeight: 220, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>
                    <code>{f.content}</code>
                  </pre>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    );
  };

  // Helper: wrap each tab so it stays mounted but hidden when inactive
  const tabPane = (id: DesignTab, content: React.ReactNode) => (
    <div
      key={id}
      style={{
        flex: 1,
        overflow: "hidden",
        display: activeTab === id ? "flex" : "none",
        flexDirection: "column",
      }}
    >
      {content}
    </div>
  );

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-header" style={{ overflow: "auto", padding: 0 }}>
        {tabDefs.map(({ id, label }) => (
          <button
            key={id}
            onClick={() => setActiveTab(id)}
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

      {/* All tabs always mounted — hidden via display:none to preserve state */}
      {tabPane("preview", renderPreview())}
      {tabPane("generate", renderGenerate())}
      {tabPane("components", renderComponents())}
      {tabPane("inspector", renderInspector())}
      {tabPane("drawio", <DrawioEditorPanel workspacePath={workspacePath} provider={provider} />)}
      {tabPane("pencil", <PencilPanel workspacePath={workspacePath} provider={provider} />)}
      {tabPane("penpot", <PenpotPanel workspacePath={workspacePath} provider={provider} />)}
      {tabPane("diagrams", <DiagramGeneratorPanel workspacePath={workspacePath} provider={provider} />)}
      {tabPane("figma", renderFigma())}
    </div>
  );
}
