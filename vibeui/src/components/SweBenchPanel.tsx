/**
 * SweBenchPanel — SWE-bench Benchmarking panel.
 *
 * Configure and run SWE-bench evaluations, view pass@1 rates and task
 * breakdowns, and compare multiple benchmark runs side by side.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState, useMemo } from "react";

// ── Types ─────────────────────────────────────────────────────────────────────

interface BenchmarkRun {
  id: string;
  suite: string;
  provider: string;
  model: string;
  status: "pending" | "running" | "completed" | "failed";
  progress: number;
  startedAt: string;
  completedAt: string | null;
  passRate: number;
  totalTasks: number;
  passed: number;
  failed: number;
  errored: number;
  avgDurationSec: number;
}

interface TaskResult {
  id: string;
  task: string;
  repo: string;
  status: "pass" | "fail" | "error";
  durationSec: number;
  tokensUsed: number;
}

// ── Mock Data ─────────────────────────────────────────────────────────────────

const SUITES = ["SWE-bench Lite", "SWE-bench Full", "SWE-bench Verified", "HumanEval", "MBPP"];
const PROVIDERS = ["Anthropic", "OpenAI", "Google", "Ollama"];
const MODELS: Record<string, string[]> = {
  Anthropic: ["claude-opus-4-20250514", "claude-sonnet-4-20250514"],
  OpenAI: ["gpt-4o", "gpt-4o-mini", "o1-preview"],
  Google: ["gemini-2.0-pro", "gemini-2.0-flash"],
  Ollama: ["llama3:70b", "codellama:34b", "deepseek-coder:33b"],
};

const MOCK_RUNS: BenchmarkRun[] = [
  { id: "r1", suite: "SWE-bench Lite", provider: "Anthropic", model: "claude-opus-4-20250514", status: "completed", progress: 100, startedAt: "2026-03-12T10:00:00Z", completedAt: "2026-03-12T12:45:00Z", passRate: 49.2, totalTasks: 300, passed: 148, failed: 140, errored: 12, avgDurationSec: 32.8 },
  { id: "r2", suite: "SWE-bench Lite", provider: "OpenAI", model: "gpt-4o", status: "completed", progress: 100, startedAt: "2026-03-12T13:00:00Z", completedAt: "2026-03-12T15:30:00Z", passRate: 43.7, totalTasks: 300, passed: 131, failed: 155, errored: 14, avgDurationSec: 29.4 },
  { id: "r3", suite: "SWE-bench Lite", provider: "Anthropic", model: "claude-sonnet-4-20250514", status: "completed", progress: 100, startedAt: "2026-03-13T06:00:00Z", completedAt: "2026-03-13T07:15:00Z", passRate: 38.3, totalTasks: 300, passed: 115, failed: 172, errored: 13, avgDurationSec: 15.2 },
  { id: "r4", suite: "SWE-bench Verified", provider: "Anthropic", model: "claude-opus-4-20250514", status: "running", progress: 67, startedAt: "2026-03-13T08:00:00Z", completedAt: null, passRate: 52.1, totalTasks: 500, passed: 174, failed: 148, errored: 12, avgDurationSec: 35.1 },
];

const MOCK_TASK_RESULTS: TaskResult[] = [
  { id: "t1", task: "django__django-16379", repo: "django/django", status: "pass", durationSec: 28, tokensUsed: 12400 },
  { id: "t2", task: "sympy__sympy-24152", repo: "sympy/sympy", status: "fail", durationSec: 45, tokensUsed: 18200 },
  { id: "t3", task: "scikit-learn__scikit-learn-25570", repo: "scikit-learn/scikit-learn", status: "pass", durationSec: 22, tokensUsed: 9800 },
  { id: "t4", task: "matplotlib__matplotlib-25311", repo: "matplotlib/matplotlib", status: "error", durationSec: 60, tokensUsed: 24500 },
  { id: "t5", task: "flask__flask-4992", repo: "pallets/flask", status: "pass", durationSec: 15, tokensUsed: 6200 },
  { id: "t6", task: "requests__requests-6028", repo: "psf/requests", status: "pass", durationSec: 18, tokensUsed: 7800 },
  { id: "t7", task: "pytest__pytest-11143", repo: "pytest-dev/pytest", status: "fail", durationSec: 38, tokensUsed: 15600 },
  { id: "t8", task: "astropy__astropy-14182", repo: "astropy/astropy", status: "pass", durationSec: 32, tokensUsed: 13100 },
];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "#fff" : "var(--text-primary)", marginRight: 4 });

const selectStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-mono, monospace)", boxSizing: "border-box", cursor: "pointer" };

const barBg: React.CSSProperties = { height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 4, background: color });

const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 10px", borderBottom: "1px solid var(--border-primary)", fontSize: 11, color: "var(--text-secondary)" };
const tdStyle: React.CSSProperties = { padding: "6px 10px", borderBottom: "1px solid var(--border-primary)", fontSize: 12 };

const statusColor: Record<string, string> = { pass: "#22c55e", fail: "#ef4444", error: "#f59e0b", completed: "#22c55e", running: "#3b82f6", pending: "#6b7280", failed: "#ef4444" };
const badgeStyle = (status: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "#fff", background: statusColor[status] || "#6b7280" });

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "run" | "results" | "compare";

export function SweBenchPanel() {
  const [tab, setTab] = useState<Tab>("run");
  const [runs] = useState<BenchmarkRun[]>(MOCK_RUNS);

  // Run config
  const [selectedSuite, setSelectedSuite] = useState(SUITES[0]);
  const [selectedProvider, setSelectedProvider] = useState(PROVIDERS[0]);
  const [selectedModel, setSelectedModel] = useState(MODELS[PROVIDERS[0]][0]);
  const [isStarting, setIsStarting] = useState(false);

  // Results
  const [selectedRunId, setSelectedRunId] = useState(MOCK_RUNS[0].id);

  // Compare
  const [compareIds, setCompareIds] = useState<string[]>(["r1", "r2"]);

  const completedRuns = useMemo(() => runs.filter((r) => r.status === "completed"), [runs]);
  const selectedRun = runs.find((r) => r.id === selectedRunId);
  const compareRuns = completedRuns.filter((r) => compareIds.includes(r.id));

  const handleProviderChange = (p: string) => {
    setSelectedProvider(p);
    setSelectedModel(MODELS[p][0]);
  };

  const startBenchmark = () => {
    setIsStarting(true);
    setTimeout(() => setIsStarting(false), 1500);
  };

  const toggleCompare = (id: string) => {
    setCompareIds((prev) => prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]);
  };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>SWE-bench Benchmarking</h2>

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "run")} onClick={() => setTab("run")}>Run</button>
        <button style={tabBtnStyle(tab === "results")} onClick={() => setTab("results")}>Results</button>
        <button style={tabBtnStyle(tab === "compare")} onClick={() => setTab("compare")}>Compare</button>
      </div>

      {tab === "run" && (
        <div>
          <div style={cardStyle}>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 }}>
              <div>
                <div style={labelStyle}>Suite</div>
                <select style={selectStyle} value={selectedSuite} onChange={(e) => setSelectedSuite(e.target.value)}>
                  {SUITES.map((s) => <option key={s} value={s}>{s}</option>)}
                </select>
              </div>
              <div>
                <div style={labelStyle}>Provider</div>
                <select style={selectStyle} value={selectedProvider} onChange={(e) => handleProviderChange(e.target.value)}>
                  {PROVIDERS.map((p) => <option key={p} value={p}>{p}</option>)}
                </select>
              </div>
              <div>
                <div style={labelStyle}>Model</div>
                <select style={selectStyle} value={selectedModel} onChange={(e) => setSelectedModel(e.target.value)}>
                  {MODELS[selectedProvider].map((m) => <option key={m} value={m}>{m}</option>)}
                </select>
              </div>
            </div>
            <button
              style={{ ...btnStyle, background: "var(--accent-primary)", color: "#fff", marginTop: 12 }}
              onClick={startBenchmark}
              disabled={isStarting}
            >
              {isStarting ? "Starting..." : "Start Benchmark"}
            </button>
          </div>

          <div style={labelStyle}>Recent Runs</div>
          {runs.map((r) => (
            <div key={r.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <div>
                  <span style={{ fontWeight: 600 }}>{r.model}</span>
                  <span style={{ fontSize: 10, color: "var(--text-secondary)", marginLeft: 6 }}>{r.suite}</span>
                </div>
                <span style={badgeStyle(r.status)}>{r.status}</span>
              </div>
              <div style={barBg}>
                <div style={barFill(r.progress, statusColor[r.status])} />
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>
                <span>{r.progress}% complete</span>
                {r.status === "completed" && <span>Pass@1: {r.passRate}%</span>}
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "results" && (
        <div>
          <div style={{ marginBottom: 10 }}>
            <div style={labelStyle}>Select Run</div>
            <select style={selectStyle} value={selectedRunId} onChange={(e) => setSelectedRunId(e.target.value)}>
              {runs.map((r) => <option key={r.id} value={r.id}>{r.model} — {r.suite} ({r.status})</option>)}
            </select>
          </div>

          {selectedRun && (
            <>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
                <div style={cardStyle}>
                  <div style={labelStyle}>Pass@1</div>
                  <div style={{ fontSize: 22, fontWeight: 700, color: "#22c55e" }}>{selectedRun.passRate}%</div>
                </div>
                <div style={cardStyle}>
                  <div style={labelStyle}>Passed</div>
                  <div style={{ fontSize: 22, fontWeight: 700 }}>{selectedRun.passed}/{selectedRun.totalTasks}</div>
                </div>
                <div style={cardStyle}>
                  <div style={labelStyle}>Failed</div>
                  <div style={{ fontSize: 22, fontWeight: 700, color: "#ef4444" }}>{selectedRun.failed}</div>
                </div>
                <div style={cardStyle}>
                  <div style={labelStyle}>Avg Duration</div>
                  <div style={{ fontSize: 22, fontWeight: 700 }}>{selectedRun.avgDurationSec}s</div>
                </div>
              </div>

              <div style={cardStyle}>
                <div style={labelStyle}>Task Breakdown</div>
                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      <th style={thStyle}>Task</th>
                      <th style={thStyle}>Repository</th>
                      <th style={thStyle}>Status</th>
                      <th style={{ ...thStyle, textAlign: "right" }}>Duration</th>
                      <th style={{ ...thStyle, textAlign: "right" }}>Tokens</th>
                    </tr>
                  </thead>
                  <tbody>
                    {MOCK_TASK_RESULTS.map((t) => (
                      <tr key={t.id}>
                        <td style={{ ...tdStyle, fontSize: 11 }}>{t.task}</td>
                        <td style={{ ...tdStyle, fontSize: 11 }}>{t.repo}</td>
                        <td style={tdStyle}><span style={badgeStyle(t.status)}>{t.status}</span></td>
                        <td style={{ ...tdStyle, textAlign: "right" }}>{t.durationSec}s</td>
                        <td style={{ ...tdStyle, textAlign: "right" }}>{t.tokensUsed.toLocaleString()}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </div>
      )}

      {tab === "compare" && (
        <div>
          <div style={cardStyle}>
            <div style={labelStyle}>Select runs to compare</div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 4 }}>
              {completedRuns.map((r) => (
                <button
                  key={r.id}
                  style={tabBtnStyle(compareIds.includes(r.id))}
                  onClick={() => toggleCompare(r.id)}
                >
                  {r.model}
                </button>
              ))}
            </div>
          </div>

          {compareRuns.length >= 2 && (
            <div style={cardStyle}>
              <table style={{ width: "100%", borderCollapse: "collapse" }}>
                <thead>
                  <tr>
                    <th style={thStyle}>Metric</th>
                    {compareRuns.map((r) => <th key={r.id} style={thStyle}>{r.model}</th>)}
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td style={tdStyle}>Suite</td>
                    {compareRuns.map((r) => <td key={r.id} style={tdStyle}>{r.suite}</td>)}
                  </tr>
                  <tr>
                    <td style={{ ...tdStyle, fontWeight: 600 }}>Pass@1</td>
                    {compareRuns.map((r) => (
                      <td key={r.id} style={{ ...tdStyle, fontWeight: 600, color: "#22c55e" }}>{r.passRate}%</td>
                    ))}
                  </tr>
                  <tr>
                    <td style={tdStyle}>Passed / Total</td>
                    {compareRuns.map((r) => <td key={r.id} style={tdStyle}>{r.passed} / {r.totalTasks}</td>)}
                  </tr>
                  <tr>
                    <td style={tdStyle}>Failed</td>
                    {compareRuns.map((r) => <td key={r.id} style={{ ...tdStyle, color: "#ef4444" }}>{r.failed}</td>)}
                  </tr>
                  <tr>
                    <td style={tdStyle}>Errors</td>
                    {compareRuns.map((r) => <td key={r.id} style={{ ...tdStyle, color: "#f59e0b" }}>{r.errored}</td>)}
                  </tr>
                  <tr>
                    <td style={tdStyle}>Avg Duration</td>
                    {compareRuns.map((r) => <td key={r.id} style={tdStyle}>{r.avgDurationSec}s</td>)}
                  </tr>
                </tbody>
              </table>
            </div>
          )}

          {compareRuns.length < 2 && (
            <div style={cardStyle}>Select at least 2 completed runs to compare.</div>
          )}
        </div>
      )}
    </div>
  );
}
