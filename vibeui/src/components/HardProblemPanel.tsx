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

type TagIntent = "info" | "success" | "warning" | "danger" | "neutral";

const COMPLEXITY_INTENT: Record<Subsystem["complexity"], TagIntent> = {
  low: "success",
  medium: "warning",
  high: "danger",
  critical: "danger",
};

const IMPACT_INTENT: Record<Assumption["impact"], TagIntent> = {
  low: "neutral",
  medium: "warning",
  high: "danger",
};

function complexityBorderColor(c: Subsystem["complexity"]): string {
  switch (c) {
    case "low": return "var(--success-color)";
    case "medium": return "var(--warning-color)";
    case "high":
    case "critical": return "var(--error-color)";
  }
}

function assumptionBorderColor(status: Assumption["status"]): string {
  switch (status) {
    case "confirmed": return "var(--success-color)";
    case "rejected": return "var(--error-color)";
    default: return "var(--border-color)";
  }
}

function assumptionStatusIntent(status: Assumption["status"]): TagIntent {
  switch (status) {
    case "confirmed": return "success";
    case "rejected": return "danger";
    default: return "neutral";
  }
}

export function HardProblemPanel() {
  const [tab, setTab] = useState<"decompose" | "assumptions" | "questions">("decompose");
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
    <div className="panel-container">
      <div className="panel-header">
        <h3>Hard Problem Decomposer</h3>
      </div>

      <div className="panel-body">
        <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
          {(["decompose", "assumptions", "questions"] as const).map(t => (
            <button
              key={t}
              className={`panel-tab${tab === t ? " active" : ""}`}
              onClick={() => setTab(t)}
            >
              {t}
            </button>
          ))}
        </div>

        {loading && <div className="panel-loading">Loading…</div>}
        {error && (
          <div className="panel-error">
            <span>{error}</span>
            <button onClick={() => setError(null)} aria-label="dismiss">✕</button>
          </div>
        )}

        {tab === "decompose" && (
          <div>
            <div style={{ marginBottom: 14 }}>
              <label className="panel-label" style={{ display: "block" }}>Problem Description</label>
              <textarea
                className="panel-input panel-input-full panel-textarea"
                value={description}
                onChange={e => setDescription(e.target.value)}
                placeholder="Describe the hard problem to decompose..."
                style={{ height: 120, fontFamily: "var(--font-mono)" }}
              />
            </div>
            <button
              className="panel-btn panel-btn-primary"
              onClick={analyze}
              disabled={analyzing || !description.trim()}
              style={{ marginBottom: 20 }}
            >
              {analyzing ? "Analyzing…" : "Analyze"}
            </button>
            {decomposeResult && (
              <div>
                <div className="panel-card" style={{ marginBottom: 14 }}>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                    <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Problem Summary</span>
                    <span className={`panel-tag panel-tag-${COMPLEXITY_INTENT[decomposeResult.overall_complexity]}`}>
                      {decomposeResult.overall_complexity} complexity
                    </span>
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", lineHeight: 1.5 }}>
                    {decomposeResult.problem_summary}
                  </div>
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  {decomposeResult.subsystems.map(sub => (
                    <div
                      key={sub.name}
                      className="panel-card"
                      style={{ borderLeft: `3px solid ${complexityBorderColor(sub.complexity)}` }}
                    >
                      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                        <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{sub.name}</span>
                        <span className={`panel-tag panel-tag-${COMPLEXITY_INTENT[sub.complexity]}`}>{sub.complexity}</span>
                      </div>
                      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>
                        {sub.description}
                      </div>
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
          assumptions.length === 0 ? (
            <div className="panel-empty">No assumptions surfaced yet. Run analysis first.</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {assumptions.map(assumption => (
                <div
                  key={assumption.id}
                  className="panel-card"
                  style={{ borderLeft: `3px solid ${assumptionBorderColor(assumption.status)}` }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                    <span className={`panel-tag panel-tag-${IMPACT_INTENT[assumption.impact]}`}>
                      {assumption.impact} impact
                    </span>
                    <span
                      className={`panel-tag panel-tag-${assumptionStatusIntent(assumption.status)}`}
                      style={{ marginLeft: "auto" }}
                    >
                      {assumption.status}
                    </span>
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", marginBottom: 10, lineHeight: 1.5 }}>
                    {assumption.text}
                  </div>
                  {assumption.status === "unconfirmed" && (
                    <div style={{ display: "flex", gap: 8 }}>
                      <button
                        className="panel-btn panel-btn-primary panel-btn-sm"
                        onClick={() => confirmAssumption(assumption.id, true)}
                      >
                        Confirm
                      </button>
                      <button
                        className="panel-btn panel-btn-danger panel-btn-sm"
                        onClick={() => confirmAssumption(assumption.id, false)}
                      >
                        Reject
                      </button>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )
        )}

        {tab === "questions" && (
          questions.length === 0 ? (
            <div className="panel-empty">No clarifying questions generated. Run analysis first.</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {questions.map(q => (
                <div key={q.id} className="panel-card">
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                    <span className={`panel-tag panel-tag-${IMPACT_INTENT[q.impact]}`}>
                      {q.impact} impact
                    </span>
                    {q.subsystem && (
                      <span className="panel-tag panel-tag-neutral">{q.subsystem}</span>
                    )}
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", lineHeight: 1.5 }}>
                    {q.question}
                  </div>
                </div>
              ))}
            </div>
          )
        )}
      </div>
    </div>
  );
}
