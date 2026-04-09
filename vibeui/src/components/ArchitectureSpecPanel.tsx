/**
 * ArchitectureSpecPanel — Enterprise Architecture dashboard.
 *
 * Workflow:
 *  1. "Generate from Codebase" scans the workspace → draft artifacts for all 5 views.
 *  2. TOGAF: expand each phase → Approve / Request Review / Revoke per artifact.
 *  3. Zachman: 6×6 matrix with generated cell content, maturity colour coding.
 *  4. C4 Model: element list (Person / System / Container / Component) + relationships.
 *  5. ADRs: generated decision records + create-your-own form; Accept / Deprecate.
 *  6. Governance: rule list with severity badges; run compliance check via CLI.
 *  7. All data persisted to WorkspaceStore (<workspace>/.vibecli/workspace.db).
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────

type ArtifactStatus = "Draft" | "Review" | "Approved" | "Deprecated";
type ArtifactType   = "Catalog" | "Matrix" | "Diagram";
type AdrStatusStr   = "Proposed" | "Accepted" | "Deprecated" | string;
type GovernanceSev  = "Info" | "Warning" | "Error" | "Critical";

interface TogafArtifact {
  id: string; name: string; phase: string;
  artifact_type: ArtifactType; description: string; content: string;
  status: ArtifactStatus; created_at: number; updated_at: number; tags: string[];
}

interface ZachmanCell {
  perspective: string; aspect: string;
  content: string; artifacts: string[]; maturity: number;
}

interface C4Element {
  id: string; name: string; element_type: string;
  description: string; technology: string; parent_id: string | null; tags: string[];
}

interface C4Relationship {
  source_id: string; target_id: string; description: string; technology: string;
}

interface Adr {
  id: string; title: string; status: AdrStatusStr; date: string;
  context: string; decision: string; consequences: string[];
  participants: string[]; tags: string[];
}

interface GovernanceRule {
  id: string; name: string; description: string;
  check_fn_description: string; severity: GovernanceSev; category: string;
}

interface ArchSpec {
  project_name: string;
  togaf:      { artifacts: TogafArtifact[] };
  zachman:    { cells: Record<string, ZachmanCell> };
  c4:         { system_name: string; elements: C4Element[]; relationships: C4Relationship[] };
  adrs:       { records: Adr[] };
  governance: { rules: GovernanceRule[] };
}

// ── Constants ─────────────────────────────────────────────────────────────

const TOGAF_PHASES = [
  { key: "Preliminary",                  label: "Preliminary",               index: 1 },
  { key: "ArchitectureVision",           label: "Architecture Vision",       index: 2 },
  { key: "BusinessArchitecture",         label: "Business Architecture",     index: 3 },
  { key: "InformationSystems",           label: "Information Systems",       index: 4 },
  { key: "TechnologyArchitecture",       label: "Technology Architecture",   index: 5 },
  { key: "OpportunitiesAndSolutions",    label: "Opportunities & Solutions", index: 6 },
  { key: "MigrationPlanning",            label: "Migration Planning",        index: 7 },
  { key: "ImplementationGovernance",     label: "Implementation Governance", index: 8 },
  { key: "ArchitectureChangeManagement", label: "Change Management",         index: 9 },
];

const ZACHMAN_PERSPECTIVES = ["Planner", "Owner", "Designer", "Builder", "Implementer", "Worker"];
const ZACHMAN_ASPECTS       = ["What", "How", "Where", "Who", "When", "Why"];

type NavTab = "togaf" | "zachman" | "c4" | "adr" | "governance";

const ARTIFACT_STATUS_COLOR: Record<ArtifactStatus, string> = {
  Draft: "var(--text-secondary)", Review: "#f0a500",
  Approved: "var(--accent-green,#22c55e)", Deprecated: "#555",
};

const ADR_STATUS_COLOR: Record<string, string> = {
  Proposed: "#f0a500", Accepted: "var(--accent-green,#22c55e)", Deprecated: "#555",
};

const SEV_COLOR: Record<GovernanceSev, string> = {
  Info: "var(--text-secondary)", Warning: "#f0a500", Error: "#f87171", Critical: "#dc2626",
};

const MATURITY_COLOR = ["#333", "#4f4","#3b3","#2a2","#1a1","#0f0"];

// ── Props ─────────────────────────────────────────────────────────────────

interface Props { workspacePath?: string | null; }

// ── Component ─────────────────────────────────────────────────────────────

export default function ArchitectureSpecPanel({ workspacePath }: Props) {
  const [tab, setTab]               = useState<NavTab>("togaf");
  const [spec, setSpec]             = useState<ArchSpec | null>(null);
  const [loading, setLoading]       = useState(false);
  const [generating, setGenerating] = useState(false);
  const [error, setError]           = useState("");
  const [expandedPhase, setExpandedPhase]   = useState<string | null>(null);
  const [statusBusy, setStatusBusy]         = useState<string | null>(null);
  const [expandedC4, setExpandedC4]         = useState<string | null>(null);

  // ADR form state
  const [adrTitle, setAdrTitle]         = useState("");
  const [adrContext, setAdrContext]     = useState("");
  const [adrDecision, setAdrDecision]   = useState("");
  const [adrConseq, setAdrConseq]       = useState("");
  const [adrBusy, setAdrBusy]           = useState(false);

  // CLI report state (Zachman / C4 / Governance tabs)
  const [cliLoading, setCliLoading] = useState(false);
  const [cliError, setCliError]     = useState("");
  const [report, setReport]         = useState("");

  // ── Load ─────────────────────────────────────────────────────────────────
  const loadSpec = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true); setError("");
    try { setSpec(await invoke<ArchSpec>("archspec_load", { workspacePath })); }
    catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  }, [workspacePath]);

  useEffect(() => { loadSpec(); }, [loadSpec]);

  // ── Generate ─────────────────────────────────────────────────────────────
  const handleGenerate = async () => {
    if (!workspacePath) { setError("No workspace open."); return; }
    setGenerating(true); setError("");
    try {
      setSpec(await invoke<ArchSpec>("archspec_generate", { workspacePath }));
      setExpandedPhase(null);
    } catch (e) { setError(String(e)); }
    finally { setGenerating(false); }
  };

  // ── TOGAF artifact status ─────────────────────────────────────────────────
  const setArtifactStatus = async (artifactId: string, status: ArtifactStatus) => {
    if (!workspacePath) return;
    setStatusBusy(artifactId);
    try { setSpec(await invoke<ArchSpec>("archspec_set_artifact_status", { workspacePath, artifactId, status })); }
    catch (e) { setError(String(e)); }
    finally { setStatusBusy(null); }
  };

  // ── ADR status ────────────────────────────────────────────────────────────
  const setAdrStatus = async (adrId: string, status: string) => {
    if (!workspacePath) return;
    setStatusBusy(adrId);
    try { setSpec(await invoke<ArchSpec>("archspec_set_adr_status", { workspacePath, adrId, status })); }
    catch (e) { setError(String(e)); }
    finally { setStatusBusy(null); }
  };

  // ── Create ADR ────────────────────────────────────────────────────────────
  const createAdr = async () => {
    if (!workspacePath || !adrTitle) return;
    setAdrBusy(true);
    try {
      const consequences = adrConseq.split("\n").map(s => s.trim()).filter(Boolean);
      setSpec(await invoke<ArchSpec>("archspec_create_adr", {
        workspacePath, title: adrTitle, context: adrContext,
        decision: adrDecision, consequences, tags: [],
      }));
      setAdrTitle(""); setAdrContext(""); setAdrDecision(""); setAdrConseq("");
    } catch (e) { setError(String(e)); }
    finally { setAdrBusy(false); }
  };

  // ── CLI run ───────────────────────────────────────────────────────────────
  const runCli = useCallback(async (args: string) => {
    setCliLoading(true); setCliError(""); setReport("");
    try { setReport(await invoke<string>("handle_archspec_command", { args })); }
    catch (e) { setCliError(String(e)); }
    finally { setCliLoading(false); }
  }, []);

  // ── Derived ───────────────────────────────────────────────────────────────
  const artsFor = (phase: string) => spec?.togaf.artifacts.filter(a => a.phase === phase) ?? [];
  const phasePct = (phase: string) => {
    const a = artsFor(phase); if (!a.length) return 0;
    return Math.round(a.filter(x => x.status === "Approved").length / a.length * 100);
  };
  const totalPct = () => {
    const all = spec?.togaf.artifacts ?? []; if (!all.length) return 0;
    return Math.round(all.filter(a => a.status === "Approved").length / all.length * 100);
  };

  // ── Shared mini-components ────────────────────────────────────────────────
  const Badge = ({ label, color }: { label: string; color: string }) => (
    <span style={{ fontSize: 10, padding: "2px 7px", borderRadius: 10, border: `1px solid ${color}`, color, whiteSpace: "nowrap" }}>
      {label}
    </span>
  );

  const ProgBar = ({ pct, total }: { pct: number; total: number }) => (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <div style={{ width: 60, height: 4, background: "var(--bg-tertiary)", borderRadius: 2, overflow: "hidden" }}>
        <div style={{ width: `${pct}%`, height: "100%", borderRadius: 2, background: pct === 100 ? "var(--accent-green,#22c55e)" : "var(--accent-color,#4f8ef7)" }} />
      </div>
      <span className="panel-label" style={{ fontSize: 10 }}>{total === 0 ? "0 artifacts" : `${Math.round(pct / 100 * total)}/${total} approved`}</span>
    </div>
  );

  const GenerateBar = () => (
    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
      <button className="panel-btn panel-btn-primary panel-btn-sm" onClick={handleGenerate}
        disabled={generating || !workspacePath} title={workspacePath ? "Scan codebase and generate draft artifacts" : "Open a workspace first"}>
        {generating ? "Generating…" : "⟳ Generate from Codebase"}
      </button>
    </div>
  );

  // ── TOGAF tab ─────────────────────────────────────────────────────────────
  const renderTogaf = () => (
    <>
      <div className="panel-card" style={{ marginBottom: 10 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <span style={{ fontWeight: 600 }}>TOGAF ADM Phases</span>
            {spec && <span className="panel-label" style={{ marginLeft: 10 }}>
              {totalPct()}% · {spec.togaf.artifacts.length} artifacts
            </span>}
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <GenerateBar />
            <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("report")} disabled={cliLoading}>
              {cliLoading ? "…" : "▶ Full Report"}
            </button>
          </div>
        </div>
      </div>

      {!workspacePath && <div className="panel-card" style={{ color: "var(--text-secondary)", fontSize: 13 }}>Open a workspace folder to enable architecture tracking.</div>}
      {loading && <div className="panel-label" style={{ padding: 8 }}>Loading…</div>}

      {TOGAF_PHASES.map(phase => {
        const arts = artsFor(phase.key);
        const pct  = phasePct(phase.key);
        const open = expandedPhase === phase.key;
        return (
          <div key={phase.key} style={{ marginBottom: 4 }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center",
              padding: "6px 0", borderBottom: "1px solid var(--border-color)", cursor: arts.length ? "pointer" : "default" }}
              onClick={() => arts.length && setExpandedPhase(open ? null : phase.key)}>
              <span style={{ fontSize: 13 }}>
                {arts.length > 0 && <span style={{ marginRight: 6, color: "var(--text-secondary)", fontSize: 10 }}>{open ? "▾" : "▸"}</span>}
                {phase.index}. {phase.label}
              </span>
              <ProgBar pct={pct} total={arts.length} />
            </div>
            {open && (
              <div style={{ background: "var(--bg-secondary)", borderRadius: 6, margin: "4px 0 8px", padding: "6px 10px" }}>
                {arts.map(art => (
                  <div key={art.id} style={{ padding: "8px 0", borderBottom: "1px solid var(--border-color)" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 8 }}>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 3 }}>
                          <span style={{ fontWeight: 600, fontSize: 12 }}>{art.name}</span>
                          <Badge label={art.artifact_type} color="var(--text-secondary)" />
                          <Badge label={art.status} color={ARTIFACT_STATUS_COLOR[art.status]} />
                        </div>
                        <div className="panel-label" style={{ fontSize: 11 }}>{art.description}</div>
                        {art.content && (
                          <pre style={{ margin: "6px 0 0", fontSize: 10, color: "var(--text-secondary)", whiteSpace: "pre-wrap",
                            wordBreak: "break-word", background: "var(--bg-tertiary)", padding: "4px 8px", borderRadius: 4,
                            maxHeight: 80, overflow: "hidden" }}>
                            {art.content.slice(0, 300)}{art.content.length > 300 ? "…" : ""}
                          </pre>
                        )}
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", gap: 4, flexShrink: 0 }}>
                        {art.status !== "Approved" && (
                          <button className="panel-btn panel-btn-sm" style={{ fontSize: 10, padding: "2px 8px", color: "var(--accent-green,#22c55e)", borderColor: "var(--accent-green,#22c55e)" }}
                            disabled={statusBusy === art.id} onClick={() => setArtifactStatus(art.id, "Approved")}>✓ Approve</button>
                        )}
                        {art.status !== "Review" && art.status !== "Approved" && (
                          <button className="panel-btn panel-btn-secondary panel-btn-sm" style={{ fontSize: 10, padding: "2px 8px" }}
                            disabled={statusBusy === art.id} onClick={() => setArtifactStatus(art.id, "Review")}>⟲ Review</button>
                        )}
                        {art.status === "Approved" && (
                          <button className="panel-btn panel-btn-sm" style={{ fontSize: 10, padding: "2px 8px", color: "#f87171", borderColor: "#f87171" }}
                            disabled={statusBusy === art.id} onClick={() => setArtifactStatus(art.id, "Draft")}>✕ Revoke</button>
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
      {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
    </>
  );

  // ── Zachman tab ───────────────────────────────────────────────────────────
  const renderZachman = () => {
    const cells = spec?.zachman.cells ?? {};
    const getCell = (p: string, a: string): ZachmanCell | null => {
      const pi = ZACHMAN_PERSPECTIVES.indexOf(p);
      const ai = ZACHMAN_ASPECTS.indexOf(a);
      return cells[`${pi}:${ai}`] ?? null;
    };
    const hasData = Object.keys(cells).length > 0;
    return (
      <>
        <div className="panel-card" style={{ marginBottom: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontWeight: 600 }}>
              Zachman Framework 6×6
              {hasData && <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{Object.keys(cells).length}/36 cells filled</span>}
            </span>
            <div style={{ display: "flex", gap: 8 }}>
              {!hasData && <GenerateBar />}
              <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("zachman")} disabled={cliLoading}>
                {cliLoading ? "…" : "▶ CLI Report"}
              </button>
            </div>
          </div>
        </div>

        {!hasData && !spec && <div className="panel-label" style={{ padding: 8 }}>Run "Generate from Codebase" to populate the Zachman matrix.</div>}

        {hasData && (
          <div style={{ overflowX: "auto" }}>
            <table style={{ borderCollapse: "collapse", width: "100%", fontSize: 11, tableLayout: "fixed" }}>
              <colgroup>
                <col style={{ width: 90 }} />
                {ZACHMAN_ASPECTS.map(a => <col key={a} />)}
              </colgroup>
              <thead>
                <tr>
                  <th style={{ padding: "6px 8px", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", textAlign: "left" }} />
                  {ZACHMAN_ASPECTS.map(a => (
                    <th key={a} style={{ padding: "6px 8px", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: 10, fontWeight: 600, textAlign: "center" }}>
                      {a}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {ZACHMAN_PERSPECTIVES.map(p => (
                  <tr key={p}>
                    <td style={{ padding: "6px 8px", border: "1px solid var(--border-color)", fontWeight: 600, background: "var(--bg-secondary)", whiteSpace: "nowrap" }}>{p}</td>
                    {ZACHMAN_ASPECTS.map(a => {
                      const cell = getCell(p, a);
                      const maturity = cell?.maturity ?? 0;
                      return (
                        <td key={a} title={cell?.content ?? ""} style={{
                          padding: "5px 7px", border: "1px solid var(--border-color)",
                          background: cell?.content ? "var(--bg-secondary)" : "var(--bg-tertiary)",
                          verticalAlign: "top", maxWidth: 0,
                        }}>
                          {cell?.content ? (
                            <>
                              <div style={{ fontSize: 10, color: "var(--text-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                {cell.content.slice(0, 60)}{cell.content.length > 60 ? "…" : ""}
                              </div>
                              {maturity > 0 && (
                                <div style={{ marginTop: 3, display: "flex", gap: 2 }}>
                                  {[1,2,3,4,5].map(i => (
                                    <div key={i} style={{ width: 6, height: 6, borderRadius: 1, background: i <= maturity ? MATURITY_COLOR[maturity] : "var(--bg-tertiary)" }} />
                                  ))}
                                </div>
                              )}
                            </>
                          ) : <span style={{ color: "var(--text-secondary)", fontSize: 10 }}>—</span>}
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="panel-label" style={{ marginTop: 6, fontSize: 10 }}>
              Hover cells for full content. Maturity dots: 1=Initial → 5=Optimised.
            </div>
          </div>
        )}

        {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
        {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
      </>
    );
  };

  // ── C4 Model tab ──────────────────────────────────────────────────────────
  const renderC4 = () => {
    const els  = spec?.c4.elements ?? [];
    const rels = spec?.c4.relationships ?? [];
    const hasData = els.length > 0;
    const byType = (t: string) => els.filter(e => e.element_type === t);
    const nameOf = (id: string) => els.find(e => e.id === id)?.name ?? id;

    const TypeSection = ({ type, label }: { type: string; label: string }) => {
      const items = byType(type);
      if (!items.length) return null;
      return (
        <div style={{ marginBottom: 12 }}>
          <div style={{ fontWeight: 600, fontSize: 12, marginBottom: 6, color: "var(--text-secondary)" }}>{label}</div>
          {items.map(el => (
            <div key={el.id} style={{ padding: "6px 0", borderBottom: "1px solid var(--border-color)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", cursor: "pointer" }}
                onClick={() => setExpandedC4(expandedC4 === el.id ? null : el.id)}>
                <div>
                  <span style={{ fontWeight: 600, fontSize: 12 }}>{el.name}</span>
                  {el.technology && <span className="panel-label" style={{ marginLeft: 8, fontSize: 10 }}>{el.technology}</span>}
                </div>
                <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>{expandedC4 === el.id ? "▾" : "▸"}</span>
              </div>
              {expandedC4 === el.id && (
                <div style={{ marginTop: 4, fontSize: 11, color: "var(--text-secondary)" }}>
                  <div style={{ marginBottom: 4 }}>{el.description}</div>
                  {rels.filter(r => r.source_id === el.id || r.target_id === el.id).map((r, i) => (
                    <div key={i} style={{ fontSize: 10, padding: "2px 0" }}>
                      {r.source_id === el.id
                        ? <span>→ <b>{nameOf(r.target_id)}</b>: {r.description}{r.technology ? ` [${r.technology}]` : ""}</span>
                        : <span>← <b>{nameOf(r.source_id)}</b>: {r.description}</span>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      );
    };

    return (
      <>
        <div className="panel-card" style={{ marginBottom: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontWeight: 600 }}>
              C4 Model
              {hasData && <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{els.length} elements · {rels.length} relationships</span>}
            </span>
            <div style={{ display: "flex", gap: 8 }}>
              {!hasData && <GenerateBar />}
              <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("c4 context")} disabled={cliLoading}>
                {cliLoading ? "…" : "▶ Context Diagram"}
              </button>
              <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("c4 container")} disabled={cliLoading}>
                {cliLoading ? "…" : "▶ Container Diagram"}
              </button>
            </div>
          </div>
        </div>

        {!hasData && !spec && <div className="panel-label" style={{ padding: 8 }}>Run "Generate from Codebase" to populate the C4 model.</div>}

        {hasData && (
          <div className="panel-card">
            <TypeSection type="Person"         label="People" />
            <TypeSection type="SoftwareSystem" label="Software Systems" />
            <TypeSection type="Container"      label="Containers" />
            <TypeSection type="Component"      label="Components" />
          </div>
        )}

        {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
        {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
      </>
    );
  };

  // ── ADRs tab ──────────────────────────────────────────────────────────────
  const renderAdr = () => {
    const adrs = spec?.adrs.records ?? [];
    const adrStatusLabel = (s: AdrStatusStr) =>
      typeof s === "string" ? s : Object.keys(s)[0];

    return (
      <>
        {/* Generated ADR list */}
        {adrs.length > 0 && (
          <div className="panel-card" style={{ marginBottom: 10 }}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>
              Architecture Decision Records
              <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{adrs.length} total</span>
            </div>
            {adrs.map(adr => {
              const statusLabel = adrStatusLabel(adr.status);
              return (
                <div key={adr.id} style={{ padding: "10px 0", borderBottom: "1px solid var(--border-color)" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 8 }}>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                        <span style={{ fontWeight: 600, fontSize: 12 }}>{adr.title}</span>
                        <Badge label={statusLabel} color={ADR_STATUS_COLOR[statusLabel] ?? "var(--text-secondary)"} />
                        {adr.date && <span className="panel-label" style={{ fontSize: 10 }}>{adr.date}</span>}
                      </div>
                      <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
                        <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>Context: </span>{adr.context}
                      </div>
                      <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
                        <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>Decision: </span>{adr.decision}
                      </div>
                      {adr.consequences.length > 0 && (
                        <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>
                          {adr.consequences.map((c, i) => <div key={i}>• {c}</div>)}
                        </div>
                      )}
                      {adr.tags.length > 0 && (
                        <div style={{ marginTop: 4, display: "flex", gap: 4, flexWrap: "wrap" }}>
                          {adr.tags.map(t => <span key={t} className="panel-label" style={{ fontSize: 9, padding: "1px 5px", border: "1px solid var(--border-color)", borderRadius: 8 }}>{t}</span>)}
                        </div>
                      )}
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4, flexShrink: 0 }}>
                      {statusLabel !== "Accepted" && (
                        <button className="panel-btn panel-btn-sm" style={{ fontSize: 10, padding: "2px 8px", color: "var(--accent-green,#22c55e)", borderColor: "var(--accent-green,#22c55e)" }}
                          disabled={statusBusy === adr.id} onClick={() => setAdrStatus(adr.id, "Accepted")}>✓ Accept</button>
                      )}
                      {statusLabel !== "Deprecated" && statusLabel !== "Accepted" && (
                        <button className="panel-btn panel-btn-secondary panel-btn-sm" style={{ fontSize: 10, padding: "2px 8px" }}
                          disabled={statusBusy === adr.id} onClick={() => setAdrStatus(adr.id, "Deprecated")}>✕ Deprecate</button>
                      )}
                      {statusLabel === "Accepted" && (
                        <button className="panel-btn panel-btn-sm" style={{ fontSize: 10, padding: "2px 8px", color: "#f87171", borderColor: "#f87171" }}
                          disabled={statusBusy === adr.id} onClick={() => setAdrStatus(adr.id, "Proposed")}>↩ Re-open</button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* Create ADR form */}
        <div className="panel-card">
          <div style={{ fontWeight: 600, marginBottom: 10 }}>Create New ADR</div>
          <div className="panel-label">Title *</div>
          <input value={adrTitle} onChange={e => setAdrTitle(e.target.value)} className="panel-input panel-input-full" style={{ marginBottom: 8 }} placeholder="Use PostgreSQL for primary database" />
          <div className="panel-label">Context</div>
          <textarea value={adrContext} onChange={e => setAdrContext(e.target.value)} rows={2} className="panel-input panel-input-full" style={{ marginBottom: 8, resize: "vertical" }} placeholder="What is the situation that motivated this decision?" />
          <div className="panel-label">Decision</div>
          <textarea value={adrDecision} onChange={e => setAdrDecision(e.target.value)} rows={2} className="panel-input panel-input-full" style={{ marginBottom: 8, resize: "vertical" }} placeholder="What was decided?" />
          <div className="panel-label">Consequences (one per line)</div>
          <textarea value={adrConseq} onChange={e => setAdrConseq(e.target.value)} rows={3} className="panel-input panel-input-full" style={{ marginBottom: 10, resize: "vertical" }} placeholder="Pro: …&#10;Con: …" />
          <button className="panel-btn panel-btn-primary" disabled={!adrTitle || adrBusy || !workspacePath} onClick={createAdr}>
            {adrBusy ? "Saving…" : "Create ADR"}
          </button>
          {!workspacePath && <span className="panel-label" style={{ marginLeft: 10, fontSize: 11 }}>Open a workspace to save ADRs.</span>}
        </div>

        {error && <div className="panel-error" style={{ marginTop: 8 }}>{error}</div>}
      </>
    );
  };

  // ── Governance tab ────────────────────────────────────────────────────────
  const renderGovernance = () => {
    const rules = spec?.governance.rules ?? [];
    const hasData = rules.length > 0;
    const byCategory = rules.reduce<Record<string, GovernanceRule[]>>((acc, r) => {
      (acc[r.category] ??= []).push(r); return acc;
    }, {});

    return (
      <>
        <div className="panel-card" style={{ marginBottom: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontWeight: 600 }}>
              Architecture Governance Rules
              {hasData && <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{rules.length} rules</span>}
            </span>
            <div style={{ display: "flex", gap: 8 }}>
              {!hasData && <GenerateBar />}
              <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCli("report")} disabled={cliLoading}>
                {cliLoading ? "…" : "▶ Compliance Check"}
              </button>
            </div>
          </div>
        </div>

        {!hasData && !spec && <div className="panel-label" style={{ padding: 8 }}>Run "Generate from Codebase" to populate governance rules.</div>}

        {hasData && Object.entries(byCategory).map(([cat, catRules]) => (
          <div key={cat} style={{ marginBottom: 12 }}>
            <div style={{ fontWeight: 600, fontSize: 11, textTransform: "uppercase", letterSpacing: 1, color: "var(--text-secondary)", marginBottom: 6 }}>{cat}</div>
            {catRules.map(rule => (
              <div key={rule.id} className="panel-card" style={{ marginBottom: 6, padding: "8px 10px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                  <Badge label={rule.severity} color={SEV_COLOR[rule.severity]} />
                  <span style={{ fontWeight: 600, fontSize: 12 }}>{rule.name}</span>
                  <span className="panel-label" style={{ fontSize: 10 }}>{rule.id}</span>
                </div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: rule.check_fn_description ? 4 : 0 }}>{rule.description}</div>
                {rule.check_fn_description && (
                  <div style={{ fontSize: 10, color: "var(--text-secondary)", fontStyle: "italic" }}>Check: {rule.check_fn_description}</div>
                )}
              </div>
            ))}
          </div>
        ))}

        {cliError && <div className="panel-error" style={{ marginTop: 8 }}>{cliError}</div>}
        {report && <div className="panel-card" style={{ marginTop: 8 }}><pre style={{ whiteSpace: "pre-wrap", margin: 0, fontSize: 11 }}>{report}</pre></div>}
      </>
    );
  };

  // ── Shell ─────────────────────────────────────────────────────────────────
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
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`}
            onClick={() => { setTab(t); setReport(""); setCliError(""); }}>
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
