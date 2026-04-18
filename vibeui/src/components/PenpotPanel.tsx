/**
 * PenpotPanel — Penpot design tool integration.
 *
 * Tabs: Connect | Projects | Components | Tokens | Export
 * - Connect: Configure self-hosted or cloud Penpot instance
 * - Projects: Browse files and frames
 * - Components: View shared component catalogue
 * - Tokens: Extract and export design tokens (CSS, Tailwind, JSON)
 * - Export: Generate React/Vue/Svelte component code from Penpot components
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PenpotPanelProps {
  workspacePath: string | null;
  provider: string;
}

type PenpotTab = "connect" | "projects" | "components" | "tokens" | "export";

const TAB_DEFS: { id: PenpotTab; label: string }[] = [
  { id: "connect", label: "Connect" },
  { id: "projects", label: "Projects" },
  { id: "components", label: "Components" },
  { id: "tokens", label: "Tokens" },
  { id: "export", label: "Export" },
];

const FRAMEWORKS = ["react", "vue", "svelte", "next.js", "html"];

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "7px 14px",
  fontSize: "var(--font-size-base)",
  fontWeight: active ? 600 : 400,
  cursor: "pointer",
  border: "none",
  borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  background: "transparent",
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  whiteSpace: "nowrap",
});

const inputStyle: React.CSSProperties = {
  width: "100%",
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: "var(--radius-sm)",
  color: "inherit",
  padding: "8px 10px",
  fontSize: "var(--font-size-md)",
  marginBottom: 12,
  boxSizing: "border-box" as const,
};

interface PenpotProject { id: string; name: string; }
interface PenpotFile { id: string; name: string; project_id: string; }
interface PenpotComponent { id: string; name: string; description: string; }
interface PenpotToken { name: string; token_type: string; value: string; }

export function PenpotPanel({ workspacePath, provider }: PenpotPanelProps) {
  const [activeTab, setActiveTab] = useState<PenpotTab>("connect");
  const [host, setHost] = useState("https://design.penpot.app");
  const [token, setToken] = useState("");
  const [isConnected, setIsConnected] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [projects, setProjects] = useState<PenpotProject[]>([]);
  const [files, setFiles] = useState<PenpotFile[]>([]);
  const [components, setComponents] = useState<PenpotComponent[]>([]);
  const [tokens, setTokens] = useState<PenpotToken[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [selectedComponent, setSelectedComponent] = useState<string | null>(null);
  const [exportFramework, setExportFramework] = useState("react");
  const [exportedCode, setExportedCode] = useState("");
  const [tokenFormat, setTokenFormat] = useState("css");
  const [tokenExport, setTokenExport] = useState("");
  const [statusMsg, setStatusMsg] = useState("");
  const [error, setError] = useState("");

  const showStatus = (msg: string) => {
    setStatusMsg(msg);
    setError("");
    setTimeout(() => setStatusMsg(""), 3000);
  };

  const handleConnect = async () => {
    if (!host.trim() || !token.trim()) return;
    setIsLoading(true);
    setError("");
    try {
      const result = await invoke<{ projects: PenpotProject[] }>("connect_penpot", {
        host: host.trim(),
        token: token.trim(),
      }).catch((e: unknown) => { throw new Error(String(e)); });
      setProjects(result.projects || []);
      setIsConnected(true);
      showStatus(`Connected — ${result.projects?.length ?? 0} project(s)`);
      setActiveTab("projects");
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  };

  const loadProjectFiles = async (projectId: string) => {
    setIsLoading(true);
    try {
      const result = await invoke<{ files: PenpotFile[] }>("list_penpot_files", {
        host, token, projectId,
      }).catch(() => ({ files: [] }));
      setFiles(result.files);
    } finally {
      setIsLoading(false);
    }
  };

  const loadFileComponents = async (fileId: string) => {
    setSelectedFile(fileId);
    setIsLoading(true);
    try {
      const result = await invoke<{ components: PenpotComponent[]; tokens: PenpotToken[] }>(
        "import_penpot_file",
        { host, token, fileId, workspacePath, provider }
      ).catch(() => ({ components: [], tokens: [] }));
      setComponents(result.components);
      setTokens(result.tokens);
      showStatus(`Loaded ${result.components.length} component(s), ${result.tokens.length} token(s)`);
    } finally {
      setIsLoading(false);
    }
  };

  const exportComponent = async () => {
    if (!selectedComponent) return;
    setIsLoading(true);
    try {
      const code = await invoke<string>("export_penpot_component", {
        host, token, componentId: selectedComponent, framework: exportFramework,
        workspacePath, provider,
      }).catch((e: unknown) => String(e));
      setExportedCode(code);
    } finally {
      setIsLoading(false);
    }
  };

  const exportTokens = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<string>("export_penpot_tokens", {
        tokens, format: tokenFormat,
      }).catch((e: unknown) => String(e));
      setTokenExport(result);
    } finally {
      setIsLoading(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text).then(() => showStatus("Copied!")).catch(() => {});
  };

  // ── Render ────────────────────────────────────────────────────────────

  const renderConnect = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 20, maxWidth: 500, margin: "0 auto" }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 4 }}>Connect to Penpot</div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.6 }}>
        Connect to a self-hosted Penpot instance or the cloud version at design.penpot.app.
        Generate a personal access token in <em>Settings → Access Tokens</em>.
      </div>
      <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Penpot Instance URL</label>
      <input value={host} onChange={(e) => setHost(e.target.value)} placeholder="https://design.penpot.app" style={inputStyle} />
      <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Personal Access Token</label>
      <input type="password" value={token} onChange={(e) => setToken(e.target.value)} placeholder="Enter your access token" style={inputStyle} />
      {error && <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color, #f85149)", marginBottom: 12, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)" }}>{error}</div>}
      <button className="panel-btn"
        onClick={handleConnect}
        disabled={isLoading || !host.trim() || !token.trim()}
        style={{ width: "100%", background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isLoading ? 0.5 : 1 }}
      >
        {isLoading ? "Connecting…" : "Connect"}
      </button>
      {isConnected && (
        <div style={{ marginTop: 16, padding: 12, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)" }}>
          <div style={{ color: "var(--text-success)", fontSize: "var(--font-size-md)", fontWeight: 600 }}>✓ Connected to {host}</div>
          <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>{projects.length} project(s) found</div>
        </div>
      )}
    </div>
  );

  const renderProjects = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      {!isConnected ? (
        <div style={{ textAlign: "center", padding: 32, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
          Connect to Penpot first to browse projects.
          <br /><button className="panel-tab" onClick={() => setActiveTab("connect")} style={{ marginTop: 12, background: "var(--accent-blue)", border: "none", borderRadius: "var(--radius-sm)", padding: "8px 16px", cursor: "pointer", color: "var(--btn-primary-fg, #fff)", fontSize: "var(--font-size-base)" }}>Connect</button>
        </div>
      ) : (
        <>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 12 }}>Projects ({projects.length})</div>
          {projects.map((p) => (
            <div key={p.id} style={{ marginBottom: 12 }}>
              <div role="button" tabIndex={0}
                onClick={() => loadProjectFiles(p.id)}
                style={{ padding: "12px 16px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-md)" }}
              >
                {p.name}
              </div>
              {files.filter((f) => f.project_id === p.id).map((f) => (
                <div role="button" tabIndex={0}
                  key={f.id}
                  onClick={() => loadFileComponents(f.id)}
                  style={{ marginTop: 4, marginLeft: 16, padding: "8px 12px", background: selectedFile === f.id ? "var(--accent-blue)" : "var(--bg-tertiary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", cursor: "pointer", fontSize: "var(--font-size-base)", color: selectedFile === f.id ? "var(--btn-primary-fg)" : "inherit" }}
                >
                  {f.name}
                </div>
              ))}
            </div>
          ))}
        </>
      )}
    </div>
  );

  const renderComponents = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      {components.length === 0 ? (
        <div style={{ textAlign: "center", padding: 32, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
          Select a Penpot file from the Projects tab to load components.
        </div>
      ) : (
        <>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 12 }}>Components ({components.length})</div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 8 }}>
            {components.map((c) => (
              <div role="button" tabIndex={0}
                key={c.id}
                onClick={() => { setSelectedComponent(c.id); setActiveTab("export"); }}
                style={{ padding: "12px 12px", background: selectedComponent === c.id ? "var(--accent-blue)" : "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: `1px solid ${selectedComponent === c.id ? "transparent" : "var(--border-color)"}`, cursor: "pointer", color: selectedComponent === c.id ? "var(--btn-primary-fg)" : "inherit" }}
              >
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{c.name}</div>
                {c.description && <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.75, marginTop: 2 }}>{c.description}</div>}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );

  const renderTokens = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>Design Tokens ({tokens.length})</div>
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          {["css", "tailwind", "json", "typescript"].map((f) => (
            <button key={f} onClick={() => setTokenFormat(f)}
              style={{ background: tokenFormat === f ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "3px 8px", cursor: "pointer", color: tokenFormat === f ? "var(--btn-primary-fg)" : "inherit", fontSize: "var(--font-size-sm)", fontWeight: tokenFormat === f ? 600 : 400 }}
            >{f}</button>
          ))}
          <button className="panel-btn" onClick={exportTokens}
            style={{ background: "var(--accent-blue)", border: "none", borderRadius: "var(--radius-xs-plus)", padding: "3px 12px", cursor: "pointer", color: "var(--btn-primary-fg, #fff)", fontSize: "var(--font-size-sm)", fontWeight: 600 }}
          >Export</button>
        </div>
      </div>
      {tokens.length === 0 ? (
        <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>Load a Penpot file to extract tokens.</div>
      ) : (
        <div>
          {tokens.slice(0, 30).map((t, i) => (
            <div key={i} style={{ display: "flex", gap: 12, alignItems: "center", padding: "8px 0", borderBottom: "1px solid var(--border-color)" }}>
              {t.token_type === "color" && (
                <div style={{ width: 20, height: 20, background: t.value, borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", flexShrink: 0 }} />
              )}
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", flex: 1 }}>{t.name}</div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{t.value}</div>
            </div>
          ))}
          {tokens.length > 30 && <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 8 }}>…and {tokens.length - 30} more</div>}
        </div>
      )}
      {tokenExport && (
        <div style={{ marginTop: 16 }}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Exported ({tokenFormat.toUpperCase()})</div>
            <button onClick={() => copyToClipboard(tokenExport)}
              style={{ background: "none", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "2px 8px", cursor: "pointer", color: "inherit", fontSize: "var(--font-size-sm)" }}>Copy</button>
          </div>
          <pre style={{ fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 400, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)", whiteSpace: "pre-wrap" }}>
            {tokenExport}
          </pre>
        </div>
      )}
    </div>
  );

  const renderExport = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 12 }}>Export Component</div>
      {!selectedComponent ? (
        <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
          Select a component from the Components tab.
        </div>
      ) : (
        <>
          <div style={{ fontSize: "var(--font-size-md)", marginBottom: 12 }}>Component: <strong>{selectedComponent}</strong></div>
          <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 6 }}>Target Framework</label>
          <div style={{ display: "flex", gap: 6, marginBottom: 14, flexWrap: "wrap" }}>
            {FRAMEWORKS.map((f) => (
              <button key={f} onClick={() => setExportFramework(f)}
                style={{ background: exportFramework === f ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "4px 12px", cursor: "pointer", color: exportFramework === f ? "var(--btn-primary-fg)" : "inherit", fontSize: "var(--font-size-base)", fontWeight: exportFramework === f ? 600 : 400 }}
              >{f}</button>
            ))}
          </div>
          <button className="panel-btn"
            onClick={exportComponent}
            disabled={isLoading}
            style={{ width: "100%", background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isLoading ? 0.5 : 1 }}
          >
            {isLoading ? "Generating…" : "Generate Component Code"}
          </button>
          {exportedCode && (
            <div style={{ marginTop: 16 }}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Generated Code ({exportFramework})</div>
                <button onClick={() => copyToClipboard(exportedCode)}
                  style={{ background: "none", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "2px 8px", cursor: "pointer", color: "inherit", fontSize: "var(--font-size-sm)" }}>Copy</button>
              </div>
              <pre style={{ fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 500, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)", color: "var(--text-success)", whiteSpace: "pre-wrap" }}>
                {exportedCode}
              </pre>
            </div>
          )}
        </>
      )}
    </div>
  );

  return (
    <div className="panel-container">
      <div className="panel-header" style={{ padding: 0, overflow: "auto", flexShrink: 0 }}>
        {TAB_DEFS.map(({ id, label }) => (
          <button className="panel-tab" key={id} onClick={() => setActiveTab(id)} style={tabStyle(activeTab === id)}>
            {label}
            {id === "connect" && isConnected && <span style={{ display: "inline-block", width: 6, height: 6, borderRadius: "50%", background: "var(--text-success)", marginLeft: 6, verticalAlign: "middle" }} />}
          </button>
        ))}
        {statusMsg && <span style={{ marginLeft: "auto", marginRight: 12, fontSize: "var(--font-size-sm)", color: "var(--text-success)", lineHeight: "30px" }}>✓ {statusMsg}</span>}
      </div>
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {activeTab === "connect" && renderConnect()}
        {activeTab === "projects" && renderProjects()}
        {activeTab === "components" && renderComponents()}
        {activeTab === "tokens" && renderTokens()}
        {activeTab === "export" && renderExport()}
      </div>
    </div>
  );
}
