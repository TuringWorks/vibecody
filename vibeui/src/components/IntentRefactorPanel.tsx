/**
 * IntentRefactorPanel — Intent-Preserving Refactoring dashboard.
 *
 * Parse refactoring intents, generate step-by-step plans, and get AI
 * suggestions for refactoring opportunities in code snippets.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PlanStep {
  description: string;
  status: string;
  file: string;
}

interface PlanResult {
  sessionId: string;
  intent: string;
  steps: PlanStep[];
}

interface Suggestion {
  intent: string;
  confidence: number;
}

type Tab = "plan" | "suggest";

const statusTag = (s: string) =>
  s.includes("Completed") ? "panel-tag panel-tag-success"
  : s.includes("InProgress") ? "panel-tag panel-tag-info"
  : s.includes("Failed") ? "panel-tag panel-tag-danger"
  : "panel-tag panel-tag-neutral";

const confColor = (c: number) =>
  c > 0.7 ? "var(--success-color)" : c > 0.4 ? "var(--warning-color)" : "var(--text-secondary)";

export default function IntentRefactorPanel() {
  const [tab, setTab] = useState<Tab>("plan");
  const [intent, setIntent] = useState("");
  const [files, setFiles] = useState("src/main.rs");
  const [plan, setPlan] = useState<PlanResult | null>(null);
  const [code, setCode] = useState("");
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const doPlan = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const fileList = files.split(",").map(f => f.trim()).filter(Boolean);
      const res = await invoke<PlanResult>("refactor_plan", { intentStr: intent, files: fileList });
      setPlan(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [intent, files]);

  const doSuggest = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const res = await invoke<{ suggestions: Suggestion[] }>("refactor_suggest", { code });
      setSuggestions(res.suggestions);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [code]);

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Intent-Preserving Refactoring</h3>
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          {(["plan", "suggest"] as Tab[]).map(t => (
            <button
              key={t}
              className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`}
              onClick={() => setTab(t)}
            >
              {t === "plan" ? "Plan" : "Suggest"}
            </button>
          ))}
        </div>
      </div>

      <div className="panel-body">
        {error && (
          <div className="panel-error" style={{ marginBottom: 10 }}>
            {error}
            <button onClick={() => setError("")}>✕</button>
          </div>
        )}

        {tab === "plan" && (
          <>
            <div className="panel-card" style={{ marginBottom: 10 }}>
              <div className="panel-label">
                Intent (e.g. "make testable", "reduce coupling")
              </div>
              <input
                className="panel-input panel-input-full"
                value={intent}
                onChange={e => setIntent(e.target.value)}
                placeholder="make this module testable"
                style={{ marginBottom: 8 }}
              />
              <div className="panel-label">Target Files (comma-separated)</div>
              <input
                className="panel-input panel-input-full"
                value={files}
                onChange={e => setFiles(e.target.value)}
                style={{ marginBottom: 8 }}
              />
              <button
                className="panel-btn panel-btn-primary"
                onClick={doPlan}
                disabled={loading || !intent}
              >
                {loading ? "Generating…" : "Generate Plan"}
              </button>
            </div>

            {plan && (
              <div className="panel-card">
                <div style={{ fontWeight: "var(--font-semibold)", marginBottom: 6, fontSize: "var(--font-size-base)" }}>
                  Plan: {plan.intent}
                </div>
                <div className="panel-label" style={{ marginBottom: 8 }}>
                  Session: {plan.sessionId} · {plan.steps.length} step{plan.steps.length !== 1 ? "s" : ""}
                </div>
                {plan.steps.map((s, i) => (
                  <div key={i} className="panel-row" style={{ padding: "4px 0", borderBottom: "1px solid var(--border-subtle)" }}>
                    <span style={{ width: 20, textAlign: "right", color: "var(--text-muted)", fontSize: "var(--font-size-xs)", flexShrink: 0 }}>
                      {i + 1}.
                    </span>
                    <span style={{ flex: 1, fontSize: "var(--font-size-base)" }}>{s.description}</span>
                    <span className={statusTag(s.status)}>{s.status}</span>
                  </div>
                ))}
              </div>
            )}

            {!plan && !loading && !error && (
              <div className="panel-empty">Describe your refactoring intent above and click Generate Plan.</div>
            )}
          </>
        )}

        {tab === "suggest" && (
          <>
            <div className="panel-card" style={{ marginBottom: 10 }}>
              <div className="panel-label">Paste code to analyze for refactoring opportunities</div>
              <textarea
                className="panel-input panel-input-full panel-textarea"
                value={code}
                onChange={e => setCode(e.target.value)}
                rows={8}
                style={{ fontFamily: "var(--font-mono)", marginBottom: 8 }}
                placeholder="fn example() { ... }"
              />
              <button
                className="panel-btn panel-btn-primary"
                onClick={doSuggest}
                disabled={loading || !code}
              >
                {loading ? "Analyzing…" : "Analyze"}
              </button>
            </div>

            {suggestions.length > 0 && (
              <div className="panel-card">
                <div style={{ fontWeight: "var(--font-semibold)", marginBottom: 8, fontSize: "var(--font-size-base)" }}>
                  Suggestions
                </div>
                {suggestions.map((s, i) => (
                  <div key={i} className="panel-row" style={{ padding: "4px 0", borderBottom: "1px solid var(--border-subtle)" }}>
                    <span style={{ flex: 1, fontSize: "var(--font-size-base)" }}>{s.intent}</span>
                    <span style={{ color: confColor(s.confidence), fontWeight: "var(--font-semibold)", fontSize: "var(--font-size-sm)" }}>
                      {(s.confidence * 100).toFixed(0)}%
                    </span>
                  </div>
                ))}
              </div>
            )}

            {suggestions.length === 0 && !loading && !error && (
              <div className="panel-empty">Paste code above and click Analyze to find refactoring opportunities.</div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
