import React, { useState } from "react";

interface GeneratedFile {
  path: string;
  layer: "Frontend" | "Backend" | "Database" | "Infra" | "Testing" | "Docs";
  lines: number;
}

const FullStackGenPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("configure");
  const [projectName, setProjectName] = useState("");
  const [frontend, setFrontend] = useState("React + TypeScript");
  const [backend, setBackend] = useState("Rust + Actix");
  const [database, setDatabase] = useState("PostgreSQL");
  const [auth, setAuth] = useState("JWT");
  const [features, setFeatures] = useState("");
  const [generating, setGenerating] = useState(false);
  const [progress, setProgress] = useState(0);
  const [files, setFiles] = useState<GeneratedFile[]>([]);
  const [expandedLayers, setExpandedLayers] = useState<Set<string>>(new Set(["Frontend", "Backend"]));

  const frontendOptions = ["React + TypeScript", "Next.js", "Vue 3 + TypeScript", "Svelte", "Angular", "Remix", "Astro"];
  const backendOptions = ["Rust + Actix", "Node.js + Express", "Python + FastAPI", "Go + Gin", "Java + Spring Boot", "Ruby on Rails", "Elixir + Phoenix"];
  const dbOptions = ["PostgreSQL", "MySQL", "SQLite", "MongoDB", "DynamoDB", "Supabase"];
  const authOptions = ["JWT", "OAuth 2.0", "Session-based", "API Keys", "None"];

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--vscode-foreground)",
    backgroundColor: "var(--vscode-editor-background)",
    fontFamily: "var(--vscode-font-family)", fontSize: "var(--vscode-font-size)",
    height: "100%", overflow: "auto",
  };
  const tabBarStyle: React.CSSProperties = {
    display: "flex", gap: "4px", marginBottom: "16px",
    borderBottom: "1px solid var(--vscode-panel-border)", paddingBottom: "8px",
  };
  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 14px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--vscode-button-background)" : "transparent",
    color: active ? "var(--vscode-button-foreground)" : "var(--vscode-foreground)",
    borderRadius: "4px", fontSize: "var(--vscode-font-size)",
  });
  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--vscode-input-background)", color: "var(--vscode-input-foreground)",
    border: "1px solid var(--vscode-input-border)", borderRadius: "4px",
  };
  const btnStyle: React.CSSProperties = {
    padding: "6px 14px", cursor: "pointer", border: "none", borderRadius: "4px",
    backgroundColor: "var(--vscode-button-background)", color: "var(--vscode-button-foreground)",
  };
  const cardStyle: React.CSSProperties = {
    padding: "10px", marginBottom: "8px", borderRadius: "4px",
    backgroundColor: "var(--vscode-editor-inactiveSelectionBackground)",
    border: "1px solid var(--vscode-panel-border)",
  };
  const labelStyle: React.CSSProperties = { display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "12px" };
  const fieldGroup: React.CSSProperties = { marginBottom: "12px" };

  const layerColors: Record<string, string> = {
    Frontend: "#1565c0", Backend: "#2e7d32", Database: "#6a1b9a",
    Infra: "#e65100", Testing: "#c62828", Docs: "#757575",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "10px",
    fontSize: "11px", fontWeight: 600, backgroundColor: color, color: "#fff",
  });

  const defaultFiles: GeneratedFile[] = [
    { path: "src/App.tsx", layer: "Frontend", lines: 120 },
    { path: "src/components/Layout.tsx", layer: "Frontend", lines: 85 },
    { path: "src/pages/Home.tsx", layer: "Frontend", lines: 64 },
    { path: "src/hooks/useAuth.ts", layer: "Frontend", lines: 42 },
    { path: "src/api/client.ts", layer: "Frontend", lines: 38 },
    { path: "server/main.rs", layer: "Backend", lines: 95 },
    { path: "server/routes/mod.rs", layer: "Backend", lines: 78 },
    { path: "server/handlers/auth.rs", layer: "Backend", lines: 110 },
    { path: "server/models/user.rs", layer: "Backend", lines: 52 },
    { path: "server/middleware/auth.rs", layer: "Backend", lines: 45 },
    { path: "migrations/001_init.sql", layer: "Database", lines: 35 },
    { path: "migrations/002_seed.sql", layer: "Database", lines: 20 },
    { path: "Dockerfile", layer: "Infra", lines: 28 },
    { path: "docker-compose.yml", layer: "Infra", lines: 42 },
    { path: ".github/workflows/ci.yml", layer: "Infra", lines: 55 },
    { path: "tests/integration/auth_test.rs", layer: "Testing", lines: 88 },
    { path: "tests/unit/models_test.rs", layer: "Testing", lines: 65 },
    { path: "src/__tests__/App.test.tsx", layer: "Testing", lines: 40 },
    { path: "README.md", layer: "Docs", lines: 95 },
    { path: "docs/API.md", layer: "Docs", lines: 120 },
  ];

  const handleGenerate = () => {
    setGenerating(true);
    setProgress(0);
    const interval = setInterval(() => {
      setProgress(prev => {
        if (prev >= 100) {
          clearInterval(interval);
          setGenerating(false);
          setFiles(defaultFiles);
          setActiveTab("files");
          return 100;
        }
        return prev + 5;
      });
    }, 120);
  };

  const toggleLayer = (layer: string) => {
    setExpandedLayers(prev => {
      const next = new Set(prev);
      next.has(layer) ? next.delete(layer) : next.add(layer);
      return next;
    });
  };

  const totalLines = files.reduce((sum, f) => sum + f.lines, 0);

  const renderConfigure = () => (
    <div>
      <div style={fieldGroup}>
        <label style={labelStyle}>Project Name</label>
        <input style={inputStyle} value={projectName} onChange={e => setProjectName(e.target.value)}
          placeholder="my-fullstack-app" />
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
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Auth</span><strong>{auth}</strong>
        </div>
      </div>
      <div style={{ ...cardStyle, textAlign: "center", margin: "16px 0" }}>
        <div style={{ fontSize: "12px", opacity: 0.7, marginBottom: "4px" }}>Estimated Output</div>
        <div style={{ display: "flex", justifyContent: "center", gap: "24px" }}>
          <div><div style={{ fontSize: "20px", fontWeight: 700 }}>~20</div><div style={{ fontSize: "11px", opacity: 0.7 }}>Files</div></div>
          <div><div style={{ fontSize: "20px", fontWeight: 700 }}>~1,200</div><div style={{ fontSize: "11px", opacity: 0.7 }}>Lines</div></div>
        </div>
      </div>
      {generating && (
        <div style={{ marginBottom: "12px" }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: "12px", marginBottom: "4px" }}>
            <span>Generating...</span><span>{progress}%</span>
          </div>
          <div style={{ height: "6px", borderRadius: "3px", backgroundColor: "var(--vscode-editor-inactiveSelectionBackground)" }}>
            <div style={{ height: "100%", width: `${progress}%`, borderRadius: "3px",
              backgroundColor: "var(--vscode-button-background)", transition: "width 0.1s" }} />
          </div>
        </div>
      )}
      <button style={{ ...btnStyle, width: "100%", padding: "10px", opacity: generating ? 0.6 : 1 }}
        onClick={handleGenerate} disabled={generating}>
        {generating ? "Generating..." : "Generate Full Stack"}
      </button>
    </div>
  );

  const layers = ["Frontend", "Backend", "Database", "Infra", "Testing", "Docs"];
  const groupedFiles = layers.map(layer => ({
    layer, files: files.filter(f => f.layer === layer),
  })).filter(g => g.files.length > 0);

  const renderFiles = () => (
    <div>
      <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", marginBottom: "12px" }}>
        <span>{files.length} files</span>
        <strong>{totalLines.toLocaleString()} lines</strong>
      </div>
      {groupedFiles.map(({ layer, files: layerFiles }) => (
        <div key={layer} style={{ marginBottom: "8px" }}>
          <div style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "8px",
            padding: "6px 0", fontWeight: 600 }} onClick={() => toggleLayer(layer)}>
            <span>{expandedLayers.has(layer) ? "\u25BC" : "\u25B6"}</span>
            <span style={badgeStyle(layerColors[layer])}>{layer}</span>
            <span style={{ opacity: 0.6, fontSize: "12px" }}>({layerFiles.length} files)</span>
          </div>
          {expandedLayers.has(layer) && layerFiles.map(f => (
            <div key={f.path} style={{ display: "flex", justifyContent: "space-between",
              padding: "4px 0 4px 24px", fontSize: "12px", borderBottom: "1px solid var(--vscode-panel-border)" }}>
              <code>{f.path}</code>
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

  return (
    <div style={containerStyle}>
      <h2 style={{ margin: "0 0 12px" }}>Full Stack Generator</h2>
      <div style={tabBarStyle}>
        {[["configure", "Configure"], ["generate", "Generate"], ["files", "Files"]].map(([id, label]) => (
          <button key={id} style={tabStyle(activeTab === id)} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      {activeTab === "configure" && renderConfigure()}
      {activeTab === "generate" && renderGenerate()}
      {activeTab === "files" && renderFiles()}
    </div>
  );
};

export default FullStackGenPanel;
