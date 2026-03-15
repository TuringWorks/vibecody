import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type PurpleTeamTab = "Exercises" | "ATT&CK Matrix" | "Simulations" | "Coverage Gaps" | "Reports";

interface Exercise {
  id: string;
  name: string;
  status: "Planned" | "Active" | "Completed" | "Cancelled";
  lead: string;
  date: string;
  coverage_score: number;
  description: string;
  technique_count: number;
}

interface MatrixCell {
  technique_id: string;
  technique_name: string;
  tactic: string;
  coverage: "Detected" | "Partial" | "Missed" | "NotTested";
}

interface Simulation {
  id: string;
  exercise_id: string;
  technique_id: string;
  technique_name: string;
  tactic: string;
  steps: string[];
  outcome: "Detected" | "Partial" | "Missed";
  detection_time_seconds: number | null;
  detection_source: string;
  notes: string;
}

interface CoverageGap {
  technique_id: string;
  technique_name: string;
  tactic: string;
  current_coverage: "Missed" | "Partial" | "NotTested";
  recommended_detection: string;
  effort: "Low" | "Medium" | "High";
  priority: number;
}

const TABS: PurpleTeamTab[] = ["Exercises", "ATT&CK Matrix", "Simulations", "Coverage Gaps", "Reports"];

const COVERAGE_COLORS: Record<string, string> = {
  Detected: "#a6e3a1",
  Partial: "#f9e2af",
  Missed: "#f38ba8",
  NotTested: "#6c7086",
};

const STATUS_COLORS: Record<string, string> = {
  Planned: "#89b4fa",
  Active: "#a6e3a1",
  Completed: "#6c7086",
  Cancelled: "#f38ba8",
};

const EFFORT_COLORS: Record<string, string> = {
  Low: "#a6e3a1",
  Medium: "#f9e2af",
  High: "#f38ba8",
};

const TACTICS = [
  "Reconnaissance", "Resource Development", "Initial Access", "Execution",
  "Persistence", "Privilege Escalation", "Defense Evasion", "Credential Access",
  "Discovery", "Lateral Movement", "Collection", "C2", "Exfiltration", "Impact",
];

const containerStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  height: "100%",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontFamily: "var(--font-mono)",
  overflow: "hidden",
};

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: 2,
  padding: "8px 12px 0",
  borderBottom: "1px solid var(--border-primary)",
  background: "var(--bg-secondary)",
  overflowX: "auto",
  flexShrink: 0,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px",
  cursor: "pointer",
  background: active ? "var(--bg-primary)" : "transparent",
  color: active ? "var(--accent-primary)" : "var(--text-secondary)",
  border: "none",
  borderBottom: active ? "2px solid var(--accent-primary)" : "2px solid transparent",
  fontSize: 13,
  fontFamily: "var(--font-mono)",
  whiteSpace: "nowrap",
});

const contentStyle: React.CSSProperties = {
  flex: 1,
  overflow: "auto",
  padding: 16,
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  background: "var(--accent-primary)",
  color: "var(--bg-primary)",
  border: "none",
  borderRadius: 4,
  cursor: "pointer",
  fontSize: 12,
  fontFamily: "var(--font-mono)",
};

const btnSecondary: React.CSSProperties = {
  ...btnStyle,
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
};

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
  border: "1px solid var(--border-primary)",
  borderRadius: 4,
  fontSize: 13,
  fontFamily: "var(--font-mono)",
  width: "100%",
  boxSizing: "border-box",
};

const tableStyle: React.CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: 13,
};

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "8px 10px",
  borderBottom: "1px solid var(--border-primary)",
  color: "var(--text-secondary)",
  fontWeight: 600,
  fontSize: 12,
};

const tdStyle: React.CSSProperties = {
  padding: "8px 10px",
  borderBottom: "1px solid var(--border-primary)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: color + "22",
  color,
});

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-primary)",
  borderRadius: 6,
  padding: 14,
  marginBottom: 10,
};

const formGroup: React.CSSProperties = {
  marginBottom: 10,
};

const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: 12,
  color: "var(--text-secondary)",
  marginBottom: 4,
};

export function PurpleTeamPanel() {
  const [activeTab, setActiveTab] = useState<PurpleTeamTab>("Exercises");
  const [exercises, setExercises] = useState<Exercise[]>([]);
  const [matrix, setMatrix] = useState<MatrixCell[]>([]);
  const [simulations, _setSimulations] = useState<Simulation[]>([]);
  const [gaps, _setGaps] = useState<CoverageGap[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Exercise form
  const [showExerciseForm, setShowExerciseForm] = useState(false);
  const [exName, setExName] = useState("");
  const [exLead, setExLead] = useState("");
  const [exDescription, setExDescription] = useState("");

  // Simulation form
  const [showSimForm, setShowSimForm] = useState(false);
  const [simExerciseId, setSimExerciseId] = useState("");
  const [simTechniqueId, setSimTechniqueId] = useState("");
  const [simTechniqueName, setSimTechniqueName] = useState("");
  const [simOutcome, setSimOutcome] = useState<"Detected" | "Partial" | "Missed">("Detected");
  const [simDetectionTime, setSimDetectionTime] = useState("");
  const [simDetectionSource, setSimDetectionSource] = useState("");
  const [simSteps, setSimSteps] = useState("");
  const [simNotes, setSimNotes] = useState("");

  // Report state
  const [reportExerciseId, setReportExerciseId] = useState("");
  const [compareExerciseId, setCompareExerciseId] = useState("");
  const [reportContent, setReportContent] = useState("");

  // Selected exercise for simulations filter
  const [selectedExercise, setSelectedExercise] = useState("");

  useEffect(() => {
    loadExercises();
  }, []);

  async function loadExercises() {
    try {
      setLoading(true);
      const result = await invoke<Exercise[]>("list_purple_team_exercises");
      setExercises(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load exercises");
    } finally {
      setLoading(false);
    }
  }

  async function loadMatrix() {
    try {
      setLoading(true);
      const result = await invoke<MatrixCell[]>("get_purple_team_matrix");
      setMatrix(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load ATT&CK matrix");
    } finally {
      setLoading(false);
    }
  }

  async function createExercise() {
    try {
      await invoke("create_purple_team_exercise", {
        name: exName,
        lead: exLead,
        description: exDescription,
      });
      setShowExerciseForm(false);
      setExName("");
      setExLead("");
      setExDescription("");
      loadExercises();
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to create exercise");
    }
  }

  async function recordSimulation() {
    try {
      await invoke("record_purple_team_simulation", {
        exerciseId: simExerciseId,
        techniqueId: simTechniqueId,
        techniqueName: simTechniqueName,
        outcome: simOutcome,
        detectionTimeSeconds: simDetectionTime ? Number(simDetectionTime) : null,
        detectionSource: simDetectionSource,
        steps: simSteps.split("\n").filter((s) => s.trim()),
        notes: simNotes,
      });
      setShowSimForm(false);
      setSimTechniqueId("");
      setSimTechniqueName("");
      setSimSteps("");
      setSimNotes("");
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to record simulation");
    }
  }

  async function generateReport() {
    try {
      setLoading(true);
      const result = await invoke<string>("generate_purple_team_report", {
        exerciseId: reportExerciseId,
        compareId: compareExerciseId || null,
      });
      setReportContent(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to generate report");
    } finally {
      setLoading(false);
    }
  }

  function getCoverageStats() {
    if (matrix.length === 0) return { detected: 0, partial: 0, missed: 0, notTested: 0, total: 0, percentage: 0 };
    const detected = matrix.filter((m) => m.coverage === "Detected").length;
    const partial = matrix.filter((m) => m.coverage === "Partial").length;
    const missed = matrix.filter((m) => m.coverage === "Missed").length;
    const notTested = matrix.filter((m) => m.coverage === "NotTested").length;
    const total = matrix.length;
    const percentage = total > 0 ? Math.round(((detected + partial * 0.5) / total) * 100) : 0;
    return { detected, partial, missed, notTested, total, percentage };
  }

  function renderExercises() {
    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Purple Team Exercises</h3>
          <button style={btnStyle} onClick={() => setShowExerciseForm(!showExerciseForm)}>
            {showExerciseForm ? "Cancel" : "+ New Exercise"}
          </button>
        </div>

        {showExerciseForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={formGroup}>
              <label style={labelStyle}>Exercise Name</label>
              <input style={inputStyle} value={exName} onChange={(e) => setExName(e.target.value)} placeholder="e.g. Q1 2026 Ransomware Simulation" />
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Lead</label>
              <input style={inputStyle} value={exLead} onChange={(e) => setExLead(e.target.value)} placeholder="Exercise lead name" />
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Description</label>
              <textarea style={{ ...inputStyle, height: 60, resize: "vertical" }} value={exDescription} onChange={(e) => setExDescription(e.target.value)} placeholder="Exercise objectives and scope..." />
            </div>
            <button style={btnStyle} onClick={createExercise} disabled={!exName || !exLead}>Create Exercise</button>
          </div>
        )}

        {exercises.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No exercises found. Create one to start testing.</p>}
        {exercises.map((ex) => (
          <div key={ex.id} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong style={{ fontSize: 14 }}>{ex.name}</strong>
                <span style={{ ...badgeStyle(STATUS_COLORS[ex.status] || "#6c7086"), marginLeft: 8 }}>{ex.status}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <div style={{ textAlign: "right" }}>
                  <div style={{ fontSize: 18, fontWeight: 700, color: ex.coverage_score >= 70 ? "#a6e3a1" : ex.coverage_score >= 40 ? "#f9e2af" : "#f38ba8" }}>
                    {ex.coverage_score}%
                  </div>
                  <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>Coverage</div>
                </div>
              </div>
            </div>
            <div style={{ display: "flex", gap: 16, marginTop: 8, fontSize: 12, color: "var(--text-secondary)" }}>
              <span>Lead: {ex.lead}</span>
              <span>Date: {ex.date}</span>
              <span>Techniques: {ex.technique_count}</span>
            </div>
            {ex.description && <p style={{ margin: "6px 0 0", fontSize: 12, color: "var(--text-secondary)" }}>{ex.description}</p>}
          </div>
        ))}
      </div>
    );
  }

  function renderATTACKMatrix() {
    const stats = getCoverageStats();
    const tacticGroups: Record<string, MatrixCell[]> = {};
    TACTICS.forEach((t) => { tacticGroups[t] = []; });
    matrix.forEach((cell) => {
      if (tacticGroups[cell.tactic]) {
        tacticGroups[cell.tactic].push(cell);
      }
    });

    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>MITRE ATT&CK Coverage Matrix</h3>
          <button style={btnSecondary} onClick={loadMatrix}>Refresh Matrix</button>
        </div>

        <div style={{ display: "flex", gap: 16, marginBottom: 16, flexWrap: "wrap" }}>
          <div style={{ ...cardStyle, flex: "1 1 120px", textAlign: "center", marginBottom: 0 }}>
            <div style={{ fontSize: 24, fontWeight: 700, color: stats.percentage >= 70 ? "#a6e3a1" : stats.percentage >= 40 ? "#f9e2af" : "#f38ba8" }}>{stats.percentage}%</div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Overall Coverage</div>
          </div>
          <div style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
            {[
              { label: "Detected", color: COVERAGE_COLORS.Detected, count: stats.detected },
              { label: "Partial", color: COVERAGE_COLORS.Partial, count: stats.partial },
              { label: "Missed", color: COVERAGE_COLORS.Missed, count: stats.missed },
              { label: "Not Tested", color: COVERAGE_COLORS.NotTested, count: stats.notTested },
            ].map((item) => (
              <div key={item.label} style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <div style={{ width: 12, height: 12, borderRadius: 2, background: item.color }} />
                <span style={{ fontSize: 12 }}>{item.label} ({item.count})</span>
              </div>
            ))}
          </div>
        </div>

        <div style={{ overflowX: "auto" }}>
          <table style={tableStyle}>
            <thead>
              <tr>
                <th style={{ ...thStyle, position: "sticky", left: 0, background: "var(--bg-primary)", zIndex: 1 }}>Technique</th>
                {TACTICS.map((t) => (
                  <th key={t} style={{ ...thStyle, textAlign: "center", fontSize: 10, minWidth: 70, writingMode: "vertical-lr", transform: "rotate(180deg)", padding: "10px 4px" }}>{t}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {matrix.length === 0 && (
                <tr><td colSpan={TACTICS.length + 1} style={{ ...tdStyle, textAlign: "center", color: "var(--text-secondary)" }}>Load the matrix to see coverage data.</td></tr>
              )}
              {/* Group unique techniques */}
              {Array.from(new Set(matrix.map((m) => m.technique_id))).map((techId) => {
                const techCells = matrix.filter((m) => m.technique_id === techId);
                const techName = techCells[0]?.technique_name || techId;
                return (
                  <tr key={techId}>
                    <td style={{ ...tdStyle, fontSize: 11, position: "sticky", left: 0, background: "var(--bg-primary)", zIndex: 1, whiteSpace: "nowrap" }}>
                      <span style={{ fontWeight: 500 }}>{techId}</span>
                      <br />
                      <span style={{ color: "var(--text-secondary)", fontSize: 10 }}>{techName}</span>
                    </td>
                    {TACTICS.map((tactic) => {
                      const cell = techCells.find((c) => c.tactic === tactic);
                      return (
                        <td key={tactic} style={{ ...tdStyle, textAlign: "center", padding: 4 }}>
                          {cell ? (
                            <div style={{ width: 20, height: 20, borderRadius: 3, background: COVERAGE_COLORS[cell.coverage], margin: "0 auto", cursor: "pointer" }} title={`${cell.technique_name}: ${cell.coverage}`} />
                          ) : null}
                        </td>
                      );
                    })}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    );
  }

  function renderSimulations() {
    const filtered = selectedExercise
      ? simulations.filter((s) => s.exercise_id === selectedExercise)
      : simulations;

    return (
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0, fontSize: 15 }}>Attack Simulations</h3>
          <button style={btnStyle} onClick={() => setShowSimForm(!showSimForm)}>
            {showSimForm ? "Cancel" : "+ Record Simulation"}
          </button>
        </div>

        <div style={{ marginBottom: 12 }}>
          <label style={labelStyle}>Filter by Exercise</label>
          <select style={{ ...inputStyle, width: 300 }} value={selectedExercise} onChange={(e) => setSelectedExercise(e.target.value)}>
            <option value="">All Exercises</option>
            {exercises.map((ex) => (
              <option key={ex.id} value={ex.id}>{ex.name}</option>
            ))}
          </select>
        </div>

        {showSimForm && (
          <div style={{ ...cardStyle, marginBottom: 16 }}>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Exercise</label>
                <select style={inputStyle} value={simExerciseId} onChange={(e) => setSimExerciseId(e.target.value)}>
                  <option value="">Select exercise...</option>
                  {exercises.map((ex) => (
                    <option key={ex.id} value={ex.id}>{ex.name}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Technique ID</label>
                <input style={inputStyle} value={simTechniqueId} onChange={(e) => setSimTechniqueId(e.target.value)} placeholder="e.g. T1059.001" />
              </div>
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 2 }}>
                <label style={labelStyle}>Technique Name</label>
                <input style={inputStyle} value={simTechniqueName} onChange={(e) => setSimTechniqueName(e.target.value)} placeholder="e.g. PowerShell" />
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Outcome</label>
                <select style={inputStyle} value={simOutcome} onChange={(e) => setSimOutcome(e.target.value as any)}>
                  <option value="Detected">Detected</option>
                  <option value="Partial">Partial</option>
                  <option value="Missed">Missed</option>
                </select>
              </div>
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Detection Time (seconds)</label>
                <input style={inputStyle} type="number" value={simDetectionTime} onChange={(e) => setSimDetectionTime(e.target.value)} placeholder="e.g. 120" />
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label style={labelStyle}>Detection Source</label>
                <input style={inputStyle} value={simDetectionSource} onChange={(e) => setSimDetectionSource(e.target.value)} placeholder="e.g. CrowdStrike EDR" />
              </div>
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Steps (one per line)</label>
              <textarea style={{ ...inputStyle, height: 60, resize: "vertical" }} value={simSteps} onChange={(e) => setSimSteps(e.target.value)} placeholder="Step 1: Execute payload&#10;Step 2: Observe detection" />
            </div>
            <div style={formGroup}>
              <label style={labelStyle}>Notes</label>
              <textarea style={{ ...inputStyle, height: 40, resize: "vertical" }} value={simNotes} onChange={(e) => setSimNotes(e.target.value)} />
            </div>
            <button style={btnStyle} onClick={recordSimulation} disabled={!simExerciseId || !simTechniqueId}>Record Simulation</button>
          </div>
        )}

        {filtered.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No simulations recorded yet.</p>}
        {filtered.map((sim) => (
          <div key={sim.id} style={cardStyle}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <div>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--accent-primary)" }}>{sim.technique_id}</span>
                <strong style={{ marginLeft: 8, fontSize: 14 }}>{sim.technique_name}</strong>
                <span style={{ marginLeft: 8, fontSize: 11, color: "var(--text-secondary)" }}>{sim.tactic}</span>
              </div>
              <span style={badgeStyle(COVERAGE_COLORS[sim.outcome])}>{sim.outcome}</span>
            </div>
            {sim.steps.length > 0 && (
              <ol style={{ margin: "8px 0", paddingLeft: 20, fontSize: 12 }}>
                {sim.steps.map((step, i) => <li key={i} style={{ marginBottom: 2 }}>{step}</li>)}
              </ol>
            )}
            <div style={{ display: "flex", gap: 16, fontSize: 11, color: "var(--text-secondary)" }}>
              {sim.detection_time_seconds != null && <span>Detection: {sim.detection_time_seconds}s</span>}
              {sim.detection_source && <span>Source: {sim.detection_source}</span>}
            </div>
            {sim.notes && <p style={{ margin: "6px 0 0", fontSize: 12, color: "var(--text-secondary)", fontStyle: "italic" }}>{sim.notes}</p>}
          </div>
        ))}
      </div>
    );
  }

  function renderCoverageGaps() {
    const sortedGaps = [...gaps].sort((a, b) => a.priority - b.priority);

    return (
      <div>
        <h3 style={{ margin: "0 0 14px", fontSize: 15 }}>Coverage Gaps (sorted by priority)</h3>
        {sortedGaps.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No coverage gaps identified. Run an ATT&CK matrix assessment first.</p>}
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>#</th>
              <th style={thStyle}>Technique</th>
              <th style={thStyle}>Tactic</th>
              <th style={thStyle}>Current</th>
              <th style={thStyle}>Recommended Detection</th>
              <th style={thStyle}>Effort</th>
            </tr>
          </thead>
          <tbody>
            {sortedGaps.map((gap, i) => (
              <tr key={gap.technique_id}>
                <td style={tdStyle}>{i + 1}</td>
                <td style={tdStyle}>
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>{gap.technique_id}</span>
                  <br />
                  <span style={{ fontSize: 12 }}>{gap.technique_name}</span>
                </td>
                <td style={tdStyle}>{gap.tactic}</td>
                <td style={tdStyle}>
                  <span style={badgeStyle(COVERAGE_COLORS[gap.current_coverage] || "#6c7086")}>{gap.current_coverage}</span>
                </td>
                <td style={{ ...tdStyle, fontSize: 12 }}>{gap.recommended_detection}</td>
                <td style={tdStyle}>
                  <span style={badgeStyle(EFFORT_COLORS[gap.effort] || "#6c7086")}>{gap.effort}</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  function renderReports() {
    return (
      <div>
        <h3 style={{ margin: "0 0 14px", fontSize: 15 }}>Exercise Reports</h3>
        <div style={cardStyle}>
          <div style={{ display: "flex", gap: 10, alignItems: "flex-end" }}>
            <div style={{ ...formGroup, flex: 1, marginBottom: 0 }}>
              <label style={labelStyle}>Exercise</label>
              <select style={inputStyle} value={reportExerciseId} onChange={(e) => setReportExerciseId(e.target.value)}>
                <option value="">Select exercise...</option>
                {exercises.map((ex) => (
                  <option key={ex.id} value={ex.id}>{ex.name}</option>
                ))}
              </select>
            </div>
            <div style={{ ...formGroup, flex: 1, marginBottom: 0 }}>
              <label style={labelStyle}>Compare With (optional)</label>
              <select style={inputStyle} value={compareExerciseId} onChange={(e) => setCompareExerciseId(e.target.value)}>
                <option value="">None</option>
                {exercises.filter((ex) => ex.id !== reportExerciseId).map((ex) => (
                  <option key={ex.id} value={ex.id}>{ex.name}</option>
                ))}
              </select>
            </div>
            <button style={btnStyle} onClick={generateReport} disabled={!reportExerciseId}>Generate Report</button>
          </div>
        </div>

        {reportContent && (
          <div style={{ marginTop: 16 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <h4 style={{ margin: 0, fontSize: 14 }}>Generated Report</h4>
              <button style={btnSecondary} onClick={() => navigator.clipboard.writeText(reportContent)}>Copy</button>
            </div>
            <pre style={{
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-primary)",
              borderRadius: 6,
              padding: 16,
              fontSize: 12,
              fontFamily: "var(--font-mono)",
              overflow: "auto",
              whiteSpace: "pre-wrap",
              maxHeight: 500,
            }}>
              {reportContent}
            </pre>
          </div>
        )}
      </div>
    );
  }

  const renderTab = () => {
    switch (activeTab) {
      case "Exercises": return renderExercises();
      case "ATT&CK Matrix": return renderATTACKMatrix();
      case "Simulations": return renderSimulations();
      case "Coverage Gaps": return renderCoverageGaps();
      case "Reports": return renderReports();
    }
  };

  return (
    <div style={containerStyle}>
      <div style={tabBarStyle}>
        {TABS.map((tab) => (
          <button key={tab} style={tabStyle(activeTab === tab)} onClick={() => setActiveTab(tab)}>
            {tab}
          </button>
        ))}
      </div>
      <div style={contentStyle}>
        {error && (
          <div style={{ padding: "8px 12px", marginBottom: 12, background: "#f38ba822", border: "1px solid #f38ba8", borderRadius: 4, fontSize: 12, color: "#f38ba8", display: "flex", justifyContent: "space-between" }}>
            <span>{error}</span>
            <button style={{ background: "none", border: "none", color: "#f38ba8", cursor: "pointer", fontSize: 14 }} onClick={() => setError(null)}>x</button>
          </div>
        )}
        {loading && <div style={{ textAlign: "center", padding: 20, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>}
        {!loading && renderTab()}
      </div>
    </div>
  );
}
