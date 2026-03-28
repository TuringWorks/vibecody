import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ClarifyingQuestion {
  id: string;
  question: string;
  answer: string;
  skipped: boolean;
  priority: "high" | "medium" | "low";
}

interface PlanStep {
  id: string;
  description: string;
  files: string[];
  effort: string;
  status: "pending" | "in-progress" | "done";
}

interface RiskItem {
  label: string;
  level: "high" | "medium" | "low";
  detail: string;
}

const ClarifyingQuestionsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("questions");
  const [taskInput, setTaskInput] = useState("");
  const [questions, setQuestions] = useState<ClarifyingQuestion[]>([]);
  const [planSteps, setPlanSteps] = useState<PlanStep[]>([]);
  const [risks, setRisks] = useState<RiskItem[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      try {
        const [q, p, r] = await Promise.all([
          invoke<ClarifyingQuestion[]>("get_clarify_questions").catch(() => []),
          invoke<PlanStep[]>("get_clarify_plan").catch(() => []),
          invoke<RiskItem[]>("get_clarify_risks").catch(() => []),
        ]);
        setQuestions(q);
        setPlanSteps(p);
        setRisks(r);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  // Persist questions on change
  const persistQuestions = async (updated: ClarifyingQuestion[]) => {
    setQuestions(updated);
    try { await invoke("save_clarify_questions", { questions: updated }); } catch { /* ignore */ }
  };

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--text-primary)", backgroundColor: "var(--bg-primary)",
    fontFamily: "inherit", fontSize: "13px", height: "100%", overflow: "auto",
  };
  const tabBarStyle: React.CSSProperties = { display: "flex", gap: "4px", borderBottom: "1px solid var(--border-color)", marginBottom: "12px" };
  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px", cursor: "pointer", border: "none",
    background: active ? "var(--bg-secondary)" : "transparent",
    color: active ? "var(--text-primary)" : "var(--text-secondary)",
    borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
    fontFamily: "inherit", fontSize: "inherit",
  });
  const cardStyle: React.CSSProperties = { padding: "10px", marginBottom: "8px", borderRadius: "4px", backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border-color)" };
  const inputStyle: React.CSSProperties = { padding: "6px 10px", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "3px", fontFamily: "inherit", fontSize: "inherit", width: "100%", boxSizing: "border-box" };
  const btnStyle: React.CSSProperties = { padding: "4px 10px", border: "1px solid var(--accent-color)", background: "var(--accent-color)", color: "var(--btn-primary-fg)", borderRadius: "3px", cursor: "pointer", fontFamily: "inherit", fontSize: "12px" };

  const priorityColor = (p: string) => p === "high" ? "var(--error-color)" : p === "medium" ? "var(--warning-color)" : "var(--text-secondary)";
  const statusColor = (s: string) => s === "done" ? "var(--success-color)" : s === "in-progress" ? "var(--warning-color)" : "var(--text-secondary)";

  const updateAnswer = (id: string, answer: string) => {
    persistQuestions(questions.map((q) => (q.id === id ? { ...q, answer, skipped: false } : q)));
  };
  const skipQuestion = (id: string) => {
    persistQuestions(questions.map((q) => (q.id === id ? { ...q, skipped: true, answer: "" } : q)));
  };

  const unansweredCount = questions.filter((q) => !q.answer && !q.skipped).length;
  const answeredCount = questions.filter((q) => q.answer).length;
  const skippedCount = questions.filter((q) => q.skipped).length;
  const totalEffort = planSteps.reduce((acc, s) => { const m = s.effort.match(/([\d.]+)/); return acc + (m ? parseFloat(m[1]) : 0); }, 0);

  const tabs = ["questions", "plan", "summary"];

  if (loading) return <div style={{ ...containerStyle, textAlign: "center", paddingTop: 40 }}>Loading...</div>;

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Clarifying Questions</h3>
      <div style={tabBarStyle}>
        {tabs.map((t) => (
          <button key={t} style={tabStyle(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "questions" && (
        <div>
          <div style={{ marginBottom: "12px" }}>
            <label style={{ fontSize: "12px", fontWeight: 600, display: "block", marginBottom: "4px" }}>Task Description</label>
            <textarea style={{ ...inputStyle, minHeight: "60px", resize: "vertical" }} placeholder="Describe what you want to build or change..." value={taskInput} onChange={(e) => setTaskInput(e.target.value)} />
          </div>
          {questions.length === 0 && (
            <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)" }}>
              No clarifying questions yet. Questions are generated when a task is analyzed.
            </div>
          )}
          {questions.map((q) => (
            <div key={q.id} style={{ ...cardStyle, opacity: q.skipped ? 0.5 : 1 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px" }}>
                <strong style={{ fontSize: "13px" }}>{q.question}</strong>
                <span style={{ fontSize: "11px", color: priorityColor(q.priority) }}>{q.priority}</span>
              </div>
              {!q.skipped ? (
                <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                  <input style={{ ...inputStyle, flex: 1 }} placeholder="Your answer..." value={q.answer} onChange={(e) => updateAnswer(q.id, e.target.value)} />
                  <button style={{ ...btnStyle, background: "transparent", color: "var(--text-secondary)" }} onClick={() => skipQuestion(q.id)}>Skip</button>
                </div>
              ) : (
                <div style={{ fontSize: "12px", opacity: 0.6, fontStyle: "italic" }}>Skipped</div>
              )}
            </div>
          ))}
        </div>
      )}

      {activeTab === "plan" && (
        <div>
          {planSteps.length === 0 ? (
            <div style={{ textAlign: "center", padding: 24, color: "var(--text-secondary)" }}>
              No plan steps yet. A plan is generated after clarifying questions are answered.
            </div>
          ) : (
            <>
              <div style={{ marginBottom: "10px", fontSize: "12px", opacity: 0.7 }}>
                MegaPlan: {planSteps.length} steps | Est. total: {totalEffort} hrs
              </div>
              {planSteps.map((step, i) => (
                <div key={step.id} style={cardStyle}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "4px" }}>
                    <strong>Step {i + 1}: {step.description}</strong>
                    <span style={{ fontSize: "11px", padding: "2px 8px", borderRadius: "10px", backgroundColor: statusColor(step.status), color: "var(--bg-primary)" }}>
                      {step.status}
                    </span>
                  </div>
                  <div style={{ fontSize: "12px", opacity: 0.7, marginBottom: "4px" }}>Effort: {step.effort}</div>
                  <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
                    {step.files.map((f) => (
                      <span key={f} style={{ padding: "2px 6px", borderRadius: "3px", fontSize: "11px", backgroundColor: "var(--bg-tertiary)", color: "var(--btn-primary-fg)" }}>
                        {f}
                      </span>
                    ))}
                  </div>
                </div>
              ))}
            </>
          )}
        </div>
      )}

      {activeTab === "summary" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: "8px" }}>Session Status</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "8px", textAlign: "center" }}>
              <div>
                <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--success-color)" }}>{answeredCount}</div>
                <div style={{ fontSize: "11px", opacity: 0.6 }}>Answered</div>
              </div>
              <div>
                <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--warning-color)" }}>{unansweredCount}</div>
                <div style={{ fontSize: "11px", opacity: 0.6 }}>Unanswered</div>
              </div>
              <div>
                <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--text-secondary)" }}>{skippedCount}</div>
                <div style={{ fontSize: "11px", opacity: 0.6 }}>Skipped</div>
              </div>
            </div>
          </div>
          <div style={{ fontWeight: 600, margin: "12px 0 8px" }}>Risk Assessment</div>
          {risks.length === 0 ? (
            <div style={{ color: "var(--text-secondary)", fontSize: 12 }}>No risks identified yet.</div>
          ) : risks.map((r, i) => (
            <div key={i} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "4px" }}>
                <strong>{r.label}</strong>
                <span style={{ fontSize: "11px", padding: "2px 8px", borderRadius: "10px", color: "var(--bg-primary)", backgroundColor: priorityColor(r.level) }}>
                  {r.level}
                </span>
              </div>
              <div style={{ fontSize: "12px", opacity: 0.8 }}>{r.detail}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default ClarifyingQuestionsPanel;
