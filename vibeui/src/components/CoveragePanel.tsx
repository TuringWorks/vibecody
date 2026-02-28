import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface FileCoverage {
  path: string;
  covered: number;
  total: number;
  pct: number;
  uncovered_lines: number[];
}

interface CoverageResult {
  framework: string;
  total_pct: number;
  files: FileCoverage[];
  raw_output: string;
}

interface CoveragePanelProps {
  workspacePath: string | null;
}

type Filter = "all" | "partial" | "uncovered";

const pctColor = (pct: number) => {
  if (pct >= 80) return "var(--text-success, #4caf50)";
  if (pct >= 50) return "var(--text-warning, #ff9800)";
  return "var(--text-danger, #f44336)";
};

const toolLabel: Record<string, string> = {
  "cargo-llvm-cov": "Cargo llvm-cov",
  nyc: "nyc (Istanbul)",
  "npm-coverage": "npm coverage",
  "coverage.py": "coverage.py",
  "go-cover": "Go cover",
};

export function CoveragePanel({ workspacePath }: CoveragePanelProps) {
  const [tool, setTool] = useState<string | null>(null);
  const [result, setResult] = useState<CoverageResult | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [showRaw, setShowRaw] = useState(false);

  useEffect(() => {
    if (!workspacePath) return;
    invoke<string>("detect_coverage_tool", { workspace: workspacePath })
      .then(setTool)
      .catch(() => setTool(null));
  }, [workspacePath]);

  const handleRun = async () => {
    if (!workspacePath || !tool) return;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<CoverageResult>("run_coverage", {
        workspace: workspacePath,
        tool,
      });
      // Sort files by pct ascending (worst coverage first)
      r.files.sort((a, b) => a.pct - b.pct);
      setResult(r);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  const toggleExpand = (path: string) => {
    setExpanded(prev => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path); else next.add(path);
      return next;
    });
  };

  const filteredFiles = result?.files.filter(f => {
    if (filter === "partial") return f.pct > 0 && f.pct < 100;
    if (filter === "uncovered") return f.pct === 0;
    return true;
  }) ?? [];

  const barWidth = (pct: number) => `${Math.max(2, Math.min(100, pct))}%`;

  return (
    <div style={{ padding: "12px", fontFamily: "monospace", fontSize: "13px", height: "100%", overflowY: "auto" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px", flexWrap: "wrap" }}>
        <span style={{ fontWeight: "bold" }}>🧪 Coverage</span>
        {tool && (
          <span style={{ background: "var(--bg-secondary, #2d2d2d)", padding: "2px 8px", borderRadius: "4px", fontSize: "11px" }}>
            {toolLabel[tool] ?? tool}
          </span>
        )}
        {!tool && !workspacePath && (
          <span style={{ color: "var(--text-muted, #888)" }}>No workspace open</span>
        )}
        {!tool && workspacePath && (
          <span style={{ color: "var(--text-muted, #888)" }}>No coverage tool detected</span>
        )}
        <button
          onClick={handleRun}
          disabled={running || !tool || !workspacePath}
          style={{
            marginLeft: "auto",
            background: running ? "var(--bg-secondary, #2d2d2d)" : "var(--accent, #007acc)",
            color: "#fff", border: "none", borderRadius: "4px",
            padding: "4px 12px", cursor: running ? "default" : "pointer",
          }}
        >
          {running ? "⏳ Running…" : "▶ Run Coverage"}
        </button>
      </div>

      {error && (
        <div style={{ background: "rgba(244,67,54,0.1)", color: "#f44336", padding: "8px", borderRadius: "4px", marginBottom: "12px", whiteSpace: "pre-wrap" }}>
          {error}
        </div>
      )}

      {result && (
        <>
          {/* Summary bar */}
          <div style={{ marginBottom: "14px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
              <span style={{ color: pctColor(result.total_pct), fontWeight: "bold", fontSize: "16px" }}>
                {result.total_pct.toFixed(1)}%
              </span>
              <span style={{ color: "var(--text-muted, #888)", fontSize: "11px" }}>
                {result.files.length} files
              </span>
            </div>
            <div style={{ background: "var(--bg-secondary, #2d2d2d)", borderRadius: "3px", height: "6px", overflow: "hidden" }}>
              <div style={{ background: pctColor(result.total_pct), width: barWidth(result.total_pct), height: "100%", transition: "width 0.4s" }} />
            </div>
          </div>

          {/* Filter tabs */}
          <div style={{ display: "flex", gap: "6px", marginBottom: "10px" }}>
            {(["all", "partial", "uncovered"] as Filter[]).map(f => (
              <button
                key={f}
                onClick={() => setFilter(f)}
                style={{
                  background: filter === f ? "var(--accent, #007acc)" : "var(--bg-secondary, #2d2d2d)",
                  color: filter === f ? "#fff" : "var(--text-muted, #888)",
                  border: "none", borderRadius: "4px", padding: "2px 10px",
                  cursor: "pointer", fontSize: "11px",
                }}
              >
                {f === "all" ? `All (${result.files.length})` : f === "partial" ? `Partial (${result.files.filter(x => x.pct > 0 && x.pct < 100).length})` : `Uncovered (${result.files.filter(x => x.pct === 0).length})`}
              </button>
            ))}
            <button
              onClick={() => setShowRaw(r => !r)}
              style={{
                marginLeft: "auto",
                background: showRaw ? "var(--accent, #007acc)" : "var(--bg-secondary, #2d2d2d)",
                color: showRaw ? "#fff" : "var(--text-muted, #888)",
                border: "none", borderRadius: "4px", padding: "2px 10px",
                cursor: "pointer", fontSize: "11px",
              }}
            >
              Raw
            </button>
          </div>

          {showRaw ? (
            <pre style={{ background: "var(--bg-secondary, #2d2d2d)", padding: "10px", borderRadius: "4px", fontSize: "11px", overflow: "auto", maxHeight: "400px", whiteSpace: "pre-wrap" }}>
              {result.raw_output || "(no output)"}
            </pre>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
              {filteredFiles.length === 0 && (
                <div style={{ color: "var(--text-muted, #888)", textAlign: "center", padding: "20px" }}>
                  No files match the filter.
                </div>
              )}
              {filteredFiles.map(file => {
                const isExpanded = expanded.has(file.path);
                const shortPath = file.path.split("/").slice(-3).join("/");
                return (
                  <div key={file.path} style={{ background: "var(--bg-secondary, #2d2d2d)", borderRadius: "4px", overflow: "hidden" }}>
                    <div
                      onClick={() => toggleExpand(file.path)}
                      style={{ padding: "6px 10px", cursor: "pointer", display: "flex", alignItems: "center", gap: "8px" }}
                    >
                      <span style={{ color: "var(--text-muted, #888)", fontSize: "10px" }}>{isExpanded ? "▼" : "▶"}</span>
                      <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: "var(--text-secondary, #ccc)" }} title={file.path}>
                        {shortPath}
                      </span>
                      <span style={{ color: pctColor(file.pct), fontWeight: "bold", minWidth: "48px", textAlign: "right" }}>
                        {file.pct.toFixed(0)}%
                      </span>
                      <span style={{ color: "var(--text-muted, #888)", fontSize: "11px", minWidth: "80px", textAlign: "right" }}>
                        {file.covered}/{file.total} lines
                      </span>
                      <div style={{ width: "80px", background: "var(--bg-primary, #1e1e1e)", borderRadius: "2px", height: "4px", overflow: "hidden" }}>
                        <div style={{ background: pctColor(file.pct), width: barWidth(file.pct), height: "100%" }} />
                      </div>
                    </div>
                    {isExpanded && file.uncovered_lines.length > 0 && (
                      <div style={{ padding: "6px 10px 8px 28px", borderTop: "1px solid var(--bg-primary, #1e1e1e)" }}>
                        <span style={{ color: "#f44336", fontSize: "11px" }}>
                          Uncovered lines: {file.uncovered_lines.slice(0, 30).join(", ")}
                          {file.uncovered_lines.length > 30 && ` … +${file.uncovered_lines.length - 30} more`}
                        </span>
                      </div>
                    )}
                    {isExpanded && file.uncovered_lines.length === 0 && (
                      <div style={{ padding: "6px 10px 8px 28px", borderTop: "1px solid var(--bg-primary, #1e1e1e)", color: "#4caf50", fontSize: "11px" }}>
                        ✓ All lines covered
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}
    </div>
  );
}
