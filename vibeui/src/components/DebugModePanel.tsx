import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DebugSession {
  id: string;
  name: string;
  status: "running" | "paused" | "stopped";
  startedAt: string;
  language: string;
}

interface Breakpoint {
  id: string;
  session_id: string;
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
  const [sessions, setSessions] = useState<DebugSession[]>([]);
  const [breakpoints, setBreakpoints] = useState<Breakpoint[]>([]);
  const [analysis, setAnalysis] = useState<AnalysisResult[]>([]);
  const [newBpFile, setNewBpFile] = useState("");
  const [newBpLine, setNewBpLine] = useState("");
  const [newBpType, setNewBpType] = useState<"line" | "conditional" | "logpoint">("line");
  const [newBpCondition, setNewBpCondition] = useState("");
  const [newSessionName, setNewSessionName] = useState("");
  const [newSessionLang, setNewSessionLang] = useState("Rust");
  const [selectedSession, setSelectedSession] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadSessions = useCallback(async () => {
    try {
      const data = await invoke<DebugSession[]>("list_debug_sessions");
      setSessions(data);
      if (data.length > 0 && !selectedSession) {
        setSelectedSession(data[0].id);
      }
    } catch (e) {
      setError(String(e));
    }
  }, [selectedSession]);

  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  const createSession = async () => {
    if (!newSessionName) return;
    setLoading(true);
    setError(null);
    try {
      const session = await invoke<DebugSession>("create_debug_session", {
        name: newSessionName,
        language: newSessionLang,
      });
      setSessions((prev) => [...prev, session]);
      setSelectedSession(session.id);
      setNewSessionName("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const deleteSession = async (id: string) => {
    setError(null);
    try {
      await invoke("delete_debug_session", { sessionId: id });
      setSessions((prev) => prev.filter((s) => s.id !== id));
      setBreakpoints((prev) => prev.filter((b) => b.session_id !== id));
      if (selectedSession === id) {
        setSelectedSession("");
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const addBreakpoint = async () => {
    if (!newBpFile || !newBpLine || !selectedSession) return;
    setLoading(true);
    setError(null);
    try {
      const bp = await invoke<Breakpoint>("add_debug_breakpoint", {
        sessionId: selectedSession,
        file: newBpFile,
        line: parseInt(newBpLine, 10),
        bpType: newBpType,
        condition: newBpCondition,
      });
      setBreakpoints((prev) => [...prev, bp]);
      setNewBpFile("");
      setNewBpLine("");
      setNewBpCondition("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const removeBreakpoint = async (id: string) => {
    setError(null);
    try {
      await invoke("remove_debug_breakpoint", { breakpointId: id });
      setBreakpoints((prev) => prev.filter((b) => b.id !== id));
    } catch (e) {
      setError(String(e));
    }
  };

  const runAnalysis = async () => {
    if (!selectedSession) return;
    setLoading(true);
    setError(null);
    try {
      const results = await invoke<AnalysisResult[]>("run_debug_analysis", {
        sessionId: selectedSession,
      });
      setAnalysis(results);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

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

  const btnDangerStyle: React.CSSProperties = {
    ...btnStyle,
    background: "var(--error-color, #e53e3e)",
    border: "1px solid var(--error-color, #e53e3e)",
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

  const sessionBreakpoints = breakpoints.filter((b) => b.session_id === selectedSession);
  const tabs = ["sessions", "breakpoints", "analysis"];

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Debug Mode</h3>

      {error && (
        <div style={{ padding: "8px", marginBottom: "8px", background: "var(--error-color, #e53e3e)", color: "white", borderRadius: "4px", fontSize: "12px" }}>
          {error}
        </div>
      )}

      <div style={tabBarStyle}>
        {tabs.map((t) => (
          <button key={t} style={tabStyle(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "sessions" && (
        <div>
          <div style={{ ...cardStyle, display: "flex", gap: "8px", flexWrap: "wrap", alignItems: "center", marginBottom: "12px" }}>
            <input
              style={{ ...inputStyle, width: "140px" }}
              placeholder="File or function name"
              value={newSessionName}
              onChange={(e) => setNewSessionName(e.target.value)}
            />
            <select style={inputStyle} value={newSessionLang} onChange={(e) => setNewSessionLang(e.target.value)}>
              <option value="Rust">Rust</option>
              <option value="TypeScript">TypeScript</option>
              <option value="JavaScript">JavaScript</option>
              <option value="Python">Python</option>
              <option value="Go">Go</option>
              <option value="Java">Java</option>
              <option value="C++">C++</option>
            </select>
            <button style={btnStyle} onClick={createSession} disabled={loading}>
              {loading ? "Creating..." : "New Session"}
            </button>
          </div>

          {sessions.length === 0 && (
            <div style={{ opacity: 0.6, textAlign: "center", padding: "20px" }}>
              No debug sessions. Create one above.
            </div>
          )}

          {sessions.map((s) => (
            <div
              key={s.id}
              style={{
                ...cardStyle,
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                border: selectedSession === s.id ? "1px solid var(--accent-color)" : cardStyle.border,
                cursor: "pointer",
              }}
              onClick={() => setSelectedSession(s.id)}
            >
              <div>
                <strong>{s.name}</strong> <span style={{ opacity: 0.7 }}>({s.language})</span>
                <div style={{ fontSize: "12px", opacity: 0.6, marginTop: "2px" }}>Started: {s.startedAt}</div>
              </div>
              <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                <span style={badgeStyle(s.status)}>{s.status}</span>
                <button style={btnDangerStyle} onClick={(e) => { e.stopPropagation(); deleteSession(s.id); }}>
                  Delete
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "breakpoints" && (
        <div>
          {!selectedSession && (
            <div style={{ opacity: 0.6, textAlign: "center", padding: "20px" }}>
              Select a session in the Sessions tab first.
            </div>
          )}

          {selectedSession && (
            <>
              <div style={{ marginBottom: "8px", fontSize: "12px", opacity: 0.7 }}>
                Session: {sessions.find((s) => s.id === selectedSession)?.name || selectedSession}
              </div>

              {sessionBreakpoints.map((bp) => (
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

              {sessionBreakpoints.length === 0 && (
                <div style={{ opacity: 0.6, textAlign: "center", padding: "12px" }}>
                  No breakpoints for this session.
                </div>
              )}

              <div style={{ ...cardStyle, display: "flex", gap: "8px", flexWrap: "wrap", alignItems: "center" }}>
                <input style={{ ...inputStyle, width: "120px" }} placeholder="File" value={newBpFile} onChange={(e) => setNewBpFile(e.target.value)} />
                <input style={{ ...inputStyle, width: "60px" }} placeholder="Line" value={newBpLine} onChange={(e) => setNewBpLine(e.target.value)} />
                <select style={inputStyle} value={newBpType} onChange={(e) => setNewBpType(e.target.value as Breakpoint["type"])}>
                  <option value="line">Line</option>
                  <option value="conditional">Conditional</option>
                  <option value="logpoint">Logpoint</option>
                </select>
                <input style={{ ...inputStyle, width: "140px" }} placeholder="Condition" value={newBpCondition} onChange={(e) => setNewBpCondition(e.target.value)} />
                <button style={btnStyle} onClick={addBreakpoint} disabled={loading}>Add</button>
              </div>
            </>
          )}
        </div>
      )}

      {activeTab === "analysis" && (
        <div>
          {!selectedSession && (
            <div style={{ opacity: 0.6, textAlign: "center", padding: "20px" }}>
              Select a session in the Sessions tab first.
            </div>
          )}

          {selectedSession && (
            <>
              <div style={{ marginBottom: "12px", display: "flex", gap: "8px", alignItems: "center" }}>
                <span style={{ fontSize: "12px", opacity: 0.7 }}>
                  Session: {sessions.find((s) => s.id === selectedSession)?.name || selectedSession}
                </span>
                <button style={btnStyle} onClick={runAnalysis} disabled={loading}>
                  {loading ? "Analyzing..." : "Run Analysis"}
                </button>
              </div>

              {analysis.length === 0 && (
                <div style={{ opacity: 0.6, textAlign: "center", padding: "20px" }}>
                  Click "Run Analysis" to detect potential issues.
                </div>
              )}

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
            </>
          )}
        </div>
      )}
    </div>
  );
};

export default DebugModePanel;
