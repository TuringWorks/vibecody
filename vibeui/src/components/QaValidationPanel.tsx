/**
 * QaValidationPanel — QA assertion validation and reporting.
 *
 * Tabs: Validate (run test assertions), Reports (validation history)
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "validate" | "reports";

interface ValidationCase {
  id: string;
  assertion: string;
  passed: boolean;
  message: string;
}

interface ValidationRun {
  id: string;
  timestamp: string;
  total: number;
  passed: number;
  failed: number;
  cases: ValidationCase[];
}

export function QaValidationPanel() {
  const [tab, setTab] = useState<Tab>("validate");
  const [inputText, setInputText] = useState("");
  const [currentResults, setCurrentResults] = useState<ValidationCase[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [history, setHistory] = useState<ValidationRun[]>([]);
  const [selectedRun, setSelectedRun] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function loadHistory() {
      setLoading(true);
      try {
        const runs = await invoke<ValidationRun[]>("get_qa_history");
        if (!cancelled) setHistory(runs);
      } catch (err) {
        console.error("Failed to load QA history:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    loadHistory();
    return () => { cancelled = true; };
  }, []);

  const handleRunValidation = async () => {
    const lines = inputText.split("\n").map((l) => l.trim()).filter((l) => l.length > 0);
    if (lines.length === 0) return;
    setIsRunning(true);
    try {
      const result = await invoke<{ cases: ValidationCase[]; run: ValidationRun }>("run_qa_validation", { assertions: lines });
      setCurrentResults(result.cases);
      setHistory((prev) => [result.run, ...prev]);
    } catch (err) {
      console.error("Failed to run QA validation:", err);
    } finally {
      setIsRunning(false);
    }
  };

  const selectedRunData = history.find((r) => r.id === selectedRun);

  const tabs: { key: Tab; label: string }[] = [
    { key: "validate", label: "Validate" },
    { key: "reports", label: "Reports" },
  ];

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-header" style={{ borderBottom: "1px solid var(--border-color)" }}>
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`panel-btn ${tab === t.key ? "panel-btn-primary" : "panel-btn-secondary"}`}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div className="panel-body">
        {loading && (
          <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", textAlign: "center", marginTop: 32 }}>Loading...</div>
        )}

        {!loading && tab === "validate" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div>
              <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>Assertions / Test Cases (one per line)</label>
              <textarea
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                placeholder={"response contains 'success'\nstatus_code == 200\nlatency_ms < 500\nresult.length > 0\nno errors in output"}
                rows={8}
                className="panel-input panel-textarea panel-input-full" style={{ fontFamily: "var(--font-mono)", resize: "vertical" }}
              />
            </div>

            <button
              onClick={handleRunValidation}
              disabled={isRunning || !inputText.trim()}
              className="panel-btn panel-btn-primary" style={{ alignSelf: "flex-start", opacity: isRunning || !inputText.trim() ? 0.5 : 1 }}
            >
              {isRunning ? "Running..." : "Run Validation"}
            </button>

            {/* Results */}
            {currentResults.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <div style={{ display: "flex", gap: 16, fontSize: "var(--font-size-base)", marginBottom: 12 }}>
                  <span style={{ color: "var(--success-color)", fontWeight: 600 }}>{currentResults.filter((c) => c.passed).length} passed</span>
                  <span style={{ color: "var(--error-color)", fontWeight: 600 }}>{currentResults.filter((c) => !c.passed).length} failed</span>
                  <span style={{ color: "var(--text-secondary)" }}>{currentResults.length} total</span>
                </div>
                {currentResults.map((c) => (
                  <div
                    key={c.id}
                    style={{
                      display: "flex",
                      alignItems: "flex-start",
                      gap: 10,
                      padding: "8px 12px",
                      marginBottom: 4,
                      background: "var(--bg-secondary)",
                      border: "1px solid var(--border)",
                      borderRadius: "var(--radius-xs-plus)",
                      borderLeft: `3px solid ${c.passed ? "var(--success-color)" : "var(--error-color)"}`,
                    }}
                  >
                    <span style={{ fontSize: "var(--font-size-lg)", lineHeight: "20px", flexShrink: 0 }}>{c.passed ? "[PASS]" : "[FAIL]"}</span>
                    <div style={{ flex: 1 }}>
                      <div style={{ fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)" }}>{c.assertion}</div>
                      <div style={{ fontSize: "var(--font-size-sm)", color: c.passed ? "var(--success-color)" : "var(--error-color)", marginTop: 2 }}>{c.message}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
            {currentResults.length === 0 && !isRunning && (
              <div style={{ textAlign: "center", opacity: 0.4, fontSize: "var(--font-size-base)", marginTop: 32 }}>Enter assertions and click Run Validation to test them.</div>
            )}
          </div>
        )}

        {!loading && tab === "reports" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {history.length === 0 && (
              <div style={{ textAlign: "center", opacity: 0.4, fontSize: "var(--font-size-base)", marginTop: 32 }}>No validation runs yet. Go to the Validate tab to run assertions.</div>
            )}

            {history.length > 0 && !selectedRun && (
              <>
                <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Validation History</div>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
                  <thead>
                    <tr style={{ background: "var(--bg-secondary)" }}>
                      <th style={{ padding: "6px 8px", textAlign: "left", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Timestamp</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Status</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Passed</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Failed</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600 }}>Total</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border)", fontWeight: 600, width: 60 }}></th>
                    </tr>
                  </thead>
                  <tbody>
                    {history.map((run, i) => (
                      <tr key={run.id} style={{ background: i % 2 === 0 ? "transparent" : "var(--bg-secondary)" }}>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{new Date(run.timestamp).toLocaleString()}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>
                          <span style={{
                            display: "inline-block",
                            padding: "2px 8px",
                            borderRadius: "var(--radius-md)",
                            fontSize: "var(--font-size-xs)",
                            fontWeight: 600,
                            background: run.failed === 0 ? "rgba(76,175,80,0.15)" : "color-mix(in srgb, var(--accent-rose) 15%, transparent)",
                            color: run.failed === 0 ? "var(--success-color)" : "var(--error-color)",
                          }}>
                            {run.failed === 0 ? "PASS" : "FAIL"}
                          </span>
                        </td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center", color: "var(--success-color)" }}>{run.passed}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center", color: run.failed > 0 ? "var(--error-color)" : "var(--text-secondary)" }}>{run.failed}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>{run.total}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>
                          <button onClick={() => setSelectedRun(run.id)} style={{ background: "none", border: "none", color: "var(--accent)", cursor: "pointer", fontSize: "var(--font-size-sm)", textDecoration: "underline" }}>Details</button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </>
            )}

            {selectedRunData && (
              <div>
                <button
                  onClick={() => setSelectedRun(null)}
                  style={{ background: "none", border: "none", color: "var(--accent)", cursor: "pointer", fontSize: "var(--font-size-base)", marginBottom: 12, padding: 0 }}
                >
                  &larr; Back to History
                </button>
                <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: 4 }}>Run {selectedRunData.id}</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 12 }}>
                  {new Date(selectedRunData.timestamp).toLocaleString()} — {selectedRunData.passed} passed, {selectedRunData.failed} failed
                </div>
                {selectedRunData.cases.map((c) => (
                  <div
                    key={c.id}
                    style={{
                      display: "flex",
                      alignItems: "flex-start",
                      gap: 10,
                      padding: "8px 12px",
                      marginBottom: 4,
                      background: "var(--bg-secondary)",
                      border: "1px solid var(--border)",
                      borderRadius: "var(--radius-xs-plus)",
                      borderLeft: `3px solid ${c.passed ? "var(--success-color)" : "var(--error-color)"}`,
                    }}
                  >
                    <span style={{ fontSize: "var(--font-size-lg)", lineHeight: "20px", flexShrink: 0 }}>{c.passed ? "[PASS]" : "[FAIL]"}</span>
                    <div style={{ flex: 1 }}>
                      <div style={{ fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)" }}>{c.assertion}</div>
                      <div style={{ fontSize: "var(--font-size-sm)", color: c.passed ? "var(--success-color)" : "var(--error-color)", marginTop: 2 }}>{c.message}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
