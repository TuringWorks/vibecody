import React, { useState } from "react";

interface DebugSession {
  id: string;
  name: string;
  status: "running" | "paused" | "stopped";
  startedAt: string;
  language: string;
}

interface Breakpoint {
  id: string;
  file: string;
  line: number;
  type: "line" | "conditional" | "logpoint";
  condition: string;
  enabled: boolean;
}

interface AnalysisResult {
  hypothesis: string;
  confidence: number;
  rootCause: string;
  autoFix: string;
}

const DebugModePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("sessions");
  const [sessions, setSessions] = useState<DebugSession[]>([
    { id: "1", name: "main.rs", status: "running", startedAt: "12:03:41", language: "Rust" },
    { id: "2", name: "App.tsx", status: "paused", startedAt: "12:10:22", language: "TypeScript" },
    { id: "3", name: "server.py", status: "stopped", startedAt: "11:45:00", language: "Python" },
  ]);
  const [breakpoints, setBreakpoints] = useState<Breakpoint[]>([
    { id: "1", file: "main.rs", line: 42, type: "line", condition: "", enabled: true },
    { id: "2", file: "handler.rs", line: 87, type: "conditional", condition: "count > 10", enabled: true },
    { id: "3", file: "App.tsx", line: 15, type: "logpoint", condition: "state={state}", enabled: false },
  ]);
  const [newBpFile, setNewBpFile] = useState("");
  const [newBpLine, setNewBpLine] = useState("");
  const [newBpType, setNewBpType] = useState<"line" | "conditional" | "logpoint">("line");
  const [newBpCondition, setNewBpCondition] = useState("");
  const [analysis] = useState<AnalysisResult[]>([
    { hypothesis: "Null pointer dereference in async handler", confidence: 0.87, rootCause: "Uninitialized optional field accessed without guard", autoFix: "Add Option::unwrap_or_default() before access" },
    { hypothesis: "Race condition in shared state update", confidence: 0.62, rootCause: "Missing mutex lock on concurrent write path", autoFix: "Wrap state update in Arc<Mutex<T>>" },
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

  const badgeStyle = (status: string): React.CSSProperties => ({
    padding: "2px 8px",
    borderRadius: "10px",
    fontSize: "11px",
    fontWeight: 600,
    backgroundColor: status === "running" ? "var(--success-color)" : status === "paused" ? "var(--warning-color)" : "var(--text-muted)",
    color: "var(--bg-primary)",
  });

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

  const inputStyle: React.CSSProperties = {
    padding: "4px 8px",
    background: "var(--bg-secondary)",
    color: "var(--text-primary)",
    border: "1px solid var(--border-color)",
    borderRadius: "3px",
    fontFamily: "inherit",
    fontSize: "inherit",
  };

  const cardStyle: React.CSSProperties = {
    padding: "10px",
    marginBottom: "8px",
    borderRadius: "4px",
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
  };

  const toggleSession = (id: string) => {
    setSessions((prev) =>
      prev.map((s) =>
        s.id === id ? { ...s, status: s.status === "running" ? "stopped" : "running" } : s
      )
    );
  };

  const addBreakpoint = () => {
    if (!newBpFile || !newBpLine) return;
    const bp: Breakpoint = {
      id: String(Date.now()),
      file: newBpFile,
      line: parseInt(newBpLine, 10),
      type: newBpType,
      condition: newBpCondition,
      enabled: true,
    };
    setBreakpoints((prev) => [...prev, bp]);
    setNewBpFile("");
    setNewBpLine("");
    setNewBpCondition("");
  };

  const removeBreakpoint = (id: string) => {
    setBreakpoints((prev) => prev.filter((b) => b.id !== id));
  };

  const tabs = ["sessions", "breakpoints", "analysis"];

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Debug Mode</h3>
      <div style={tabBarStyle}>
        {tabs.map((t) => (
          <button key={t} style={tabStyle(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "sessions" && (
        <div>
          {sessions.map((s) => (
            <div key={s.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{s.name}</strong> <span style={{ opacity: 0.7 }}>({s.language})</span>
                <div style={{ fontSize: "12px", opacity: 0.6, marginTop: "2px" }}>Started: {s.startedAt}</div>
              </div>
              <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                <span style={badgeStyle(s.status)}>{s.status}</span>
                <button style={btnStyle} onClick={() => toggleSession(s.id)}>
                  {s.status === "running" ? "Stop" : "Start"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "breakpoints" && (
        <div>
          {breakpoints.map((bp) => (
            <div key={bp.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{bp.file}:{bp.line}</strong>{" "}
                <span style={{ opacity: 0.7, fontSize: "12px" }}>[{bp.type}]</span>
                {bp.condition && <div style={{ fontSize: "12px", opacity: 0.6 }}>Condition: {bp.condition}</div>}
              </div>
              <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                <span style={{ fontSize: "12px", color: bp.enabled ? "var(--success-color)" : "var(--text-muted)" }}>
                  {bp.enabled ? "Enabled" : "Disabled"}
                </span>
                <button style={btnStyle} onClick={() => removeBreakpoint(bp.id)}>Remove</button>
              </div>
            </div>
          ))}
          <div style={{ ...cardStyle, display: "flex", gap: "8px", flexWrap: "wrap", alignItems: "center" }}>
            <input style={{ ...inputStyle, width: "120px" }} placeholder="File" value={newBpFile} onChange={(e) => setNewBpFile(e.target.value)} />
            <input style={{ ...inputStyle, width: "60px" }} placeholder="Line" value={newBpLine} onChange={(e) => setNewBpLine(e.target.value)} />
            <select style={inputStyle} value={newBpType} onChange={(e) => setNewBpType(e.target.value as Breakpoint["type"])}>
              <option value="line">Line</option>
              <option value="conditional">Conditional</option>
              <option value="logpoint">Logpoint</option>
            </select>
            <input style={{ ...inputStyle, width: "140px" }} placeholder="Condition" value={newBpCondition} onChange={(e) => setNewBpCondition(e.target.value)} />
            <button style={btnStyle} onClick={addBreakpoint}>Add</button>
          </div>
        </div>
      )}

      {activeTab === "analysis" && (
        <div>
          {analysis.map((a, i) => (
            <div key={i} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "6px" }}>
                <strong>Hypothesis {i + 1}</strong>
                <span style={{ fontSize: "12px", opacity: 0.7 }}>Confidence: {(a.confidence * 100).toFixed(0)}%</span>
              </div>
              <p style={{ margin: "4px 0" }}>{a.hypothesis}</p>
              <div style={{ marginTop: "8px", padding: "6px 8px", borderRadius: "3px", backgroundColor: "var(--bg-secondary)" }}>
                <div style={{ fontSize: "12px", fontWeight: 600, marginBottom: "2px" }}>Root Cause</div>
                <div style={{ fontSize: "12px" }}>{a.rootCause}</div>
              </div>
              <div style={{ marginTop: "6px", padding: "6px 8px", borderRadius: "3px", backgroundColor: "var(--bg-secondary)" }}>
                <div style={{ fontSize: "12px", fontWeight: 600, marginBottom: "2px" }}>Auto-Fix Suggestion</div>
                <div style={{ fontSize: "12px" }}>{a.autoFix}</div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default DebugModePanel;
