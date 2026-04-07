/**
 * SweBenchPanel — SWE-bench Benchmarking panel.
 *
 * Configure and run SWE-bench evaluations, view pass@1 rates and task
 * breakdowns, and compare multiple benchmark runs side by side.
 * Wired to Tauri backend commands for real data persistence.
 */
import { useState, useMemo, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

interface SuitesResponse {
  suites: { name: string; taskCount: number }[];
  providers: string[];
  models: Record<string, string[]>;
}

interface RunsResponse {
  runs: BenchmarkRun[];
  total: number;
}

interface ResultsResponse {
  run: BenchmarkRun | null;
  taskResults: TaskResult[];
}

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--btn-primary-fg)" : "var(--text-primary)", marginRight: 4 });

const selectStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-family)", boxSizing: "border-box", cursor: "pointer" };

const barBg: React.CSSProperties = { height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 4, background: color });

const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 10px", borderBottom: "1px solid var(--border-color)", fontSize: 11, color: "var(--text-secondary)" };
const tdStyle: React.CSSProperties = { padding: "6px 10px", borderBottom: "1px solid var(--border-color)", fontSize: 12 };

const statusColor: Record<string, string> = { pass: "var(--success-color)", fail: "var(--error-color)", error: "var(--warning-color)", completed: "var(--success-color)", running: "var(--info-color)", pending: "var(--text-secondary)", failed: "var(--error-color)" };
const badgeStyle = (status: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "var(--text-primary)", background: statusColor[status] || "var(--text-secondary)" });

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "run" | "results" | "compare";

export function SweBenchPanel() {
  const [tab, setTab] = useState<Tab>("run");
  const [runs, setRuns] = useState<BenchmarkRun[]>([]);
  const [suites, setSuites] = useState<string[]>([]);
  const [providers, setProviders] = useState<string[]>([]);
  const [models, setModels] = useState<Record<string, string[]>>({});
  const [loading, setLoading] = useState(true);

  // Run config
  const [selectedSuite, setSelectedSuite] = useState("");
  const [selectedProvider, setSelectedProvider] = useState("");
  const [selectedModel, setSelectedModel] = useState("");
  const [isStarting, setIsStarting] = useState(false);

  // Results
  const [selectedRunId, setSelectedRunId] = useState("");
  const [taskResults, setTaskResults] = useState<TaskResult[]>([]);

  // Compare
  const [compareIds, setCompareIds] = useState<string[]>([]);

  const loadSuites = useCallback(async () => {
    try {
      const res = await invoke<SuitesResponse>("swe_bench_get_suites");
      const suiteNames = res.suites.map((s) => s.name);
      setSuites(suiteNames);
      setProviders(res.providers);
      setModels(res.models);
      if (suiteNames.length > 0 && !selectedSuite) setSelectedSuite(suiteNames[0]);
      if (res.providers.length > 0 && !selectedProvider) {
        setSelectedProvider(res.providers[0]);
        const firstModels = res.models[res.providers[0]];
        if (firstModels && firstModels.length > 0 && !selectedModel) setSelectedModel(firstModels[0]);
      }
    } catch {
      // Backend unavailable — leave empty
    }
  }, [selectedSuite, selectedProvider, selectedModel]);

  const loadRuns = useCallback(async () => {
    try {
      const res = await invoke<RunsResponse>("swe_bench_list_runs");
      setRuns(res.runs);
      if (res.runs.length > 0 && !selectedRunId) {
        setSelectedRunId(res.runs[0].id);
      }
    } catch {
      // Backend unavailable — leave empty
    }
  }, [selectedRunId]);

  const loadResults = useCallback(async (runId: string) => {
    if (!runId) return;
    try {
      const res = await invoke<ResultsResponse>("swe_bench_get_results", { runId });
      setTaskResults(res.taskResults);
    } catch {
      setTaskResults([]);
    }
  }, []);

  useEffect(() => {
    const init = async () => {
      setLoading(true);
      await Promise.all([loadSuites(), loadRuns()]);
      setLoading(false);
    };
    init();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (selectedRunId && tab === "results") {
      loadResults(selectedRunId);
    }
  }, [selectedRunId, tab, loadResults]);

  const completedRuns = useMemo(() => runs.filter((r) => r.status === "completed"), [runs]);
  const selectedRun = runs.find((r) => r.id === selectedRunId);
  const compareRuns = completedRuns.filter((r) => compareIds.includes(r.id));

  const handleProviderChange = (p: string) => {
    setSelectedProvider(p);
    const providerModels = models[p];
    if (providerModels && providerModels.length > 0) {
      setSelectedModel(providerModels[0]);
    }
  };

  const startBenchmark = async () => {
    setIsStarting(true);
    try {
      await invoke<BenchmarkRun>("swe_bench_start_run", {
        suite: selectedSuite,
        provider: selectedProvider,
        model: selectedModel,
      });
      await loadRuns();
    } catch {
      // Failed to start
    } finally {
      setIsStarting(false);
    }
  };

  const toggleCompare = (id: string) => {
    setCompareIds((prev) => prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]);
  };

  if (loading) {
    return (
      <div style={panelStyle}>
        <h2 style={headingStyle}>SWE-bench Benchmarking</h2>
        <div style={cardStyle}>Loading...</div>
      </div>
    );
  }

  const availableModels = models[selectedProvider] || [];

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
                  {suites.map((s) => <option key={s} value={s}>{s}</option>)}
                </select>
              </div>
              <div>
                <div style={labelStyle}>Provider</div>
                <select style={selectStyle} value={selectedProvider} onChange={(e) => handleProviderChange(e.target.value)}>
                  {providers.map((p) => <option key={p} value={p}>{p}</option>)}
                </select>
              </div>
              <div>
                <div style={labelStyle}>Model</div>
                <select style={selectStyle} value={selectedModel} onChange={(e) => setSelectedModel(e.target.value)}>
                  {availableModels.map((m) => <option key={m} value={m}>{m}</option>)}
                </select>
              </div>
            </div>
            <button
              style={{ ...btnStyle, background: "var(--accent-primary)", color: "var(--btn-primary-fg)", marginTop: 12 }}
              onClick={startBenchmark}
              disabled={isStarting}
            >
              {isStarting ? "Starting..." : "Start Benchmark"}
            </button>
          </div>

          <div style={labelStyle}>Recent Runs</div>
          {runs.length === 0 && <div style={cardStyle}>No benchmark runs yet. Start one above.</div>}
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
              {runs.length === 0 && <option value="">No runs available</option>}
              {runs.map((r) => <option key={r.id} value={r.id}>{r.model} — {r.suite} ({r.status})</option>)}
            </select>
          </div>

          {selectedRun && (
            <>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
                <div style={cardStyle}>
                  <div style={labelStyle}>Pass@1</div>
                  <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--success-color)" }}>{selectedRun.passRate}%</div>
                </div>
                <div style={cardStyle}>
                  <div style={labelStyle}>Passed</div>
                  <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{selectedRun.passed}/{selectedRun.totalTasks}</div>
                </div>
                <div style={cardStyle}>
                  <div style={labelStyle}>Failed</div>
                  <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--error-color)" }}>{selectedRun.failed}</div>
                </div>
                <div style={cardStyle}>
                  <div style={labelStyle}>Avg Duration</div>
                  <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{selectedRun.avgDurationSec}s</div>
                </div>
              </div>

              <div style={cardStyle}>
                <div style={labelStyle}>Task Breakdown</div>
                {taskResults.length === 0 && <div style={{ fontSize: 12, color: "var(--text-secondary)", padding: "8px 0" }}>No task results available for this run.</div>}
                {taskResults.length > 0 && (
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
                      {taskResults.map((t) => (
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
                )}
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
              {completedRuns.length === 0 && <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>No completed runs to compare.</div>}
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
                      <td key={r.id} style={{ ...tdStyle, fontWeight: 600, color: "var(--success-color)" }}>{r.passRate}%</td>
                    ))}
                  </tr>
                  <tr>
                    <td style={tdStyle}>Passed / Total</td>
                    {compareRuns.map((r) => <td key={r.id} style={tdStyle}>{r.passed} / {r.totalTasks}</td>)}
                  </tr>
                  <tr>
                    <td style={tdStyle}>Failed</td>
                    {compareRuns.map((r) => <td key={r.id} style={{ ...tdStyle, color: "var(--error-color)" }}>{r.failed}</td>)}
                  </tr>
                  <tr>
                    <td style={tdStyle}>Errors</td>
                    {compareRuns.map((r) => <td key={r.id} style={{ ...tdStyle, color: "var(--warning-color)" }}>{r.errored}</td>)}
                  </tr>
                  <tr>
                    <td style={tdStyle}>Avg Duration</td>
                    {compareRuns.map((r) => <td key={r.id} style={tdStyle}>{r.avgDurationSec}s</td>)}
                  </tr>
                </tbody>
              </table>
            </div>
          )}

          {compareRuns.length < 2 && completedRuns.length >= 2 && (
            <div style={cardStyle}>Select at least 2 completed runs to compare.</div>
          )}
        </div>
      )}
    </div>
  );
}
