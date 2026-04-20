import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GeneratedFile {
  path: string;
  absolute_path: string;
  layer: string;
  lines: number;
  content: string;
}

interface FullStackResult {
  files: GeneratedFile[];
  total_lines: number;
  output_dir: string;
}

const FullStackGenPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("configure");
  const [projectName, setProjectName] = useState("");
  const [frontend, setFrontend] = useState("React + TypeScript");
  const [backend, setBackend] = useState("Rust + Actix");
  const [database, setDatabase] = useState("PostgreSQL");
  const [auth, setAuth] = useState("JWT");
  const [features, setFeatures] = useState("");
  const [outputDir, setOutputDir] = useState("~/projects");
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState("");
  const [files, setFiles] = useState<GeneratedFile[]>([]);
  const [totalLines, setTotalLines] = useState(0);
  const [generatedDir, setGeneratedDir] = useState("");
  const [expandedLayers, setExpandedLayers] = useState<Set<string>>(new Set(["Frontend", "Backend"]));
  const [selectedFile, setSelectedFile] = useState<GeneratedFile | null>(null);
  const [editContent, setEditContent] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState("");

  const frontendOptions = ["React + TypeScript", "Next.js", "Vue 3 + TypeScript", "Svelte", "Angular", "Remix", "Astro"];
  const backendOptions = ["Rust + Actix", "Node.js + Express", "Python + FastAPI", "Go + Gin", "Java + Spring Boot", "Ruby on Rails", "Elixir + Phoenix"];
  const dbOptions = ["PostgreSQL", "MySQL", "SQLite", "MongoDB", "DynamoDB", "Supabase"];
  const authOptions = ["JWT", "OAuth 2.0", "Session-based", "API Keys", "None"];


  const layerColors: Record<string, string> = {
    Frontend: "var(--accent-blue)", Backend: "var(--accent-green)", Database: "var(--accent-purple)",
    Infra: "var(--warning-color)", Testing: "var(--error-color)", Docs: "var(--text-secondary)",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
    fontSize: "var(--font-size-sm)", fontWeight: 600, backgroundColor: color, color: "var(--btn-primary-fg)",
  });

  const handleGenerate = async () => {
    if (!projectName.trim()) {
      setError("Project name is required");
      return;
    }
    setGenerating(true);
    setError("");
    try {
      const result = await invoke<FullStackResult>("fullstack_generate", {
        spec: {
          project_name: projectName.trim(),
          frontend,
          backend,
          database,
          auth,
          features,
          output_dir: outputDir || "~/projects",
        },
      });
      setFiles(result.files);
      setTotalLines(result.total_lines);
      setGeneratedDir(result.output_dir);
      setActiveTab("files");
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const openFile = async (file: GeneratedFile) => {
    try {
      const content = await invoke<string>("fullstack_read_file", { path: file.absolute_path });
      setSelectedFile(file);
      setEditContent(content);
      setSaveMsg("");
      setActiveTab("editor");
    } catch (e) {
      setError(`Failed to read file: ${e}`);
    }
  };

  const saveFile = async () => {
    if (!selectedFile) return;
    setSaving(true);
    setSaveMsg("");
    try {
      await invoke("fullstack_write_file", { path: selectedFile.absolute_path, content: editContent });
      setSaveMsg("Saved");
      // Update lines count in local state
      const newLines = editContent.split("\n").length;
      setFiles(prev => prev.map(f =>
        f.absolute_path === selectedFile.absolute_path ? { ...f, lines: newLines, content: editContent } : f
      ));
      setTimeout(() => setSaveMsg(""), 2000);
    } catch (e) {
      setSaveMsg(`Error: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const toggleLayer = (layer: string) => {
    setExpandedLayers(prev => {
      const next = new Set(prev);
      if (next.has(layer)) next.delete(layer); else next.add(layer);
      return next;
    });
  };

  const layers = ["Frontend", "Backend", "Database", "Infra", "Testing", "Docs"];
  const groupedFiles = layers.map(layer => ({
    layer, files: files.filter(f => f.layer === layer),
  })).filter(g => g.files.length > 0);

  const renderConfigure = () => (
    <div>
      <div style={{ marginBottom: "12px" }}>
        <label style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "var(--font-size-base)" }}>Project Name</label>
        <input className="panel-input panel-input-full" value={projectName} onChange={e => setProjectName(e.target.value)}
          placeholder="my-fullstack-app" />
      </div>
      <div style={{ marginBottom: "12px" }}>
        <label style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "var(--font-size-base)" }}>Output Directory</label>
        <input className="panel-input panel-input-full" value={outputDir} onChange={e => setOutputDir(e.target.value)}
          placeholder="~/projects" />
      </div>
      <div style={{ display: "flex", gap: "12px", marginBottom: "12px" }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "var(--font-size-base)" }}>Frontend</label>
          <select className="panel-select" value={frontend} onChange={e => setFrontend(e.target.value)}>
            {frontendOptions.map(o => <option key={o} value={o}>{o}</option>)}
          </select>
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "var(--font-size-base)" }}>Backend</label>
          <select className="panel-select" value={backend} onChange={e => setBackend(e.target.value)}>
            {backendOptions.map(o => <option key={o} value={o}>{o}</option>)}
          </select>
        </div>
      </div>
      <div style={{ display: "flex", gap: "12px", marginBottom: "12px" }}>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "var(--font-size-base)" }}>Database</label>
          <select className="panel-select" value={database} onChange={e => setDatabase(e.target.value)}>
            {dbOptions.map(o => <option key={o} value={o}>{o}</option>)}
          </select>
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "var(--font-size-base)" }}>Auth</label>
          <select className="panel-select" value={auth} onChange={e => setAuth(e.target.value)}>
            {authOptions.map(o => <option key={o} value={o}>{o}</option>)}
          </select>
        </div>
      </div>
      <div style={{ marginBottom: "12px" }}>
        <label style={{ display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "var(--font-size-base)" }}>Features / Requirements</label>
        <textarea className="panel-textarea panel-input-full" style={{ minHeight: "80px", resize: "vertical" }} value={features}
          onChange={e => setFeatures(e.target.value)}
          placeholder="Describe additional features (e.g., real-time chat, file uploads, admin dashboard)..." />
      </div>
      {error && <div className="panel-error" style={{ marginTop: 8 }}>{error}</div>}
    </div>
  );

  const renderGenerate = () => (
    <div>
      <div className="panel-card" style={{ marginBottom: "8px" }}>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
          <span>Project</span><strong>{projectName || "Unnamed Project"}</strong>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
          <span>Stack</span><strong>{frontend} + {backend}</strong>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
          <span>Database</span><strong>{database}</strong>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
          <span>Auth</span><strong>{auth}</strong>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Output</span><strong style={{ fontSize: "var(--font-size-sm)", wordBreak: "break-all" }}>{outputDir}/{projectName || "project"}</strong>
        </div>
      </div>
      {error && <div className="panel-error" style={{ marginBottom: 8 }}>{error}</div>}
      <button className="panel-btn panel-btn-primary" style={{ width: "100%", padding: "12px", opacity: generating ? 0.6 : 1 }}
        onClick={handleGenerate} disabled={generating}>
        {generating ? "Generating project files..." : "Generate Full Stack"}
      </button>
    </div>
  );

  const renderFiles = () => (
    <div>
      {generatedDir && (
        <div className="panel-card" style={{ fontSize: "var(--font-size-sm)", wordBreak: "break-all", marginBottom: "8px" }}>
          Output: <strong>{generatedDir}</strong>
        </div>
      )}
      <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", marginBottom: "12px" }}>
        <span>{files.length} files generated</span>
        <strong>{totalLines.toLocaleString()} lines</strong>
      </div>
      {groupedFiles.map(({ layer, files: layerFiles }) => (
        <div key={layer} style={{ marginBottom: "8px" }}>
          <div role="button" tabIndex={0} style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "8px",
            padding: "8px 0", fontWeight: 600 }} onClick={() => toggleLayer(layer)}>
            <span>{expandedLayers.has(layer) ? "\u25BC" : "\u25B6"}</span>
            <span style={badgeStyle(layerColors[layer] || "var(--border-color)")}>{layer}</span>
            <span style={{ opacity: 0.6, fontSize: "var(--font-size-base)" }}>({layerFiles.length} files)</span>
          </div>
          {expandedLayers.has(layer) && layerFiles.map(f => (
            <div role="button" tabIndex={0} key={f.path} style={{ display: "flex", justifyContent: "space-between", alignItems: "center",
              padding: "4px 0 4px 24px", fontSize: "var(--font-size-base)", borderBottom: "1px solid var(--border-color)",
              cursor: "pointer" }}
              onClick={() => openFile(f)}>
              <code style={{ color: "var(--accent-blue)" }}>{f.path}</code>
              <span style={{ opacity: 0.6 }}>{f.lines} lines</span>
            </div>
          ))}
        </div>
      ))}
      {files.length === 0 && <div className="panel-empty">
        No files generated yet. Go to the Generate tab to create your project.
      </div>}
    </div>
  );

  const renderEditor = () => (
    <div style={{ display: "flex", flexDirection: "column", height: "calc(100vh - 140px)" }}>
      {selectedFile && (
        <>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
            <div>
              <code style={{ fontSize: "var(--font-size-base)" }}>{selectedFile.path}</code>
              <span style={{ ...badgeStyle(layerColors[selectedFile.layer] || "var(--border-color)"), marginLeft: 8 }}>{selectedFile.layer}</span>
            </div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              {saveMsg && <span style={{ fontSize: "var(--font-size-sm)", color: saveMsg === "Saved" ? "var(--success-color)" : "var(--error-color)" }}>{saveMsg}</span>}
              <button className="panel-btn panel-btn-primary" style={{ opacity: saving ? 0.6 : 1 }} onClick={saveFile} disabled={saving}>
                {saving ? "Saving..." : "Save"}
              </button>
              <button className="panel-btn panel-btn-secondary"
                onClick={() => { setSelectedFile(null); setActiveTab("files"); }}>
                Back
              </button>
            </div>
          </div>
          <textarea
            style={{
              flex: 1, width: "100%", boxSizing: "border-box",
              backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)",
              border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)",
              fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", padding: "12px",
              resize: "none", lineHeight: 1.5,
            }}
            value={editContent}
            onChange={e => setEditContent(e.target.value)}
            spellCheck={false}
          />
        </>
      )}
    </div>
  );

  return (
    <div className="panel-container">
      <div className="panel-header">Full Stack Generator</div>
      <div className="panel-tab-bar">
        {[["configure", "Configure"], ["generate", "Generate"], ["files", "Files"], ...(selectedFile ? [["editor", "Editor"]] : [])].map(([id, label]) => (
          <button key={id} className={`panel-tab ${activeTab === id ? "active" : ""}`} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      <div className="panel-body">
        {activeTab === "configure" && renderConfigure()}
        {activeTab === "generate" && renderGenerate()}
        {activeTab === "files" && renderFiles()}
        {activeTab === "editor" && renderEditor()}
      </div>
    </div>
  );
};

export default FullStackGenPanel;
