import { useState, useCallback } from "react";

/* ── Types ───────────────────────────────────────────────────────────── */

type DepthLevel = "Full" | "Summary" | "Skeleton" | "Signatures";

interface ContextChunk {
  id: number;
  filePath: string;
  depth: DepthLevel;
  relevance: number;
  tokenCount: number;
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

type SortKey = "relevance" | "filePath" | "tokenCount";
type TabId = "context" | "projectMap" | "settings";

/* ── Constants ───────────────────────────────────────────────────────── */

const DEPTH_COLORS: Record<DepthLevel, string> = {
  Full: "var(--success, #4caf50)",
  Summary: "var(--accent, #007acc)",
  Skeleton: "var(--warning, #ff9800)",
  Signatures: "var(--text-secondary, #888)",
};

const STATUS_ICONS: Record<string, string> = {
  loaded: "\u25CF",
  summarized: "\u25D2",
  "not-loaded": "\u25CB",
};

const STATUS_COLORS: Record<string, string> = {
  loaded: "var(--success, #4caf50)",
  summarized: "var(--warning, #ff9800)",
  "not-loaded": "var(--text-secondary, #888)",
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

/* ── Helpers ─────────────────────────────────────────────────────────── */

const fmtTokens = (n: number): string =>
  n >= 1_000_000
    ? `${(n / 1_000_000).toFixed(1)}M`
    : n >= 1_000
      ? `${(n / 1_000).toFixed(1)}K`
      : String(n);

const fmtPct = (v: number): string => `${Math.round(v * 100)}%`;

let nextChunkId = 100;

/* ── Sample data ─────────────────────────────────────────────────────── */

const SAMPLE_CHUNKS: ContextChunk[] = [
  { id: 1, filePath: "src/main.rs", depth: "Full", relevance: 0.95, tokenCount: 3200 },
  { id: 2, filePath: "src/config.rs", depth: "Full", relevance: 0.88, tokenCount: 1800 },
  { id: 3, filePath: "src/agent.rs", depth: "Summary", relevance: 0.72, tokenCount: 850 },
  { id: 4, filePath: "src/provider.rs", depth: "Skeleton", relevance: 0.55, tokenCount: 320 },
  { id: 5, filePath: "src/tools/mod.rs", depth: "Signatures", relevance: 0.41, tokenCount: 120 },
  { id: 6, filePath: "src/hooks.rs", depth: "Summary", relevance: 0.68, tokenCount: 640 },
];

const SAMPLE_PROJECT: ProjectFile[] = [
  {
    path: "src",
    isDirectory: true,
    contextStatus: "loaded",
    tokenEstimate: 12400,
    lastModified: "2026-03-08 10:30",
    relevance: 0.85,
    expanded: true,
    children: [
      { path: "src/main.rs", isDirectory: false, contextStatus: "loaded", tokenEstimate: 3200, lastModified: "2026-03-08 10:30", relevance: 0.95 },
      { path: "src/config.rs", isDirectory: false, contextStatus: "loaded", tokenEstimate: 1800, lastModified: "2026-03-07 14:22", relevance: 0.88 },
      { path: "src/agent.rs", isDirectory: false, contextStatus: "summarized", tokenEstimate: 4200, lastModified: "2026-03-08 09:15", relevance: 0.72 },
      { path: "src/provider.rs", isDirectory: false, contextStatus: "summarized", tokenEstimate: 2100, lastModified: "2026-03-06 16:45", relevance: 0.55 },
      { path: "src/hooks.rs", isDirectory: false, contextStatus: "not-loaded", tokenEstimate: 1100, lastModified: "2026-03-05 11:00", relevance: 0.41 },
    ],
  },
  {
    path: "tests",
    isDirectory: true,
    contextStatus: "not-loaded",
    tokenEstimate: 8600,
    lastModified: "2026-03-08 08:00",
    relevance: 0.35,
    expanded: false,
    children: [
      { path: "tests/integration.rs", isDirectory: false, contextStatus: "not-loaded", tokenEstimate: 5200, lastModified: "2026-03-08 08:00", relevance: 0.35 },
      { path: "tests/unit.rs", isDirectory: false, contextStatus: "not-loaded", tokenEstimate: 3400, lastModified: "2026-03-07 17:30", relevance: 0.28 },
    ],
  },
  { path: "Cargo.toml", isDirectory: false, contextStatus: "not-loaded", tokenEstimate: 420, lastModified: "2026-03-06 12:00", relevance: 0.62 },
];

/* ── Component ───────────────────────────────────────────────────────── */

export function InfiniteContextPanel({ workspacePath }: { workspacePath: string }) {
  const _workspacePath = workspacePath;
  void _workspacePath;

  const [activeTab, setActiveTab] = useState<TabId>("context");

  // Context Window state
  const [chunks, setChunks] = useState<ContextChunk[]>(SAMPLE_CHUNKS);
  const [sortKey, setSortKey] = useState<SortKey>("relevance");
  const [maxTokens, setMaxTokens] = useState(100_000);

  // Project Map state
  const [projectFiles, setProjectFiles] = useState<ProjectFile[]>(SAMPLE_PROJECT);
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

  const evictChunk = useCallback((id: number) => {
    setChunks(prev => prev.filter(c => c.id !== id));
  }, []);

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

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 16px",
    cursor: "pointer",
    background: "none",
    border: "none",
    borderBottom: "2px solid",
    borderBottomColor: active ? "var(--accent, #007acc)" : "transparent",
    color: active ? "var(--text-primary, #fff)" : "var(--text-secondary, #888)",
    fontSize: "13px",
    fontWeight: active ? 600 : 400,
  });

  const btnStyle: React.CSSProperties = {
    padding: "4px 10px",
    fontSize: "11px",
    border: "1px solid var(--border, #444)",
    borderRadius: "4px",
    background: "var(--bg-secondary, #2d2d2d)",
    color: "var(--text-primary, #fff)",
    cursor: "pointer",
  };

  const btnSmall: React.CSSProperties = {
    ...btnStyle,
    padding: "2px 8px",
    fontSize: "10px",
  };

  const btnDanger: React.CSSProperties = {
    ...btnStyle,
    borderColor: "var(--error, #f44336)",
    color: "var(--error, #f44336)",
  };

  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block",
    padding: "1px 8px",
    borderRadius: "10px",
    fontSize: "10px",
    fontWeight: 600,
    background: color,
    color: "#000",
    marginRight: "6px",
  });

  const cardStyle: React.CSSProperties = {
    background: "var(--bg-secondary, #2d2d2d)",
    border: "1px solid var(--border, #444)",
    borderRadius: "6px",
    padding: "10px",
    marginBottom: "8px",
  };

  const sliderLabelStyle: React.CSSProperties = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    marginBottom: "4px",
    fontSize: "12px",
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
            fontSize: "12px",
            borderBottom: "1px solid var(--border, #333)",
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
          <span style={{ color: STATUS_COLORS[f.contextStatus], fontSize: "10px" }}>
            {STATUS_ICONS[f.contextStatus]}
          </span>
          <span style={{ flex: 1, color: "var(--text-primary, #fff)" }}>
            {f.isDirectory ? f.path + "/" : f.path.split("/").pop()}
          </span>
          <span style={{ color: "var(--text-secondary, #888)", fontSize: "11px", minWidth: "50px", textAlign: "right" }}>
            {fmtTokens(f.tokenEstimate)}
          </span>
          <span style={{ color: "var(--text-secondary, #888)", fontSize: "11px", minWidth: "120px", textAlign: "right" }}>
            {f.lastModified}
          </span>
          <span style={{ color: "var(--accent, #007acc)", fontSize: "11px", minWidth: "36px", textAlign: "right" }}>
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
        <span style={{ color: "var(--text-primary, #fff)" }}>{label}</span>
        <span style={{ color: "var(--accent, #007acc)", fontWeight: 600 }}>{value.toFixed(2)}</span>
      </div>
      <input
        type="range"
        min={0}
        max={1}
        step={0.05}
        value={value}
        onChange={e => setValue(parseFloat(e.target.value))}
        style={{ width: "100%", accentColor: "var(--accent, #007acc)" }}
      />
    </div>
  );

  /* ── Main render ─────────────────────────────────────────────────── */

  return (
    <div
      style={{
        padding: "12px",
        fontFamily: "var(--font-family, monospace)",
        color: "var(--text-primary, #fff)",
        height: "100%",
        overflow: "auto",
        fontSize: "13px",
      }}
    >
      <div style={{ fontWeight: "bold", marginBottom: "8px", fontSize: "14px" }}>
        Infinite Context Manager
      </div>

      {/* Tabs */}
      <div
        style={{
          display: "flex",
          gap: "4px",
          borderBottom: "1px solid var(--border, #333)",
          marginBottom: "12px",
        }}
      >
        <button style={tabStyle(activeTab === "context")} onClick={() => setActiveTab("context")}>
          Context Window
        </button>
        <button style={tabStyle(activeTab === "projectMap")} onClick={() => setActiveTab("projectMap")}>
          Project Map
        </button>
        <button style={tabStyle(activeTab === "settings")} onClick={() => setActiveTab("settings")}>
          Settings
        </button>
      </div>

      {/* ── Tab 1: Context Window ─────────────────────────────────────── */}
      {activeTab === "context" && (
        <div>
          {/* Token usage bar */}
          <div style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "6px" }}>
              <span style={{ fontSize: "12px", color: "var(--text-secondary, #888)" }}>Token Usage</span>
              <span style={{ fontSize: "12px", fontWeight: 600 }}>
                {fmtTokens(usedTokens)} / {fmtTokens(maxTokens)} tokens
              </span>
            </div>
            <div
              style={{
                height: "8px",
                borderRadius: "4px",
                background: "var(--bg-primary, #1e1e1e)",
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  height: "100%",
                  width: `${Math.min(usagePct, 100)}%`,
                  borderRadius: "4px",
                  background:
                    usagePct > 90
                      ? "var(--error, #f44336)"
                      : usagePct > 70
                        ? "var(--warning, #ff9800)"
                        : "var(--success, #4caf50)",
                  transition: "width 0.3s ease",
                }}
              />
            </div>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                marginTop: "6px",
                fontSize: "11px",
                color: "var(--text-secondary, #888)",
              }}
            >
              <span>{usagePct.toFixed(1)}% used</span>
              <span>Compression ratio: {fmtPct(compressionRatio)}</span>
            </div>
          </div>

          {/* Sort controls */}
          <div style={{ display: "flex", gap: "6px", alignItems: "center", marginBottom: "10px" }}>
            <span style={{ fontSize: "11px", color: "var(--text-secondary, #888)" }}>Sort by:</span>
            {(["relevance", "filePath", "tokenCount"] as SortKey[]).map(key => (
              <button
                key={key}
                style={{
                  ...btnSmall,
                  borderColor: sortKey === key ? "var(--accent, #007acc)" : "var(--border, #444)",
                  color: sortKey === key ? "var(--accent, #007acc)" : "var(--text-secondary, #888)",
                }}
                onClick={() => setSortKey(key)}
              >
                {key === "filePath" ? "Path" : key === "tokenCount" ? "Size" : "Relevance"}
              </button>
            ))}
          </div>

          {/* Depth legend */}
          <div style={{ display: "flex", gap: "10px", marginBottom: "10px", fontSize: "10px" }}>
            {(Object.keys(DEPTH_COLORS) as DepthLevel[]).map(d => (
              <span key={d} style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                <span
                  style={{
                    width: "8px",
                    height: "8px",
                    borderRadius: "50%",
                    background: DEPTH_COLORS[d],
                    display: "inline-block",
                  }}
                />
                <span style={{ color: "var(--text-secondary, #888)" }}>{d}</span>
              </span>
            ))}
          </div>

          {/* Chunk list */}
          {sortedChunks.length === 0 && (
            <div style={{ color: "var(--text-secondary, #888)", textAlign: "center", padding: "20px" }}>
              No context chunks loaded. Use the Project Map to load files.
            </div>
          )}
          {sortedChunks.map(chunk => (
            <div key={chunk.id} style={{ ...cardStyle, display: "flex", alignItems: "center", gap: "8px" }}>
              <div style={{ flex: 1 }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px", marginBottom: "4px" }}>
                  <span style={{ fontWeight: 600, color: "var(--text-primary, #fff)" }}>{chunk.filePath}</span>
                  <span style={badgeStyle(DEPTH_COLORS[chunk.depth])}>{chunk.depth}</span>
                </div>
                <div style={{ display: "flex", gap: "12px", fontSize: "11px", color: "var(--text-secondary, #888)" }}>
                  <span>Relevance: <span style={{ color: "var(--accent, #007acc)" }}>{fmtPct(chunk.relevance)}</span></span>
                  <span>Tokens: {fmtTokens(chunk.tokenCount)}</span>
                </div>
              </div>
              <div style={{ display: "flex", gap: "4px" }}>
                <button
                  style={{
                    ...btnSmall,
                    opacity: chunk.depth === "Full" ? 0.4 : 1,
                  }}
                  disabled={chunk.depth === "Full"}
                  onClick={() => expandChunk(chunk.id)}
                >
                  Expand
                </button>
                <button
                  style={{
                    ...btnSmall,
                    opacity: chunk.depth === "Signatures" ? 0.4 : 1,
                  }}
                  disabled={chunk.depth === "Signatures"}
                  onClick={() => compressChunk(chunk.id)}
                >
                  Compress
                </button>
                <button style={{ ...btnSmall, ...btnDanger }} onClick={() => evictChunk(chunk.id)}>
                  Evict
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* ── Tab 2: Project Map ────────────────────────────────────────── */}
      {activeTab === "projectMap" && (
        <div>
          {/* Stats bar */}
          <div
            style={{
              display: "flex",
              gap: "12px",
              flexWrap: "wrap",
              marginBottom: "12px",
            }}
          >
            {[
              { label: "Total Files", value: String(fileStats.total) },
              { label: "Indexed", value: String(fileStats.indexed) },
              { label: "Coverage", value: `${coveragePct}%` },
            ].map(({ label, value }) => (
              <div
                key={label}
                style={{
                  background: "var(--bg-secondary, #2d2d2d)",
                  padding: "8px 14px",
                  borderRadius: "6px",
                  textAlign: "center",
                  minWidth: "80px",
                }}
              >
                <div style={{ fontSize: "18px", fontWeight: "bold", color: "var(--accent, #007acc)" }}>
                  {value}
                </div>
                <div style={{ fontSize: "11px", color: "var(--text-secondary, #888)", marginTop: "2px" }}>
                  {label}
                </div>
              </div>
            ))}
          </div>

          {/* Search/filter */}
          <div style={{ marginBottom: "10px" }}>
            <input
              value={fileFilter}
              onChange={e => setFileFilter(e.target.value)}
              placeholder="Filter files..."
              style={{
                width: "100%",
                padding: "6px 10px",
                fontSize: "12px",
                background: "var(--bg-primary, #1e1e1e)",
                border: "1px solid var(--border, #444)",
                borderRadius: "4px",
                color: "var(--text-primary, #fff)",
                boxSizing: "border-box",
              }}
            />
          </div>

          {/* Legend */}
          <div style={{ display: "flex", gap: "12px", marginBottom: "8px", fontSize: "10px" }}>
            {(["loaded", "summarized", "not-loaded"] as const).map(s => (
              <span key={s} style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                <span style={{ color: STATUS_COLORS[s] }}>{STATUS_ICONS[s]}</span>
                <span style={{ color: "var(--text-secondary, #888)" }}>{s}</span>
              </span>
            ))}
          </div>

          {/* File tree */}
          <div style={{ ...cardStyle, padding: "6px 10px" }}>
            {renderFileTree(filterFiles(projectFiles))}
            {filterFiles(projectFiles).length === 0 && (
              <div style={{ color: "var(--text-secondary, #888)", textAlign: "center", padding: "12px" }}>
                No files match filter.
              </div>
            )}
          </div>
        </div>
      )}

      {/* ── Tab 3: Settings ───────────────────────────────────────────── */}
      {activeTab === "settings" && (
        <div>
          {/* Max tokens slider */}
          <div style={cardStyle}>
            <div style={sliderLabelStyle}>
              <span style={{ color: "var(--text-primary, #fff)", fontWeight: 600 }}>Max Tokens</span>
              <span style={{ color: "var(--accent, #007acc)", fontWeight: 600 }}>
                {fmtTokens(settingsMaxTokens)}
              </span>
            </div>
            <input
              type="range"
              min={10_000}
              max={500_000}
              step={10_000}
              value={settingsMaxTokens}
              onChange={e => setSettingsMaxTokens(parseInt(e.target.value, 10))}
              style={{ width: "100%", accentColor: "var(--accent, #007acc)" }}
            />
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                fontSize: "10px",
                color: "var(--text-secondary, #888)",
                marginTop: "2px",
              }}
            >
              <span>10K</span>
              <span>500K</span>
            </div>
            <button
              style={{ ...btnStyle, marginTop: "8px" }}
              onClick={applyMaxTokens}
            >
              Apply
            </button>
          </div>

          {/* Scoring weights */}
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: "10px", fontSize: "13px" }}>
              Scoring Weights
            </div>
            {renderSlider("Recency", recencyWeight, setRecencyWeight)}
            {renderSlider("Proximity", proximityWeight, setProximityWeight)}
            {renderSlider("Keyword Match", keywordWeight, setKeywordWeight)}
            {renderSlider("Dependency", dependencyWeight, setDependencyWeight)}
            {renderSlider("Access Frequency", accessFreqWeight, setAccessFreqWeight)}
          </div>

          {/* Auto-compress toggle */}
          <div style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: "13px", color: "var(--text-primary, #fff)" }}>
                  Auto-Compress
                </div>
                <div style={{ fontSize: "11px", color: "var(--text-secondary, #888)", marginTop: "2px" }}>
                  Automatically compress chunks when context window is 90% full
                </div>
              </div>
              <button
                style={{
                  ...btnStyle,
                  background: autoCompress ? "var(--success, #4caf50)" : "var(--bg-secondary, #2d2d2d)",
                  color: autoCompress ? "#000" : "var(--text-primary, #fff)",
                  fontWeight: 600,
                  minWidth: "50px",
                }}
                onClick={() => setAutoCompress(prev => !prev)}
              >
                {autoCompress ? "ON" : "OFF"}
              </button>
            </div>
          </div>

          {/* Cache settings */}
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: "8px", fontSize: "13px" }}>Cache</div>
            <div style={{ display: "flex", gap: "8px", alignItems: "center", marginBottom: "8px" }}>
              <label style={{ fontSize: "12px", color: "var(--text-secondary, #888)", minWidth: "100px" }}>
                Cache Size
              </label>
              <input
                type="number"
                min={16}
                max={4096}
                value={cacheSize}
                onChange={e => setCacheSize(parseInt(e.target.value, 10) || 256)}
                style={{
                  width: "80px",
                  padding: "4px 8px",
                  fontSize: "12px",
                  background: "var(--bg-primary, #1e1e1e)",
                  border: "1px solid var(--border, #444)",
                  borderRadius: "4px",
                  color: "var(--text-primary, #fff)",
                }}
              />
              <span style={{ fontSize: "11px", color: "var(--text-secondary, #888)" }}>summaries</span>
            </div>
            <div style={{ display: "flex", gap: "8px" }}>
              <button style={btnDanger} onClick={() => setCacheSize(256)}>
                Clear Cache
              </button>
              <button style={btnStyle} onClick={() => void 0}>
                Rebuild Index
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
