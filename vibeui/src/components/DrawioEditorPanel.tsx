/**
 * DrawioEditorPanel — full draw.io editor + deep integration.
 *
 * Tabs: Editor | Preview | Generate | Templates | MCP
 * - Editor: Embed draw.io via app.diagrams.net in iframe with postMessage bridge
 * - Preview: Read-only viewer.diagrams.net embed
 * - Generate: AI-powered diagram generation from description
 * - Templates: Architecture, flowchart, ERD, sequence, C4 templates
 * - MCP: drawio-mcp command builder for file operations
 */
import { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";

interface DrawioEditorPanelProps {
  workspacePath: string | null;
  provider: string;
}

type DioTab = "editor" | "preview" | "generate" | "templates" | "mcp";

const TAB_DEFS: { id: DioTab; label: string }[] = [
  { id: "editor", label: "Editor" },
  { id: "preview", label: "Preview" },
  { id: "generate", label: "AI Generate" },
  { id: "templates", label: "Templates" },
  { id: "mcp", label: "MCP Bridge" },
];

const DIAGRAM_KINDS = [
  "flowchart", "sequence", "class_diagram", "entity_relationship",
  "component_diagram", "deployment_diagram", "c4_context", "c4_container",
  "c4_component", "architecture", "state_machine", "network_topology",
];

const TEMPLATES = [
  { id: "microservices", label: "Microservices Architecture", kind: "architecture" },
  { id: "ci_cd", label: "CI/CD Pipeline", kind: "flowchart" },
  { id: "er_saas", label: "SaaS ERD", kind: "entity_relationship" },
  { id: "c4_context", label: "C4 Context", kind: "c4_context" },
  { id: "c4_container", label: "C4 Container", kind: "c4_container" },
  { id: "api_sequence", label: "REST API Sequence", kind: "sequence" },
  { id: "state_order", label: "Order State Machine", kind: "state_machine" },
  { id: "domain_model", label: "Domain Class Diagram", kind: "class_diagram" },
];

export function DrawioEditorPanel({ workspacePath, provider }: DrawioEditorPanelProps) {
  const [activeTab, setActiveTab] = useState<DioTab>("editor");
  const [diagramXml, setDiagramXml] = useState("");
  const [previewXml, setPreviewXml] = useState("");
  const [genDescription, setGenDescription] = useState("");
  const [genKind, setGenKind] = useState("flowchart");
  const [isGenerating, setIsGenerating] = useState(false);
  const [generatedXml, setGeneratedXml] = useState("");
  const [mcpFilePath, setMcpFilePath] = useState("");
  const [mcpCommand, setMcpCommand] = useState("read_file");
  const [mcpResult, setMcpResult] = useState("");
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [statusMsg, setStatusMsg] = useState("");
  const editorRef = useRef<HTMLIFrameElement>(null);

  const showStatus = (msg: string) => {
    setStatusMsg(msg);
    setTimeout(() => setStatusMsg(""), 3000);
  };

  // ── Editor tab: embed draw.io editor ──────────────────────────────────

  const editorSrc = "https://embed.diagrams.net/?embed=1&ui=dark&spin=1&proto=json&configure=1&noSaveBtn=1";

  const handleEditorMessage = useCallback((event: MessageEvent) => {
    if (!event.data || typeof event.data !== "string") return;
    try {
      const msg = JSON.parse(event.data);
      if (msg.event === "init") {
        // Load existing XML if any
        editorRef.current?.contentWindow?.postMessage(
          JSON.stringify({ action: "load", xml: diagramXml || "<mxGraphModel><root><mxCell id='0'/><mxCell id='1' parent='0'/></root></mxGraphModel>" }),
          "*"
        );
      } else if (msg.event === "save" || msg.event === "export") {
        if (msg.xml) {
          setDiagramXml(msg.xml);
          setPreviewXml(msg.xml);
          showStatus("Diagram saved");
        }
      } else if (msg.event === "autosave" && msg.xml) {
        setDiagramXml(msg.xml);
      }
    } catch {
      // ignore non-JSON messages
    }
  }, [diagramXml]);

  // ── AI generation ─────────────────────────────────────────────────────

  const handleGenerate = async () => {
    if (!genDescription.trim()) return;
    setIsGenerating(true);
    setGeneratedXml("");
    try {
      const result = await invoke<string>("generate_drawio_xml", {
        description: genDescription,
        kind: genKind,
        workspacePath,
        provider,
      }).catch(() => "");
      if (result) {
        setGeneratedXml(result);
        setPreviewXml(result);
        setActiveTab("preview");
        showStatus("Diagram generated — preview opened");
      }
    } finally {
      setIsGenerating(false);
    }
  };

  const loadGeneratedInEditor = () => {
    setDiagramXml(generatedXml);
    setActiveTab("editor");
    editorRef.current?.contentWindow?.postMessage(
      JSON.stringify({ action: "load", xml: generatedXml }),
      "*"
    );
  };

  // ── Templates ─────────────────────────────────────────────────────────

  const loadTemplate = async (templateId: string) => {
    setSelectedTemplate(templateId);
    try {
      const xml = await invoke<string>("get_drawio_template", {
        templateId,
        workspacePath,
      }).catch(() => "");
      if (xml) {
        setDiagramXml(xml);
        setPreviewXml(xml);
        setActiveTab("preview");
        showStatus(`Template "${templateId}" loaded`);
      }
    } catch {
      showStatus("Template load failed — using preview placeholder");
      setPreviewXml(`<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="2" value="${templateId}" style="rounded=1;whiteSpace=wrap;" vertex="1" parent="1"><mxGeometry x="200" y="200" width="200" height="60" as="geometry"/></mxCell></root></mxGraphModel>`);
      setActiveTab("preview");
    }
  };

  // ── MCP bridge ────────────────────────────────────────────────────────

  const executeMcpCommand = async () => {
    if (!mcpFilePath.trim()) { showStatus("Enter a file path"); return; }
    try {
      const result = await invoke<string>("execute_drawio_mcp", {
        command: mcpCommand,
        filePath: mcpFilePath,
        content: mcpCommand === "write_file" ? diagramXml : undefined,
      }).catch((e: unknown) => String(e));
      setMcpResult(result);
    } catch (e) {
      setMcpResult(String(e));
    }
  };

  // ── Preview using viewer.diagrams.net ─────────────────────────────────

  const buildViewerSrc = (xml: string) => {
    const encoded = encodeURIComponent(xml);
    return `https://viewer.diagrams.net/?lightbox=0&highlight=0000ff&edit=_blank&layers=1&nav=1&title=Diagram#R${encoded}`;
  };

  // ── Render ────────────────────────────────────────────────────────────

  const renderEditor = () => (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      <div style={{ padding: "8px 12px", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", display: "flex", gap: 8, flexShrink: 0 }}>
        <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: "28px" }}>
          Full draw.io editor — save with Ctrl+S or File → Save
        </span>
        <button className="panel-btn panel-btn-secondary panel-btn-sm"
          onClick={() => { setPreviewXml(diagramXml); setActiveTab("preview"); }}
          disabled={!diagramXml}
          style={{ marginLeft: "auto" }}
        >
          Preview
        </button>
        {workspacePath && (
          <button
            onClick={async () => {
              await invoke("save_drawio_file", { xml: diagramXml, workspacePath }).catch(() => {});
              showStatus("Saved to workspace");
            }}
            disabled={!diagramXml}
            style={{ background: "var(--accent-blue)", border: "none", borderRadius: "var(--radius-xs-plus)", padding: "4px 12px", cursor: "pointer", color: "var(--btn-primary-fg, #fff)", fontSize: "var(--font-size-base)", fontWeight: 600 }}
          >
            Save
          </button>
        )}
      </div>
      <iframe
        ref={editorRef}
        src={editorSrc}
        title="Draw.io Editor"
        onLoad={() => {
          window.addEventListener("message", handleEditorMessage);
        }}
        sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
        style={{ flex: 1, border: "none" }}
      />
    </div>
  );

  const renderPreview = () => (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      {previewXml ? (
        <iframe
          src={buildViewerSrc(previewXml)}
          title="Diagram Preview"
          sandbox="allow-scripts allow-same-origin"
          style={{ flex: 1, border: "none", background: "var(--btn-primary-fg)" }}
        />
      ) : (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 12, color: "var(--text-secondary)" }}>
          <Icon name="chart-bar" size={48} style={{ opacity: 0.3 }} />
          <div style={{ fontSize: "var(--font-size-lg)" }}>No diagram to preview</div>
          <div style={{ fontSize: "var(--font-size-base)", maxWidth: 300, textAlign: "center", lineHeight: 1.6 }}>
            Create a diagram in the Editor tab, generate one with AI, or load a template.
          </div>
        </div>
      )}
    </div>
  );

  const renderGenerate = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 20, maxWidth: 600, margin: "0 auto" }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 16 }}>AI Diagram Generation</div>
      <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Diagram Type</label>
      <select
        value={genKind}
        onChange={(e) => setGenKind(e.target.value)}
        style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "8px 12px", fontSize: "var(--font-size-md)", marginBottom: 14 }}
      >
        {DIAGRAM_KINDS.map((k) => (
          <option key={k} value={k}>{k.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}</option>
        ))}
      </select>
      <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Description</label>
      <textarea
        value={genDescription}
        onChange={(e) => setGenDescription(e.target.value)}
        placeholder="Describe the diagram you want to generate..."
        rows={6}
        style={{ width: "100%", resize: "vertical", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: 10, fontSize: "var(--font-size-md)", boxSizing: "border-box" }}
      />
      <button className="panel-btn"
        onClick={handleGenerate}
        disabled={isGenerating || !genDescription.trim()}
        style={{ width: "100%", marginTop: 8, background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isGenerating || !genDescription.trim() ? 0.5 : 1 }}
      >
        {isGenerating ? "Generating…" : "Generate Diagram"}
      </button>
      {generatedXml && (
        <div style={{ marginTop: 16 }}>
          <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
            <button className="panel-btn panel-btn-secondary" onClick={() => setActiveTab("preview")} style={{ flex: 1 }}>View Preview</button>
            <button className="panel-btn panel-btn-primary" onClick={loadGeneratedInEditor} style={{ flex: 1 }}>Open in Editor</button>
          </div>
          <pre style={{ fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 300, whiteSpace: "pre", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 10, border: "1px solid var(--border-color)", color: "var(--text-success)" }}>
            {generatedXml.slice(0, 800)}{generatedXml.length > 800 ? "\n…" : ""}
          </pre>
        </div>
      )}
    </div>
  );

  const renderTemplates = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 12 }}>Diagram Templates</div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))", gap: 10 }}>
        {TEMPLATES.map((t) => (
          <button
            key={t.id}
            onClick={() => loadTemplate(t.id)}
            style={{
              background: selectedTemplate === t.id ? "var(--accent-blue)" : "var(--bg-secondary)",
              border: `1px solid ${selectedTemplate === t.id ? "var(--accent-blue)" : "var(--border-color)"}`,
              borderRadius: "var(--radius-sm-alt)",
              padding: "16px 16px",
              cursor: "pointer",
              color: selectedTemplate === t.id ? "var(--btn-primary-fg)" : "inherit",
              textAlign: "left",
              transition: "all 0.15s",
            }}
          >
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 4 }}>{t.label}</div>
            <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.75 }}>{t.kind.replace(/_/g, " ")}</div>
          </button>
        ))}
      </div>
    </div>
  );

  const renderMcp = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 20, maxWidth: 600, margin: "0 auto" }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 4 }}>drawio-mcp Bridge</div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.6 }}>
        Execute draw.io MCP commands (requires jgraph/drawio-mcp server running).
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        {["read_file", "write_file", "list_pages", "export_svg"].map((cmd) => (
          <button
            key={cmd}
            onClick={() => setMcpCommand(cmd)}
            style={{
              background: mcpCommand === cmd ? "var(--accent-blue)" : "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              borderRadius: "var(--radius-xs-plus)",
              padding: "4px 12px",
              cursor: "pointer",
              color: mcpCommand === cmd ? "var(--btn-primary-fg)" : "inherit",
              fontSize: "var(--font-size-sm)",
              fontWeight: mcpCommand === cmd ? 600 : 400,
            }}
          >
            {cmd}
          </button>
        ))}
      </div>
      <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>File Path</label>
      <input
        value={mcpFilePath}
        onChange={(e) => setMcpFilePath(e.target.value)}
        placeholder="/path/to/diagram.drawio"
        style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "8px 12px", fontSize: "var(--font-size-md)", marginBottom: 12, boxSizing: "border-box" }}
      />
      <button className="panel-btn"
        onClick={executeMcpCommand}
        style={{ width: "100%", background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)" }}
      >
        Execute {mcpCommand}
      </button>
      {mcpResult && (
        <pre style={{ marginTop: 16, fontSize: "var(--font-size-base)", overflow: "auto", maxHeight: 400, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)", color: "var(--text-primary)", whiteSpace: "pre-wrap" }}>
          {mcpResult}
        </pre>
      )}
    </div>
  );

  if (!workspacePath) {
    return <div className="empty-state"><p>Open a workspace to use the Draw.io editor.</p></div>;
  }

  return (
    <div className="panel-container">
      <div className="panel-tab-bar" style={{ overflow: "auto" }}>
        {TAB_DEFS.map(({ id, label }) => (
          <button className={`panel-tab${activeTab === id ? " active" : ""}`} key={id} onClick={() => setActiveTab(id)}>
            {label}
          </button>
        ))}
        {statusMsg && (
          <span style={{ marginLeft: "auto", marginRight: 12, fontSize: "var(--font-size-sm)", color: "var(--text-success)", lineHeight: "30px" }}>
            ✓ {statusMsg}
          </span>
        )}
      </div>
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {activeTab === "editor" && renderEditor()}
        {activeTab === "preview" && renderPreview()}
        {activeTab === "generate" && renderGenerate()}
        {activeTab === "templates" && renderTemplates()}
        {activeTab === "mcp" && renderMcp()}
      </div>
    </div>
  );
}
