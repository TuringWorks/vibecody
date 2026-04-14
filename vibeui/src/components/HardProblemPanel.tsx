import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Subsystem {
  name: string;
  description: string;
  complexity: "low" | "medium" | "high" | "critical";
  dependencies: string[];
}

interface DecomposeResult {
  problem_summary: string;
  subsystems: Subsystem[];
  overall_complexity: "low" | "medium" | "high" | "critical";
}

interface Assumption {
  id: string;
  text: string;
  status: "unconfirmed" | "confirmed" | "rejected";
  impact: "low" | "medium" | "high";
}

interface ClarifyingQuestion {
  id: string;
  question: string;
  impact: "low" | "medium" | "high";
  subsystem: string | null;
}

const COMPLEXITY_COLORS: Record<string, string> = {
  low: "var(--success-color)",
  medium: "var(--warning-color)",
  high: "#f06060",
  critical: "var(--error-color)",
};

const IMPACT_COLORS: Record<string, string> = {
  low: "var(--text-muted)",
  medium: "var(--warning-color)",
  high: "var(--error-color)",
};

export function HardProblemPanel() {
  const [tab, setTab] = useState("decompose");
  const [description, setDescription] = useState("");
  const [decomposeResult, setDecomposeResult] = useState<DecomposeResult | null>(null);
  const [assumptions, setAssumptions] = useState<Assumption[]>([]);
  const [questions, setQuestions] = useState<ClarifyingQuestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [analyzing, setAnalyzing] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [assumRes, qRes] = await Promise.all([
          invoke<Assumption[]>("hard_problem_confirm_assumption"),
          invoke<ClarifyingQuestion[]>("hard_problem_questions"),
        ]);
        setAssumptions(Array.isArray(assumRes) ? assumRes : []);
        setQuestions(Array.isArray(qRes) ? qRes : []);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function analyze() {
    if (!description.trim()) return;
    setAnalyzing(true);
    setError(null);
    try {
      const res = await invoke<DecomposeResult>("hard_problem_decompose", { description: description.trim() });
      setDecomposeResult(res ?? null);
    } catch (e) {
      setError(String(e));
    } finally {
      setAnalyzing(false);
    }
  }

  async function confirmAssumption(id: string, confirm: boolean) {
    try {
      const res = await invoke<Assumption[]>("hard_problem_confirm_assumption", { assumptionId: id, confirmed: confirm });
      setAssumptions(Array.isArray(res) ? res : assumptions.map(a => a.id === id ? { ...a, status: confirm ? "confirmed" : "rejected" } : a));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>Hard Problem Decomposer</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["decompose", "assumptions", "questions"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {tab === "decompose" && (
        <div>
          <div style={{ marginBottom: 14 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Problem Description</label>
            <textarea value={description} onChange={e => setDescription(e.target.value)}
              placeholder="Describe the hard problem to decompose..."
              style={{ width: "100%", height: 120, padding: "10px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", resize: "vertical", boxSizing: "border-box" }} />
          </div>
          <button onClick={analyze} disabled={analyzing || !description.trim()}
            style={{ padding: "8px 24px", borderRadius: "var(--radius-sm)", cursor: analyzing || !description.trim() ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600, opacity: analyzing || !description.trim() ? 0.6 : 1, marginBottom: 20 }}>
            {analyzing ? "Analyzing…" : "Analyze"}
          </button>
          {decomposeResult && (
            <div>
              <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 16, marginBottom: 14 }}>
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                  <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Problem Summary</span>
                  <span style={{ fontSize: "var(--font-size-sm)", padding: "2px 10px", borderRadius: "var(--radius-sm-alt)", fontWeight: 700, background: COMPLEXITY_COLORS[decomposeResult.overall_complexity] + "22", color: COMPLEXITY_COLORS[decomposeResult.overall_complexity] }}>
                    {decomposeResult.overall_complexity} complexity
                  </span>
                </div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", lineHeight: 1.5 }}>{decomposeResult.problem_summary}</div>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {decomposeResult.subsystems.map(sub => (
                  <div key={sub.name} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", borderLeft: `3px solid ${COMPLEXITY_COLORS[sub.complexity]}`, padding: "10px 14px" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                      <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{sub.name}</span>
                      <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 7px", borderRadius: "var(--radius-sm-alt)", background: COMPLEXITY_COLORS[sub.complexity] + "22", color: COMPLEXITY_COLORS[sub.complexity] }}>{sub.complexity}</span>
                    </div>
                    <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>{sub.description}</div>
                    {sub.dependencies.length > 0 && (
                      <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>
                        Deps: {sub.dependencies.join(", ")}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {tab === "assumptions" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {assumptions.length === 0 && <div style={{ color: "var(--text-muted)" }}>No assumptions surfaced yet. Run analysis first.</div>}
          {assumptions.map(assumption => (
            <div key={assumption.id} style={{
              background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: "12px 14px",
              borderLeft: `3px solid ${assumption.status === "confirmed" ? "var(--success-color)" : assumption.status === "rejected" ? "var(--error-color)" : "var(--text-muted)"}`
            }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontSize: "var(--font-size-sm)", padding: "1px 8px", borderRadius: "var(--radius-sm-alt)", background: IMPACT_COLORS[assumption.impact] + "22", color: IMPACT_COLORS[assumption.impact], fontWeight: 600 }}>
                  {assumption.impact} impact
                </span>
                <span style={{ fontSize: "var(--font-size-sm)", marginLeft: "auto", color: assumption.status === "confirmed" ? "var(--success-color)" : assumption.status === "rejected" ? "var(--error-color)" : "var(--text-muted)" }}>
                  {assumption.status}
                </span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", marginBottom: 10, lineHeight: 1.5 }}>{assumption.text}</div>
              {assumption.status === "unconfirmed" && (
                <div style={{ display: "flex", gap: 8 }}>
                  <button onClick={() => confirmAssumption(assumption.id, true)}
                    style={{ padding: "4px 14px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: "var(--success-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-base)" }}>
                    Confirm
                  </button>
                  <button onClick={() => confirmAssumption(assumption.id, false)}
                    style={{ padding: "4px 14px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: "var(--error-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-base)" }}>
                    Reject
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {tab === "questions" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {questions.length === 0 && <div style={{ color: "var(--text-muted)" }}>No clarifying questions generated. Run analysis first.</div>}
          {questions.map(q => (
            <div key={q.id} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: "12px 14px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontSize: "var(--font-size-sm)", padding: "1px 8px", borderRadius: "var(--radius-sm-alt)", background: IMPACT_COLORS[q.impact] + "22", color: IMPACT_COLORS[q.impact], fontWeight: 600 }}>
                  {q.impact} impact
                </span>
                {q.subsystem && (
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", background: "var(--bg-primary)", padding: "1px 8px", borderRadius: "var(--radius-sm)" }}>{q.subsystem}</span>
                )}
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", lineHeight: 1.5 }}>{q.question}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
