/**
 * CiStatusPanel — CI/CD AI Status Checks.
 *
 * Tabs: Suites (check suites with state badge), Checks (individual checks
 * with annotations), Config (enabled/required checks, thresholds).
 * Pure TypeScript — no Tauri commands.
 */
import { useState } from "react";

type Tab = "suites" | "checks" | "config";
type SuiteState = "success" | "failure" | "pending" | "running";
type CheckState = "pass" | "fail" | "warn" | "skip" | "running";

interface CheckSuite {
  id: string;
  name: string;
  state: SuiteState;
  branch: string;
  commit: string;
  duration: string;
  checksCount: number;
  passCount: number;
}

interface Check {
  id: string;
  suiteId: string;
  name: string;
  state: CheckState;
  annotations: number;
  duration: string;
  message: string;
}

interface CheckConfig {
  id: string;
  name: string;
  enabled: boolean;
  required: boolean;
  threshold: number;
}

const MOCK_SUITES: CheckSuite[] = [
  { id: "s1", name: "AI Code Review", state: "success", branch: "main", commit: "a3f9c2d", duration: "1m 23s", checksCount: 5, passCount: 5 },
  { id: "s2", name: "Security Scan", state: "failure", branch: "main", commit: "a3f9c2d", duration: "2m 45s", checksCount: 4, passCount: 2 },
  { id: "s3", name: "Performance Check", state: "running", branch: "feat/new-ui", commit: "b7e1f4a", duration: "0m 42s", checksCount: 3, passCount: 1 },
  { id: "s4", name: "Style & Lint", state: "pending", branch: "feat/new-ui", commit: "b7e1f4a", duration: "--", checksCount: 6, passCount: 0 },
];

const MOCK_CHECKS: Check[] = [
  { id: "c1", suiteId: "s1", name: "Code Quality", state: "pass", annotations: 0, duration: "18s", message: "All patterns clean" },
  { id: "c2", suiteId: "s1", name: "Complexity", state: "pass", annotations: 2, duration: "12s", message: "2 advisory notes" },
  { id: "c3", suiteId: "s2", name: "Dependency Audit", state: "fail", annotations: 3, duration: "45s", message: "3 vulnerable packages" },
  { id: "c4", suiteId: "s2", name: "Secret Detection", state: "pass", annotations: 0, duration: "8s", message: "No secrets found" },
  { id: "c5", suiteId: "s2", name: "SAST Analysis", state: "fail", annotations: 5, duration: "1m 12s", message: "5 findings (2 high)" },
  { id: "c6", suiteId: "s2", name: "License Check", state: "warn", annotations: 1, duration: "6s", message: "1 copyleft dependency" },
  { id: "c7", suiteId: "s3", name: "Benchmark Regression", state: "running", annotations: 0, duration: "0m 30s", message: "Running benchmarks..." },
  { id: "c8", suiteId: "s3", name: "Bundle Size", state: "pass", annotations: 0, duration: "12s", message: "Within budget (2.1MB)" },
];

const MOCK_CONFIG: CheckConfig[] = [
  { id: "cfg1", name: "Code Quality", enabled: true, required: true, threshold: 80 },
  { id: "cfg2", name: "Complexity", enabled: true, required: false, threshold: 15 },
  { id: "cfg3", name: "Dependency Audit", enabled: true, required: true, threshold: 0 },
  { id: "cfg4", name: "Secret Detection", enabled: true, required: true, threshold: 0 },
  { id: "cfg5", name: "SAST Analysis", enabled: true, required: true, threshold: 0 },
  { id: "cfg6", name: "License Check", enabled: true, required: false, threshold: 0 },
  { id: "cfg7", name: "Benchmark Regression", enabled: false, required: false, threshold: 5 },
  { id: "cfg8", name: "Bundle Size", enabled: true, required: false, threshold: 5000 },
];

const stateColors: Record<string, string> = {
  success: "var(--text-success, #a6e3a1)", pass: "var(--text-success, #a6e3a1)",
  failure: "var(--text-danger, #f38ba8)", fail: "var(--text-danger, #f38ba8)",
  pending: "var(--text-muted)", skip: "var(--text-muted)",
  running: "var(--text-warning, #f9e2af)", warn: "var(--text-warning, #f9e2af)",
};

const tabBtn = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px", fontSize: 11, fontWeight: active ? 600 : 400,
  background: active ? "var(--accent-bg, rgba(99,102,241,0.15))" : "transparent",
  border: "1px solid " + (active ? "var(--accent-primary, #6366f1)" : "var(--border-color)"),
  borderRadius: 4, color: active ? "var(--text-info, #89b4fa)" : "var(--text-muted)", cursor: "pointer",
});

export default function CiStatusPanel() {
  const [tab, setTab] = useState<Tab>("suites");
  const [selectedSuite, setSelectedSuite] = useState<string | null>(null);
  const [config, setConfig] = useState(MOCK_CONFIG);

  const toggleConfig = (id: string, field: "enabled" | "required") => {
    setConfig(cs => cs.map(c => c.id === id ? { ...c, [field]: !c[field] } : c));
  };

  const filteredChecks = selectedSuite ? MOCK_CHECKS.filter(c => c.suiteId === selectedSuite) : MOCK_CHECKS;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div style={{ display: "flex", gap: 6, padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        {(["suites", "checks", "config"] as Tab[]).map(t => (
          <button key={t} onClick={() => setTab(t)} style={tabBtn(tab === t)}>
            {t[0].toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 8 }}>
        {tab === "suites" && MOCK_SUITES.map(s => (
          <div key={s.id} onClick={() => { setSelectedSuite(s.id); setTab("checks"); }}
            style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", cursor: "pointer" }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
              <span style={{ width: 8, height: 8, borderRadius: "50%", background: stateColors[s.state] }} />
              <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{s.name}</span>
              <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: `${stateColors[s.state]}22`, color: stateColors[s.state] }}>{s.state}</span>
              <span style={{ fontSize: 10, color: "var(--text-muted)", marginLeft: "auto" }}>{s.duration}</span>
            </div>
            <div style={{ display: "flex", gap: 12, fontSize: 10, color: "var(--text-muted)" }}>
              <span>Branch: <span style={{ fontFamily: "monospace", color: "var(--text-info, #89b4fa)" }}>{s.branch}</span></span>
              <span>Commit: <span style={{ fontFamily: "monospace" }}>{s.commit}</span></span>
              <span style={{ marginLeft: "auto" }}>{s.passCount}/{s.checksCount} passed</span>
            </div>
            <div style={{ marginTop: 6, height: 3, background: "var(--bg-primary)", borderRadius: 2, overflow: "hidden" }}>
              <div style={{ width: `${(s.passCount / s.checksCount) * 100}%`, height: "100%", background: stateColors[s.state], borderRadius: 2 }} />
            </div>
          </div>
        ))}

        {tab === "checks" && (
          <>
            {selectedSuite && (
              <button onClick={() => setSelectedSuite(null)}
                style={{ alignSelf: "flex-start", padding: "4px 10px", fontSize: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer", marginBottom: 4 }}>
                Show all checks
              </button>
            )}
            {filteredChecks.map(c => (
              <div key={c.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", display: "flex", gap: 10, alignItems: "center" }}>
                <span style={{ width: 8, height: 8, borderRadius: "50%", background: stateColors[c.state], flexShrink: 0 }} />
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{c.name}</div>
                  <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 2 }}>{c.message}</div>
                </div>
                {c.annotations > 0 && (
                  <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 10, background: "rgba(243,139,168,0.15)", color: "var(--text-danger, #f38ba8)" }}>
                    {c.annotations} annotations
                  </span>
                )}
                <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{c.duration}</span>
              </div>
            ))}
          </>
        )}

        {tab === "config" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 60px 60px 80px", gap: 4, padding: "6px 10px", fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>
              <span>Check</span><span>Enabled</span><span>Required</span><span>Threshold</span>
            </div>
            {config.map(c => (
              <div key={c.id} style={{ display: "grid", gridTemplateColumns: "1fr 60px 60px 80px", gap: 4, padding: "8px 10px", background: "var(--bg-secondary)", borderRadius: 4, border: "1px solid var(--border-color)", alignItems: "center" }}>
                <span style={{ fontSize: 11, color: c.enabled ? "var(--text-primary)" : "var(--text-muted)" }}>{c.name}</span>
                <input type="checkbox" checked={c.enabled} onChange={() => toggleConfig(c.id, "enabled")} style={{ cursor: "pointer" }} />
                <input type="checkbox" checked={c.required} onChange={() => toggleConfig(c.id, "required")} disabled={!c.enabled} style={{ cursor: "pointer" }} />
                <span style={{ fontSize: 11, fontFamily: "monospace", color: "var(--text-muted)" }}>{c.threshold}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
