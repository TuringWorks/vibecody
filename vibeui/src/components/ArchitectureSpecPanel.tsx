/**
 * ArchitectureSpecPanel — Enterprise Architecture dashboard.
 *
 * Workflow:
 *  1. "Generate from Codebase" scans the workspace and produces draft TOGAF artifacts.
 *  2. Each artifact is reviewed in-panel: Approve / Request Review / Reject.
 *  3. Phase completion % is based on approved artifacts vs required types.
 *  4. ADRs are human-authored via the form in the ADRs tab.
 *  5. All data is persisted to WorkspaceStore (<workspace>/.vibecli/workspace.db).
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types (mirrors Rust structs) ──────────────────────────────────────────

type ArtifactStatus = "Draft" | "Review" | "Approved" | "Deprecated";
type ArtifactType   = "Catalog" | "Matrix" | "Diagram";

interface TogafPhaseKey {
  key: string;
  label: string;
  index: number;
}

interface TogafArtifact {
  id: string;
  name: string;
  phase: string;
  artifact_type: ArtifactType;
  description: string;
  content: string;
  status: ArtifactStatus;
  created_at: number;
  updated_at: number;
  tags: string[];
}

interface TogafAdm {
  artifacts: TogafArtifact[];
}

interface ArchSpec {
  project_name: string;
  togaf: TogafAdm;
}

// ── Constants ─────────────────────────────────────────────────────────────

const TOGAF_PHASES: TogafPhaseKey[] = [
  { key: "Preliminary",                  label: "Preliminary",                   index: 1 },
  { key: "ArchitectureVision",           label: "Architecture Vision",           index: 2 },
  { key: "BusinessArchitecture",         label: "Business Architecture",         index: 3 },
  { key: "InformationSystems",           label: "Information Systems",           index: 4 },
  { key: "TechnologyArchitecture",       label: "Technology Architecture",       index: 5 },
  { key: "OpportunitiesAndSolutions",    label: "Opportunities & Solutions",     index: 6 },
  { key: "MigrationPlanning",            label: "Migration Planning",            index: 7 },
  { key: "ImplementationGovernance",     label: "Implementation Governance",     index: 8 },
  { key: "ArchitectureChangeManagement", label: "Change Management",             index: 9 },
];

type NavTab = "togaf" | "zachman" | "c4" | "adr" | "governance";

const STATUS_COLORS: Record<ArtifactStatus, string> = {
  Draft:      "var(--text-secondary)",
  Review:     "#f0a500",
  Approved:   "var(--accent-green, #22c55e)",
  Deprecated: "var(--text-muted, #555)",
};

// ── Props ─────────────────────────────────────────────────────────────────

interface Props {
  workspacePath?: string | null;
}

// ── Component ─────────────────────────────────────────────────────────────

export default function ArchitectureSpecPanel({ workspacePath }: Props) {
  const [tab, setTab]           = useState<NavTab>("togaf");
  const [spec, setSpec]         = useState<ArchSpec | null>(null);
  const [loading, setLoading]   = useState(false);
  const [generating, setGenerating] = useState(false);
  const [error, setError]       = useState("");
  const [expandedPhase, setExpandedPhase] = useState<string | null>(null);
  const [statusBusy, setStatusBusy] = useState<string | null>(null);
  const [report, setReport]     = useState("");
  const [adrTitle, setAdrTitle] = useState("");
  const [adrContext, setAdrContext] = useState("");
  const [adrDecision, setAdrDecision] = useState("");
  const [cliLoading, setCliLoading] = useState(false);
  const [cliError, setCliError] = useState("");

  // ── Load on mount / workspace change ────────────────────────────────────
  const loadSpec = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    setError("");
    try {
      const data = await invoke<ArchSpec>("archspec_load", { workspacePath });
      setSpec(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [workspacePath]);

  useEffect(() => { loadSpec(); }, [loadSpec]);

  // ── Generate from codebase ───────────────────────────────────────────────
  const handleGenerate = async () => {
    if (!workspacePath) { setError("No workspace open."); return; }
    setGenerating(true);
    setError("");
    try {
      const data = await invoke<ArchSpec>("archspec_generate", { workspacePath });
      setSpec(data);
      setExpandedPhase(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  // ── Artifact status update ───────────────────────────────────────────────
  const setArtifactStatus = async (artifactId: string, status: ArtifactStatus) => {
    if (!workspacePath) return;
    setStatusBusy(artifactId);
    try {
      const data = await invoke<ArchSpec>("archspec_set_artifact_status", {
        workspacePath, artifactId, status,
      });
      setSpec(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setStatusBusy(null);
    }
  };

  // ── CLI button (for Zachman, C4, Governance tabs) ────────────────────────
  const runCli = useCallback(async (args: string) => {
    setCliLoading(true);
    setCliError("");
    setReport("");
    try {
      const res = await invoke<string>("handle_archspec_command", { args });
      setReport(res);
    } catch (e) {
      setCliError(String(e));
    } finally {
      setCliLoading(false);
    }
  }, []);

  // ── Derived: artifact counts per phase ───────────────────────────────────
  const artifactsForPhase = (phaseKey: string): TogafArtifact[] =>
    spec?.togaf.artifacts.filter(a => a.phase === phaseKey) ?? [];

  const phaseCompletion = (phaseKey: string): number => {
    const arts = artifactsForPhase(phaseKey);
    if (arts.length === 0) return 0;
    const approved = arts.filter(a => a.status === "Approved").length;
    return Math.round((approved / arts.length) * 100);
  };

  const totalCompletion = (): number => {
    if (!spec || spec.togaf.artifacts.length === 0) return 0;
    const approved = spec.togaf.artifacts.filter(a => a.status === "Approved").length;
    return Math.round((approved / spec.togaf.artifacts.length) * 100);
  };

  // ── Status badge ─────────────────────────────────────────────────────────
  const StatusBadge = ({ status }: { status: ArtifactStatus }) => (
    <span style={{
      fontSize: 10, padding: "2px 7px", borderRadius: 10,
      border: `1px solid ${STATUS_COLORS[status]}`,
      color: STATUS_COLORS[status],
      whiteSpace: "nowrap",
    }}>
      {status}
    </span>
  );

  // ── TOGAF tab ────────────────────────────────────────────────────────────
  const renderTogaf = () => (
    <>
      <div className="panel-card" style={{ marginBottom: 10 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <span style={{ fontWeight: 600 }}>TOGAF ADM Phases</span>
            {spec && (
              <span className="panel-label" style={{ marginLeft: 10 }}>
                {totalCompletion()}% complete · {spec.togaf.artifacts.length} artifacts
              </span>
            )}
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button
              className="panel-btn panel-btn-primary panel-btn-sm"
              onClick={handleGenerate}
              disabled={generating || !workspacePath}
              title={workspacePath ? "Scan codebase and generate draft artifacts" : "Open a workspace first"}
            >
              {generating ? "Generating…" : "⟳ Generate from Codebase"}
            </button>
            <button
              className="panel-btn panel-btn-secondary panel-btn-sm"
              onClick={() => runCli("report")}
              disabled={cliLoading}
            >
              {cliLoading ? "…" : "▶ Full Report"}
            </button>
          </div>
        </div>
      </div>

      {!workspacePath && (
        <div className="panel-card" style={{ color: "var(--text-secondary)", fontSize: 13 }}>
          Open a workspace folder to enable architecture tracking.
        </div>
      )}

      {loading && <div className="panel-label" style={{ padding: 8 }}>Loading…</div>}

      {TOGAF_PHASES.map(phase => {
        const arts = artifactsForPhase(phase.key);
        const pct = phaseCompletion(phase.key);
        const isOpen = expandedPhase === phase.key;

        return (
          <div key={phase.key} style={{ marginBottom: 4 }}>
            {/* Phase row */}
            <div
              style={{
                display: "flex", justifyContent: "space-between", alignItems: "center",
                padding: "6px 0", borderBottom: "1px solid var(--border-color)", cursor: arts.length > 0 ? "pointer" : "default",
              }}
              onClick={() => arts.length > 0 && setExpandedPhase(isOpen ? null : phase.key)}
            >
              <span style={{ fontSize: 13 }}>
                {arts.length > 0 && (
                  <span style={{ marginRight: 6, color: "var(--text-secondary)", fontSize: 10 }}>
                    {isOpen ? "▾" : "▸"}
                  </span>
                )}
                {phase.index}. {phase.label}
              </span>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                {arts.length > 0 && (
                  <div style={{ width: 60, height: 4, background: "var(--bg-tertiary)", borderRadius: 2, overflow: "hidden" }}>
                    <div style={{ width: `${pct}%`, height: "100%", background: pct === 100 ? "var(--accent-green, #22c55e)" : "var(--accent-color, #4f8ef7)", borderRadius: 2 }} />
                  </div>
                )}
                <span className="panel-label" style={{ minWidth: 80, textAlign: "right" }}>
                  {arts.length === 0 ? "0 artifacts" : `${arts.filter(a => a.status === "Approved").length}/${arts.length} approved`}
                </span>
              </div>
            </div>

            {/* Artifact list (expanded) */}
            {isOpen && (
              <div style={{ background: "var(--bg-secondary)", borderRadius: 6, margin: "4px 0 8px 0", padding: "6px 10px" }}>
                {arts.map(art => (
                  <div key={art.id} style={{ padding: "8px 0", borderBottom: "1px solid var(--border-color)" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 8 }}>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 3 }}>
                          <span style={{ fontWeight: 600, fontSize: 12 }}>{art.name}</span>
                          <span className="panel-label" style={{ fontSize: 10 }}>{art.artifact_type}</span>
                          <StatusBadge status={art.status} />
                        </div>
                        <div className="panel-label" style={{ fontSize: 11 }}>{art.description}</div>
                        {art.content && (
                          <pre style={{
                            margin: "6px 0 0", fontSize: 10, color: "var(--text-secondary)",
                            whiteSpace: "pre-wrap", wordBreak: "break-word",
                            background: "var(--bg-tertiary)", padding: "4px 8px", borderRadius: 4,
                            maxHeight: 80, overflow: "hidden",
                          }}>
                            {art.content.slice(0, 300)}{art.content.length > 300 ? "…" : ""}
                          </pre>
                        )}
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", gap: 4, flexShrink: 0 }}>
                        {art.status !== "Approved" && (
                          <button
                            className="panel-btn panel-btn-sm"
                            style={{ fontSize: 10, padding: "2px 8px", color: "var(--accent-green, #22c55e)", borderColor: "var(--accent-green, #22c55e)" }}
                            disabled={statusBusy === art.id}
                            onClick={() => setArtifactStatus(art.id, "Approved")}
                          >
                            ✓ Approve
                          </button>
                        )}
                        {art.status !== "Review" && art.status !== "Approved" && (
                          <button
                            className="panel-btn panel-btn-secondary panel-btn-sm"
                            style={{ fontSize: 10, padding: "2px 8px" }}
                            disabled={statusBusy === art.id}
                            onClick={() => setArtifactStatus(art.id, "Review")}
                          >
                            ⟲ Request Review
                          </button>
                        )}
                        {art.status === "Approved" && (
                          <button
                            className="panel-btn panel-btn-sm"
                            style={{ fontSize: 10, padding: "2px 8px", color: "#f87171", borderColor: "#f87171" }}
                            disabled={statusBusy === art.id}
                            onClick={() => setArtifactStatus(art.id, "Draft")}
                          >
                            ✕ Revoke
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}

      {error && <div className="panel-error" style={{ marginTop: 8 }}>{error}</div>}
      {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
      {report && (
        <div className="panel-card" style={{ marginTop: 8 }}>
          <pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre>
        </div>
      )}
    </>
  );

  // ── Zachman tab ──────────────────────────────────────────────────────────
  const zachmanPerspectives = ["Planner", "Owner", "Designer", "Builder", "Implementer", "Worker"];
  const zachmanAspects      = ["What", "How", "Where", "Who", "When", "Why"];
  const cellStyle = (filled: boolean): React.CSSProperties => ({
    padding: 6, fontSize: 10, textAlign: "center",
    border: "1px solid var(--border-color)",
    background: filled ? "var(--bg-secondary)" : "var(--bg-tertiary)",
    minWidth: 80,
    color: filled ? "var(--text-primary)" : "var(--text-secondary)",
  });

  const renderZachman = () => (
    <>
      <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 8 }}>
        <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("zachman")} disabled={cliLoading}>
          {cliLoading ? "…" : "▶ /archspec zachman"}
        </button>
      </div>
      <div style={{ overflowX: "auto" }}>
        <table style={{ borderCollapse: "collapse", width: "100%", fontSize: 11 }}>
          <thead>
            <tr>
              <th style={{ ...cellStyle(true), fontWeight: 600 }} />
              {zachmanAspects.map(a => <th key={a} style={{ ...cellStyle(true), fontWeight: 600 }}>{a}</th>)}
            </tr>
          </thead>
          <tbody>
            {zachmanPerspectives.map(p => (
              <tr key={p}>
                <td style={{ ...cellStyle(true), fontWeight: 600 }}>{p}</td>
                {zachmanAspects.map(a => <td key={a} style={cellStyle(false)}>—</td>)}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
      {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
    </>
  );

  // ── C4 tab ───────────────────────────────────────────────────────────────
  const renderC4 = () => (
    <div className="panel-card">
      <div style={{ fontWeight: 600, marginBottom: 8 }}>C4 Model Levels</div>
      {(["context", "container", "component"] as const).map(level => (
        <div key={level} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 0", borderBottom: "1px solid var(--border-color)" }}>
          <div>
            <div style={{ fontWeight: 600, textTransform: "capitalize" }}>{level}</div>
            <div className="panel-label">Generate {level} diagram</div>
          </div>
          <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli(`c4 ${level}`)} disabled={cliLoading}>
            {cliLoading ? "…" : `▶ ${level}`}
          </button>
        </div>
      ))}
      {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
      {report && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: 11 }}>{report}</pre>}
    </div>
  );

  // ── ADRs tab ─────────────────────────────────────────────────────────────
  const renderAdr = () => (
    <>
      <div className="panel-card">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <span style={{ fontWeight: 600 }}>Architecture Decision Records</span>
          <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("adr list")} disabled={cliLoading}>
            {cliLoading ? "…" : "▶ List ADRs"}
          </button>
        </div>
        <div className="panel-label">Title</div>
        <input value={adrTitle} onChange={e => setAdrTitle(e.target.value)} className="panel-input panel-input-full" style={{ marginBottom: 8 }} placeholder="Use PostgreSQL for primary database" />
        <div className="panel-label">Context</div>
        <textarea value={adrContext} onChange={e => setAdrContext(e.target.value)} rows={3} className="panel-input panel-input-full" style={{ marginBottom: 8, resize: "vertical" }} placeholder="We need a reliable RDBMS that supports…" />
        <div className="panel-label">Decision</div>
        <textarea value={adrDecision} onChange={e => setAdrDecision(e.target.value)} rows={3} className="panel-input panel-input-full" style={{ marginBottom: 8, resize: "vertical" }} placeholder="We will use PostgreSQL because…" />
        <div style={{ display: "flex", gap: 8 }}>
          <button className="panel-btn panel-btn-primary" disabled={!adrTitle || cliLoading}>Create ADR</button>
          <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("adr list")} disabled={cliLoading}>
            {cliLoading ? "…" : "▶ Refresh List"}
          </button>
        </div>
      </div>
      {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
      {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
    </>
  );

  // ── Governance tab ───────────────────────────────────────────────────────
  const renderGovernance = () => (
    <div className="panel-card">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
        <span style={{ fontWeight: 600 }}>Architecture Governance</span>
        <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("report")} disabled={cliLoading}>
          {cliLoading ? "…" : "▶ Full Report"}
        </button>
      </div>
      <div className="panel-label">Governance rules validate architecture decisions against organizational standards.</div>
      <div style={{ marginTop: 8 }}>
        <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("report")} disabled={cliLoading}>
          {cliLoading ? "…" : "▶ Run Governance Check"}
        </button>
      </div>
      {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
      {report && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: 11 }}>{report}</pre>}
    </div>
  );

  // ── Shell ────────────────────────────────────────────────────────────────
  return (
    <div className="panel-container">
      <div className="panel-header">
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>
          Architecture Specification
          {spec && <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{spec.project_name}</span>}
        </h2>
      </div>

      <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
        {(["togaf", "zachman", "c4", "adr", "governance"] as NavTab[]).map(t => (
          <button
            key={t}
            className={`panel-tab ${tab === t ? "active" : ""}`}
            onClick={() => { setTab(t); setReport(""); setCliError(""); }}
          >
            {t === "togaf" ? "TOGAF ADM" : t === "zachman" ? "Zachman" : t === "c4" ? "C4 Model" : t === "adr" ? "ADRs" : "Governance"}
          </button>
        ))}
      </div>

      {tab === "togaf"      && renderTogaf()}
      {tab === "zachman"    && renderZachman()}
      {tab === "c4"         && renderC4()}
      {tab === "adr"        && renderAdr()}
      {tab === "governance" && renderGovernance()}
    </div>
  );
}
