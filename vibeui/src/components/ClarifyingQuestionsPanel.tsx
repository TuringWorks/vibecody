import React, { useState } from "react";

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
  const [questions, setQuestions] = useState<ClarifyingQuestion[]>([
    { id: "1", question: "Should this feature support multi-file refactoring?", answer: "", skipped: false, priority: "high" },
    { id: "2", question: "Do you want to preserve backward compatibility?", answer: "", skipped: false, priority: "high" },
    { id: "3", question: "Should tests be generated automatically?", answer: "Yes, unit tests for all new functions", skipped: false, priority: "medium" },
    { id: "4", question: "Is there a preferred error handling strategy?", answer: "", skipped: true, priority: "low" },
  ]);
  const [planSteps] = useState<PlanStep[]>([
    { id: "1", description: "Refactor provider trait to support streaming", files: ["provider.rs", "claude.rs", "openai.rs"], effort: "2 hrs", status: "done" },
    { id: "2", description: "Add streaming response handler", files: ["agent.rs", "stream.rs"], effort: "1.5 hrs", status: "in-progress" },
    { id: "3", description: "Update CLI output for streaming", files: ["main.rs", "tui.rs"], effort: "1 hr", status: "pending" },
    { id: "4", description: "Write integration tests", files: ["tests/streaming.rs"], effort: "45 min", status: "pending" },
  ]);
  const [risks] = useState<RiskItem[]>([
    { label: "Breaking change", level: "high", detail: "Provider trait signature change affects all 17 implementations" },
    { label: "Token overhead", level: "medium", detail: "Streaming adds ~5% token overhead due to chunked responses" },
    { label: "Test coverage", level: "low", detail: "Existing tests cover 82% of affected paths" },
  ]);

  const containerStyle: React.CSSProperties = {
    padding: "16px",
    color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "inherit",
    fontSize: "13px",
    height: "100%",
    overflow: "auto",
  };

  const tabBarStyle: React.CSSProperties = {
    display: "flex",
    gap: "4px",
    borderBottom: "1px solid var(--border-color)",
    marginBottom: "12px",
  };

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px",
    cursor: "pointer",
    border: "none",
    background: active ? "var(--bg-secondary)" : "transparent",
    color: active ? "var(--text-primary)" : "var(--text-secondary)",
    borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
    fontFamily: "inherit",
    fontSize: "inherit",
  });

  const cardStyle: React.CSSProperties = {
    padding: "10px",
    marginBottom: "8px",
    borderRadius: "4px",
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
  };

  const inputStyle: React.CSSProperties = {
    padding: "6px 10px",
    background: "var(--bg-secondary)",
    color: "var(--text-primary)",
    border: "1px solid var(--border-color)",
    borderRadius: "3px",
    fontFamily: "inherit",
    fontSize: "inherit",
    width: "100%",
    boxSizing: "border-box",
  };

  const btnStyle: React.CSSProperties = {
    padding: "4px 10px",
    border: "1px solid var(--accent-color)",
    background: "var(--accent-color)",
    color: "white",
    borderRadius: "3px",
    cursor: "pointer",
    fontFamily: "inherit",
    fontSize: "12px",
  };

  const priorityColor = (p: string) =>
    p === "high" ? "var(--error-color)" : p === "medium" ? "var(--warning-color)" : "var(--text-muted)";

  const statusColor = (s: string) =>
    s === "done" ? "var(--success-color)" : s === "in-progress" ? "var(--warning-color)" : "var(--text-muted)";

  const updateAnswer = (id: string, answer: string) => {
    setQuestions((prev) => prev.map((q) => (q.id === id ? { ...q, answer, skipped: false } : q)));
  };

  const skipQuestion = (id: string) => {
    setQuestions((prev) => prev.map((q) => (q.id === id ? { ...q, skipped: true, answer: "" } : q)));
  };

  const unansweredCount = questions.filter((q) => !q.answer && !q.skipped).length;
  const answeredCount = questions.filter((q) => q.answer).length;
  const skippedCount = questions.filter((q) => q.skipped).length;
  const totalEffort = planSteps.reduce((acc, s) => {
    const match = s.effort.match(/([\d.]+)/);
    return acc + (match ? parseFloat(match[1]) : 0);
  }, 0);

  const tabs = ["questions", "plan", "summary"];

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
          {questions.map((q) => (
            <div key={q.id} style={{ ...cardStyle, opacity: q.skipped ? 0.5 : 1 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px" }}>
                <strong style={{ fontSize: "13px" }}>{q.question}</strong>
                <span style={{ fontSize: "11px", color: priorityColor(q.priority) }}>{q.priority}</span>
              </div>
              {!q.skipped ? (
                <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                  <input style={{ ...inputStyle, flex: 1 }} placeholder="Your answer..." value={q.answer} onChange={(e) => updateAnswer(q.id, e.target.value)} />
                  <button style={{ ...btnStyle, background: "transparent", color: "var(--text-muted)" }} onClick={() => skipQuestion(q.id)}>Skip</button>
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
                  <span key={f} style={{ padding: "2px 6px", borderRadius: "3px", fontSize: "11px", backgroundColor: "var(--bg-tertiary)", color: "white" }}>
                    {f}
                  </span>
                ))}
              </div>
            </div>
          ))}
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
                <div style={{ fontSize: "24px", fontWeight: 700, color: "var(--text-muted)" }}>{skippedCount}</div>
                <div style={{ fontSize: "11px", opacity: 0.6 }}>Skipped</div>
              </div>
            </div>
          </div>
          <div style={{ fontWeight: 600, margin: "12px 0 8px" }}>Risk Assessment</div>
          {risks.map((r, i) => (
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
