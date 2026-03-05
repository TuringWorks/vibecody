/**
 * TraceDashboard — Visual timeline inspector for agent session traces.
 *
 * Shows a timeline of steps, token/cost attribution, step details,
 * and filterable views of agent execution history.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TraceSession {
  session_id: string;
  timestamp: number;
  step_count: number;
}

interface TraceStep {
  timestamp: number;
  session_id: string;
  step: number;
  tool: string;
  input_summary: string;
  output: string;
  success: boolean;
  duration_ms: number;
  approved_by: string;
}

type StepKind = "prompt" | "tool_call" | "file_edit" | "test" | "error" | "other";

const KIND_COLORS: Record<StepKind, string> = {
  prompt: "#89b4fa",
  tool_call: "#a6e3a1",
  file_edit: "#fab387",
  test: "#cba6f7",
  error: "#f38ba8",
  other: "#6c7086",
};

function classifyStep(tool: string, success: boolean): StepKind {
  if (!success) return "error";
  const t = tool.toLowerCase();
  if (t.includes("write_file") || t.includes("edit") || t.includes("patch")) return "file_edit";
  if (t.includes("bash") || t.includes("shell") || t.includes("test")) return "test";
  if (t.includes("prompt") || t.includes("chat") || t.includes("llm")) return "prompt";
  if (t === "none" || t === "") return "prompt";
  return "tool_call";
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleTimeString();
}

export function TraceDashboard() {
  const [sessions, setSessions] = useState<TraceSession[]>([]);
  const [selectedSession, setSelectedSession] = useState<string | null>(null);
  const [steps, setSteps] = useState<TraceStep[]>([]);
  const [expandedStep, setExpandedStep] = useState<number | null>(null);
  const [filter, setFilter] = useState<StepKind | "all">("all");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<TraceSession[]>("list_trace_sessions")
      .then(setSessions)
      .catch(() => {});
  }, []);

  const loadSession = async (sessionId: string) => {
    setLoading(true);
    setSelectedSession(sessionId);
    setExpandedStep(null);
    try {
      const data = await invoke<TraceStep[]>("load_trace_session", { sessionId });
      setSteps(data);
    } catch {
      setSteps([]);
    }
    setLoading(false);
  };

  const filteredSteps = filter === "all"
    ? steps
    : steps.filter((s) => classifyStep(s.tool, s.success) === filter);

  const totalDuration = steps.reduce((sum, s) => sum + s.duration_ms, 0);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div style={{
        padding: "8px 12px", borderBottom: "1px solid var(--border, #2a2a3e)",
        display: "flex", alignItems: "center", gap: 8,
      }}>
        <span style={{ fontSize: 14, fontWeight: 700 }}>Trace Dashboard</span>
        <div style={{ flex: 1 }} />
        {selectedSession && (
          <button onClick={() => { setSelectedSession(null); setSteps([]); }} style={{
            ...chipStyle, cursor: "pointer", background: "rgba(99,102,241,0.15)",
            border: "1px solid #6366f1",
          }}>
            Back to list
          </button>
        )}
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px" }}>
        {!selectedSession ? (
          /* Session list */
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ fontSize: 11, color: "var(--text-secondary, #a6adc8)", marginBottom: 4 }}>
              Select a session to view its execution timeline.
            </div>
            {sessions.length === 0 && (
              <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
                No trace sessions found. Run an agent task to generate traces.
              </div>
            )}
            {sessions.map((s) => (
              <button key={s.session_id} onClick={() => loadSession(s.session_id)} style={{
                padding: "6px 8px", borderRadius: 4, textAlign: "left", cursor: "pointer",
                border: "1px solid var(--border, #2a2a3e)",
                background: "var(--bg-primary, #11111b)",
                color: "var(--text-primary, #cdd6f4)",
              }}>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <span style={{ fontSize: 10, fontFamily: "monospace", color: "#89b4fa" }}>
                    {s.session_id.slice(0, 12)}
                  </span>
                  <div style={{ flex: 1 }} />
                  <span style={{ fontSize: 9, opacity: 0.5 }}>
                    {s.step_count} steps
                  </span>
                  <span style={{ fontSize: 9, opacity: 0.4, fontFamily: "monospace" }}>
                    {new Date(s.timestamp).toLocaleString()}
                  </span>
                </div>
              </button>
            ))}
          </div>
        ) : loading ? (
          <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
            Loading session...
          </div>
        ) : (
          /* Timeline view */
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {/* Summary bar */}
            <div style={{
              display: "flex", gap: 12, padding: "6px 8px", borderRadius: 4,
              background: "var(--bg-primary, #11111b)", fontSize: 10,
            }}>
              <span><strong>{steps.length}</strong> steps</span>
              <span><strong>{formatDuration(totalDuration)}</strong> total</span>
              <span style={{ color: "#a6e3a1" }}>
                {steps.filter((s) => s.success).length} success
              </span>
              <span style={{ color: "#f38ba8" }}>
                {steps.filter((s) => !s.success).length} errors
              </span>
            </div>

            {/* Progress bar (step types) */}
            {steps.length > 0 && (
              <div style={{ display: "flex", height: 6, borderRadius: 3, overflow: "hidden", gap: 1 }}>
                {steps.map((s, i) => {
                  const kind = classifyStep(s.tool, s.success);
                  return (
                    <div key={i} style={{
                      flex: Math.max(s.duration_ms, 100),
                      background: KIND_COLORS[kind],
                      opacity: 0.8,
                    }} title={`${s.tool} (${formatDuration(s.duration_ms)})`} />
                  );
                })}
              </div>
            )}

            {/* Legend + Filter */}
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {(["all", "prompt", "tool_call", "file_edit", "test", "error"] as const).map((k) => (
                <button key={k} onClick={() => setFilter(k)} style={{
                  ...chipStyle, cursor: "pointer",
                  border: filter === k ? "1px solid #6366f1" : "1px solid var(--border, #2a2a3e)",
                  background: filter === k ? "rgba(99,102,241,0.15)" : "transparent",
                }}>
                  {k !== "all" && (
                    <span style={{
                      display: "inline-block", width: 8, height: 8, borderRadius: 2,
                      background: KIND_COLORS[k as StepKind], marginRight: 4,
                    }} />
                  )}
                  {k === "all" ? "All" : k.replace("_", " ")}
                </button>
              ))}
            </div>

            {/* Timeline steps */}
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              {filteredSteps.map((s, i) => {
                const kind = classifyStep(s.tool, s.success);
                const isExpanded = expandedStep === s.step;
                return (
                  <div key={i} style={{
                    borderRadius: 4, overflow: "hidden",
                    border: "1px solid var(--border, #2a2a3e)",
                    background: "var(--bg-primary, #11111b)",
                  }}>
                    <button onClick={() => setExpandedStep(isExpanded ? null : s.step)} style={{
                      display: "flex", gap: 6, alignItems: "center", padding: "5px 8px",
                      width: "100%", cursor: "pointer", border: "none",
                      background: "transparent", color: "var(--text-primary, #cdd6f4)",
                      textAlign: "left",
                    }}>
                      {/* Step indicator */}
                      <span style={{
                        width: 8, height: 8, borderRadius: 2, flexShrink: 0,
                        background: KIND_COLORS[kind],
                      }} />
                      <span style={{ fontSize: 9, fontWeight: 700, minWidth: 20 }}>
                        #{s.step}
                      </span>
                      <span style={{ fontSize: 10, fontWeight: 600, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {s.tool || "LLM"}
                      </span>
                      {!s.success && (
                        <span style={{ fontSize: 9, color: "#f38ba8", fontWeight: 700 }}>FAIL</span>
                      )}
                      <span style={{ fontSize: 9, opacity: 0.5, fontFamily: "monospace" }}>
                        {formatDuration(s.duration_ms)}
                      </span>
                      <span style={{ fontSize: 9, opacity: 0.3, fontFamily: "monospace" }}>
                        {formatTimestamp(s.timestamp)}
                      </span>
                      <span style={{ fontSize: 10, opacity: 0.4 }}>{isExpanded ? "▼" : "▶"}</span>
                    </button>

                    {isExpanded && (
                      <div style={{ padding: "6px 8px 8px", borderTop: "1px solid var(--border, #2a2a3e)" }}>
                        {s.input_summary && (
                          <div style={{ marginBottom: 6 }}>
                            <div style={{ fontSize: 9, fontWeight: 700, opacity: 0.5, marginBottom: 2 }}>INPUT</div>
                            <pre style={{
                              fontSize: 10, padding: "4px 6px", borderRadius: 3, margin: 0,
                              background: "rgba(0,0,0,0.2)", whiteSpace: "pre-wrap",
                              maxHeight: 150, overflowY: "auto",
                            }}>
                              {s.input_summary}
                            </pre>
                          </div>
                        )}
                        {s.output && (
                          <div>
                            <div style={{ fontSize: 9, fontWeight: 700, opacity: 0.5, marginBottom: 2 }}>OUTPUT</div>
                            <pre style={{
                              fontSize: 10, padding: "4px 6px", borderRadius: 3, margin: 0,
                              background: "rgba(0,0,0,0.2)", whiteSpace: "pre-wrap",
                              maxHeight: 200, overflowY: "auto",
                            }}>
                              {s.output.slice(0, 2000)}{s.output.length > 2000 ? "..." : ""}
                            </pre>
                          </div>
                        )}
                        {s.approved_by && (
                          <div style={{ fontSize: 9, opacity: 0.4, marginTop: 4 }}>
                            Approved by: {s.approved_by}
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
              {filteredSteps.length === 0 && (
                <div style={{ padding: 16, textAlign: "center", opacity: 0.5, fontSize: 11 }}>
                  {filter === "all" ? "No steps in this session." : `No ${filter.replace("_", " ")} steps.`}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

const chipStyle: React.CSSProperties = {
  padding: "2px 8px", fontSize: 9, fontWeight: 600, borderRadius: 4,
  border: "1px solid var(--border, #2a2a3e)",
  background: "transparent", color: "var(--text-primary, #cdd6f4)",
};
