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

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "#fff",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: color,
  color: "#fff",
  marginRight: 4,
});

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: 8,
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontSize: 13,
};

const viewports = ["1920x1080", "1440x900", "1024x768", "768x1024", "375x812"];
const scoreColor = (s: number) => s >= 95 ? "#22c55e" : s >= 80 ? "#f59e0b" : "#ef4444";
const statusColor: Record<string, string> = { pass: "#22c55e", fail: "#ef4444", warning: "#f59e0b" };

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
    <div style={panelStyle}>
      <h2 style={headingStyle}>Visual Verification</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "verify")} onClick={() => setTab("verify")}>Verify</button>
        <button style={tabStyle(tab === "baselines")} onClick={() => setTab("baselines")}>Baselines</button>
        <button style={tabStyle(tab === "diffs")} onClick={() => setTab("diffs")}>Diffs</button>
        <button style={tabStyle(tab === "ci")} onClick={() => setTab("ci")}>CI</button>
      </div>

      {tab === "verify" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Capture Page</div>
            <input style={{ ...inputStyle, marginBottom: 8 }} placeholder="https://example.com" value={url} onChange={(e) => setUrl(e.target.value)} />
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
              <span style={{ fontSize: 13, fontWeight: 600 }}>Viewport:</span>
              <select value={viewport} onChange={(e) => setViewport(e.target.value)} style={{ ...inputStyle, width: "auto" }}>
                {viewports.map((v) => <option key={v} value={v}>{v}</option>)}
              </select>
            </div>
            <button style={btnStyle} onClick={handleCapture}>Capture</button>
          </div>
        </div>
      )}

      {tab === "baselines" && (
        <div>
          {loading ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>Loading baselines...</div>
          ) : baselines.length === 0 ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>No baselines captured yet. Use the Verify tab to capture a page.</div>
          ) : (
            baselines.map((b) => (
              <div key={b.id} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <strong>{b.name}</strong>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={badgeStyle("#6366f1")}>{b.viewport}</span>
                    <button
                      onClick={() => handleDeleteBaseline(b.id)}
                      style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: 14 }}
                      title="Delete baseline"
                    >
                      x
                    </button>
                  </div>
                </div>
                <div style={{ fontSize: 12, color: "var(--accent-color)", marginTop: 2 }}>{b.url}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>Captured: {b.capturedAt}</div>
                <div style={{ marginTop: 6, background: "var(--bg-primary)", borderRadius: 4, height: 60, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 12, color: "var(--text-secondary)" }}>
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
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>Loading diffs...</div>
          ) : diffs.length === 0 ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>No visual diffs found. Capture baselines and run comparisons to see diffs.</div>
          ) : (
            diffs.map((d) => (
              <div key={d.id} style={cardStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <strong>{d.baseline}</strong>
                  <div>
                    <span style={badgeStyle(scoreColor(d.complianceScore))}>{d.complianceScore}%</span>
                    <span style={badgeStyle(statusColor[d.status])}>{d.status}</span>
                  </div>
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Viewport: {d.viewport} | Pixel diff: {d.pixelDiff}%</div>
              </div>
            ))
          )}
        </div>
      )}

      {tab === "ci" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Generate Report</div>
            <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
              {["json", "markdown", "html"].map((f) => (
                <button key={f} style={{ ...btnStyle, background: reportFormat === f ? "var(--accent-color)" : "var(--bg-primary)", color: reportFormat === f ? "#fff" : "var(--text-primary)" }} onClick={() => setReportFormat(f)}>
                  {f.toUpperCase()}
                </button>
              ))}
            </div>
            <button style={btnStyle}>Generate {reportFormat.toUpperCase()} Report</button>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Summary</div>
            <div style={{ fontSize: 13 }}>
              <div style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}><span>Total baselines</span><strong>{baselines.length}</strong></div>
              <div style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}><span>Passing</span><strong style={{ color: "#22c55e" }}>{diffs.filter((d) => d.status === "pass").length}</strong></div>
              <div style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}><span>Warnings</span><strong style={{ color: "#f59e0b" }}>{diffs.filter((d) => d.status === "warning").length}</strong></div>
              <div style={{ display: "flex", justifyContent: "space-between", padding: "4px 0" }}><span>Failing</span><strong style={{ color: "#ef4444" }}>{diffs.filter((d) => d.status === "fail").length}</strong></div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
