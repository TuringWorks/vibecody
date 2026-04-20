/**
 * PencilPanel — Evolus Pencil .ep format + TuringWorks Pencil MCP integration.
 *
 * Tabs: Templates | Import | MCP | Export
 * - Templates: Landing page, dashboard, mobile app wireframe generation
 * - Import: Parse .ep XML files and display structure
 * - MCP: TuringWorks Pencil MCP command builder for .pen files
 * - Export: Download EP XML, SVG, or generate React components
 */
import { useState } from "react";
import { Icon } from "./Icon";
import { invoke } from "@tauri-apps/api/core";

interface PencilPanelProps {
  workspacePath: string | null;
  provider: string;
}

type PencilTab = "templates" | "import" | "mcp" | "export";

const TAB_DEFS: { id: PencilTab; label: string }[] = [
  { id: "templates", label: "Templates" },
  { id: "import", label: "Import" },
  { id: "mcp", label: "Pencil MCP" },
  { id: "export", label: "Export" },
];

const WIREFRAME_TEMPLATES = [
  { id: "landing_page", label: "Landing Page", icon: "layout-grid", description: "Hero section, nav, features, footer" },
  { id: "dashboard", label: "Dashboard", icon: "chart-bar", description: "Sidebar, stats, chart, activity" },
  { id: "mobile_app", label: "Mobile App", icon: "monitor-play", description: "Status bar, nav, tab bar screens" },
  { id: "login_form", label: "Login Form", icon: "lock", description: "Email/password login with social auth" },
  { id: "settings_page", label: "Settings Page", icon: "settings", description: "Grouped settings with toggle switches" },
  { id: "data_table", label: "Data Table", icon: "clipboard-list", description: "Filterable sortable data table view" },
] as const;


interface GeneratedWireframe {
  title: string;
  pages: Array<{ name: string; shapes: number }>;
  epXml: string;
}

export function PencilPanel({ workspacePath, provider }: PencilPanelProps) {
  const [activeTab, setActiveTab] = useState<PencilTab>("templates");
  const [generatedWireframe, setGeneratedWireframe] = useState<GeneratedWireframe | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [customTitle, setCustomTitle] = useState("");
  const [customSections, setCustomSections] = useState("Overview,Analytics,Settings,Users");
  const [importXml, setImportXml] = useState("");
  const [parseResult, setParseResult] = useState<string>("");
  const [mcpOp, setMcpOp] = useState("get_editor_state");
  const [mcpPath, setMcpPath] = useState("");
  const [mcpResult, setMcpResult] = useState("");
  const [exportFormat, setExportFormat] = useState("ep_xml");
  const [statusMsg, setStatusMsg] = useState("");

  const showStatus = (msg: string) => {
    setStatusMsg(msg);
    setTimeout(() => setStatusMsg(""), 3000);
  };

  const generateWireframe = async (templateId: string) => {
    setSelectedTemplate(templateId);
    setIsLoading(true);
    const title = customTitle || templateId.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
    const sections = customSections.split(",").map((s) => s.trim()).filter(Boolean);
    try {
      const result = await invoke<GeneratedWireframe>("generate_pencil_wireframe", {
        templateId, title, sections, workspacePath, provider,
      }).catch(() => null);
      if (result) {
        setGeneratedWireframe(result);
        showStatus(`Wireframe generated: ${result.pages.length} page(s)`);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const parseEpXml = async () => {
    if (!importXml.trim()) return;
    setIsLoading(true);
    try {
      const result = await invoke<string>("parse_pencil_ep", { xml: importXml }).catch((e: unknown) => String(e));
      setParseResult(result);
    } finally {
      setIsLoading(false);
    }
  };

  const executeMcpOp = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<string>("execute_pencil_mcp", {
        operation: mcpOp,
        filePath: mcpPath || undefined,
      }).catch((e: unknown) => String(e));
      setMcpResult(result);
    } finally {
      setIsLoading(false);
    }
  };

  const exportWireframe = async () => {
    if (!generatedWireframe) return;
    setIsLoading(true);
    try {
      const result = await invoke<string>("export_pencil_wireframe", {
        xml: generatedWireframe.epXml,
        format: exportFormat,
        workspacePath,
        provider,
      }).catch((e: unknown) => String(e));
      if (exportFormat === "ep_xml" || exportFormat === "react") {
        const blob = new Blob([result], { type: "text/plain" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `wireframe.${exportFormat === "react" ? "tsx" : "ep"}`;
        a.click();
        URL.revokeObjectURL(url);
        showStatus("Downloaded");
      }
    } finally {
      setIsLoading(false);
    }
  };

  const copyEpXml = () => {
    if (!generatedWireframe?.epXml) return;
    navigator.clipboard.writeText(generatedWireframe.epXml).then(() => showStatus("EP XML copied!")).catch(() => {});
  };

  // ── Render ────────────────────────────────────────────────────────────

  const renderTemplates = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 4 }}>Wireframe Templates</div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.6 }}>
        Generate Evolus Pencil (.ep) wireframes from pre-built templates.
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 10, marginBottom: 20 }}>
        {WIREFRAME_TEMPLATES.map((t) => (
          <button
            key={t.id}
            onClick={() => generateWireframe(t.id)}
            disabled={isLoading}
            style={{
              background: selectedTemplate === t.id && generatedWireframe ? "var(--accent-blue)" : "var(--bg-secondary)",
              border: `1px solid ${selectedTemplate === t.id && generatedWireframe ? "var(--accent-blue)" : "var(--border-color)"}`,
              borderRadius: "var(--radius-sm-alt)",
              padding: "16px 16px",
              cursor: "pointer",
              textAlign: "left",
              color: selectedTemplate === t.id && generatedWireframe ? "var(--btn-primary-fg)" : "inherit",
              opacity: isLoading && selectedTemplate === t.id ? 0.5 : 1,
            }}
          >
            <Icon name={t.icon} size={20} style={{ marginBottom: 6 }} />
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{t.label}</div>
            <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.75, marginTop: 2 }}>{t.description}</div>
          </button>
        ))}
      </div>
      <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: 16, marginBottom: 16 }}>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 10 }}>Customize</div>
        <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Title</label>
        <input
          value={customTitle}
          onChange={(e) => setCustomTitle(e.target.value)}
          placeholder="Leave blank to use template name"
          style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "8px 12px", fontSize: "var(--font-size-base)", marginBottom: 10, boxSizing: "border-box" as const }}
        />
        <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Dashboard Sections (comma-separated)</label>
        <input
          value={customSections}
          onChange={(e) => setCustomSections(e.target.value)}
          style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "8px 12px", fontSize: "var(--font-size-base)", boxSizing: "border-box" as const }}
        />
      </div>
      {generatedWireframe && (
        <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: 16 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8, color: "var(--text-success)" }}>✓ Generated: {generatedWireframe.title}</div>
          {generatedWireframe.pages.map((p, i) => (
            <div key={i} style={{ fontSize: "var(--font-size-base)", padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
              <span style={{ fontFamily: "var(--font-mono)" }}>{p.name}</span>
              <span style={{ marginLeft: 8, color: "var(--text-secondary)" }}>{p.shapes} shapes</span>
            </div>
          ))}
          <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
            <button className="panel-btn" onClick={copyEpXml}
              style={{ flex: 1, background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", padding: "8px 0", cursor: "pointer", color: "inherit", fontSize: "var(--font-size-base)" }}>
              Copy EP XML
            </button>
            <button className="panel-btn panel-btn-primary" onClick={() => setActiveTab("export")} style={{ flex: 1 }}>
              Export
            </button>
          </div>
        </div>
      )}
    </div>
  );

  const renderImport = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 4 }}>Import Pencil EP XML</div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 12, lineHeight: 1.6 }}>
        Paste the inner content.xml from a .ep file (open .ep as ZIP to extract it).
      </div>
      <textarea
        value={importXml}
        onChange={(e) => setImportXml(e.target.value)}
        placeholder="<?xml version='1.0'?><Document name='...'>"
        rows={12}
        style={{ width: "100%", resize: "vertical", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: 10, fontSize: "var(--font-size-base)", boxSizing: "border-box" as const, fontFamily: "var(--font-mono)" }}
      />
      <button className="panel-btn"
        onClick={parseEpXml}
        disabled={isLoading || !importXml.trim()}
        style={{ width: "100%", marginTop: 8, background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isLoading || !importXml.trim() ? 0.5 : 1 }}
      >
        {isLoading ? "Parsing…" : "Parse EP XML"}
      </button>
      {parseResult && (
        <pre style={{ marginTop: 12, fontSize: "var(--font-size-base)", overflow: "auto", maxHeight: 400, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)", whiteSpace: "pre-wrap" }}>
          {parseResult}
        </pre>
      )}
    </div>
  );

  const renderMcp = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 4 }}>TuringWorks Pencil MCP</div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.6 }}>
        Interact with .pen files via the Pencil MCP server (TuringWorks/pencil).
      </div>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 14 }}>
        {["get_editor_state", "open_document", "batch_get", "get_guidelines", "get_screenshot"].map((op) => (
          <button key={op} onClick={() => setMcpOp(op)}
            style={{ background: mcpOp === op ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "4px 12px", cursor: "pointer", color: mcpOp === op ? "var(--btn-primary-fg)" : "inherit", fontSize: "var(--font-size-sm)", fontWeight: mcpOp === op ? 600 : 400 }}
          >{op}</button>
        ))}
      </div>
      {(mcpOp === "open_document" || mcpOp === "batch_get") && (
        <>
          <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>
            {mcpOp === "open_document" ? "File Path (.pen)" : "Search Pattern"}
          </label>
          <input
            value={mcpPath}
            onChange={(e) => setMcpPath(e.target.value)}
            placeholder={mcpOp === "open_document" ? "/path/to/design.pen" : "**"}
            style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "8px 12px", fontSize: "var(--font-size-md)", marginBottom: 12, boxSizing: "border-box" as const }}
          />
        </>
      )}
      <button className="panel-btn"
        onClick={executeMcpOp}
        disabled={isLoading}
        style={{ width: "100%", background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isLoading ? 0.5 : 1 }}
      >
        {isLoading ? "Executing…" : `Execute ${mcpOp}`}
      </button>
      {mcpResult && (
        <pre style={{ marginTop: 12, fontSize: "var(--font-size-base)", overflow: "auto", maxHeight: 500, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)", whiteSpace: "pre-wrap" }}>
          {mcpResult}
        </pre>
      )}
    </div>
  );

  const renderExport = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 12 }}>Export Wireframe</div>
      {!generatedWireframe ? (
        <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
          Generate a wireframe from the Templates tab first.
        </div>
      ) : (
        <>
          <div style={{ marginBottom: 16, padding: 12, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)" }}>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{generatedWireframe.title}</div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 2 }}>{generatedWireframe.pages.length} page(s)</div>
          </div>
          <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 6 }}>Export Format</label>
          <div style={{ display: "flex", gap: 6, marginBottom: 14 }}>
            {[
              { id: "ep_xml", label: "Pencil EP (.ep)" },
              { id: "react", label: "React Component" },
              { id: "html", label: "HTML/CSS" },
            ].map((f) => (
              <button key={f.id} onClick={() => setExportFormat(f.id)}
                style={{ background: exportFormat === f.id ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "8px 12px", cursor: "pointer", color: exportFormat === f.id ? "var(--btn-primary-fg)" : "inherit", fontSize: "var(--font-size-base)", fontWeight: exportFormat === f.id ? 600 : 400 }}
              >{f.label}</button>
            ))}
          </div>
          <button className="panel-btn"
            onClick={exportWireframe}
            disabled={isLoading}
            style={{ width: "100%", background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)" }}
          >
            {isLoading ? "Exporting…" : "Download Export"}
          </button>
        </>
      )}
    </div>
  );

  return (
    <div className="panel-container">
      <div className="panel-tab-bar" style={{ overflow: "auto" }}>
        {TAB_DEFS.map(({ id, label }) => (
          <button className={`panel-tab${activeTab === id ? " active" : ""}`} key={id} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
        {statusMsg && <span style={{ marginLeft: "auto", marginRight: 12, fontSize: "var(--font-size-sm)", color: "var(--text-success)", lineHeight: "30px" }}>✓ {statusMsg}</span>}
      </div>
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {activeTab === "templates" && renderTemplates()}
        {activeTab === "import" && renderImport()}
        {activeTab === "mcp" && renderMcp()}
        {activeTab === "export" && renderExport()}
      </div>
    </div>
  );
}
