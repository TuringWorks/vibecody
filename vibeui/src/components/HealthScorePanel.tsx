/**
 * HealthScorePanel — Codebase Health Score dashboard.
 *
 * Scan a project to get health scores across 12 dimensions (test coverage,
 * complexity, security, docs, etc.), view trends, and get remediation suggestions.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";

interface DimensionResult {
  dimension: string;
  score: number;
  weight: number;
  details: string;
  remediation: string | null;
}

interface ScanResult {
  overall: number;
  dimensions: DimensionResult[];
  timestamp: number;
}

interface RemediationItem {
  dimension: string;
  priority: string;
  title: string;
  description: string;
  impact: number;
  autoFixable: boolean;
}

type Tab = "scan" | "remediate";

const scoreColor = (s: number) =>
  s >= 80 ? "var(--success-color)" : s >= 60 ? "var(--warning-color)" : "var(--error-color)";

const priorityTag = (p: string) =>
  p === "Critical" ? "panel-tag panel-tag-danger"
  : p === "High" ? "panel-tag panel-tag-warning"
  : "panel-tag panel-tag-neutral";

export default function HealthScorePanel() {
  const [tab, setTab] = useState<Tab>("scan");
  const [path, setPath] = useState(() => localStorage.getItem("vibeui_workspace") || ".");
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [remediations, setRemediations] = useState<RemediationItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const doScan = useCallback(async () => {
    setLoading(true);
    setScan(null);
    setError("");
    try {
      const res = await invoke<ScanResult>("healthscore_scan", { path });
      setScan(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [path]);

  const doRemediate = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const res = await invoke<{ remediations: RemediationItem[] }>("healthscore_remediate", { path });
      setRemediations(res.remediations);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [path]);

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Codebase Health Score</h3>
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          {(["scan", "remediate"] as Tab[]).map(t => (
            <button
              key={t}
              className={`panel-tab panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`}
              onClick={() => setTab(t)}
            >
              {t === "scan" ? "Scan" : "Remediate"}
            </button>
          ))}
        </div>
      </div>

      <div className="panel-body">
        {/* Path + action row */}
        <div className="panel-card panel-row" style={{ marginBottom: 10 }}>
          <span className="panel-label" style={{ marginBottom: 0, whiteSpace: "nowrap" }}>Path</span>
          <input
            className="panel-input panel-input-full"
            value={path}
            onChange={e => setPath(e.target.value)}
          />
          <button
            className="panel-btn panel-btn-primary"
            onClick={tab === "scan" ? doScan : doRemediate}
            disabled={loading}
          >
            {loading ? "…" : tab === "scan" ? "Scan" : "Analyze"}
          </button>
        </div>

        {error && (
          <div className="panel-error" style={{ marginBottom: 10 }}>
            {error}
            <button onClick={() => setError("")} aria-label="Dismiss error"><X size={12} /></button>
          </div>
        )}

        {loading && <div className="panel-loading">Analyzing codebase…</div>}

        {/* Scan results */}
        {tab === "scan" && scan && !loading && (
          <>
            {/* Overall score */}
            <div className="panel-card" style={{ textAlign: "center", marginBottom: 10 }}>
              <div style={{ fontSize: "var(--font-size-3xl)", fontWeight: "var(--font-bold)", color: scoreColor(scan.overall) }}>
                {scan.overall.toFixed(0)}
              </div>
              <div className="panel-label" style={{ marginTop: 4, marginBottom: 0 }}>Overall Health Score</div>
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginTop: 2 }}>
                {scan.dimensions.length} dimensions · {path}
              </div>
            </div>

            {/* Dimension cards */}
            {scan.dimensions.map(d => (
              <div key={d.dimension} className="panel-card" style={{ marginBottom: 8 }}>
                <div className="panel-row" style={{ marginBottom: 6 }}>
                  <span style={{ fontWeight: "var(--font-semibold)", fontSize: "var(--font-size-base)" }}>{d.dimension}</span>
                  <span style={{ marginLeft: "auto", color: scoreColor(d.score), fontWeight: "var(--font-semibold)" }}>
                    {d.score.toFixed(0)}/100
                  </span>
                </div>
                <div className="progress-bar" style={{ marginBottom: 6 }}>
                  <div
                    className="progress-bar-fill"
                    style={{ width: `${d.score}%`, background: scoreColor(d.score) }}
                  />
                </div>
                <div className="panel-label" style={{ marginBottom: 0 }}>{d.details}</div>
              </div>
            ))}
          </>
        )}

        {tab === "scan" && !scan && !loading && !error && (
          <div className="panel-empty">Click Scan to analyze your codebase health.</div>
        )}

        {/* Remediation results */}
        {tab === "remediate" && remediations.length > 0 && !loading && (
          <>
            <div className="panel-label" style={{ marginBottom: 8 }}>
              {remediations.length} suggestion{remediations.length !== 1 ? "s" : ""}
            </div>
            {remediations.map((r, i) => (
              <div key={i} className="panel-card" style={{ marginBottom: 8 }}>
                <div className="panel-row" style={{ marginBottom: 6 }}>
                  <span style={{ fontWeight: "var(--font-semibold)", fontSize: "var(--font-size-base)" }}>{r.title}</span>
                  <span className={priorityTag(r.priority)} style={{ marginLeft: 8 }}>{r.priority}</span>
                </div>
                <div className="panel-label" style={{ marginBottom: 4 }}>{r.dimension}</div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", marginBottom: 6 }}>
                  {r.description}
                </div>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                  Impact: <span style={{ color: "var(--text-success)", fontWeight: "var(--font-semibold)" }}>
                    +{r.impact.toFixed(0)} pts
                  </span>
                  {r.autoFixable && (
                    <span className="panel-tag panel-tag-success" style={{ marginLeft: 8 }}>auto-fixable</span>
                  )}
                </div>
              </div>
            ))}
          </>
        )}

        {tab === "remediate" && remediations.length === 0 && !loading && !error && (
          <div className="panel-empty">Click Analyze to get improvement suggestions.</div>
        )}
      </div>
    </div>
  );
}
