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

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "inherit", fontSize: 13,
    height: "100%", overflow: "auto",
  };
  const tabBarStyle: React.CSSProperties = {
    display: "flex", gap: "4px", marginBottom: "16px",
    borderBottom: "1px solid var(--border-color)", paddingBottom: "8px",
  };
  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 14px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--accent-blue)" : "transparent",
    color: active ? "var(--btn-primary-fg)" : "var(--text-primary)",
    borderRadius: "4px", fontSize: 12,
  });
  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)",
    border: "1px solid var(--border-color)", borderRadius: "4px",
    fontFamily: "inherit", fontSize: 12,
  };
  const btnStyle: React.CSSProperties = {
    padding: "6px 14px", cursor: "pointer", border: "none", borderRadius: "4px",
    backgroundColor: "var(--accent-blue)", color: "var(--btn-primary-fg)", fontSize: 12,
  };
  const cardStyle: React.CSSProperties = {
    padding: "10px", marginBottom: "8px", borderRadius: "4px",
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
  };
  const labelStyle: React.CSSProperties = { display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "12px" };
  const fieldGroup: React.CSSProperties = { marginBottom: "12px" };

  const layerColors: Record<string, string> = {
    Frontend: "#1565c0", Backend: "#2e7d32", Database: "var(--accent-purple)",
    Infra: "#e65100", Testing: "#c62828", Docs: "#757575",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "10px",
    fontSize: "11px", fontWeight: 600, backgroundColor: color, color: "var(--btn-primary-fg)",
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
      next.has(layer) ? next.delete(layer) : next.add(layer);
      return next;
    });
  };

  const layers = ["Frontend", "Backend", "Database", "Infra", "Testing", "Docs"];
  const groupedFiles = layers.map(layer => ({
    layer, files: files.filter(f => f.layer === layer),
  })).filter(g => g.files.length > 0);

  const renderConfigure = () => (
    <div>
      <div style={fieldGroup}>
        <label style={labelStyle}>Project Name</label>
        <input style={inputStyle} value={projectName} onChange={e => setProjectName(e.target.value)}
          placeholder="my-fullstack-app" />
      </div>
      <div style={fieldGroup}>
        <label style={labelStyle}>Output Directory</label>
        <input style={inputStyle} value={outputDir} onChange={e => setOutputDir(e.target.value)}
          placeholder="~/projects" />
      </div>
      <div style={{ display: "flex", gap: "12px", marginBottom: "12px" }}>
        <div style={{ flex: 1 }}>
          <label style={labelStyle}>Frontend</label>
          <select style={inputStyle} value={frontend} onChange={e => setFrontend(e.target.value)}>
            {frontendOptions.map(o => <option key={o} value={o}>{o}</option>)}
          </select>
        </div>
        <div style={{ flex: 1 }}>
          <label style={labelStyle}>Backend</label>
          <select style={inputStyle} value={backend} onChange={e => setBackend(e.target.value)}>
            {backendOptions.map(o => <option key={o} value={o}>{o}</option>)}
          </select>
        </div>
      </div>
      <div style={{ display: "flex", gap: "12px", marginBottom: "12px" }}>
        <div style={{ flex: 1 }}>
          <label style={labelStyle}>Database</label>
          <select style={inputStyle} value={database} onChange={e => setDatabase(e.target.value)}>
            {dbOptions.map(o => <option key={o} value={o}>{o}</option>)}
          </select>
        </div>
        <div style={{ flex: 1 }}>
          <label style={labelStyle}>Auth</label>
          <select style={inputStyle} value={auth} onChange={e => setAuth(e.target.value)}>
            {authOptions.map(o => <option key={o} value={o}>{o}</option>)}
          </select>
        </div>
      </div>
      <div style={fieldGroup}>
        <label style={labelStyle}>Features / Requirements</label>
        <textarea style={{ ...inputStyle, minHeight: "80px", resize: "vertical" }} value={features}
          onChange={e => setFeatures(e.target.value)}
          placeholder="Describe additional features (e.g., real-time chat, file uploads, admin dashboard)..." />
      </div>
      {error && <div style={{ color: "var(--error-color)", fontSize: 12, marginTop: 8 }}>{error}</div>}
    </div>
  );

  const renderGenerate = () => (
    <div>
      <div style={cardStyle}>
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
          <span>Output</span><strong style={{ fontSize: 11, wordBreak: "break-all" }}>{outputDir}/{projectName || "project"}</strong>
        </div>
      </div>
      {error && <div style={{ color: "var(--error-color)", fontSize: 12, marginBottom: 8, padding: "8px", background: "var(--bg-tertiary)", borderRadius: 4 }}>{error}</div>}
      <button style={{ ...btnStyle, width: "100%", padding: "10px", opacity: generating ? 0.6 : 1 }}
        onClick={handleGenerate} disabled={generating}>
        {generating ? "Generating project files..." : "Generate Full Stack"}
      </button>
    </div>
  );

  const renderFiles = () => (
    <div>
      {generatedDir && (
        <div style={{ ...cardStyle, fontSize: 11, wordBreak: "break-all" }}>
          Output: <strong>{generatedDir}</strong>
        </div>
      )}
      <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", marginBottom: "12px" }}>
        <span>{files.length} files generated</span>
        <strong>{totalLines.toLocaleString()} lines</strong>
      </div>
      {groupedFiles.map(({ layer, files: layerFiles }) => (
        <div key={layer} style={{ marginBottom: "8px" }}>
          <div style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "8px",
            padding: "6px 0", fontWeight: 600 }} onClick={() => toggleLayer(layer)}>
            <span>{expandedLayers.has(layer) ? "\u25BC" : "\u25B6"}</span>
            <span style={badgeStyle(layerColors[layer] || "var(--border-color)")}>{layer}</span>
            <span style={{ opacity: 0.6, fontSize: "12px" }}>({layerFiles.length} files)</span>
          </div>
          {expandedLayers.has(layer) && layerFiles.map(f => (
            <div key={f.path} style={{ display: "flex", justifyContent: "space-between", alignItems: "center",
              padding: "4px 0 4px 24px", fontSize: "12px", borderBottom: "1px solid var(--border-color)",
              cursor: "pointer" }}
              onClick={() => openFile(f)}>
              <code style={{ color: "var(--accent-blue)" }}>{f.path}</code>
              <span style={{ opacity: 0.6 }}>{f.lines} lines</span>
            </div>
          ))}
        </div>
      ))}
      {files.length === 0 && <div style={{ opacity: 0.6, textAlign: "center", padding: "24px" }}>
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
              <code style={{ fontSize: 12 }}>{selectedFile.path}</code>
              <span style={{ ...badgeStyle(layerColors[selectedFile.layer] || "var(--border-color)"), marginLeft: 8 }}>{selectedFile.layer}</span>
            </div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              {saveMsg && <span style={{ fontSize: 11, color: saveMsg === "Saved" ? "var(--success-color)" : "var(--error-color)" }}>{saveMsg}</span>}
              <button style={{ ...btnStyle, opacity: saving ? 0.6 : 1 }} onClick={saveFile} disabled={saving}>
                {saving ? "Saving..." : "Save"}
              </button>
              <button style={{ ...btnStyle, backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)" }}
                onClick={() => { setSelectedFile(null); setActiveTab("files"); }}>
                Back
              </button>
            </div>
          </div>
          <textarea
            style={{
              flex: 1, width: "100%", boxSizing: "border-box",
              backgroundColor: "var(--bg-tertiary)", color: "var(--text-primary)",
              border: "1px solid var(--border-color)", borderRadius: "4px",
              fontFamily: "var(--font-mono)", fontSize: 12, padding: "10px",
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
    <div style={containerStyle}>
      <h2 style={{ margin: "0 0 12px" }}>Full Stack Generator</h2>
      <div style={tabBarStyle}>
        {[["configure", "Configure"], ["generate", "Generate"], ["files", "Files"], ...(selectedFile ? [["editor", "Editor"]] : [])].map(([id, label]) => (
          <button key={id} style={tabStyle(activeTab === id)} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      {activeTab === "configure" && renderConfigure()}
      {activeTab === "generate" && renderGenerate()}
      {activeTab === "files" && renderFiles()}
      {activeTab === "editor" && renderEditor()}
    </div>
  );
};

export default FullStackGenPanel;
