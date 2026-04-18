/**
 * CodeMetricsPanel — Code Metrics & Complexity Analyzer.
 *
 * Scans a workspace for source files, reports language breakdown (LOC,
 * code/comment/blank lines), top-10 largest files, and top-10 most complex
 * files (branch-count proxy for cyclomatic complexity).
 */
import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, RefreshCw } from "lucide-react";

interface LanguageStat {
  language: string;
  extension: string;
  file_count: number;
  lines: number;
  code_lines: number;
  comment_lines: number;
  blank_lines: number;
}

interface FileComplexity {
  path: string;
  lines: number;
  complexity: number;
  language: string;
}

interface CodeMetrics {
  total_files: number;
  total_lines: number;
  total_code_lines: number;
  total_comment_lines: number;
  total_blank_lines: number;
  languages: LanguageStat[];
  largest_files: FileComplexity[];
  most_complex: FileComplexity[];
}

interface CodeMetricsPanelProps {
  workspacePath: string | null;
}

const LANG_COLORS: Record<string, string> = {
  // Original entries
  Rust: "#dea584", TypeScript: "#3178c6", JavaScript: "#f7df1e",
  Python: "#4584b6", Go: "#00add8", "C++": "#f34b7d", C: "#555555",
  Java: "#b07219", "C#": "#178600", Ruby: "#701516", Kotlin: "#a97bff",
  Swift: "#fa7343", Shell: "#89e051", SQL: "#e38c00", HTML: "#e34c26",
  CSS: "#563d7c", JSON: "var(--accent-gold)", YAML: "#cb171e", TOML: "#9c4121",
  Markdown: "#083fa1", Dart: "#00b4ab", Zig: "#ec915c", Lua: "#000080",
  // TIOBE top-50 additions
  PHP: "#777bb3", Perl: "#39457e", Fortran: "#4d41b1", MATLAB: "#e16737",
  Assembly: "#6e4c13", Ada: "#02f88c",
  "Objective-C": "#438eff", Haskell: "#5e5086", Scala: "#dc322f",
  Erlang: "#b83998", Julia: "#a270ba",
  "Visual Basic": "#945db7", R: "#276dc3",
  COBOL: "#005ca5", PowerShell: "#012456", Solidity: "#363636",
  Lisp: "#3fb68b", "PL/SQL": "#da2b2b", "Transact-SQL": "#e38c00",
  OCaml: "#ef7a08", Prolog: "#74283c", ABAP: "#e8274b", SAS: "#1e90ff",
};

function pct(part: number, total: number) {
  return total === 0 ? 0 : Math.round((part / total) * 100);
}

function fmt(n: number) {
  return n.toLocaleString();
}

function LangBar({ value, max, color }: { value: number; max: number; color: string }) {
  const w = max === 0 ? 0 : Math.max(2, Math.round((value / max) * 100));
  return (
    <div className="progress-bar" style={{ marginTop: 4 }}>
      <div className="progress-bar-fill" style={{ width: `${w}%`, background: color, transition: "width 0.3s" }} />
    </div>
  );
}

export function CodeMetricsPanel({ workspacePath }: CodeMetricsPanelProps) {
  const [metrics, setMetrics] = useState<CodeMetrics | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<"languages" | "files" | "complexity">("languages");
  const cancelRef = useRef(false);
  const taskIdRef = useRef(0);

  const handleSuspend = () => {
    cancelRef.current = true;
    setScanning(false);
    setError("Scan suspended by user.");
  };

  const scan = async () => {
    if (!workspacePath || scanning) return;
    cancelRef.current = false;
    taskIdRef.current += 1;
    const thisId = taskIdRef.current;
    setScanning(true);
    setError(null);
    try {
      const result = await invoke<CodeMetrics>("analyze_code_metrics", { workspace: workspacePath });
      if (cancelRef.current || taskIdRef.current !== thisId) return;
      setMetrics(result);
    } catch (e) {
      if (cancelRef.current || taskIdRef.current !== thisId) return;
      setError(String(e));
    } finally {
      if (!cancelRef.current && taskIdRef.current === thisId) {
        setScanning(false);
      }
    }
  };

  if (!workspacePath) {
    return (
      <div className="panel-container">
        <div className="panel-empty">Open a workspace to analyze code metrics.</div>
      </div>
    );
  }

  const maxLines = metrics ? Math.max(...metrics.languages.map(l => l.lines), 1) : 1;

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <div style={{ flex: 1 }}>
          <span style={{ fontWeight: "var(--font-semibold)", fontSize: "var(--font-size-lg)" }}>Code Metrics</span>
          {metrics && (
            <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 2 }}>
              {fmt(metrics.total_files)} files · {fmt(metrics.total_lines)} lines · {metrics.languages.length} languages
            </div>
          )}
        </div>
        {scanning ? (
          <button className="panel-btn panel-btn-danger" onClick={handleSuspend}>
            Suspend
          </button>
        ) : (
          <button className="panel-btn panel-btn-primary" onClick={scan}>
            {metrics ? <><RefreshCw size={13} /> Re-scan</> : "Scan"}
          </button>
        )}
      </div>

      {error && (
        <div className="panel-error" style={{ margin: "8px 12px" }}>
          {error}
          <button onClick={() => setError(null)}><X size={12} /></button>
        </div>
      )}

      {/* Summary stat row */}
      {metrics && (
        <div className="panel-stats" style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)" }}>
          {[
            { label: "Total LOC", value: fmt(metrics.total_lines), sub: "lines" },
            { label: "Code", value: fmt(metrics.total_code_lines), sub: `${pct(metrics.total_code_lines, metrics.total_lines)}%` },
            { label: "Comments", value: fmt(metrics.total_comment_lines), sub: `${pct(metrics.total_comment_lines, metrics.total_lines)}%` },
            { label: "Blank", value: fmt(metrics.total_blank_lines), sub: `${pct(metrics.total_blank_lines, metrics.total_lines)}%` },
            { label: "Files", value: fmt(metrics.total_files), sub: `${metrics.languages.length} langs` },
          ].map(({ label, value, sub }) => (
            <div key={label} className="panel-stat" style={{ padding: "8px 4px" }}>
              <div style={{ fontSize: "var(--font-size-lg)", fontWeight: "var(--font-bold)" }}>{value}</div>
              <div className="panel-stat-label">{label}</div>
              <div className="panel-stat-sub">{sub}</div>
            </div>
          ))}
        </div>
      )}

      {/* Sub-tab bar */}
      {metrics && (
        <div className="panel-tab-bar">
          {(["languages", "files", "complexity"] as const).map(v => (
            <button
              key={v}
              className={`panel-tab ${view === v ? "active" : ""}`}
              onClick={() => setView(v)}
            >
              {v === "languages" ? `Languages (${metrics.languages.length})` : v === "files" ? "Largest Files" : "Most Complex"}
            </button>
          ))}
        </div>
      )}

      {/* Content */}
      <div className="panel-body">
        {scanning && <div className="panel-loading">Scanning workspace…</div>}

        {!metrics && !scanning && !error && (
          <div className="panel-empty">Click Scan to analyse this workspace.</div>
        )}

        {/* Languages view */}
        {metrics && view === "languages" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {metrics.languages.map(lang => {
              const color = LANG_COLORS[lang.language] ?? "var(--accent-blue)";
              return (
                <div key={lang.language} className="panel-card">
                  <div className="panel-row" style={{ marginBottom: 4 }}>
                    <span style={{ width: 10, height: 10, borderRadius: "50%", background: color, flexShrink: 0, display: "inline-block" }} />
                    <span style={{ fontSize: "var(--font-size-base)", fontWeight: "var(--font-semibold)", flex: 1 }}>{lang.language}</span>
                    <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{lang.file_count} files</span>
                    <span style={{ fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", fontWeight: "var(--font-semibold)" }}>
                      {fmt(lang.lines)}
                    </span>
                    <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", width: 34, textAlign: "right" }}>
                      {pct(lang.lines, metrics.total_lines)}%
                    </span>
                  </div>
                  <LangBar value={lang.lines} max={maxLines} color={color} />
                  <div style={{ display: "flex", gap: 12, marginTop: 5, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                    <span>Code: {fmt(lang.code_lines)} ({pct(lang.code_lines, lang.lines)}%)</span>
                    <span>Comments: {fmt(lang.comment_lines)} ({pct(lang.comment_lines, lang.lines)}%)</span>
                    <span>Blank: {fmt(lang.blank_lines)}</span>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* Largest files view */}
        {metrics && view === "files" && (
          <table className="panel-table">
            <thead>
              <tr>
                <th>File</th>
                <th style={{ textAlign: "right", width: 70 }}>Lines</th>
                <th style={{ textAlign: "right", width: 60 }}>Lang</th>
              </tr>
            </thead>
            <tbody>
              {metrics.largest_files.map((f, i) => (
                <tr key={f.path}>
                  <td className="panel-mono" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 0 }} title={f.path}>
                    <span style={{ color: "var(--text-muted)", marginRight: 6 }}>{i + 1}.</span>{f.path}
                  </td>
                  <td className="panel-mono" style={{ textAlign: "right" }}>{fmt(f.lines)}</td>
                  <td style={{ textAlign: "right", color: LANG_COLORS[f.language] ?? "var(--accent-blue)", fontSize: "var(--font-size-xs)" }}>
                    {f.language}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {/* Most complex view */}
        {metrics && view === "complexity" && (
          <div style={{ display: "flex", flexDirection: "column" }}>
            <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", padding: "4px 0 8px", fontStyle: "italic" }}>
              Complexity = count of branch-inducing keywords (if/for/while/match/&&/||…)
            </div>
            {metrics.most_complex.map((f, i) => {
              const maxC = metrics.most_complex[0]?.complexity ?? 1;
              const bar = Math.max(4, Math.round((f.complexity / maxC) * 100));
              const color = f.complexity > maxC * 0.7 ? "var(--error-color)"
                : f.complexity > maxC * 0.4 ? "var(--warning-color)"
                : "var(--success-color)";
              return (
                <div key={f.path} style={{ padding: "8px 0", borderBottom: "1px solid var(--border-subtle)" }}>
                  <div className="panel-row" style={{ marginBottom: 4 }}>
                    <span className="panel-mono" style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: "var(--font-size-sm)" }} title={f.path}>
                      <span style={{ color: "var(--text-muted)", marginRight: 6 }}>{i + 1}.</span>{f.path}
                    </span>
                    <span className="panel-mono" style={{ fontWeight: "var(--font-semibold)", color, minWidth: 50, textAlign: "right" }}>
                      {fmt(f.complexity)}
                    </span>
                    <span className="panel-mono" style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", minWidth: 50, textAlign: "right" }}>
                      {fmt(f.lines)} ln
                    </span>
                  </div>
                  <div className="progress-bar progress-bar-sm">
                    <div className="progress-bar-fill" style={{ width: `${bar}%`, background: color }} />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
