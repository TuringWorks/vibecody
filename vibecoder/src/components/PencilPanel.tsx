/**
 * PencilPanel — Evolus Pencil .ep format + TuringWorks Pencil MCP integration.
 *
 * Tabs: Templates | Import | MCP | Export
 * - Templates: six wireframe templates, generated locally (no provider needed)
 * - Import: Parse .ep XML files and display structure
 * - MCP: TuringWorks Pencil MCP command builder for .pen files
 * - Export: .ep archive, content.xml, standalone HTML, or a React component
 *
 * Every Tauri call here surfaces its error. The panel used to `.catch(() =>
 * null)` them, so a failing generate looked exactly like a slow one and the
 * Export button silently downloaded the error text as a `.ep` file.
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

/** Kept in step with `pencil_connector::TEMPLATE_IDS` — an id the backend does
 *  not know is now an error rather than a silently different wireframe. */
const WIREFRAME_TEMPLATES = [
  { id: "landing_page", label: "Landing Page", icon: "layout-grid", description: "Hero section, nav, features, footer" },
  { id: "dashboard", label: "Dashboard", icon: "chart-bar", description: "Sidebar, stats, chart, activity" },
  { id: "mobile_app", label: "Mobile App", icon: "monitor-play", description: "Status bar, nav, tab bar screens" },
  { id: "login_form", label: "Login Form", icon: "lock", description: "Email/password login with social auth" },
  { id: "settings_page", label: "Settings Page", icon: "settings", description: "Grouped settings with toggle switches" },
  { id: "data_table", label: "Data Table", icon: "clipboard-list", description: "Filterable sortable data table view" },
] as const;

/** Which templates read the comma-separated list, and as what. */
const SECTIONS_USED_AS: Record<string, string> = {
  dashboard: "sidebar sections",
  mobile_app: "one screen per entry",
  settings_page: "setting groups",
  data_table: "table columns",
};

const EXPORT_FORMATS = [
  { id: "ep", label: "Pencil (.ep)", hint: "ZIP archive containing content.xml — what Evolus Pencil opens" },
  { id: "ep_xml", label: "content.xml", hint: "The raw document XML, unzipped" },
  { id: "html", label: "HTML", hint: "Standalone page, rendered locally — no provider needed" },
  { id: "react", label: "React Component", hint: "Converted by the selected provider" },
] as const;

type ExportFormat = (typeof EXPORT_FORMATS)[number]["id"];

interface WireframePage {
  name: string;
  shapes: number;
  width: number;
  height: number;
}

interface GeneratedWireframe {
  title: string;
  template: string;
  pages: WireframePage[];
  epXml: string;
}

/** What `export_pencil_wireframe` returns: the bytes plus how to save them. */
interface ExportPayload {
  filename: string;
  mimeType: string;
  encoding: "utf8" | "base64";
  data: string;
}

interface ParsedDocument {
  name: string;
  id: string;
  pages: WireframePage[];
  page_count: number;
  total_shapes: number;
}

/** Tauri rejects with a plain string; anything else still deserves a message. */
const errText = (e: unknown): string =>
  typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);

function base64ToBytes(b64: string): Uint8Array<ArrayBuffer> {
  const binary = atob(b64);
  // Allocate the ArrayBuffer explicitly: a bare `new Uint8Array(n)` is typed
  // over `ArrayBufferLike`, which `BlobPart` does not accept.
  const bytes = new Uint8Array(new ArrayBuffer(binary.length));
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function downloadPayload(payload: ExportPayload) {
  const body: BlobPart =
    payload.encoding === "base64" ? base64ToBytes(payload.data) : payload.data;
  const blob = new Blob([body], { type: payload.mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = payload.filename;
  // Some webviews ignore a click on a node that was never in the document, and
  // revoking in the same tick can cancel the save before it starts.
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

export function PencilPanel({ workspacePath, provider }: PencilPanelProps) {
  const [activeTab, setActiveTab] = useState<PencilTab>("templates");
  const [generatedWireframe, setGeneratedWireframe] = useState<GeneratedWireframe | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);
  const [customTitle, setCustomTitle] = useState("");
  const [customSections, setCustomSections] = useState("Overview,Analytics,Settings,Users");
  const [importXml, setImportXml] = useState("");
  const [parseResult, setParseResult] = useState<ParsedDocument | null>(null);
  const [mcpOp, setMcpOp] = useState("get_editor_state");
  const [mcpPath, setMcpPath] = useState("");
  const [mcpResult, setMcpResult] = useState("");
  const [exportFormat, setExportFormat] = useState<ExportFormat>("ep");
  const [previewHtml, setPreviewHtml] = useState<string | null>(null);
  const [statusMsg, setStatusMsg] = useState("");
  const [errorMsg, setErrorMsg] = useState("");

  const showStatus = (msg: string) => {
    setErrorMsg("");
    setStatusMsg(msg);
    setTimeout(() => setStatusMsg(""), 3000);
  };

  const showError = (msg: string) => {
    setStatusMsg("");
    setErrorMsg(msg);
  };

  const generateWireframe = async (templateId: string) => {
    setSelectedTemplate(templateId);
    setIsLoading(true);
    setPreviewHtml(null);
    const title = customTitle.trim() || templateId.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
    const sections = customSections.split(",").map((s) => s.trim()).filter(Boolean);
    try {
      const result = await invoke<GeneratedWireframe>("generate_pencil_wireframe", {
        templateId, title, sections, workspacePath, provider,
      });
      setGeneratedWireframe(result);
      const shapes = result.pages.reduce((n, p) => n + p.shapes, 0);
      showStatus(`Generated ${result.pages.length} page(s), ${shapes} shapes`);
    } catch (e) {
      setGeneratedWireframe(null);
      showError(`Generate failed: ${errText(e)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const parseEpXml = async () => {
    if (!importXml.trim()) return;
    setIsLoading(true);
    try {
      const result = await invoke<ParsedDocument>("parse_pencil_ep", { xml: importXml });
      setParseResult(result);
      showStatus(`Parsed ${result.page_count} page(s)`);
    } catch (e) {
      setParseResult(null);
      showError(`Parse failed: ${errText(e)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const executeMcpOp = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<string>("execute_pencil_mcp", {
        operation: mcpOp,
        filePath: mcpPath.trim() || undefined,
      });
      setMcpResult(result);
      showStatus("Request built");
    } catch (e) {
      setMcpResult("");
      showError(errText(e));
    } finally {
      setIsLoading(false);
    }
  };

  /** Ask the backend for one format. Shared by download and preview. */
  const requestExport = async (format: ExportFormat): Promise<ExportPayload | null> => {
    if (!generatedWireframe) return null;
    try {
      return await invoke<ExportPayload>("export_pencil_wireframe", {
        xml: generatedWireframe.epXml,
        format,
        workspacePath,
        provider,
      });
    } catch (e) {
      showError(`Export failed: ${errText(e)}`);
      return null;
    }
  };

  const exportWireframe = async () => {
    if (!generatedWireframe) return;
    setIsLoading(true);
    try {
      const payload = await requestExport(exportFormat);
      if (!payload) return;
      downloadPayload(payload);
      showStatus(`Downloaded ${payload.filename}`);
    } finally {
      setIsLoading(false);
    }
  };

  const togglePreview = async () => {
    if (previewHtml) {
      setPreviewHtml(null);
      return;
    }
    setIsLoading(true);
    try {
      const payload = await requestExport("html");
      if (payload) setPreviewHtml(payload.data);
    } finally {
      setIsLoading(false);
    }
  };

  const copyEpXml = () => {
    if (!generatedWireframe?.epXml) return;
    navigator.clipboard
      .writeText(generatedWireframe.epXml)
      .then(() => showStatus("EP XML copied"))
      .catch((e: unknown) => showError(`Clipboard unavailable: ${errText(e)}`));
  };

  // ── Render ────────────────────────────────────────────────────────────

  const inputStyle = {
    width: "100%",
    background: "var(--bg-tertiary)",
    border: "1px solid var(--border-color)",
    borderRadius: "var(--radius-sm)",
    color: "inherit",
    padding: "8px 12px",
    fontSize: "var(--font-size-base)",
    boxSizing: "border-box" as const,
  };

  const renderTemplates = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 4 }}>Wireframe Templates</div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.6 }}>
        Generate Evolus Pencil (.ep) wireframes from pre-built templates. Runs locally — no provider or workspace needed.
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 10, marginBottom: 20 }}>
        {WIREFRAME_TEMPLATES.map((t) => {
          const isActive = selectedTemplate === t.id && !!generatedWireframe;
          return (
            <button
              key={t.id}
              onClick={() => generateWireframe(t.id)}
              disabled={isLoading}
              aria-pressed={isActive}
              aria-label={`Generate ${t.label} wireframe`}
              style={{
                background: isActive ? "var(--accent-blue)" : "var(--bg-secondary)",
                border: `1px solid ${isActive ? "var(--accent-blue)" : "var(--border-color)"}`,
                borderRadius: "var(--radius-sm-alt)",
                padding: "16px 16px",
                cursor: isLoading ? "progress" : "pointer",
                textAlign: "left",
                color: isActive ? "var(--btn-primary-fg)" : "inherit",
                opacity: isLoading && selectedTemplate === t.id ? 0.5 : 1,
              }}
            >
              <Icon name={t.icon} size={20} style={{ marginBottom: 6 }} />
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{t.label}</div>
              <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.75, marginTop: 2 }}>{t.description}</div>
            </button>
          );
        })}
      </div>
      <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: 16, marginBottom: 16 }}>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 10 }}>Customize</div>
        <label htmlFor="pencil-title" style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Title</label>
        <input
          id="pencil-title"
          value={customTitle}
          onChange={(e) => setCustomTitle(e.target.value)}
          placeholder="Leave blank to use template name"
          style={{ ...inputStyle, marginBottom: 10 }}
        />
        <label htmlFor="pencil-sections" style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>
          Sections (comma-separated)
        </label>
        <input
          id="pencil-sections"
          value={customSections}
          onChange={(e) => setCustomSections(e.target.value)}
          style={inputStyle}
        />
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 6, lineHeight: 1.5 }}>
          {selectedTemplate && SECTIONS_USED_AS[selectedTemplate]
            ? `Used as ${SECTIONS_USED_AS[selectedTemplate]}.`
            : "Used by Dashboard, Mobile App, Settings and Data Table. Landing Page and Login Form ignore it."}
        </div>
      </div>
      {generatedWireframe && (
        <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: 16 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8, color: "var(--text-success)" }}>✓ Generated: {generatedWireframe.title}</div>
          {generatedWireframe.pages.map((p, i) => (
            <div key={`${p.name}-${i}`} style={{ fontSize: "var(--font-size-base)", padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
              <span style={{ fontFamily: "var(--font-mono)" }}>{p.name}</span>
              <span style={{ marginLeft: 8, color: "var(--text-secondary)" }}>
                {`${p.shapes} shapes · ${p.width}×${p.height}`}
              </span>
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
        aria-label="Pencil EP XML to parse"
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
        <div style={{ marginTop: 12, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)" }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{parseResult.name}</div>
          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>
            {`${parseResult.page_count} page(s) · ${parseResult.total_shapes} shapes`}
          </div>
          {parseResult.pages.map((p, i) => (
            <div key={`${p.name}-${i}`} style={{ fontSize: "var(--font-size-base)", padding: "4px 0", borderTop: "1px solid var(--border-color)" }}>
              <span style={{ fontFamily: "var(--font-mono)" }}>{p.name}</span>
              <span style={{ marginLeft: 8, color: "var(--text-secondary)" }}>
                {`${p.shapes} shapes · ${p.width}×${p.height}`}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  const renderMcp = () => {
    const needsArg = mcpOp === "open_document" || mcpOp === "batch_get" || mcpOp === "batch_design" || mcpOp === "get_guidelines";
    const argLabel =
      mcpOp === "open_document" ? "File Path (.pen)"
        : mcpOp === "batch_get" ? "Search Pattern"
          : mcpOp === "batch_design" ? "Operations Script"
            : "Guideline Category (optional)";
    return (
      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 4 }}>TuringWorks Pencil MCP</div>
        <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.6 }}>
          Build a request for the Pencil MCP server (TuringWorks/pencil). VibeCody does not dispatch it yet — the
          result below is the exact tool call to run against a connected server.
        </div>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 14 }}>
          {["get_editor_state", "open_document", "batch_get", "batch_design", "get_guidelines", "get_screenshot"].map((op) => (
            <button key={op} onClick={() => setMcpOp(op)} aria-pressed={mcpOp === op}
              style={{ background: mcpOp === op ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "4px 12px", cursor: "pointer", color: mcpOp === op ? "var(--btn-primary-fg)" : "inherit", fontSize: "var(--font-size-sm)", fontWeight: mcpOp === op ? 600 : 400 }}
            >{op}</button>
          ))}
        </div>
        {needsArg && (
          <>
            <label htmlFor="pencil-mcp-arg" style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>
              {argLabel}
            </label>
            <input
              id="pencil-mcp-arg"
              value={mcpPath}
              onChange={(e) => setMcpPath(e.target.value)}
              placeholder={mcpOp === "open_document" ? "/path/to/design.pen" : mcpOp === "batch_get" ? "**" : ""}
              style={{ ...inputStyle, background: "var(--bg-secondary)", fontSize: "var(--font-size-md)", marginBottom: 12 }}
            />
          </>
        )}
        <button className="panel-btn"
          onClick={executeMcpOp}
          disabled={isLoading}
          style={{ width: "100%", background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isLoading ? 0.5 : 1 }}
        >
          {isLoading ? "Building…" : `Build ${mcpOp} request`}
        </button>
        {mcpResult && (
          <pre style={{ marginTop: 12, fontSize: "var(--font-size-base)", overflow: "auto", maxHeight: 500, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)", whiteSpace: "pre-wrap" }}>
            {mcpResult}
          </pre>
        )}
      </div>
    );
  };

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
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 2 }}>
              {`${generatedWireframe.pages.length} page(s) · ${generatedWireframe.pages.reduce((n, p) => n + p.shapes, 0)} shapes`}
            </div>
          </div>
          <div id="pencil-format-label" style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 6 }}>Export Format</div>
          <div role="radiogroup" aria-labelledby="pencil-format-label" style={{ display: "flex", gap: 6, marginBottom: 6, flexWrap: "wrap" }}>
            {EXPORT_FORMATS.map((f) => (
              <button key={f.id} onClick={() => setExportFormat(f.id)} role="radio" aria-checked={exportFormat === f.id} title={f.hint}
                style={{ background: exportFormat === f.id ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "8px 12px", cursor: "pointer", color: exportFormat === f.id ? "var(--btn-primary-fg)" : "inherit", fontSize: "var(--font-size-base)", fontWeight: exportFormat === f.id ? 600 : 400 }}
              >{f.label}</button>
            ))}
          </div>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 14, lineHeight: 1.5 }}>
            {EXPORT_FORMATS.find((f) => f.id === exportFormat)?.hint}
            {exportFormat === "react" && !provider && " — select a model in the toolbar first."}
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="panel-btn"
              onClick={exportWireframe}
              disabled={isLoading}
              style={{ flex: 2, background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isLoading ? 0.5 : 1 }}
            >
              {isLoading ? "Exporting…" : "Download Export"}
            </button>
            <button className="panel-btn"
              onClick={togglePreview}
              disabled={isLoading}
              aria-pressed={!!previewHtml}
              style={{ flex: 1, background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", color: "inherit", fontSize: "var(--font-size-base)" }}
            >
              {previewHtml ? "Hide preview" : "Preview"}
            </button>
          </div>
          {previewHtml && (
            <iframe
              title="Wireframe preview"
              sandbox=""
              srcDoc={previewHtml}
              style={{ width: "100%", height: 460, marginTop: 12, border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", background: "#ffffff" }}
            />
          )}
        </>
      )}
    </div>
  );

  return (
    <div className="panel-container">
      <div className="panel-tab-bar" role="tablist" aria-label="Pencil sections" style={{ overflow: "auto" }}>
        {TAB_DEFS.map(({ id, label }) => (
          <button
            className={`panel-tab${activeTab === id ? " active" : ""}`}
            key={id}
            role="tab"
            id={`pencil-tab-${id}`}
            aria-selected={activeTab === id}
            aria-controls={`pencil-panel-${id}`}
            onClick={() => setActiveTab(id)}
          >{label}</button>
        ))}
        {statusMsg && <span style={{ marginLeft: "auto", marginRight: 12, fontSize: "var(--font-size-sm)", color: "var(--text-success)", lineHeight: "30px" }}>✓ {statusMsg}</span>}
      </div>
      {errorMsg && (
        <div role="alert" style={{ display: "flex", gap: 8, alignItems: "flex-start", padding: "8px 12px", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", color: "var(--text-error, #ef4444)", fontSize: "var(--font-size-base)" }}>
          <span style={{ flex: 1 }}>{errorMsg}</span>
          <button onClick={() => setErrorMsg("")} aria-label="Dismiss error"
            style={{ background: "none", border: "none", color: "inherit", cursor: "pointer", fontSize: "var(--font-size-base)" }}>✕</button>
        </div>
      )}
      <div
        role="tabpanel"
        id={`pencil-panel-${activeTab}`}
        aria-labelledby={`pencil-tab-${activeTab}`}
        style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}
      >
        {activeTab === "templates" && renderTemplates()}
        {activeTab === "import" && renderImport()}
        {activeTab === "mcp" && renderMcp()}
        {activeTab === "export" && renderExport()}
      </div>
    </div>
  );
}
