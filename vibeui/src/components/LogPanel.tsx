/**
 * LogPanel — Log Viewer & Analyzer.
 *
 * Discovers log files in workspace, tails them with level filtering,
 * search, and AI-powered analysis.
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LogEntry {
  line_number: number;
  timestamp: string | null;
  level: string;
  message: string;
  raw: string;
}

interface LogResult {
  source: string;
  entries: LogEntry[];
  total_lines: number;
  error_count: number;
  warn_count: number;
}

interface LogSource {
  name: string;
  path: string;
  size_bytes: number;
  source_type: string;
}

interface LogPanelProps {
  workspacePath: string | null;
}

type LevelFilter = "all" | "error" | "warn" | "info" | "debug";

const levelColor: Record<string, string> = {
  error: "var(--error-color)",
  warn: "var(--warning-color)",
  info: "var(--accent-color)",
  debug: "var(--text-secondary)",
  trace: "var(--text-secondary)",
  unknown: "var(--text-muted)",
};

const levelBadge: Record<string, string> = {
  error: "ERR",
  warn: "WRN",
  info: "INF",
  debug: "DBG",
  trace: "TRC",
  unknown: "---",
};

function fmtBytes(b: number): string {
  if (b >= 1_048_576) return `${(b / 1_048_576).toFixed(1)} MB`;
  if (b >= 1_024) return `${(b / 1_024).toFixed(1)} KB`;
  return `${b} B`;
}

export function LogPanel({ workspacePath }: LogPanelProps) {
  const [sources, setSources] = useState<LogSource[]>([]);
  const [selectedSource, setSelectedSource] = useState<string>("");
  const [customPath, setCustomPath] = useState("");
  const [result, setResult] = useState<LogResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [levelFilter, setLevelFilter] = useState<LevelFilter>("all");
  const [search, setSearch] = useState("");
  const [lineCount, setLineCount] = useState(500);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [analysis, setAnalysis] = useState<string | null>(null);
  const logEndRef = useRef<HTMLDivElement>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Discover log sources on mount
  useEffect(() => {
    if (!workspacePath) return;
    invoke<LogSource[]>("discover_log_sources", { workspace: workspacePath })
      .then((s) => {
        setSources(s);
        if (s.length > 0 && !selectedSource) setSelectedSource(s[0].path);
      })
      .catch(() => setSources([]));
  }, [workspacePath]);

  // Auto-refresh
  useEffect(() => {
    if (autoRefresh && selectedSource) {
      intervalRef.current = setInterval(() => handleTail(), 5_000);
    }
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
      intervalRef.current = null;
    };
  }, [autoRefresh, selectedSource, lineCount, levelFilter]);

  if (!workspacePath) {
    return (
      <div style={{ padding: 16, opacity: 0.6, textAlign: "center" }}>
        <p>Open a workspace folder to view logs.</p>
      </div>
    );
  }

  const handleTail = async () => {
    const src = customPath.trim() || selectedSource;
    if (!src) return;
    setLoading(true);
    setError(null);
    try {
      const r = await invoke<LogResult>("tail_log_file", {
        workspace: workspacePath,
        source: src,
        lines: lineCount,
        filterLevel: levelFilter === "all" ? null : levelFilter,
      });
      setResult(r);
      // Auto-scroll to bottom
      setTimeout(() => logEndRef.current?.scrollIntoView({ behavior: "smooth" }), 50);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleAnalyze = async () => {
    if (!result || result.entries.length === 0) return;
    setAnalyzing(true);
    setAnalysis(null);
    try {
      const lines = result.entries.map((e) => e.raw);
      const text = await invoke<string>("analyze_logs", { entries: lines });
      setAnalysis(text);
    } catch (e: unknown) {
      setAnalysis(`Analysis failed: ${e}`);
    } finally {
      setAnalyzing(false);
    }
  };

  const filtered = result
    ? result.entries.filter((e) => {
        if (search && !e.message.toLowerCase().includes(search.toLowerCase())) return false;
        return true;
      })
    : [];

  return (
    <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflowY: "auto" }}>
      {/* Source selector */}
      <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
        <select
          value={selectedSource}
          onChange={(e) => { setSelectedSource(e.target.value); setCustomPath(""); }}
          style={{
            flex: 1, minWidth: 180, padding: "5px 8px", fontSize: 12,
            background: "var(--bg-secondary)", color: "var(--text-primary)",
            border: "1px solid var(--border-color)", borderRadius: 6,
          }}
        >
          {sources.length === 0 && <option value="">No log files found</option>}
          {sources.map((s) => (
            <option key={s.path} value={s.path}>
              {s.name} ({fmtBytes(s.size_bytes)})
            </option>
          ))}
        </select>
        <input
          type="text"
          placeholder="Custom path or cmd:..."
          value={customPath}
          onChange={(e) => setCustomPath(e.target.value)}
          style={{
            flex: 1, minWidth: 150, padding: "5px 8px", fontSize: 12,
            background: "var(--bg-secondary)", color: "var(--text-primary)",
            border: "1px solid var(--border-color)", borderRadius: 6,
          }}
        />
        <button
          onClick={handleTail}
          disabled={loading}
          style={{
            padding: "5px 14px", fontSize: 12, fontWeight: 600,
            background: loading ? "var(--bg-tertiary)" : "var(--accent-color)",
            color: "var(--text-primary)", border: "none", borderRadius: 6,
            cursor: loading ? "not-allowed" : "pointer",
          }}
        >
          {loading ? "Loading..." : "Load Logs"}
        </button>
      </div>

      {/* Controls bar */}
      <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
        <select
          value={lineCount}
          onChange={(e) => setLineCount(Number(e.target.value))}
          style={{
            padding: "4px 6px", fontSize: 11,
            background: "var(--bg-secondary)", color: "var(--text-primary)",
            border: "1px solid var(--border-color)", borderRadius: 4,
          }}
        >
          <option value={100}>100 lines</option>
          <option value={500}>500 lines</option>
          <option value={1000}>1000 lines</option>
          <option value={2000}>2000 lines</option>
        </select>

        {/* Level filter pills */}
        {(["all", "error", "warn", "info", "debug"] as LevelFilter[]).map((lv) => (
          <button
            key={lv}
            onClick={() => setLevelFilter(lv)}
            style={{
              padding: "3px 10px", fontSize: 11, borderRadius: 12,
              background: levelFilter === lv ? (lv === "all" ? "var(--accent-color)" : levelColor[lv]) : "var(--bg-secondary)",
              border: `1px solid ${levelFilter === lv ? "transparent" : "var(--border-color)"}`,
              color: levelFilter === lv ? "var(--text-primary)" : "var(--text-primary)",
              cursor: "pointer", fontWeight: levelFilter === lv ? 600 : 400,
            }}
          >
            {lv === "all" ? "All" : lv.charAt(0).toUpperCase() + lv.slice(1)}
            {result && lv === "error" && result.error_count > 0 ? ` (${result.error_count})` : ""}
            {result && lv === "warn" && result.warn_count > 0 ? ` (${result.warn_count})` : ""}
          </button>
        ))}

        <label style={{ fontSize: 11, display: "flex", alignItems: "center", gap: 4, cursor: "pointer" }}>
          <input
            type="checkbox"
            checked={autoRefresh}
            onChange={(e) => setAutoRefresh(e.target.checked)}
          />
          Auto-refresh
        </label>

        <input
          type="search"
          placeholder="Search logs..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          style={{
            flex: 1, minWidth: 120, padding: "4px 8px", fontSize: 11,
            background: "var(--bg-secondary)", color: "var(--text-primary)",
            border: "1px solid var(--border-color)", borderRadius: 4,
          }}
        />
      </div>

      {/* Error */}
      {error && (
        <div style={{ background: "color-mix(in srgb, var(--accent-rose) 15%, transparent)", border: "1px solid var(--error-color)", borderRadius: 6, padding: 8, fontSize: 11, color: "var(--error-color)" }}>
          {error}
        </div>
      )}

      {/* Summary bar */}
      {result && (
        <div style={{
          display: "flex", gap: 12, fontSize: 12,
          background: "var(--bg-secondary)", borderRadius: 6, padding: "6px 10px",
          border: "1px solid var(--border-color)",
        }}>
          <span>Lines: <strong>{result.total_lines}</strong></span>
          <span style={{ color: result.error_count > 0 ? "var(--error-color)" : "inherit" }}>
            Errors: <strong>{result.error_count}</strong>
          </span>
          <span style={{ color: result.warn_count > 0 ? "var(--warning-color)" : "inherit" }}>
            Warnings: <strong>{result.warn_count}</strong>
          </span>
          <span>Showing: <strong>{filtered.length}</strong></span>
          <div style={{ flex: 1 }} />
          <button
            onClick={handleAnalyze}
            disabled={analyzing || filtered.length === 0}
            style={{
              padding: "2px 10px", fontSize: 11, fontWeight: 600,
              background: analyzing ? "var(--bg-tertiary)" : "var(--success-color)",
              color: "var(--bg-tertiary)", border: "none", borderRadius: 4,
              cursor: analyzing ? "not-allowed" : "pointer",
            }}
          >
            {analyzing ? "Analyzing..." : "AI Analyze"}
          </button>
        </div>
      )}

      {/* AI Analysis */}
      {analysis && (
        <div style={{
          background: "var(--bg-secondary)", borderRadius: 6, padding: 10,
          border: "1px solid var(--border-color)", fontSize: 12,
          maxHeight: 200, overflowY: "auto", whiteSpace: "pre-wrap",
          fontFamily: "inherit", lineHeight: 1.5,
        }}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
            <strong style={{ color: "var(--success-color)" }}>AI Analysis</strong>
            <button
              onClick={() => setAnalysis(null)}
              style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", fontSize: 11 }}
            >
              Dismiss
            </button>
          </div>
          {analysis}
        </div>
      )}

      {/* Log entries */}
      {result && filtered.length > 0 ? (
        <div style={{
          flex: 1, overflowY: "auto", fontFamily: "var(--font-mono)", fontSize: 11,
          background: "var(--bg-secondary)", borderRadius: 6,
          border: "1px solid var(--border-color)", padding: 4,
        }}>
          {filtered.map((entry) => (
            <div
              key={entry.line_number}
              style={{
                padding: "1px 6px", display: "flex", gap: 6,
                background: entry.level === "error" ? "color-mix(in srgb, var(--accent-rose) 6%, transparent)"
                  : entry.level === "warn" ? "rgba(250,179,135,0.04)"
                  : "transparent",
                borderBottom: "1px solid var(--border-color)",
              }}
            >
              <span style={{ color: "var(--text-secondary)", minWidth: 36, textAlign: "right", userSelect: "none" }}>
                {entry.line_number}
              </span>
              <span style={{
                minWidth: 28, fontWeight: 600, fontSize: 10,
                color: levelColor[entry.level] || "var(--text-secondary)",
              }}>
                {levelBadge[entry.level] || "---"}
              </span>
              {entry.timestamp && (
                <span style={{ color: "var(--text-secondary)", minWidth: 140, fontSize: 10 }}>
                  {entry.timestamp}
                </span>
              )}
              <span style={{ flex: 1, whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
                {entry.message}
              </span>
            </div>
          ))}
          <div ref={logEndRef} />
        </div>
      ) : result ? (
        <div style={{ padding: 16, opacity: 0.5, fontSize: 12, textAlign: "center" }}>
          No log entries match the current filter.
        </div>
      ) : null}

      {/* Empty state */}
      {!result && !loading && !error && (
        <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", opacity: 0.4, fontSize: 12 }}>
          {sources.length > 0 ? "Select a log source and click Load Logs." : "No log files found. Enter a custom path or command (cmd:docker logs ...)."}
        </div>
      )}
    </div>
  );
}
