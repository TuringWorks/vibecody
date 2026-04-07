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
  recommendation: string;
  effort: "Low" | "Medium" | "High";
  priority: string;
}

const TABS: PurpleTeamTab[] = ["Exercises", "ATT&CK Matrix", "Simulations", "Coverage Gaps", "Reports"];

const COVERAGE_COLORS: Record<string, string> = {
  Detected: "var(--success-color)",
  Partial: "var(--warning-color)",
  Missed: "var(--error-color)",
  NotTested: "var(--text-secondary)",
};

const STATUS_COLORS: Record<string, string> = {
  Planned: "var(--accent-blue)",
  Active: "var(--success-color)",
  Completed: "var(--text-secondary)",
  Cancelled: "var(--error-color)",
};

const EFFORT_COLORS: Record<string, string> = {
  Low: "var(--success-color)",
  Medium: "var(--warning-color)",
  High: "var(--error-color)",
};

const TACTICS = [
  "Reconnaissance", "Resource Development", "Initial Access", "Execution",
  "Persistence", "Privilege Escalation", "Defense Evasion", "Credential Access",
  "Discovery", "Lateral Movement", "Collection", "C2", "Exfiltration", "Impact",
];

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
  border: "1px solid var(--border-color)",
  borderRadius: 4,
  fontSize: 13,
  fontFamily: "inherit",
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
  borderBottom: "1px solid var(--border-color)",
  color: "var(--text-secondary)",
  fontWeight: 600,
  fontSize: 12,
};

const tdStyle: React.CSSProperties = {
  padding: "8px 10px",
  borderBottom: "1px solid var(--border-color)",
};

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: "color-mix(in srgb, " + color + " 13%, transparent)",
  color,
});

const formGroup: React.CSSProperties = {
  marginBottom: 10,
};

export function PurpleTeamPanel() {
  const [activeTab, setActiveTab] = useState<PurpleTeamTab>("Exercises");
  const [exercises, setExercises] = useState<Exercise[]>([]);
  const [matrix, setMatrix] = useState<MatrixCell[]>([]);
  const [simulations, setSimulations] = useState<Simulation[]>([]);
  const [gaps, setGaps] = useState<CoverageGap[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Exercise form
  const [showExerciseForm, setShowExerciseForm] = useState(false);
  const [exName, setExName] = useState("");
  const [exLead, setExLead] = useState("");
  const [exDescription, setExDescription] = useState("");

  // AI generation
  const [showAiGenerate, setShowAiGenerate] = useState(false);
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiGenerating, setAiGenerating] = useState(false);
  const [aiPlan, setAiPlan] = useState<any>(null);

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

  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const showSuccess = (msg: string) => { setSuccessMsg(msg); setTimeout(() => setSuccessMsg(null), 3000); };

  useEffect(() => {
    loadExercises();
  }, []);

  useEffect(() => {
    if (activeTab === "ATT&CK Matrix" && matrix.length === 0) loadMatrix();
    if (activeTab === "Simulations" && simulations.length === 0) loadSimulations();
    if (activeTab === "Coverage Gaps" && gaps.length === 0) loadGaps();
  }, [activeTab]);

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
      const raw = await invoke<any[]>("get_purple_team_matrix");
      // Backend returns nested: [{ tactic, techniques: [{ id, name, coverage, ... }] }]
      // Flatten into MatrixCell[]
      const cells: MatrixCell[] = [];
      for (const entry of raw) {
        const tactic = entry.tactic || "";
        for (const tech of (entry.techniques || [])) {
          cells.push({
            technique_id: tech.id || tech.technique_id || "",
            technique_name: tech.name || tech.technique_name || "",
            tactic,
            coverage: tech.coverage || "NotTested",
          });
        }
      }
      setMatrix(cells);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load ATT&CK matrix");
    } finally {
      setLoading(false);
    }
  }

  async function loadSimulations() {
    try {
      setLoading(true);
      const result = await invoke<Simulation[]>("get_purple_team_simulations");
      setSimulations(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load simulations");
    } finally {
      setLoading(false);
    }
  }

  async function loadGaps() {
    try {
      setLoading(true);
      const result = await invoke<CoverageGap[]>("get_purple_team_gaps");
      setGaps(result);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load coverage gaps");
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
      showSuccess("Exercise created");
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
      showSuccess("Simulation recorded");
      loadSimulations();
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
          <div style={{ display: "flex", gap: 8 }}>
            <button className="panel-btn panel-btn-primary" onClick={() => { setShowAiGenerate(!showAiGenerate); setShowExerciseForm(false); }}>
              {showAiGenerate ? "Cancel" : "AI Generate Exercise"}
            </button>
            <button className="panel-btn panel-btn-primary" onClick={() => { setShowExerciseForm(!showExerciseForm); setShowAiGenerate(false); }}>
              {showExerciseForm ? "Cancel" : "+ Manual"}
            </button>
          </div>
        </div>

        {/* AI Exercise Generation */}
        {showAiGenerate && (
          <div className="panel-card" style={{ marginBottom: 16 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8, lineHeight: 1.5 }}>
              Describe a threat scenario and the AI will generate a complete purple team exercise with ATT&CK technique mappings, attack steps, and detection expectations.
            </div>
            <textarea
              style={{ ...inputStyle, height: 80, resize: "vertical" }}
              value={aiPrompt}
              onChange={(e) => setAiPrompt(e.target.value)}
              placeholder="e.g., Simulate a ransomware attack targeting healthcare organizations via spear-phishing with macro-enabled documents, followed by lateral movement and data exfiltration before encryption..."
            />
            <div style={{ display: "flex", gap: 8, marginTop: 8, alignItems: "center" }}>
              <button
                className="panel-btn panel-btn-primary"
                onClick={async () => {
                  if (!aiPrompt.trim()) return;
                  setAiGenerating(true);
                  setAiPlan(null);
                  setError(null);
                  try {
                    const result = await invoke<{ exercise: Exercise; plan: any }>("purple_team_ai_generate_exercise", { prompt: aiPrompt.trim() });
                    setAiPlan(result.plan);
                    setExercises(prev => [result.exercise, ...prev]);
                    showSuccess(`Exercise "${result.exercise.name}" created with ${result.plan?.simulations?.length || 0} simulations`);
                  } catch (e: any) {
                    setError(e?.toString() ?? "AI generation failed");
                  }
                  setAiGenerating(false);
                }}
                disabled={aiGenerating || !aiPrompt.trim()}
              >
                {aiGenerating ? "Generating exercise plan..." : "Generate"}
              </button>
              {aiGenerating && <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>AI is mapping ATT&CK techniques...</span>}
            </div>

            {/* Show generated plan */}
            {aiPlan && (
              <div style={{ marginTop: 12 }}>
                <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>{aiPlan.name}</div>
                {aiPlan.threat_scenario && <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>{aiPlan.threat_scenario}</div>}
                {aiPlan.objectives?.length > 0 && (
                  <div style={{ marginBottom: 8 }}>
                    <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 4 }}>Objectives</div>
                    <ul style={{ margin: 0, paddingLeft: 16, fontSize: 12 }}>
                      {aiPlan.objectives.map((o: string, i: number) => <li key={i}>{o}</li>)}
                    </ul>
                  </div>
                )}
                {aiPlan.simulations?.length > 0 && (
                  <div style={{ marginBottom: 8 }}>
                    <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 4 }}>Attack Simulations ({aiPlan.simulations.length})</div>
                    {aiPlan.simulations.map((sim: any, i: number) => (
                      <div key={i} style={{ padding: "6px 10px", marginBottom: 4, borderRadius: 4, background: "var(--bg-secondary)", border: "1px solid var(--border-color)" }}>
                        <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 2 }}>
                          <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--accent-color)" }}>{sim.technique_id}</span>
                          <span style={{ fontWeight: 500, fontSize: 12 }}>{sim.technique_name}</span>
                          <span style={{ ...badgeStyle("var(--bg-tertiary)"), fontSize: 9 }}>{sim.tactic}</span>
                          {sim.difficulty && <span style={{ ...badgeStyle(sim.difficulty === "High" ? "var(--error-bg)" : sim.difficulty === "Medium" ? "var(--warning-bg)" : "var(--success-bg)"), fontSize: 9 }}>{sim.difficulty}</span>}
                        </div>
                        {sim.expected_detection && <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Expected: {sim.expected_detection}</div>}
                        {sim.steps?.length > 0 && (
                          <details style={{ marginTop: 2 }}>
                            <summary style={{ fontSize: 10, color: "var(--text-secondary)", cursor: "pointer" }}>{sim.steps.length} steps</summary>
                            <ol style={{ margin: "4px 0 0 16px", fontSize: 11, color: "var(--text-secondary)", lineHeight: 1.5 }}>
                              {sim.steps.map((s: string, j: number) => <li key={j}>{s}</li>)}
                            </ol>
                          </details>
                        )}
                      </div>
                    ))}
                  </div>
                )}
                {aiPlan.success_criteria?.length > 0 && (
                  <div style={{ marginBottom: 8 }}>
                    <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 4 }}>Success Criteria</div>
                    <ul style={{ margin: 0, paddingLeft: 16, fontSize: 12 }}>
                      {aiPlan.success_criteria.map((c: string, i: number) => <li key={i}>{c}</li>)}
                    </ul>
                  </div>
                )}
                {aiPlan.required_resources?.length > 0 && (
                  <div>
                    <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 4 }}>Required Resources</div>
                    <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                      {aiPlan.required_resources.map((r: string, i: number) => <span key={i} style={badgeStyle("var(--bg-tertiary)")}>{r}</span>)}
                    </div>
                  </div>
                )}
                {aiPlan.duration_hours && <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 8 }}>Duration: {aiPlan.duration_hours} hours</div>}
              </div>
            )}
          </div>
        )}

        {showExerciseForm && (
          <div className="panel-card" style={{ marginBottom: 16 }}>
            <div style={formGroup}>
              <label className="panel-label">Exercise Name</label>
              <input style={inputStyle} value={exName} onChange={(e) => setExName(e.target.value)} placeholder="e.g. Q1 2026 Ransomware Simulation" />
            </div>
            <div style={formGroup}>
              <label className="panel-label">Lead</label>
              <input style={inputStyle} value={exLead} onChange={(e) => setExLead(e.target.value)} placeholder="Exercise lead name" />
            </div>
            <div style={formGroup}>
              <label className="panel-label">Description</label>
              <textarea style={{ ...inputStyle, height: 60, resize: "vertical" }} value={exDescription} onChange={(e) => setExDescription(e.target.value)} placeholder="Exercise objectives and scope..." />
            </div>
            <button className="panel-btn panel-btn-primary" onClick={createExercise} disabled={!exName || !exLead}>Create Exercise</button>
          </div>
        )}

        {exercises.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No exercises found. Create one to start testing.</p>}
        {exercises.map((ex) => (
          <div key={ex.id} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong style={{ fontSize: 14 }}>{ex.name}</strong>
                <span style={{ ...badgeStyle(STATUS_COLORS[ex.status] || "var(--text-secondary)"), marginLeft: 8 }}>{ex.status}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <div style={{ textAlign: "right" }}>
                  <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: ex.coverage_score >= 70 ? "var(--success-color)" : ex.coverage_score >= 40 ? "var(--warning-color)" : "var(--error-color)" }}>
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
          <button className="panel-btn panel-btn-secondary" onClick={loadMatrix}>Refresh Matrix</button>
        </div>

        <div style={{ display: "flex", gap: 16, marginBottom: 16, flexWrap: "wrap" }}>
          <div className="panel-card" style={{ flex: "1 1 120px", textAlign: "center", marginBottom: 0 }}>
            <div style={{ fontSize: 24, fontWeight: 700, fontFamily: "var(--font-mono)", color: stats.percentage >= 70 ? "var(--success-color)" : stats.percentage >= 40 ? "var(--warning-color)" : "var(--error-color)" }}>{stats.percentage}%</div>
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
          <button className="panel-btn panel-btn-primary" onClick={() => setShowSimForm(!showSimForm)}>
            {showSimForm ? "Cancel" : "+ Record Simulation"}
          </button>
        </div>

        <div style={{ marginBottom: 12 }}>
          <label className="panel-label">Filter by Exercise</label>
          <select style={{ ...inputStyle, width: 300 }} value={selectedExercise} onChange={(e) => setSelectedExercise(e.target.value)}>
            <option value="">All Exercises</option>
            {exercises.map((ex) => (
              <option key={ex.id} value={ex.id}>{ex.name}</option>
            ))}
          </select>
        </div>

        {showSimForm && (
          <div className="panel-card" style={{ marginBottom: 16 }}>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Exercise</label>
                <select style={inputStyle} value={simExerciseId} onChange={(e) => setSimExerciseId(e.target.value)}>
                  <option value="">Select exercise...</option>
                  {exercises.map((ex) => (
                    <option key={ex.id} value={ex.id}>{ex.name}</option>
                  ))}
                </select>
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Technique ID</label>
                <input style={inputStyle} value={simTechniqueId} onChange={(e) => setSimTechniqueId(e.target.value)} placeholder="e.g. T1059.001" />
              </div>
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 2 }}>
                <label className="panel-label">Technique Name</label>
                <input style={inputStyle} value={simTechniqueName} onChange={(e) => setSimTechniqueName(e.target.value)} placeholder="e.g. PowerShell" />
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Outcome</label>
                <select style={inputStyle} value={simOutcome} onChange={(e) => setSimOutcome(e.target.value as "Detected" | "Partial" | "Missed")}>
                  <option value="Detected">Detected</option>
                  <option value="Partial">Partial</option>
                  <option value="Missed">Missed</option>
                </select>
              </div>
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Detection Time (seconds)</label>
                <input style={inputStyle} type="number" value={simDetectionTime} onChange={(e) => setSimDetectionTime(e.target.value)} placeholder="e.g. 120" />
              </div>
              <div style={{ ...formGroup, flex: 1 }}>
                <label className="panel-label">Detection Source</label>
                <input style={inputStyle} value={simDetectionSource} onChange={(e) => setSimDetectionSource(e.target.value)} placeholder="e.g. CrowdStrike EDR" />
              </div>
            </div>
            <div style={formGroup}>
              <label className="panel-label">Steps (one per line)</label>
              <textarea style={{ ...inputStyle, height: 60, resize: "vertical" }} value={simSteps} onChange={(e) => setSimSteps(e.target.value)} placeholder="Step 1: Execute payload&#10;Step 2: Observe detection" />
            </div>
            <div style={formGroup}>
              <label className="panel-label">Notes</label>
              <textarea style={{ ...inputStyle, height: 40, resize: "vertical" }} value={simNotes} onChange={(e) => setSimNotes(e.target.value)} />
            </div>
            <button className="panel-btn panel-btn-primary" onClick={recordSimulation} disabled={!simExerciseId || !simTechniqueId}>Record Simulation</button>
          </div>
        )}

        {filtered.length === 0 && <p style={{ color: "var(--text-secondary)", textAlign: "center" }}>No simulations recorded yet.</p>}
        {filtered.map((sim) => (
          <div key={sim.id} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <div>
                <span style={{ fontFamily: "inherit", fontSize: 12, color: "var(--accent-blue)" }}>{sim.technique_id}</span>
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
    const priorityOrder: Record<string, number> = { High: 1, Medium: 2, Low: 3 };
    const sortedGaps = [...gaps].sort((a, b) => (priorityOrder[a.priority] || 9) - (priorityOrder[b.priority] || 9));

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
                  <span style={{ fontFamily: "inherit", fontSize: 11 }}>{gap.technique_id}</span>
                  <br />
                  <span style={{ fontSize: 12 }}>{gap.technique_name}</span>
                </td>
                <td style={tdStyle}>{gap.tactic}</td>
                <td style={tdStyle}>
                  <span style={badgeStyle(COVERAGE_COLORS[gap.current_coverage] || "var(--text-secondary)")}>{gap.current_coverage}</span>
                </td>
                <td style={{ ...tdStyle, fontSize: 12 }}>{gap.recommendation}</td>
                <td style={tdStyle}>
                  <span style={badgeStyle(EFFORT_COLORS[gap.effort] || "var(--text-secondary)")}>{gap.effort}</span>
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
        <div className="panel-card">
          <div style={{ display: "flex", gap: 10, alignItems: "flex-end" }}>
            <div style={{ ...formGroup, flex: 1, marginBottom: 0 }}>
              <label className="panel-label">Exercise</label>
              <select style={inputStyle} value={reportExerciseId} onChange={(e) => setReportExerciseId(e.target.value)}>
                <option value="">Select exercise...</option>
                {exercises.map((ex) => (
                  <option key={ex.id} value={ex.id}>{ex.name}</option>
                ))}
              </select>
            </div>
            <div style={{ ...formGroup, flex: 1, marginBottom: 0 }}>
              <label className="panel-label">Compare With (optional)</label>
              <select style={inputStyle} value={compareExerciseId} onChange={(e) => setCompareExerciseId(e.target.value)}>
                <option value="">None</option>
                {exercises.filter((ex) => ex.id !== reportExerciseId).map((ex) => (
                  <option key={ex.id} value={ex.id}>{ex.name}</option>
                ))}
              </select>
            </div>
            <button className="panel-btn panel-btn-primary" onClick={generateReport} disabled={!reportExerciseId}>Generate Report</button>
          </div>
        </div>

        {reportContent && (
          <div style={{ marginTop: 16 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
              <h4 style={{ margin: 0, fontSize: 14 }}>Generated Report</h4>
              <button className="panel-btn panel-btn-secondary" onClick={() => navigator.clipboard.writeText(reportContent)}>Copy</button>
            </div>
            <pre style={{
              background: "var(--bg-tertiary)",
              border: "1px solid var(--border-color)",
              borderRadius: 6,
              padding: 16,
              fontSize: 12,
              fontFamily: "inherit",
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
    <div className="panel-container">
      <div className="panel-tab-bar">
        {TABS.map((tab) => (
          <button key={tab} className={`panel-tab ${activeTab === tab ? "active" : ""}`} onClick={() => setActiveTab(tab)}>
            {tab}
          </button>
        ))}
      </div>
      <div className="panel-body">
        {successMsg && (
          <div style={{ padding: "8px 12px", marginBottom: 12, background: "rgba(76,175,80,0.13)", border: "1px solid var(--success-color)", borderRadius: 4, fontSize: 12, color: "var(--success-color)" }}>
            {successMsg}
          </div>
        )}
        {error && (
          <div className="panel-error" style={{ marginBottom: 12, display: "flex", justifyContent: "space-between" }}>
            <span>{error}</span>
            <button style={{ background: "none", border: "none", color: "var(--error-color)", cursor: "pointer", fontSize: 14 }} onClick={() => setError(null)}>x</button>
          </div>
        )}
        {loading && <div className="panel-loading">Loading...</div>}
        {!loading && renderTab()}
      </div>
    </div>
  );
}
