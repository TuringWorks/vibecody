import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

/* ── Types ───────────────────────────────────────────────────────────── */

type DepthLevel = "Full" | "Summary" | "Skeleton" | "Signatures";

interface ContextChunk {
  id: number;
  filePath: string;
  depth: DepthLevel;
  relevance: number;
  tokenCount: number;
  pinned?: boolean;
}

interface ProjectFile {
  path: string;
  isDirectory: boolean;
  contextStatus: "loaded" | "summarized" | "not-loaded";
  tokenEstimate: number;
  lastModified: string;
  relevance: number;
  children?: ProjectFile[];
  expanded?: boolean;
}

interface ContextWindowStats {
  usedTokens: number;
  maxTokens: number;
  usagePct: number;
  chunkCount: number;
  compressionRatio: number;
}

type SortKey = "relevance" | "filePath" | "tokenCount";
type TabId = "context" | "projectMap" | "settings";

/* ── Constants ───────────────────────────────────────────────────────── */

const DEPTH_COLORS: Record<DepthLevel, string> = {
  Full: "var(--success-color)",
  Summary: "var(--accent-color)",
  Skeleton: "var(--warning-color)",
  Signatures: "var(--text-secondary)",
};

const STATUS_ICONS: Record<string, string> = {
  loaded: "\u25CF",
  summarized: "\u25D2",
  "not-loaded": "\u25CB",
};

const STATUS_COLORS: Record<string, string> = {
  loaded: "var(--success-color)",
  summarized: "var(--warning-color)",
  "not-loaded": "var(--text-secondary)",
};

const DEPTH_PROMOTE: Record<DepthLevel, DepthLevel> = {
  Full: "Full",
  Summary: "Full",
  Skeleton: "Summary",
  Signatures: "Skeleton",
};

const DEPTH_DEMOTE: Record<DepthLevel, DepthLevel> = {
  Full: "Summary",
  Summary: "Skeleton",
  Skeleton: "Signatures",
  Signatures: "Signatures",
};

/* ── Helpers ──────────────────────────────────────────────────────────── */

const fmtTokens = (n: number): string =>
  n >= 1_000_000
    ? `${(n / 1_000_000).toFixed(1)}M`
    : n >= 1_000
      ? `${(n / 1_000).toFixed(1)}K`
      : String(n);

const fmtPct = (v: number): string => `${Math.round(v * 100)}%`;

let nextChunkId = 100_000;

/* ── Component ───────────────────────────────────────────────────────── */

export function InfiniteContextPanel({ workspacePath }: { workspacePath: string }) {
  const [activeTab, setActiveTab] = useState<TabId>("context");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Context Window state
  const [chunks, setChunks] = useState<ContextChunk[]>([]);
  const [sortKey, setSortKey] = useState<SortKey>("relevance");
  const [maxTokens, setMaxTokens] = useState(100_000);

  // Project Map state
  const [projectFiles, setProjectFiles] = useState<ProjectFile[]>([]);
  const [fileFilter, setFileFilter] = useState("");

  // Settings state
  const [settingsMaxTokens, setSettingsMaxTokens] = useState(100_000);
  const [recencyWeight, setRecencyWeight] = useState(0.6);
  const [proximityWeight, setProximityWeight] = useState(0.7);
  const [keywordWeight, setKeywordWeight] = useState(0.8);
  const [dependencyWeight, setDependencyWeight] = useState(0.5);
  const [accessFreqWeight, setAccessFreqWeight] = useState(0.4);
  const [autoCompress, setAutoCompress] = useState(true);
  const [cacheSize, setCacheSize] = useState(256);

  const hasWorkspace = !!workspacePath;

  /* ── Load data from backend ─────────────────────────────────────── */

  const loadChunks = useCallback(async () => {
    if (!hasWorkspace) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ContextChunk[]>("get_context_chunks", { workspace: workspacePath });
      setChunks(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [workspacePath, hasWorkspace]);

  const loadProjectTree = useCallback(async () => {
    if (!hasWorkspace) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ProjectFile[]>("get_project_file_tree", { workspace: workspacePath });
      setProjectFiles(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [workspacePath, hasWorkspace]);

  const loadStats = useCallback(async () => {
    if (!hasWorkspace) return;
    try {
      const stats = await invoke<ContextWindowStats>("get_context_window_stats", { workspace: workspacePath });
      setMaxTokens(stats.maxTokens);
    } catch (_e) {
      // stats are supplementary, don't block on error
    }
  }, [workspacePath, hasWorkspace]);

  // Load chunks and project tree on mount
  useEffect(() => {
    loadChunks();
    loadProjectTree();
    loadStats();
  }, [loadChunks, loadProjectTree, loadStats]);

  /* ── Context Window actions ──────────────────────────────────────── */

  const usedTokens = chunks.reduce((s, c) => s + c.tokenCount, 0);
  const compressionRatio = maxTokens > 0 ? 1 - usedTokens / maxTokens : 0;
  const usagePct = maxTokens > 0 ? (usedTokens / maxTokens) * 100 : 0;

  const sortedChunks = [...chunks].sort((a, b) => {
    if (sortKey === "relevance") return b.relevance - a.relevance;
    if (sortKey === "filePath") return a.filePath.localeCompare(b.filePath);
    return b.tokenCount - a.tokenCount;
  });

  const expandChunk = useCallback((id: number) => {
    setChunks(prev =>
      prev.map(c => (c.id === id ? { ...c, depth: DEPTH_PROMOTE[c.depth], tokenCount: Math.round(c.tokenCount * 1.8) } : c))
    );
  }, []);

  const compressChunk = useCallback((id: number) => {
    setChunks(prev =>
      prev.map(c => (c.id === id ? { ...c, depth: DEPTH_DEMOTE[c.depth], tokenCount: Math.round(c.tokenCount * 0.4) } : c))
    );
  }, []);

  const evictChunk = useCallback(async (id: number) => {
    // Optimistically remove from local state
    setChunks(prev => prev.filter(c => c.id !== id));
    try {
      await invoke<ContextChunk[]>("evict_context_chunk", { workspace: workspacePath, chunkId: id });
    } catch (_e) {
      // Already removed locally; backend call is best-effort
    }
  }, [workspacePath]);

  const pinChunk = useCallback(async (id: number, pinned: boolean) => {
    setChunks(prev => prev.map(c => (c.id === id ? { ...c, pinned } : c)));
    try {
      await invoke<ContextChunk[]>("pin_context_chunk", { workspace: workspacePath, chunkId: id, pinned });
    } catch (_e) {
      // best-effort
    }
  }, [workspacePath]);

  /* ── Project Map actions ─────────────────────────────────────────── */

  const toggleDir = useCallback((path: string) => {
    const toggle = (files: ProjectFile[]): ProjectFile[] =>
      files.map(f =>
        f.path === path && f.isDirectory
          ? { ...f, expanded: !f.expanded }
          : f.children
            ? { ...f, children: toggle(f.children) }
            : f
      );
    setProjectFiles(prev => toggle(prev));
  }, []);

  const loadFile = useCallback((path: string) => {
    const update = (files: ProjectFile[]): ProjectFile[] =>
      files.map(f =>
        f.path === path
          ? { ...f, contextStatus: "loaded" as const }
          : f.children
            ? { ...f, children: update(f.children) }
            : f
      );
    setProjectFiles(prev => update(prev));
    // Also add a chunk
    setChunks(prev => [
      ...prev,
      { id: nextChunkId++, filePath: path, depth: "Full", relevance: 0.5, tokenCount: 500 },
    ]);
  }, []);

  const summarizeDir = useCallback((path: string) => {
    const update = (files: ProjectFile[]): ProjectFile[] =>
      files.map(f => {
        if (f.path === path && f.isDirectory && f.children) {
          return { ...f, children: f.children.map(c => ({ ...c, contextStatus: "summarized" as const })) };
        }
        return f.children ? { ...f, children: update(f.children) } : f;
      });
    setProjectFiles(prev => update(prev));
  }, []);

  const filterFiles = useCallback(
    (files: ProjectFile[]): ProjectFile[] => {
      if (!fileFilter.trim()) return files;
      const q = fileFilter.toLowerCase();
      return files
        .map(f => {
          if (f.isDirectory && f.children) {
            const filteredChildren = filterFiles(f.children);
            if (filteredChildren.length > 0) return { ...f, children: filteredChildren, expanded: true };
          }
          if (f.path.toLowerCase().includes(q)) return f;
          return null;
        })
        .filter(Boolean) as ProjectFile[];
    },
    [fileFilter]
  );

  // Project stats
  const countFiles = (files: ProjectFile[]): { total: number; indexed: number } => {
    let total = 0;
    let indexed = 0;
    for (const f of files) {
      if (!f.isDirectory) {
        total++;
        if (f.contextStatus !== "not-loaded") indexed++;
      }
      if (f.children) {
        const sub = countFiles(f.children);
        total += sub.total;
        indexed += sub.indexed;
      }
    }
    return { total, indexed };
  };
  const fileStats = countFiles(projectFiles);
  const coveragePct = fileStats.total > 0 ? Math.round((fileStats.indexed / fileStats.total) * 100) : 0;

  /* ── Settings actions ────────────────────────────────────────────── */

  const applyMaxTokens = useCallback(() => {
    setMaxTokens(settingsMaxTokens);
  }, [settingsMaxTokens]);

  /* ── Styles ──────────────────────────────────────────────────────── */

  const btnSmall: React.CSSProperties = {
    padding: "2px 8px",
    fontSize: "var(--font-size-xs)",
    border: "1px solid var(--border-color)",
    borderRadius: "var(--radius-xs-plus)",
    background: "var(--bg-secondary)",
    color: "var(--text-primary)",
    cursor: "pointer",
  };

  const btnDanger: React.CSSProperties = {
    ...btnSmall,
    borderColor: "var(--error-color)",
    color: "var(--error-color)",
  };

  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block",
    padding: "1px 8px",
    borderRadius: "var(--radius-md)",
    fontSize: "var(--font-size-xs)",
    fontWeight: 600,
    background: color,
    color: "var(--bg-primary)",
    marginRight: "6px",
  });

  const sliderLabelStyle: React.CSSProperties = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    marginBottom: "4px",
    fontSize: "var(--font-size-base)",
  };

  /* ── Render helpers ──────────────────────────────────────────────── */

  const renderFileTree = (files: ProjectFile[], depth: number = 0): React.ReactNode =>
    files.map(f => (
      <div key={f.path}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "6px",
            padding: "4px 0",
            paddingLeft: `${depth * 16}px`,
            fontSize: "var(--font-size-base)",
            borderBottom: "1px solid var(--border-color)",
          }}
        >
          {f.isDirectory ? (
            <span
              style={{ cursor: "pointer", userSelect: "none", width: "14px", textAlign: "center" }}
              onClick={() => toggleDir(f.path)}
            >
              {f.expanded ? "\u25BE" : "\u25B8"}
            </span>
          ) : (
            <span style={{ width: "14px" }} />
          )}
          <span style={{ color: STATUS_COLORS[f.contextStatus], fontSize: "var(--font-size-xs)" }}>
            {STATUS_ICONS[f.contextStatus]}
          </span>
          <span style={{ flex: 1, color: "var(--text-primary)" }}>
            {f.isDirectory ? f.path + "/" : f.path.split("/").pop()}
          </span>
          <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", minWidth: "50px", textAlign: "right" }}>
            {fmtTokens(f.tokenEstimate)}
          </span>
          <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", minWidth: "120px", textAlign: "right" }}>
            {f.lastModified}
          </span>
          <span style={{ color: "var(--accent-color)", fontSize: "var(--font-size-sm)", minWidth: "36px", textAlign: "right" }}>
            {fmtPct(f.relevance)}
          </span>
          {!f.isDirectory && f.contextStatus === "not-loaded" && (
            <button style={btnSmall} onClick={() => loadFile(f.path)}>
              Load
            </button>
          )}
          {f.isDirectory && (
            <button style={btnSmall} onClick={() => summarizeDir(f.path)}>
              Summarize All
            </button>
          )}
        </div>
        {f.isDirectory && f.expanded && f.children && renderFileTree(f.children, depth + 1)}
      </div>
    ));

  const renderSlider = (
    label: string,
    value: number,
    setValue: (v: number) => void
  ): React.ReactNode => (
    <div style={{ marginBottom: "12px" }}>
      <div style={sliderLabelStyle}>
        <span style={{ color: "var(--text-primary)" }}>{label}</span>
        <span style={{ color: "var(--accent-color)", fontWeight: 600 }}>{value.toFixed(2)}</span>
      </div>
      <input
        type="range"
        min={0}
        max={1}
        step={0.05}
        value={value}
        onChange={e => setValue(parseFloat(e.target.value))}
        style={{ width: "100%", accentColor: "var(--accent-color)" }}
      />
    </div>
  );

  /* ── Tab pane helper — keeps all tabs mounted ────────────────────── */

  const tabPane = (id: TabId, content: React.ReactNode) => (
    <div
      key={id}
      style={{
        flex: 1,
        overflow: "auto",
        padding: "12px",
        display: activeTab === id ? "block" : "none",
      }}
    >
      {content}
    </div>
  );

  /* ── Main render ─────────────────────────────────────────────────── */

  if (!hasWorkspace) {
    return (
      <div className="panel-empty">
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 8, color: "var(--text-primary)" }}>Infinite Context Manager</div>
        <p>Open a folder to use context indexing.</p>
      </div>
    );
  }

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)" }}>Infinite Context Manager</div>
        <button className="panel-btn panel-btn-secondary" onClick={() => { loadChunks(); loadProjectTree(); }} disabled={loading}>
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {error && (
        <div className="panel-error">{error}</div>
      )}

      {/* Tab bar */}
      <div className="panel-tab-bar">
        <button className={`panel-tab${activeTab === "context" ? " active" : ""}`} onClick={() => setActiveTab("context")}>
          Context Window
        </button>
        <button className={`panel-tab${activeTab === "projectMap" ? " active" : ""}`} onClick={() => setActiveTab("projectMap")}>
          Project Map
        </button>
        <button className={`panel-tab${activeTab === "settings" ? " active" : ""}`} onClick={() => setActiveTab("settings")}>
          Settings
        </button>
      </div>

      {/* ── Tab 1: Context Window ─────────────────────────────────────── */}
      {tabPane("context", (
        <div>
          {/* Token usage bar */}
          <div className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "6px" }}>
              <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Token Usage</span>
              <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600 }}>
                {fmtTokens(usedTokens)} / {fmtTokens(maxTokens)} tokens
              </span>
            </div>
            <div style={{ height: "8px", borderRadius: "var(--radius-xs-plus)", background: "var(--bg-primary)", overflow: "hidden" }}>
              <div style={{
                height: "100%",
                width: `${Math.min(usagePct, 100)}%`,
                borderRadius: "var(--radius-xs-plus)",
                background: usagePct > 90 ? "var(--error-color)" : usagePct > 70 ? "var(--warning-color)" : "var(--success-color)",
                transition: "width 0.3s ease",
              }} />
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", marginTop: "6px", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
              <span>{usagePct.toFixed(1)}% used</span>
              <span>Compression: {fmtPct(compressionRatio)}</span>
            </div>
          </div>

          {/* Sort + legend row */}
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "10px", flexWrap: "wrap", gap: "8px" }}>
            <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
              <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Sort:</span>
              {(["relevance", "filePath", "tokenCount"] as SortKey[]).map(key => (
                <button
                  key={key}
                  style={{ ...btnSmall, borderColor: sortKey === key ? "var(--accent-color)" : "var(--border-color)", color: sortKey === key ? "var(--accent-color)" : "var(--text-secondary)" }}
                  onClick={() => setSortKey(key)}
                >
                  {key === "filePath" ? "Path" : key === "tokenCount" ? "Size" : "Relevance"}
                </button>
              ))}
            </div>
            <div style={{ display: "flex", gap: "10px", fontSize: "var(--font-size-xs)" }}>
              {(Object.keys(DEPTH_COLORS) as DepthLevel[]).map(d => (
                <span key={d} style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                  <span style={{ width: "8px", height: "8px", borderRadius: "50%", background: DEPTH_COLORS[d], display: "inline-block" }} />
                  <span style={{ color: "var(--text-secondary)" }}>{d}</span>
                </span>
              ))}
            </div>
          </div>

          {/* Chunk list */}
          {loading && chunks.length === 0 && (
            <div className="panel-loading">Loading context chunks...</div>
          )}
          {!loading && sortedChunks.length === 0 && (
            <div className="panel-empty">No context chunks loaded. Use the Project Map to load files.</div>
          )}
          {sortedChunks.map(chunk => (
            <div key={chunk.id} className="panel-card" style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px", marginBottom: "4px", flexWrap: "wrap" }}>
                  <span style={{ fontWeight: 600, color: "var(--text-primary)", wordBreak: "break-all" }}>{chunk.filePath}</span>
                  <span style={badgeStyle(DEPTH_COLORS[chunk.depth])}>{chunk.depth}</span>
                  {chunk.pinned && <span style={{ fontSize: "var(--font-size-xs)", color: "var(--warning-color)", fontWeight: 600 }}>PINNED</span>}
                </div>
                <div style={{ display: "flex", gap: "12px", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                  <span>Relevance: <span style={{ color: "var(--accent-color)" }}>{fmtPct(chunk.relevance)}</span></span>
                  <span>Tokens: {fmtTokens(chunk.tokenCount)}</span>
                </div>
              </div>
              <div style={{ display: "flex", gap: "4px", flexShrink: 0, flexWrap: "wrap" }}>
                <button style={btnSmall} onClick={() => pinChunk(chunk.id, !chunk.pinned)}>{chunk.pinned ? "Unpin" : "Pin"}</button>
                <button style={{ ...btnSmall, opacity: chunk.depth === "Full" ? 0.4 : 1 }} disabled={chunk.depth === "Full"} onClick={() => expandChunk(chunk.id)}>Expand</button>
                <button style={{ ...btnSmall, opacity: chunk.depth === "Signatures" ? 0.4 : 1 }} disabled={chunk.depth === "Signatures"} onClick={() => compressChunk(chunk.id)}>Compress</button>
                <button style={{ ...btnSmall, ...btnDanger }} onClick={() => evictChunk(chunk.id)}>Evict</button>
              </div>
            </div>
          ))}
        </div>
      ))}

      {/* ── Tab 2: Project Map ────────────────────────────────────────── */}
      {tabPane("projectMap", (
        <div>
          {/* Stats bar */}
          <div style={{ display: "flex", gap: "12px", flexWrap: "wrap", marginBottom: "12px" }}>
            {[
              { label: "Total Files", value: String(fileStats.total) },
              { label: "Indexed", value: String(fileStats.indexed) },
              { label: "Coverage", value: `${coveragePct}%` },
            ].map(({ label, value }) => (
              <div key={label} style={{ background: "var(--bg-secondary)", padding: "8px 14px", borderRadius: "var(--radius-sm)", textAlign: "center", minWidth: "80px" }}>
                <div style={{ fontSize: "18px", fontWeight: "bold", color: "var(--accent-color)" }}>{value}</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: "2px" }}>{label}</div>
              </div>
            ))}
          </div>

          {/* Search/filter */}
          <input
            value={fileFilter}
            onChange={e => setFileFilter(e.target.value)}
            placeholder="Filter files..."
            style={{ width: "100%", padding: "6px 10px", fontSize: "var(--font-size-base)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", boxSizing: "border-box", marginBottom: "10px" }}
          />

          {/* Legend */}
          <div style={{ display: "flex", gap: "12px", marginBottom: "8px", fontSize: "var(--font-size-xs)" }}>
            {(["loaded", "summarized", "not-loaded"] as const).map(s => (
              <span key={s} style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                <span style={{ color: STATUS_COLORS[s] }}>{STATUS_ICONS[s]}</span>
                <span style={{ color: "var(--text-secondary)" }}>{s}</span>
              </span>
            ))}
          </div>

          {/* File tree */}
          <div className="panel-card" style={{ padding: "6px 10px", overflow: "auto" }}>
            {loading && projectFiles.length === 0 && (
              <div className="panel-loading">Loading project tree...</div>
            )}
            {renderFileTree(filterFiles(projectFiles))}
            {!loading && filterFiles(projectFiles).length === 0 && (
              <div className="panel-empty">No files match filter.</div>
            )}
          </div>
        </div>
      ))}

      {/* ── Tab 3: Settings ───────────────────────────────────────────── */}
      {tabPane("settings", (
        <div>
          {/* Max tokens slider */}
          <div className="panel-card">
            <div style={sliderLabelStyle}>
              <span style={{ color: "var(--text-primary)", fontWeight: 600 }}>Max Tokens</span>
              <span style={{ color: "var(--accent-color)", fontWeight: 600 }}>{fmtTokens(settingsMaxTokens)}</span>
            </div>
            <input
              type="range" min={10_000} max={500_000} step={10_000}
              value={settingsMaxTokens}
              onChange={e => setSettingsMaxTokens(parseInt(e.target.value, 10))}
              style={{ width: "100%", accentColor: "var(--accent-color)" }}
            />
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: "2px" }}>
              <span>10K</span><span>500K</span>
            </div>
            <button className="panel-btn panel-btn-secondary" style={{ marginTop: "8px" }} onClick={applyMaxTokens}>Apply</button>
          </div>

          {/* Scoring weights */}
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: "10px", fontSize: "var(--font-size-md)" }}>Scoring Weights</div>
            {renderSlider("Recency", recencyWeight, setRecencyWeight)}
            {renderSlider("Proximity", proximityWeight, setProximityWeight)}
            {renderSlider("Keyword Match", keywordWeight, setKeywordWeight)}
            {renderSlider("Dependency", dependencyWeight, setDependencyWeight)}
            {renderSlider("Access Frequency", accessFreqWeight, setAccessFreqWeight)}
          </div>

          {/* Auto-compress toggle */}
          <div className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>Auto-Compress</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: "2px" }}>Automatically compress when 90% full</div>
              </div>
              <button
                style={{ ...btnSmall, background: autoCompress ? "var(--success-color)" : "var(--bg-secondary)", color: autoCompress ? "#000" : "var(--text-primary)", fontWeight: 600, minWidth: "50px" }}
                onClick={() => setAutoCompress(prev => !prev)}
              >
                {autoCompress ? "ON" : "OFF"}
              </button>
            </div>
          </div>

          {/* Cache settings */}
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: "8px", fontSize: "var(--font-size-md)" }}>Cache</div>
            <div style={{ display: "flex", gap: "8px", alignItems: "center", marginBottom: "8px" }}>
              <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", minWidth: "100px" }}>Cache Size</label>
              <input
                type="number" min={16} max={4096} value={cacheSize}
                onChange={e => setCacheSize(parseInt(e.target.value, 10) || 256)}
                style={{ width: "80px", padding: "4px 8px", fontSize: "var(--font-size-base)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)" }}
              />
              <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>summaries</span>
            </div>
            <div style={{ display: "flex", gap: "8px" }}>
              <button style={btnDanger} onClick={() => setCacheSize(256)}>Clear Cache</button>
              <button style={btnSmall} onClick={() => { loadChunks(); loadProjectTree(); }}>Rebuild Index</button>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
