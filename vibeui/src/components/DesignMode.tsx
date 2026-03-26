/**
 * DesignMode — full-screen visual design editor with tabbed layout.
 *
 * Tabs: Preview | Generate | Components | Inspector | Figma
 */
import { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { VisualEditor, SelectedElement } from "./VisualEditor";

interface DesignModeProps {
  workspacePath: string | null;
  provider: string;
}

interface GeneratedFile {
  path: string;
  content: string;
}

type DesignTab = "preview" | "generate" | "components" | "inspector" | "figma";

const tabDefs: { id: DesignTab; label: string }[] = [
  { id: "preview", label: "Preview" },
  { id: "generate", label: "Generate" },
  { id: "components", label: "Components" },
  { id: "inspector", label: "Inspector" },
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
  const [previewUrl, setPreviewUrl] = useState("http://localhost:5173");
  const [visualEditEnabled, setVisualEditEnabled] = useState(false);
  const [selectedElement, setSelectedElement] = useState<SelectedElement | null>(null);
  const [aiInstruction, setAiInstruction] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [generationResult, setGenerationResult] = useState("");
  const [previewSrcdoc, setPreviewSrcdoc] = useState<string | null>(null);
  const [figmaUrl, setFigmaUrl] = useState("");
  const [figmaToken, setFigmaToken] = useState("");
  const [figmaResult, setFigmaResult] = useState<GeneratedFile[]>([]);
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const iframeContainerRef = useRef<HTMLDivElement>(null);

  // Build an inline HTML document that renders the generated component
  const buildPreviewSrcdoc = useCallback((code: string) => {
    let clean = code.replace(/^```[a-z]*\n?/i, "").replace(/\n?```$/, "").trim();
    clean = clean.replace(/^import\s+.*?['"]\s*;?\s*$/gm, "");
    const nameMatch = clean.match(/(?:const|function)\s+([A-Z]\w*)/);
    const componentName = nameMatch?.[1] ?? "App";
    clean = clean.replace(/^(export\s+)?(interface|type)\s+\w+[\s\S]*?^\}/gm, "");
    clean = clean.replace(/:\s*React\.FC<\w+>/g, "");
    clean = clean.replace(/<(\w+)Props>/g, "");
    clean = clean.replace(/useState<[^>]+>/g, "useState");
    return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8"/>
<script src="https://unpkg.com/react@18/umd/react.development.js" crossorigin></script>
<script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js" crossorigin></script>
<script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
<style>
  *, *::before, *::after { box-sizing: border-box; }
  body { margin: 0; padding: 16px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
</style>
</head>
<body>
<div id="root"></div>
<script type="text/babel" data-type="module">
const { useState, useEffect, useRef, useCallback, useMemo } = React;
${clean}

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(React.createElement(${componentName}));
</script>
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
      if (result && (result.includes("React") || result.includes("useState") || result.includes("return (") || result.includes("export"))) {
        setPreviewSrcdoc(buildPreviewSrcdoc(result));
        setActiveTab("preview");
      }
    } finally {
      setIsGenerating(false);
    }
  };

  const handleFigmaImport = async () => {
    if (!figmaUrl.trim() || !figmaToken.trim()) return;
    setIsGenerating(true);
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
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Toolbar */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
        <input
          value={previewUrl}
          onChange={(e) => setPreviewUrl(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") { setPreviewSrcdoc(null); iframeRef.current?.setAttribute("src", previewUrl); } }}
          style={{ flex: 1, background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "inherit", padding: "4px 8px", fontSize: 12 }}
          placeholder={previewSrcdoc ? "Showing generated preview — enter URL to load external" : "http://localhost:5173"}
        />
        <button
          onClick={() => { setPreviewSrcdoc(null); iframeRef.current?.setAttribute("src", previewUrl); }}
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
        <iframe
          ref={iframeRef}
          {...(previewSrcdoc ? { srcDoc: previewSrcdoc } : { src: previewUrl })}
          title="Live Preview"
          sandbox="allow-scripts allow-same-origin allow-forms allow-modals"
          style={{ width: "100%", height: "100%", border: "none", background: "var(--bg-elevated)" }}
        />
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

  const renderFigma = () => (
    <div style={{ ...panelStyle, maxWidth: 500, margin: "0 auto" }}>
      <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 12 }}>Import from Figma</div>
      <label style={{ fontSize: 12, display: "block", marginBottom: 4, color: "var(--text-secondary)" }}>Figma File URL</label>
      <input
        value={figmaUrl}
        onChange={(e) => setFigmaUrl(e.target.value)}
        placeholder="https://www.figma.com/file/..."
        style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, color: "inherit", padding: "8px 10px", fontSize: 13, marginBottom: 14, boxSizing: "border-box" }}
      />
      <label style={{ fontSize: 12, display: "block", marginBottom: 4, color: "var(--text-secondary)" }}>Figma API Token</label>
      <input
        type="password"
        value={figmaToken}
        onChange={(e) => setFigmaToken(e.target.value)}
        placeholder="figd_..."
        style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, color: "inherit", padding: "8px 10px", fontSize: 13, marginBottom: 16, boxSizing: "border-box" }}
      />
      <button
        onClick={handleFigmaImport}
        disabled={isGenerating || !figmaUrl.trim() || !figmaToken.trim()}
        style={{ width: "100%", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: 6, padding: "10px 0", cursor: "pointer", fontWeight: 600, fontSize: 14, opacity: isGenerating || !figmaUrl.trim() || !figmaToken.trim() ? 0.5 : 1 }}
      >
        {isGenerating ? "Importing..." : "Import"}
      </button>
      {figmaResult.length > 0 && (
        <div style={{ marginTop: 16, padding: 12, background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)" }}>
          <div style={{ fontSize: 13, color: "var(--text-success)", marginBottom: 8, fontWeight: 600 }}>Generated {figmaResult.length} file(s)</div>
          {figmaResult.map((f) => (
            <div key={f.path} style={{ fontSize: 12, fontFamily: "var(--font-mono)", color: "var(--text-secondary)", padding: "2px 0" }}>{f.path}</div>
          ))}
        </div>
      )}
    </div>
  );

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
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)", overflow: "hidden" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", alignItems: "center", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, overflow: "auto" }}>
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
      {tabPane("figma", renderFigma())}
    </div>
  );
}
