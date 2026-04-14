import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Baseline {
  id: string;
  name: string;
  url: string;
  viewport: string;
  capturedAt: string;
}

interface DiffResult {
  id: string;
  baseline: string;
  viewport: string;
  complianceScore: number;
  pixelDiff: number;
  status: "pass" | "fail" | "warning";
}


const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  background: color,
  color: "var(--btn-primary-fg, #fff)",
  marginRight: 4,
});

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: 8,
  borderRadius: "var(--radius-sm)",
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontSize: "var(--font-size-md)",
};

const viewports = ["1920x1080", "1440x900", "1024x768", "768x1024", "375x812"];
const scoreColor = (s: number) => s >= 95 ? "var(--success-color)" : s >= 80 ? "var(--warning-color)" : "var(--error-color)";
const statusColor: Record<string, string> = { pass: "var(--success-color)", fail: "var(--error-color)", warning: "var(--warning-color)" };

export function VisualVerifyPanel() {
  const [tab, setTab] = useState("verify");
  const [url, setUrl] = useState("");
  const [viewport, setViewport] = useState("1920x1080");
  const [baselines, setBaselines] = useState<Baseline[]>([]);
  const [diffs, setDiffs] = useState<DiffResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [reportFormat, setReportFormat] = useState("json");

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      try {
        const [baselinesData, diffsData] = await Promise.all([
          invoke<Baseline[]>("get_visual_baselines"),
          invoke<DiffResult[]>("get_visual_diffs"),
        ]);
        setBaselines(baselinesData);
        setDiffs(diffsData);
      } catch (err) {
        console.error("Failed to load visual verify data:", err);
      } finally {
        setLoading(false);
      }
    };
    loadData();
  }, []);

  const handleCapture = useCallback(async () => {
    if (!url.trim()) return;
    const newBaseline: Baseline = {
      id: `b${Date.now()}`,
      name: new URL(url).pathname || "/",
      url,
      viewport,
      capturedAt: new Date().toISOString().slice(0, 16).replace("T", " "),
    };
    setBaselines((prev) => [...prev, newBaseline]);
    try {
      await invoke("save_visual_baseline", { baseline: newBaseline });
    } catch (err) {
      console.error("Failed to save baseline:", err);
    }
  }, [url, viewport]);

  const handleDeleteBaseline = useCallback(async (id: string) => {
    setBaselines((prev) => prev.filter((b) => b.id !== id));
    try {
      await invoke("delete_visual_baseline", { id });
    } catch (err) {
      console.error("Failed to delete baseline:", err);
    }
  }, []);

  return (
    <div className="panel-container">
      <div className="panel-tab-bar">
        <button className={`panel-tab ${tab === "verify" ? "active" : ""}`} onClick={() => setTab("verify")}>Verify</button>
        <button className={`panel-tab ${tab === "baselines" ? "active" : ""}`} onClick={() => setTab("baselines")}>Baselines</button>
        <button className={`panel-tab ${tab === "diffs" ? "active" : ""}`} onClick={() => setTab("diffs")}>Diffs</button>
        <button className={`panel-tab ${tab === "ci" ? "active" : ""}`} onClick={() => setTab("ci")}>CI</button>
      </div>

      <div className="panel-body">
        {tab === "verify" && (
          <div>
            <div className="panel-card">
              <div style={{ fontWeight: 600, marginBottom: 8 }}>Capture Page</div>
              <input style={{ ...inputStyle, marginBottom: 8 }} placeholder="https://example.com" value={url} onChange={(e) => setUrl(e.target.value)} />
              <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
                <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Viewport:</span>
                <select value={viewport} onChange={(e) => setViewport(e.target.value)} style={{ ...inputStyle, width: "auto" }}>
                  {viewports.map((v) => <option key={v} value={v}>{v}</option>)}
                </select>
              </div>
              <button className="panel-btn panel-btn-primary" onClick={handleCapture}>Capture</button>
            </div>
          </div>
        )}

        {tab === "baselines" && (
          <div>
            {loading ? (
              <div className="panel-loading">Loading baselines...</div>
            ) : baselines.length === 0 ? (
              <div className="panel-empty">No baselines captured yet. Use the Verify tab to capture a page.</div>
            ) : (
              baselines.map((b) => (
                <div key={b.id} className="panel-card">
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <strong>{b.name}</strong>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <span style={badgeStyle("#6366f1")}>{b.viewport}</span>
                      <button
                        onClick={() => handleDeleteBaseline(b.id)}
                        style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: "var(--font-size-lg)" }}
                        title="Delete baseline"
                      >
                        x
                      </button>
                    </div>
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--accent-color)", marginTop: 2 }}>{b.url}</div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 2 }}>Captured: {b.capturedAt}</div>
                  <div style={{ marginTop: 6, background: "var(--bg-primary)", borderRadius: "var(--radius-xs-plus)", height: 60, display: "flex", alignItems: "center", justifyContent: "center", fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
                    [{b.viewport} thumbnail]
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {tab === "diffs" && (
          <div>
            {loading ? (
              <div className="panel-loading">Loading diffs...</div>
            ) : diffs.length === 0 ? (
              <div className="panel-empty">No visual diffs found. Capture baselines and run comparisons to see diffs.</div>
            ) : (
              diffs.map((d) => (
                <div key={d.id} className="panel-card">
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                    <strong>{d.baseline}</strong>
                    <div>
                      <span style={badgeStyle(scoreColor(d.complianceScore))}>{d.complianceScore}%</span>
                      <span style={badgeStyle(statusColor[d.status])}>{d.status}</span>
                    </div>
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Viewport: {d.viewport} | Pixel diff: {d.pixelDiff}%</div>
                </div>
              ))
            )}
          </div>
        )}

        {tab === "ci" && (
          <div>
            <div className="panel-card">
              <div style={{ fontWeight: 600, marginBottom: 8 }}>Generate Report</div>
              <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
                {["json", "markdown", "html"].map((f) => (
                  <button key={f} className={`panel-btn ${reportFormat === f ? "panel-btn-primary" : "panel-btn-secondary"}`} onClick={() => setReportFormat(f)}>
                    {f.toUpperCase()}
                  </button>
                ))}
              </div>
              <button className="panel-btn panel-btn-primary">Generate {reportFormat.toUpperCase()} Report</button>
            </div>
            <div className="panel-card">
              <div style={{ fontWeight: 600, marginBottom: 8 }}>Summary</div>
              <div style={{ fontSize: "var(--font-size-md)" }}>
                <div style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}><span>Total baselines</span><strong>{baselines.length}</strong></div>
                <div style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}><span>Passing</span><strong style={{ color: "var(--success-color)" }}>{diffs.filter((d) => d.status === "pass").length}</strong></div>
                <div style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}><span>Warnings</span><strong style={{ color: "var(--warning-color)" }}>{diffs.filter((d) => d.status === "warning").length}</strong></div>
                <div style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}><span>Failing</span><strong style={{ color: "var(--error-color)" }}>{diffs.filter((d) => d.status === "fail").length}</strong></div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
