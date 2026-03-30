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

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, boxSizing: "border-box" };
const tabRow: React.CSSProperties = { display: "flex", gap: 4, marginBottom: 12 };

type Tab = "plan" | "suggest";

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
    setLoading(true); setError("");
    try {
      const fileList = files.split(",").map(f => f.trim()).filter(Boolean);
      const res = await invoke<PlanResult>("refactor_plan", { intentStr: intent, files: fileList });
      setPlan(res);
    } catch (e: any) { setError(String(e)); }
    setLoading(false);
  }, [intent, files]);

  const doSuggest = useCallback(async () => {
    setLoading(true); setError("");
    try {
      const res = await invoke<{ suggestions: Suggestion[] }>("refactor_suggest", { code });
      setSuggestions(res.suggestions);
    } catch (e: any) { setError(String(e)); }
    setLoading(false);
  }, [code]);

  const statusColor = (s: string) => s.includes("Completed") ? "#4caf50" : s.includes("InProgress") ? "#2196f3" : s.includes("Failed") ? "#f44336" : "var(--text-secondary)";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Intent-Preserving Refactoring</h2>
      <div style={tabRow}>
        {(["plan", "suggest"] as Tab[]).map(t => (
          <button key={t} style={{ ...btnStyle, background: tab === t ? "var(--accent-color)" : "var(--bg-tertiary)", color: tab === t ? "#fff" : "var(--text-primary)" }} onClick={() => setTab(t)}>
            {t === "plan" ? "Plan" : "Suggest"}
          </button>
        ))}
      </div>

      {error && <div style={{ color: "#f44336", marginBottom: 8, fontSize: 12 }}>{error}</div>}

      {tab === "plan" && (
        <>
          <div style={cardStyle}>
            <div style={labelStyle}>Intent (e.g. "make testable", "reduce coupling")</div>
            <input value={intent} onChange={e => setIntent(e.target.value)} style={{ ...inputStyle, marginBottom: 8 }} placeholder="make this module testable" />
            <div style={labelStyle}>Target Files (comma-separated)</div>
            <input value={files} onChange={e => setFiles(e.target.value)} style={{ ...inputStyle, marginBottom: 8 }} />
            <button style={btnStyle} onClick={doPlan} disabled={loading || !intent}>
              {loading ? "..." : "Generate Plan"}
            </button>
          </div>

          {plan && (
            <div style={cardStyle}>
              <div style={{ fontWeight: 600, marginBottom: 8 }}>Plan: {plan.intent}</div>
              <div style={labelStyle}>Session: {plan.sessionId} | {plan.steps.length} step(s)</div>
              {plan.steps.map((s, i) => (
                <div key={i} style={{ display: "flex", gap: 8, padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
                  <span style={{ width: 20, textAlign: "right", color: "var(--text-secondary)" }}>{i + 1}.</span>
                  <span style={{ flex: 1 }}>{s.description}</span>
                  <span style={{ fontSize: 11, color: statusColor(s.status) }}>{s.status}</span>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {tab === "suggest" && (
        <>
          <div style={cardStyle}>
            <div style={labelStyle}>Paste code to analyze for refactoring opportunities</div>
            <textarea value={code} onChange={e => setCode(e.target.value)} rows={8} style={{ ...inputStyle, fontFamily: "monospace", resize: "vertical", marginBottom: 8 }} placeholder="fn example() { ... }" />
            <button style={btnStyle} onClick={doSuggest} disabled={loading || !code}>
              {loading ? "..." : "Analyze"}
            </button>
          </div>

          {suggestions.length > 0 && (
            <div style={cardStyle}>
              <div style={{ fontWeight: 600, marginBottom: 8 }}>Suggestions</div>
              {suggestions.map((s, i) => (
                <div key={i} style={{ display: "flex", justifyContent: "space-between", padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
                  <span>{s.intent}</span>
                  <span style={{ color: s.confidence > 0.7 ? "#4caf50" : s.confidence > 0.4 ? "#ff9800" : "var(--text-secondary)" }}>
                    {(s.confidence * 100).toFixed(0)}%
                  </span>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
