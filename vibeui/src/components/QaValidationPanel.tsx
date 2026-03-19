/**
 * QaValidationPanel — QA assertion validation and reporting.
 *
 * Tabs: Validate (run test assertions), Reports (validation history)
 */
import { useState } from "react";

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

  const handleRunValidation = () => {
    const lines = inputText.split("\n").map((l) => l.trim()).filter((l) => l.length > 0);
    if (lines.length === 0) return;
    setIsRunning(true);
    setTimeout(() => {
      const cases: ValidationCase[] = lines.map((line) => {
        const passed = Math.random() > 0.25;
        return {
          id: crypto.randomUUID().slice(0, 8),
          assertion: line,
          passed,
          message: passed ? "Assertion passed" : "Expected condition not met",
        };
      });
      setCurrentResults(cases);

      const run: ValidationRun = {
        id: crypto.randomUUID().slice(0, 8),
        timestamp: new Date().toISOString(),
        total: cases.length,
        passed: cases.filter((c) => c.passed).length,
        failed: cases.filter((c) => !c.passed).length,
        cases,
      };
      setHistory((prev) => [run, ...prev]);
      setIsRunning(false);
    }, 800);
  };

  const selectedRunData = history.find((r) => r.id === selectedRun);

  const tabs: { key: Tab; label: string }[] = [
    { key: "validate", label: "Validate" },
    { key: "reports", label: "Reports" },
  ];

  const inputStyle: React.CSSProperties = {
    width: "100%",
    background: "var(--bg-secondary)",
    border: "1px solid var(--border)",
    borderRadius: 4,
    color: "var(--text-primary)",
    padding: "6px 8px",
    fontSize: 12,
    boxSizing: "border-box",
  };

  const btnPrimary: React.CSSProperties = {
    background: "var(--accent)",
    color: "var(--btn-primary-fg)",
    border: "none",
    borderRadius: 4,
    padding: "8px 16px",
    cursor: "pointer",
    fontSize: 12,
    fontWeight: 600,
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border)", background: "var(--bg-secondary)" }}>
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            style={{
              padding: "8px 16px",
              background: tab === t.key ? "var(--bg-primary)" : "transparent",
              border: "none",
              borderBottom: tab === t.key ? "2px solid var(--accent)" : "2px solid transparent",
              color: tab === t.key ? "var(--text-primary)" : "var(--text-secondary)",
              cursor: "pointer",
              fontSize: 12,
              fontWeight: tab === t.key ? 600 : 400,
            }}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {tab === "validate" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div>
              <label style={{ fontSize: 11, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>Assertions / Test Cases (one per line)</label>
              <textarea
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                placeholder={"response contains 'success'\nstatus_code == 200\nlatency_ms < 500\nresult.length > 0\nno errors in output"}
                rows={8}
                style={{ ...inputStyle, fontFamily: "var(--font-mono)", resize: "vertical" }}
              />
            </div>

            <button
              onClick={handleRunValidation}
              disabled={isRunning || !inputText.trim()}
              style={{ ...btnPrimary, alignSelf: "flex-start", opacity: isRunning || !inputText.trim() ? 0.5 : 1 }}
            >
              {isRunning ? "Running..." : "Run Validation"}
            </button>

            {/* Results */}
            {currentResults.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <div style={{ display: "flex", gap: 16, fontSize: 12, marginBottom: 12 }}>
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
                      borderRadius: 4,
                      borderLeft: `3px solid ${c.passed ? "var(--success-color)" : "var(--error-color)"}`,
                    }}
                  >
                    <span style={{ fontSize: 14, lineHeight: "20px", flexShrink: 0 }}>{c.passed ? "[PASS]" : "[FAIL]"}</span>
                    <div style={{ flex: 1 }}>
                      <div style={{ fontSize: 12, fontFamily: "var(--font-mono)" }}>{c.assertion}</div>
                      <div style={{ fontSize: 11, color: c.passed ? "var(--success-color)" : "var(--error-color)", marginTop: 2 }}>{c.message}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
            {currentResults.length === 0 && !isRunning && (
              <div style={{ textAlign: "center", opacity: 0.4, fontSize: 12, marginTop: 32 }}>Enter assertions and click Run Validation to test them.</div>
            )}
          </div>
        )}

        {tab === "reports" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {history.length === 0 && (
              <div style={{ textAlign: "center", opacity: 0.4, fontSize: 12, marginTop: 32 }}>No validation runs yet. Go to the Validate tab to run assertions.</div>
            )}

            {history.length > 0 && !selectedRun && (
              <>
                <div style={{ fontSize: 13, fontWeight: 600 }}>Validation History</div>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
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
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", fontFamily: "var(--font-mono)", fontSize: 11 }}>{new Date(run.timestamp).toLocaleString()}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>
                          <span style={{
                            display: "inline-block",
                            padding: "2px 8px",
                            borderRadius: 10,
                            fontSize: 10,
                            fontWeight: 600,
                            background: run.failed === 0 ? "rgba(76,175,80,0.15)" : "rgba(244,67,54,0.15)",
                            color: run.failed === 0 ? "var(--success-color)" : "var(--error-color)",
                          }}>
                            {run.failed === 0 ? "PASS" : "FAIL"}
                          </span>
                        </td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center", color: "var(--success-color)" }}>{run.passed}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center", color: run.failed > 0 ? "var(--error-color)" : "var(--text-secondary)" }}>{run.failed}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>{run.total}</td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border)", textAlign: "center" }}>
                          <button onClick={() => setSelectedRun(run.id)} style={{ background: "none", border: "none", color: "var(--accent)", cursor: "pointer", fontSize: 11, textDecoration: "underline" }}>Details</button>
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
                  style={{ background: "none", border: "none", color: "var(--accent)", cursor: "pointer", fontSize: 12, marginBottom: 12, padding: 0 }}
                >
                  &larr; Back to History
                </button>
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 4 }}>Run {selectedRunData.id}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 12 }}>
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
                      borderRadius: 4,
                      borderLeft: `3px solid ${c.passed ? "var(--success-color)" : "var(--error-color)"}`,
                    }}
                  >
                    <span style={{ fontSize: 14, lineHeight: "20px", flexShrink: 0 }}>{c.passed ? "[PASS]" : "[FAIL]"}</span>
                    <div style={{ flex: 1 }}>
                      <div style={{ fontSize: 12, fontFamily: "var(--font-mono)" }}>{c.assertion}</div>
                      <div style={{ fontSize: 11, color: c.passed ? "var(--success-color)" : "var(--error-color)", marginTop: 2 }}>{c.message}</div>
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
