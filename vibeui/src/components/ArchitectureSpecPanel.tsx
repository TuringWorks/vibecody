/**
 * ArchitectureSpecPanel — Enterprise Architecture dashboard.
 *
 * Workflow:
 *  1. "Generate from Codebase" scans the workspace → draft artifacts for all 5 views.
 *  2. TOGAF: expand each phase → edit content inline → Approve / Review / Revoke.
 *  3. Zachman: 6×6 scrollable matrix; click any cell to edit its content.
 *  4. C4 Model: element list with inline description/technology editing.
 *  5. ADRs: inline editing of context / decision / consequences + create form.
 *  6. Governance: inline description editing; generate compliance report.
 *  7. All report buttons generate content from loaded spec (no CLI required).
 *  8. All data persisted to WorkspaceStore (<workspace>/.vibecli/workspace.db).
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  RefreshCw, Save, FileText, Table2, Network, Layers, ShieldCheck,
  Check, RotateCcw, X, ChevronDown, ChevronRight, Circle, Trash2,
} from "lucide-react";

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
  last_scanned_at?: number;
  scan_count?: number;
  togaf:      { artifacts: TogafArtifact[] };
  zachman:    { cells: Record<string, ZachmanCell> };
  c4:         { system_name: string; elements: C4Element[]; relationships: C4Relationship[] };
  adrs:       { records: Adr[] };
  governance: { rules: GovernanceRule[] };
}

interface ScanEntry {
  timestamp: number;
  project_name: string;
  scan_count: number;
  togaf_artifacts: number;
  c4_elements: number;
  adr_count: number;
  governance_rules: number;
  current?: boolean;
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
  Draft: "var(--text-secondary)", Review: "var(--warning-color)",
  Approved: "var(--accent-green)", Deprecated: "var(--text-tertiary)",
};

const ADR_STATUS_COLOR: Record<string, string> = {
  Proposed: "var(--warning-color)", Accepted: "var(--accent-green)", Deprecated: "var(--text-tertiary)",
};

const SEV_COLOR: Record<GovernanceSev, string> = {
  Info: "var(--text-secondary)", Warning: "var(--warning-color)", Error: "var(--error-color)", Critical: "var(--error-color)",
};

/* Maturity gradient: empty → full, using semantic tokens */
const MATURITY_COLOR = [
  "var(--bg-tertiary)",
  "color-mix(in srgb, var(--accent-green) 30%, transparent)",
  "color-mix(in srgb, var(--accent-green) 50%, transparent)",
  "color-mix(in srgb, var(--accent-green) 70%, transparent)",
  "color-mix(in srgb, var(--accent-green) 85%, transparent)",
  "var(--accent-green)",
];

// ── Props ─────────────────────────────────────────────────────────────────

interface Props { workspacePath?: string | null; }

// ── Component ─────────────────────────────────────────────────────────────

export default function ArchitectureSpecPanel({ workspacePath }: Props) {
  const [tab, setTab]               = useState<NavTab>("togaf");
  const [spec, setSpec]             = useState<ArchSpec | null>(null);
  const [localSpec, setLocalSpec]   = useState<ArchSpec | null>(null); // editable copy
  const [dirty, setDirty]           = useState(false);
  const [loading, setLoading]       = useState(false);
  const [generating, setGenerating] = useState(false);
  const [saving, setSaving]         = useState(false);
  const [error, setError]           = useState("");
  const [expandedPhase, setExpandedPhase] = useState<string | null>(null);
  const [statusBusy, setStatusBusy]       = useState<string | null>(null);
  const [expandedC4, setExpandedC4]       = useState<string | null>(null);
  const [editingCell, setEditingCell]     = useState<string | null>(null); // "pi:ai"

  // ADR form state
  const [adrTitle, setAdrTitle]       = useState("");
  const [adrContext, setAdrContext]   = useState("");
  const [adrDecision, setAdrDecision] = useState("");
  const [adrConseq, setAdrConseq]     = useState("");
  const [adrBusy, setAdrBusy]         = useState(false);

  // Scan history
  const [scanHistory, setScanHistory] = useState<ScanEntry[]>([]);
  const [showHistory, setShowHistory] = useState(false);

  // Report pane (editable text output)
  const [report, setReport]       = useState("");
  const [reportLabel, setReportLabel] = useState("");

  // ── Load ─────────────────────────────────────────────────────────────────
  const loadSpec = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true); setError("");
    try {
      const s = await invoke<ArchSpec>("archspec_load", { workspacePath });
      setSpec(s); setLocalSpec(s); setDirty(false);
    } catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  }, [workspacePath]);

  useEffect(() => { loadSpec(); }, [loadSpec]);

  // ── Apply remote spec update ──────────────────────────────────────────────
  const applySpec = (s: ArchSpec) => { setSpec(s); setLocalSpec(s); setDirty(false); };

  // ── Load scan history ─────────────────────────────────────────────────────
  const loadScanHistory = useCallback(async () => {
    if (!workspacePath) return;
    try {
      const scans = await invoke<ScanEntry[]>("archspec_list_scans", { workspacePath });
      setScanHistory(scans);
    } catch { /* ignore — history is optional */ }
  }, [workspacePath]);

  useEffect(() => { loadScanHistory(); }, [loadScanHistory]);

  const loadHistoricalScan = async (timestamp: number) => {
    if (!workspacePath) return;
    try {
      const s = await invoke<ArchSpec>("archspec_load_scan", { workspacePath, timestamp });
      setSpec(s); setLocalSpec(s); setDirty(false);
      setShowHistory(false);
    } catch (e) { setError(String(e)); }
  };

  const handleClearHistory = async () => {
    if (!workspacePath) return;
    if (!confirm("Delete all scan history? This cannot be undone.")) return;
    try {
      await invoke("archspec_clear_history", { workspacePath });
      setScanHistory([]);
      setShowHistory(false);
    } catch (e) { setError(String(e)); }
  };

  // ── Generate ─────────────────────────────────────────────────────────────
  const handleGenerate = async () => {
    if (!workspacePath) { setError("No workspace open."); return; }
    setGenerating(true); setError("");
    try {
      applySpec(await invoke<ArchSpec>("archspec_generate", { workspacePath }));
      setExpandedPhase(null);
      loadScanHistory(); // refresh history after new scan
    } catch (e) { setError(String(e)); }
    finally { setGenerating(false); }
  };

  // ── Local edits (mutate localSpec) ───────────────────────────────────────
  const patchLocal = (updater: (s: ArchSpec) => ArchSpec) => {
    setLocalSpec(prev => { if (!prev) return prev; const n = updater(prev); setDirty(true); return n; });
  };

  const patchArtifact = (id: string, field: "content" | "description", value: string) =>
    patchLocal(s => ({
      ...s,
      togaf: {
        ...s.togaf,
        artifacts: s.togaf.artifacts.map(a => a.id === id ? { ...a, [field]: value } : a),
      },
    }));

  const patchZachmanCell = (key: string, field: "content" | "maturity", value: string | number) =>
    patchLocal(s => ({
      ...s,
      zachman: {
        ...s.zachman,
        cells: {
          ...s.zachman.cells,
          [key]: { ...(s.zachman.cells[key] ?? { perspective: "", aspect: "", content: "", artifacts: [], maturity: 0 }), [field]: value },
        },
      },
    }));

  const patchC4Element = (id: string, field: "description" | "technology", value: string) =>
    patchLocal(s => ({
      ...s,
      c4: { ...s.c4, elements: s.c4.elements.map(e => e.id === id ? { ...e, [field]: value } : e) },
    }));

  const patchAdr = (id: string, field: "context" | "decision" | "consequences" | "title", value: string | string[]) =>
    patchLocal(s => ({
      ...s,
      adrs: { ...s.adrs, records: s.adrs.records.map(a => a.id === id ? { ...a, [field]: value } : a) },
    }));

  const patchRule = (id: string, field: "description" | "check_fn_description", value: string) =>
    patchLocal(s => ({
      ...s,
      governance: { ...s.governance, rules: s.governance.rules.map(r => r.id === id ? { ...r, [field]: value } : r) },
    }));

  // ── Save edits ────────────────────────────────────────────────────────────
  const handleSave = async () => {
    if (!workspacePath || !localSpec) return;
    setSaving(true); setError("");
    try {
      applySpec(await invoke<ArchSpec>("archspec_save", {
        workspacePath,
        specJson: JSON.stringify(localSpec),
      }));
    } catch (e) { setError(String(e)); }
    finally { setSaving(false); }
  };

  // ── TOGAF artifact status (server-side) ───────────────────────────────────
  const setArtifactStatus = async (artifactId: string, status: ArtifactStatus) => {
    if (!workspacePath) return;
    setStatusBusy(artifactId);
    try { applySpec(await invoke<ArchSpec>("archspec_set_artifact_status", { workspacePath, artifactId, status })); }
    catch (e) { setError(String(e)); }
    finally { setStatusBusy(null); }
  };

  // ── ADR status ────────────────────────────────────────────────────────────
  const setAdrStatus = async (adrId: string, status: string) => {
    if (!workspacePath) return;
    setStatusBusy(adrId);
    try { applySpec(await invoke<ArchSpec>("archspec_set_adr_status", { workspacePath, adrId, status })); }
    catch (e) { setError(String(e)); }
    finally { setStatusBusy(null); }
  };

  // ── Create ADR ────────────────────────────────────────────────────────────
  const createAdr = async () => {
    if (!workspacePath || !adrTitle) return;
    setAdrBusy(true);
    try {
      const consequences = adrConseq.split("\n").map(s => s.trim()).filter(Boolean);
      applySpec(await invoke<ArchSpec>("archspec_create_adr", {
        workspacePath, title: adrTitle, context: adrContext,
        decision: adrDecision, consequences, tags: [],
      }));
      setAdrTitle(""); setAdrContext(""); setAdrDecision(""); setAdrConseq("");
    } catch (e) { setError(String(e)); }
    finally { setAdrBusy(false); }
  };

  // ── Report generation (frontend, from spec) ───────────────────────────────
  const generateReport = (kind: "full" | "zachman" | "c4-context" | "c4-container" | "compliance") => {
    const s = localSpec ?? spec;
    if (!s) { setReport("No spec loaded. Run \"Generate from Codebase\" first."); setReportLabel("Report"); return; }

    let out = "";
    if (kind === "full") {
      setReportLabel("Full Architecture Report");
      const pct = s.togaf.artifacts.length
        ? Math.round(s.togaf.artifacts.filter(a => a.status === "Approved").length / s.togaf.artifacts.length * 100) : 0;
      out += `# Architecture Report: ${s.project_name}\n`;
      out += `Generated: ${new Date().toISOString()}\n\n`;
      out += `## TOGAF ADM (${pct}% approved, ${s.togaf.artifacts.length} artifacts)\n\n`;
      for (const phase of TOGAF_PHASES) {
        const arts = s.togaf.artifacts.filter(a => a.phase === phase.key);
        if (!arts.length) continue;
        out += `### Phase ${phase.index}: ${phase.label}\n`;
        for (const a of arts) {
          out += `#### [${a.status}] ${a.name} (${a.artifact_type})\n`;
          if (a.description) out += `${a.description}\n\n`;
          if (a.content)     out += `${a.content}\n\n`;
        }
      }
      out += `---\n\n## Zachman Framework (${Object.keys(s.zachman.cells).length}/36 cells)\n\n`;
      for (const p of ZACHMAN_PERSPECTIVES) {
        const pi = ZACHMAN_PERSPECTIVES.indexOf(p);
        out += `### ${p}\n`;
        for (const a of ZACHMAN_ASPECTS) {
          const ai = ZACHMAN_ASPECTS.indexOf(a);
          const cell = s.zachman.cells[`${pi}:${ai}`];
          if (cell?.content) out += `**${a}**: ${cell.content}\n`;
        }
        out += "\n";
      }
      out += `---\n\n## C4 Model (${s.c4.elements.length} elements, ${s.c4.relationships.length} relationships)\n\n`;
      for (const type of ["Person", "SoftwareSystem", "Container", "Component"]) {
        const els = s.c4.elements.filter(e => e.element_type === type);
        if (!els.length) continue;
        out += `### ${type === "SoftwareSystem" ? "Software Systems" : type + "s"}\n`;
        for (const el of els) {
          out += `- **${el.name}**${el.technology ? ` [${el.technology}]` : ""}: ${el.description}\n`;
        }
        out += "\n";
      }
      out += `---\n\n## ADRs (${s.adrs.records.length} records)\n\n`;
      for (const adr of s.adrs.records) {
        const st = typeof adr.status === "string" ? adr.status : Object.keys(adr.status)[0];
        out += `### ADR: ${adr.title} [${st}]\n`;
        out += `**Context:** ${adr.context}\n`;
        out += `**Decision:** ${adr.decision}\n`;
        if (adr.consequences.length) out += `**Consequences:**\n${adr.consequences.map(c => `- ${c}`).join("\n")}\n`;
        out += "\n";
      }
      out += `---\n\n## Governance Rules (${s.governance.rules.length})\n\n`;
      for (const rule of s.governance.rules) {
        out += `[${rule.severity}] **${rule.name}** (${rule.category})\n${rule.description}\n\n`;
      }
    } else if (kind === "zachman") {
      setReportLabel("Zachman Matrix Report");
      out += `# Zachman Framework: ${s.project_name}\n\n`;
      const filled = Object.keys(s.zachman.cells).length;
      out += `${filled}/36 cells populated\n\n`;
      const colWidths = [14, ...ZACHMAN_ASPECTS.map(() => 30)];
      const row = (cols: string[]) => cols.map((c, i) => c.padEnd(colWidths[i]).slice(0, colWidths[i])).join(" | ");
      out += row(["Perspective", ...ZACHMAN_ASPECTS]) + "\n";
      out += row(colWidths.map(w => "-".repeat(w))) + "\n";
      for (const p of ZACHMAN_PERSPECTIVES) {
        const pi = ZACHMAN_PERSPECTIVES.indexOf(p);
        const cols = ZACHMAN_ASPECTS.map(a => {
          const ai = ZACHMAN_ASPECTS.indexOf(a);
          const cell = s.zachman.cells[`${pi}:${ai}`];
          return cell?.content ? cell.content.slice(0, 28) + (cell.content.length > 28 ? "…" : "") : "—";
        });
        out += row([p, ...cols]) + "\n";
      }
      out += "\n\n## Full Cell Contents\n\n";
      for (const p of ZACHMAN_PERSPECTIVES) {
        const pi = ZACHMAN_PERSPECTIVES.indexOf(p);
        for (const a of ZACHMAN_ASPECTS) {
          const ai = ZACHMAN_ASPECTS.indexOf(a);
          const cell = s.zachman.cells[`${pi}:${ai}`];
          if (cell?.content) {
            out += `### ${p} / ${a} (maturity ${cell.maturity}/5)\n${cell.content}\n\n`;
          }
        }
      }
    } else if (kind === "c4-context") {
      setReportLabel("C4 Context Diagram");
      out += `# C4 Context Diagram: ${s.c4.system_name || s.project_name}\n\n`;
      out += `\`\`\`mermaid\nC4Context\n`;
      out += `  title System Context for ${s.c4.system_name || s.project_name}\n\n`;
      for (const el of s.c4.elements.filter(e => e.element_type === "Person"))
        out += `  Person(${el.id}, "${el.name}", "${el.description || ""}")\n`;
      for (const el of s.c4.elements.filter(e => e.element_type === "SoftwareSystem"))
        out += `  System(${el.id}, "${el.name}", "${el.description || ""}")\n`;
      out += "\n";
      for (const rel of s.c4.relationships) {
        const src = s.c4.elements.find(e => e.id === rel.source_id);
        const tgt = s.c4.elements.find(e => e.id === rel.target_id);
        if (!src || !tgt) continue;
        const st = src.element_type; const tt = tgt.element_type;
        if ((st === "Person" || st === "SoftwareSystem") && (tt === "Person" || tt === "SoftwareSystem"))
          out += `  Rel(${rel.source_id}, ${rel.target_id}, "${rel.description}"${rel.technology ? `, "${rel.technology}"` : ""})\n`;
      }
      out += `\`\`\`\n\n`;
      out += `## People\n`;
      for (const el of s.c4.elements.filter(e => e.element_type === "Person"))
        out += `- **${el.name}**: ${el.description}\n`;
      out += `\n## Systems\n`;
      for (const el of s.c4.elements.filter(e => e.element_type === "SoftwareSystem"))
        out += `- **${el.name}**: ${el.description}\n`;
    } else if (kind === "c4-container") {
      setReportLabel("C4 Container Diagram");
      out += `# C4 Container Diagram: ${s.c4.system_name || s.project_name}\n\n`;
      out += `\`\`\`mermaid\nC4Container\n`;
      out += `  title Container Diagram for ${s.c4.system_name || s.project_name}\n\n`;
      for (const el of s.c4.elements.filter(e => e.element_type === "Container"))
        out += `  Container(${el.id}, "${el.name}", "${el.technology || ""}", "${el.description || ""}")\n`;
      for (const el of s.c4.elements.filter(e => e.element_type === "Component"))
        out += `  Component(${el.id}, "${el.name}", "${el.technology || ""}", "${el.description || ""}")\n`;
      out += "\n";
      for (const rel of s.c4.relationships) {
        const src = s.c4.elements.find(e => e.id === rel.source_id);
        const tgt = s.c4.elements.find(e => e.id === rel.target_id);
        if (!src || !tgt) continue;
        const st = src.element_type; const tt = tgt.element_type;
        if ((st === "Container" || st === "Component") || (tt === "Container" || tt === "Component"))
          out += `  Rel(${rel.source_id}, ${rel.target_id}, "${rel.description}"${rel.technology ? `, "${rel.technology}"` : ""})\n`;
      }
      out += `\`\`\`\n\n`;
      out += `## Containers\n`;
      for (const el of s.c4.elements.filter(e => e.element_type === "Container"))
        out += `- **${el.name}** [${el.technology || ""}]: ${el.description}\n`;
      out += `\n## Components\n`;
      for (const el of s.c4.elements.filter(e => e.element_type === "Component"))
        out += `- **${el.name}** [${el.technology || ""}]: ${el.description}\n`;
    } else if (kind === "compliance") {
      setReportLabel("Compliance Check Report");
      out += `# Compliance Check: ${s.project_name}\n`;
      out += `Date: ${new Date().toISOString()}\n\n`;
      const byCategory = s.governance.rules.reduce<Record<string, GovernanceRule[]>>((acc, r) => {
        (acc[r.category] ??= []).push(r); return acc;
      }, {});
      for (const [cat, rules] of Object.entries(byCategory)) {
        out += `## ${cat}\n\n`;
        for (const rule of rules) {
          const sevMark = rule.severity === "Critical" ? "[CRITICAL]" : rule.severity === "Error" ? "[ERROR]" : rule.severity === "Warning" ? "[WARN]" : "[INFO]";
          out += `${sevMark} **${rule.name}** (${rule.id})\n`;
          out += `${rule.description}\n`;
          if (rule.check_fn_description) out += `*Check: ${rule.check_fn_description}*\n`;
          out += "\n";
        }
      }
      const critCount = s.governance.rules.filter(r => r.severity === "Critical").length;
      const errCount  = s.governance.rules.filter(r => r.severity === "Error").length;
      out += `---\nSummary: ${s.governance.rules.length} rules | ${critCount} Critical | ${errCount} Error\n`;
    }

    setReport(out || "(no content generated)");
  };

  // ── Derived ───────────────────────────────────────────────────────────────
  const disp = localSpec ?? spec;
  const artsFor = (phase: string) => disp?.togaf.artifacts.filter(a => a.phase === phase) ?? [];
  const phasePct = (phase: string) => {
    const a = artsFor(phase); if (!a.length) return 0;
    return Math.round(a.filter(x => x.status === "Approved").length / a.length * 100);
  };
  const totalPct = () => {
    const all = disp?.togaf.artifacts ?? []; if (!all.length) return 0;
    return Math.round(all.filter(a => a.status === "Approved").length / all.length * 100);
  };

  // ── Shared mini-components ────────────────────────────────────────────────
  const Badge = ({ label, color }: { label: string; color: string }) => (
    <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: "var(--radius-md)", border: `1px solid ${color}`, color, whiteSpace: "nowrap" }}>
      {label}
    </span>
  );

  const ProgBar = ({ pct, total }: { pct: number; total: number }) => (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <div style={{ width: 60, height: 4, background: "var(--bg-tertiary)", borderRadius: 2, overflow: "hidden" }}>
        <div style={{ width: `${pct}%`, height: "100%", borderRadius: 2, background: pct === 100 ? "var(--accent-green)" : "var(--accent-color)" }} />
      </div>
      <span className="panel-label" style={{ fontSize: "var(--font-size-xs)" }}>{total === 0 ? "0 artifacts" : `${Math.round(pct / 100 * total)}/${total} approved`}</span>
    </div>
  );

  const formatScanTime = (ts?: number) => {
    if (!ts || ts === 0) return null;
    const d = new Date(ts * 1000);
    const now = Date.now();
    const diffMs = now - d.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);
    let ago = "";
    if (diffMins < 1) ago = "just now";
    else if (diffMins < 60) ago = `${diffMins}m ago`;
    else if (diffHours < 24) ago = `${diffHours}h ago`;
    else ago = `${diffDays}d ago`;
    return { date: d.toLocaleString(), ago };
  };

  const scanInfo = formatScanTime(disp?.last_scanned_at);

  const GenerateBar = () => (
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <button className="panel-btn panel-btn-primary panel-btn-sm" onClick={handleGenerate}
        disabled={generating || !workspacePath}
        title={workspacePath
          ? (scanInfo ? `Rescan codebase (last: ${scanInfo.date})` : "Scan codebase and generate draft artifacts")
          : "Open a workspace first"}>
        {generating
          ? <><RefreshCw size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />Scanning…</>
          : <><RefreshCw size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />{scanInfo ? "Rescan" : "Generate from Codebase"}</>}
      </button>
      {scanHistory.length > 1 && (
        <button className="panel-btn panel-btn-secondary panel-btn-sm"
          onClick={() => setShowHistory(!showHistory)}
          title="View scan history">
          <ChevronDown size={12} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />History ({scanHistory.length})
        </button>
      )}
    </div>
  );

  const ScanStatusBar = () => {
    if (!scanInfo) return null;
    return (
      <div style={{
        display: "flex", alignItems: "center", gap: 8, padding: "4px 8px",
        fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", background: "var(--bg-tertiary)",
        borderRadius: "var(--radius-xs-plus)", marginBottom: 6,
      }}>
        <span>Last scanned: <strong style={{ color: "var(--text-primary)" }}>{scanInfo.ago}</strong></span>
        <span title={scanInfo.date}>({scanInfo.date})</span>
        {(disp?.scan_count ?? 0) > 1 && (
          <span>· Scan #{disp?.scan_count}</span>
        )}
      </div>
    );
  };

  const ScanHistoryPanel = () => {
    if (!showHistory || scanHistory.length === 0) return null;
    return (
      <div className="panel-card" style={{ marginBottom: 8, maxHeight: 200, overflowY: "auto" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
          <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>Scan History</span>
          <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={handleClearHistory}
            title="Delete all scan history" style={{ color: "var(--error-color)" }}>
            <Trash2 size={11} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 3 }} />Clear All
          </button>
        </div>
        <table style={{ width: "100%", fontSize: "var(--font-size-sm)", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)" }}>
              <th style={{ textAlign: "left", padding: "3px 8px" }}>Date</th>
              <th style={{ textAlign: "right", padding: "3px 8px" }}>Scan #</th>
              <th style={{ textAlign: "right", padding: "3px 8px" }}>TOGAF</th>
              <th style={{ textAlign: "right", padding: "3px 8px" }}>C4</th>
              <th style={{ textAlign: "right", padding: "3px 8px" }}>ADRs</th>
              <th style={{ textAlign: "right", padding: "3px 8px" }}>Rules</th>
              <th style={{ padding: "3px 8px" }}></th>
            </tr>
          </thead>
          <tbody>
            {scanHistory.map((s) => (
              <tr key={s.timestamp} style={{
                borderBottom: "1px solid var(--border-color)",
                background: s.current ? "color-mix(in srgb, var(--accent-color) 8%, transparent)" : undefined,
              }}>
                <td style={{ padding: "3px 8px" }}>{new Date(s.timestamp * 1000).toLocaleString()}</td>
                <td style={{ textAlign: "right", padding: "3px 8px" }}>{s.scan_count}</td>
                <td style={{ textAlign: "right", padding: "3px 8px" }}>{s.togaf_artifacts}</td>
                <td style={{ textAlign: "right", padding: "3px 8px" }}>{s.c4_elements}</td>
                <td style={{ textAlign: "right", padding: "3px 8px" }}>{s.adr_count}</td>
                <td style={{ textAlign: "right", padding: "3px 8px" }}>{s.governance_rules}</td>
                <td style={{ padding: "3px 8px" }}>
                  {s.current
                    ? <span style={{ fontSize: "var(--font-size-xs)", color: "var(--accent-green)" }}>current</span>
                    : <button className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-xs)", padding: "1px 8px" }}
                        onClick={() => loadHistoricalScan(s.timestamp)}>Load</button>
                  }
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  };

  const SaveBar = () => dirty ? (
    <button className="panel-btn panel-btn-primary panel-btn-sm" onClick={handleSave} disabled={saving}>
      <Save size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />{saving ? "Saving…" : "Save Changes"}
    </button>
  ) : null;

  // ── Report pane (editable) ────────────────────────────────────────────────
  const ReportPane = () => report ? (
    <div className="panel-card" style={{ marginTop: 10 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{reportLabel}</span>
        <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => setReport("")} aria-label="Close report"><X size={12} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle" }} /></button>
      </div>
      <textarea
        value={report}
        onChange={e => setReport(e.target.value)}
        style={{
          width: "100%", minHeight: 320, fontFamily: "monospace", fontSize: "var(--font-size-sm)",
          background: "var(--bg-tertiary)", color: "var(--text-primary)",
          border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)",
          padding: "8px", resize: "vertical", boxSizing: "border-box",
        }}
      />
    </div>
  ) : null;

  // ── TOGAF tab ─────────────────────────────────────────────────────────────
  const renderTogaf = () => (
    <>
      <div className="panel-card" style={{ marginBottom: 10 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 8 }}>
          <div>
            <span style={{ fontWeight: 600 }}>TOGAF ADM Phases</span>
            {disp && <span className="panel-label" style={{ marginLeft: 10 }}>
              {totalPct()}% · {disp.togaf.artifacts.length} artifacts
            </span>}
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <GenerateBar />
            <SaveBar />
            <button className="panel-btn panel-btn-secondary panel-btn-sm"
              onClick={() => generateReport("full")} disabled={!disp}>
              <FileText size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />Full Report
            </button>
          </div>
        </div>
      </div>

      <ScanStatusBar />
      <ScanHistoryPanel />

      {!workspacePath && <div className="panel-card" style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>Open a workspace folder to enable architecture tracking.</div>}
      {loading && <div className="panel-loading panel-label" style={{ padding: 8 }}>Loading…</div>}

      <div style={{ overflowY: "auto", maxHeight: "calc(100vh - 280px)" }}>
        {TOGAF_PHASES.map(phase => {
          const arts = artsFor(phase.key);
          const pct  = phasePct(phase.key);
          const open = expandedPhase === phase.key;
          return (
            <div key={phase.key} style={{ marginBottom: 4 }}>
              <div role="button" tabIndex={0} style={{ display: "flex", justifyContent: "space-between", alignItems: "center",
                padding: "8px 0", borderBottom: "1px solid var(--border-color)", cursor: arts.length ? "pointer" : "default" }}
                onClick={() => arts.length && setExpandedPhase(open ? null : phase.key)}>
                <span style={{ fontSize: "var(--font-size-md)" }}>
                  {arts.length > 0 && (open
                    ? <ChevronDown size={12} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4, color: "var(--text-secondary)" }} />
                    : <ChevronRight size={12} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4, color: "var(--text-secondary)" }} />
                  )}
                  {phase.index}. {phase.label}
                </span>
                <ProgBar pct={pct} total={arts.length} />
              </div>
              {open && (
                <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", margin: "4px 0 8px", padding: "8px 12px" }}>
                  {arts.map(art => (
                    <div key={art.id} style={{ padding: "12px 0", borderBottom: "1px solid var(--border-color)" }}>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 8 }}>
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                            <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{art.name}</span>
                            <Badge label={art.artifact_type} color="var(--text-secondary)" />
                            <Badge label={art.status} color={ARTIFACT_STATUS_COLOR[art.status]} />
                          </div>
                          <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 6 }}>Description</div>
                          <textarea
                            value={art.description}
                            onChange={e => patchArtifact(art.id, "description", e.target.value)}
                            rows={2}
                            style={{
                              width: "100%", fontSize: "var(--font-size-sm)", padding: "4px 8px", resize: "vertical",
                              background: "var(--bg-tertiary)", color: "var(--text-primary)",
                              border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box",
                              marginBottom: 6,
                            }}
                          />
                          <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 4 }}>Content</div>
                          <textarea
                            value={art.content}
                            onChange={e => patchArtifact(art.id, "content", e.target.value)}
                            rows={6}
                            style={{
                              width: "100%", fontFamily: "monospace", fontSize: "var(--font-size-sm)", padding: "8px 8px",
                              resize: "vertical", background: "var(--bg-tertiary)", color: "var(--text-primary)",
                              border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box",
                            }}
                            placeholder="Artifact content (generated or manually authored)…"
                          />
                        </div>
                        <div style={{ display: "flex", flexDirection: "column", gap: 4, flexShrink: 0 }}>
                          {art.status !== "Approved" && (
                            <button className="panel-btn panel-btn-sm" style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", color: "var(--accent-green)", borderColor: "var(--accent-green)" }}
                              disabled={statusBusy === art.id} onClick={() => setArtifactStatus(art.id, "Approved")}><Check size={11} strokeWidth={2} style={{ display: "inline", verticalAlign: "middle", marginRight: 3 }} />Approve</button>
                          )}
                          {art.status !== "Review" && art.status !== "Approved" && (
                            <button className="panel-btn panel-btn-secondary panel-btn-sm" style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px" }}
                              disabled={statusBusy === art.id} onClick={() => setArtifactStatus(art.id, "Review")}><RotateCcw size={11} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 3 }} />Review</button>
                          )}
                          {art.status === "Approved" && (
                            <button className="panel-btn panel-btn-sm" style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", color: "var(--error-color)", borderColor: "var(--error-color)" }}
                              disabled={statusBusy === art.id} onClick={() => setArtifactStatus(art.id, "Draft")}><X size={11} strokeWidth={2} style={{ display: "inline", verticalAlign: "middle", marginRight: 3 }} />Revoke</button>
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
      </div>

      {error && <div className="panel-error" style={{ marginTop: 8 }}>{error}</div>}
      <ReportPane />
    </>
  );

  // ── Zachman tab ───────────────────────────────────────────────────────────
  const renderZachman = () => {
    const cells = disp?.zachman.cells ?? {};
    const hasData = Object.keys(cells).length > 0;

    return (
      <>
        <div className="panel-card" style={{ marginBottom: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 8 }}>
            <span style={{ fontWeight: 600 }}>
              Zachman Framework 6×6
              {hasData && <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{Object.keys(cells).length}/36 cells filled</span>}
            </span>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {!hasData && <GenerateBar />}
              <SaveBar />
              <button className="panel-btn panel-btn-secondary panel-btn-sm"
                onClick={() => generateReport("zachman")} disabled={!disp}>
                <Table2 size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />Matrix Report
              </button>
            </div>
          </div>
        </div>

        {!hasData && !disp && <div className="panel-label" style={{ padding: 8 }}>Run "Generate from Codebase" to populate the Zachman matrix.</div>}

        {/* Cell editor panel */}
        {editingCell && (() => {
          const [piStr, aiStr] = editingCell.split(":");
          const pi = Number(piStr); const ai = Number(aiStr);
          const p = ZACHMAN_PERSPECTIVES[pi]; const a = ZACHMAN_ASPECTS[ai];
          const cell = cells[editingCell] ?? { perspective: p, aspect: a, content: "", artifacts: [], maturity: 0 };
          return (
            <div className="panel-card" style={{ marginBottom: 10, border: "1px solid var(--accent-color)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Edit Cell: {p} / {a}</span>
                <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => setEditingCell(null)} aria-label="Close cell editor"><X size={12} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle" }} /></button>
              </div>
              <div className="panel-label" style={{ marginBottom: 4 }}>Content</div>
              <textarea
                value={cell.content}
                autoFocus
                onChange={e => patchZachmanCell(editingCell, "content", e.target.value)}
                rows={5}
                style={{
                  width: "100%", fontSize: "var(--font-size-base)", padding: "8px 8px", resize: "vertical",
                  background: "var(--bg-tertiary)", color: "var(--text-primary)",
                  border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box", marginBottom: 8,
                }}
                placeholder={`Describe the ${a.toLowerCase()} dimension from the ${p.toLowerCase()} perspective…`}
              />
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <div className="panel-label" style={{ fontSize: "var(--font-size-sm)" }}>Maturity: {cell.maturity}/5</div>
                <input type="range" min={0} max={5} value={cell.maturity}
                  onChange={e => patchZachmanCell(editingCell, "maturity", Number(e.target.value))}
                  style={{ width: 120 }} />
              </div>
            </div>
          );
        })()}

        {hasData && (
          <div style={{ overflowX: "auto", overflowY: "auto", maxHeight: "calc(100vh - 320px)" }}>
            <table style={{ borderCollapse: "collapse", fontSize: "var(--font-size-sm)", tableLayout: "fixed", minWidth: 700 }}>
              <colgroup>
                <col style={{ width: 100 }} />
                {ZACHMAN_ASPECTS.map(a => <col key={a} style={{ width: 140 }} />)}
              </colgroup>
              <thead>
                <tr>
                  <th style={{ padding: "8px 8px", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", textAlign: "left", position: "sticky", top: 0, zIndex: 2 }} />
                  {ZACHMAN_ASPECTS.map(a => (
                    <th key={a} style={{ padding: "8px 8px", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: "var(--font-size-xs)", fontWeight: 600, textAlign: "center", position: "sticky", top: 0, zIndex: 2 }}>
                      {a}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {ZACHMAN_PERSPECTIVES.map((p, pi) => (
                  <tr key={p}>
                    <td style={{ padding: "8px 8px", border: "1px solid var(--border-color)", fontWeight: 600, background: "var(--bg-secondary)", whiteSpace: "nowrap", position: "sticky", left: 0, zIndex: 1 }}>{p}</td>
                    {ZACHMAN_ASPECTS.map((a, ai) => {
                      const key = `${pi}:${ai}`;
                      const cell = cells[key];
                      const maturity = cell?.maturity ?? 0;
                      const isEditing = editingCell === key;
                      return (
                        <td key={a} style={{
                          padding: "4px 8px", border: "1px solid var(--border-color)",
                          background: isEditing ? "var(--bg-tertiary)" : cell?.content ? "var(--bg-secondary)" : "var(--bg-tertiary)",
                          verticalAlign: "top", cursor: "pointer", minHeight: 60,
                          outline: isEditing ? "2px solid var(--accent-color)" : "none",
                        }}
                          onClick={() => setEditingCell(isEditing ? null : key)}
                          title={cell?.content ? `${p} / ${a}: ${cell.content}` : `Click to add ${p} / ${a}`}>
                          {cell?.content ? (
                            <>
                              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-primary)", overflow: "hidden", display: "-webkit-box", WebkitLineClamp: 3, WebkitBoxOrient: "vertical" }}>
                                {cell.content}
                              </div>
                              {maturity > 0 && (
                                <div style={{ marginTop: 3, display: "flex", gap: 2 }}>
                                  {[1,2,3,4,5].map(i => (
                                    <div key={i} style={{ width: 6, height: 6, borderRadius: 1, background: i <= maturity ? MATURITY_COLOR[maturity] : "var(--bg-tertiary)" }} />
                                  ))}
                                </div>
                              )}
                            </>
                          ) : <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)" }}>+ Add</span>}
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="panel-label" style={{ marginTop: 6, fontSize: "var(--font-size-xs)" }}>
              Click any cell to edit. Sticky header/perspective column. Maturity dots: 1=Initial → 5=Optimised.
            </div>
          </div>
        )}

        {error && <div className="panel-error" style={{ marginTop: 8 }}>{error}</div>}
        <ReportPane />
      </>
    );
  };

  // ── C4 Model tab ──────────────────────────────────────────────────────────
  const renderC4 = () => {
    const els  = disp?.c4.elements ?? [];
    const rels = disp?.c4.relationships ?? [];
    const hasData = els.length > 0;
    const byType = (t: string) => els.filter(e => e.element_type === t);
    const nameOf = (id: string) => els.find(e => e.id === id)?.name ?? id;

    const TypeSection = ({ type, label }: { type: string; label: string }) => {
      const items = byType(type);
      if (!items.length) return null;
      return (
        <div style={{ marginBottom: 12 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)", marginBottom: 6, color: "var(--text-secondary)" }}>{label}</div>
          {items.map(el => (
            <div key={el.id} style={{ padding: "8px 0", borderBottom: "1px solid var(--border-color)" }}>
              <div role="button" tabIndex={0} style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", cursor: "pointer", marginBottom: 4 }}
                onClick={() => setExpandedC4(expandedC4 === el.id ? null : el.id)}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{el.name}</span>
                  {expandedC4 === el.id
                    ? <ChevronDown size={12} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />
                    : <ChevronRight size={12} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />}
                </div>
                {el.technology && <span className="panel-label" style={{ fontSize: "var(--font-size-xs)" }}>{el.technology}</span>}
              </div>
              {expandedC4 === el.id && (
                <div style={{ marginTop: 6 }}>
                  <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 4 }}>Description</div>
                  <textarea
                    value={el.description}
                    onChange={e => patchC4Element(el.id, "description", e.target.value)}
                    rows={2}
                    style={{
                      width: "100%", fontSize: "var(--font-size-sm)", padding: "4px 8px", resize: "vertical",
                      background: "var(--bg-tertiary)", color: "var(--text-primary)",
                      border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box", marginBottom: 6,
                    }}
                  />
                  <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 4 }}>Technology</div>
                  <input
                    value={el.technology}
                    onChange={e => patchC4Element(el.id, "technology", e.target.value)}
                    style={{
                      width: "100%", fontSize: "var(--font-size-sm)", padding: "4px 8px",
                      background: "var(--bg-tertiary)", color: "var(--text-primary)",
                      border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box", marginBottom: 6,
                    }}
                  />
                  {rels.filter(r => r.source_id === el.id || r.target_id === el.id).map((r, i) => (
                    <div key={i} style={{ fontSize: "var(--font-size-xs)", padding: "2px 0", color: "var(--text-secondary)" }}>
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
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 8 }}>
            <span style={{ fontWeight: 600 }}>
              C4 Model
              {hasData && <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{els.length} elements · {rels.length} relationships</span>}
            </span>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {!hasData && <GenerateBar />}
              <SaveBar />
              <button className="panel-btn panel-btn-secondary panel-btn-sm"
                onClick={() => generateReport("c4-context")} disabled={!disp}>
                <Network size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />Context Diagram
              </button>
              <button className="panel-btn panel-btn-secondary panel-btn-sm"
                onClick={() => generateReport("c4-container")} disabled={!disp}>
                <Layers size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />Container Diagram
              </button>
            </div>
          </div>
        </div>

        {!hasData && !disp && <div className="panel-label" style={{ padding: 8 }}>Run "Generate from Codebase" to populate the C4 model.</div>}

        {hasData && (
          <div className="panel-card" style={{ overflowY: "auto", maxHeight: "calc(100vh - 300px)" }}>
            <TypeSection type="Person"         label="People" />
            <TypeSection type="SoftwareSystem" label="Software Systems" />
            <TypeSection type="Container"      label="Containers" />
            <TypeSection type="Component"      label="Components" />
          </div>
        )}

        <ReportPane />
        {error && <div className="panel-error" style={{ marginTop: 8 }}>{error}</div>}
      </>
    );
  };

  // ── ADRs tab ──────────────────────────────────────────────────────────────
  const renderAdr = () => {
    const adrs = disp?.adrs.records ?? [];
    const adrStatusLabel = (s: AdrStatusStr) =>
      typeof s === "string" ? s : Object.keys(s)[0];

    return (
      <>
        <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 8, gap: 8 }}>
          <SaveBar />
        </div>
        {adrs.length > 0 && (
          <div style={{ overflowY: "auto", maxHeight: "calc(100vh - 360px)" }}>
            <div className="panel-card" style={{ marginBottom: 10 }}>
              <div style={{ fontWeight: 600, marginBottom: 8 }}>
                Architecture Decision Records
                <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{adrs.length} total</span>
              </div>
              {adrs.map(adr => {
                const statusLabel = adrStatusLabel(adr.status);
                return (
                  <div key={adr.id} style={{ padding: "12px 0", borderBottom: "1px solid var(--border-color)" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", gap: 8 }}>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                          <input
                            value={adr.title}
                            onChange={e => patchAdr(adr.id, "title", e.target.value)}
                            style={{
                              fontSize: "var(--font-size-base)", fontWeight: 600, padding: "2px 8px", flex: 1,
                              background: "var(--bg-tertiary)", color: "var(--text-primary)",
                              border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box",
                            }}
                          />
                          <Badge label={statusLabel} color={ADR_STATUS_COLOR[statusLabel] ?? "var(--text-secondary)"} />
                          {adr.date && <span className="panel-label" style={{ fontSize: "var(--font-size-xs)" }}>{adr.date}</span>}
                        </div>
                        <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 3 }}>Context</div>
                        <textarea
                          value={adr.context}
                          onChange={e => patchAdr(adr.id, "context", e.target.value)}
                          rows={2}
                          style={{
                            width: "100%", fontSize: "var(--font-size-sm)", padding: "4px 8px", resize: "vertical", marginBottom: 6,
                            background: "var(--bg-tertiary)", color: "var(--text-primary)",
                            border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box",
                          }}
                        />
                        <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 3 }}>Decision</div>
                        <textarea
                          value={adr.decision}
                          onChange={e => patchAdr(adr.id, "decision", e.target.value)}
                          rows={2}
                          style={{
                            width: "100%", fontSize: "var(--font-size-sm)", padding: "4px 8px", resize: "vertical", marginBottom: 6,
                            background: "var(--bg-tertiary)", color: "var(--text-primary)",
                            border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box",
                          }}
                        />
                        <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 3 }}>Consequences</div>
                        <textarea
                          value={adr.consequences.join("\n")}
                          onChange={e => patchAdr(adr.id, "consequences", e.target.value.split("\n"))}
                          rows={3}
                          style={{
                            width: "100%", fontSize: "var(--font-size-sm)", padding: "4px 8px", resize: "vertical",
                            background: "var(--bg-tertiary)", color: "var(--text-primary)",
                            border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box",
                          }}
                        />
                        {adr.tags.length > 0 && (
                          <div style={{ marginTop: 6, display: "flex", gap: 4, flexWrap: "wrap" }}>
                            {adr.tags.map(t => <span key={t} className="panel-label" style={{ fontSize: 9, padding: "1px 4px", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)" }}>{t}</span>)}
                          </div>
                        )}
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", gap: 4, flexShrink: 0 }}>
                        {statusLabel !== "Accepted" && (
                          <button className="panel-btn panel-btn-sm" style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", color: "var(--accent-green)", borderColor: "var(--accent-green)" }}
                            disabled={statusBusy === adr.id} onClick={() => setAdrStatus(adr.id, "Accepted")}><Check size={11} strokeWidth={2} style={{ display: "inline", verticalAlign: "middle", marginRight: 3 }} />Accept</button>
                        )}
                        {statusLabel !== "Deprecated" && statusLabel !== "Accepted" && (
                          <button className="panel-btn panel-btn-secondary panel-btn-sm" style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px" }}
                            disabled={statusBusy === adr.id} onClick={() => setAdrStatus(adr.id, "Deprecated")}><X size={11} strokeWidth={2} style={{ display: "inline", verticalAlign: "middle", marginRight: 3 }} />Deprecate</button>
                        )}
                        {statusLabel === "Accepted" && (
                          <button className="panel-btn panel-btn-sm" style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", color: "var(--error-color)", borderColor: "var(--error-color)" }}
                            disabled={statusBusy === adr.id} onClick={() => setAdrStatus(adr.id, "Proposed")}><RotateCcw size={11} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 3 }} />Re-open</button>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
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
          <textarea value={adrConseq} onChange={e => setAdrConseq(e.target.value)} rows={3} className="panel-input panel-input-full" style={{ marginBottom: 10, resize: "vertical" }} placeholder={"Pro: …\nCon: …"} />
          <button className="panel-btn panel-btn-primary" disabled={!adrTitle || adrBusy || !workspacePath} onClick={createAdr}>
            {adrBusy ? "Saving…" : "Create ADR"}
          </button>
          {!workspacePath && <span className="panel-label" style={{ marginLeft: 10, fontSize: "var(--font-size-sm)" }}>Open a workspace to save ADRs.</span>}
        </div>

        {error && <div className="panel-error" style={{ marginTop: 8 }}>{error}</div>}
      </>
    );
  };

  // ── Governance tab ────────────────────────────────────────────────────────
  const renderGovernance = () => {
    const rules = disp?.governance.rules ?? [];
    const hasData = rules.length > 0;
    const byCategory = rules.reduce<Record<string, GovernanceRule[]>>((acc, r) => {
      (acc[r.category] ??= []).push(r); return acc;
    }, {});

    return (
      <>
        <div className="panel-card" style={{ marginBottom: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 8 }}>
            <span style={{ fontWeight: 600 }}>
              Architecture Governance Rules
              {hasData && <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{rules.length} rules</span>}
            </span>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              {!hasData && <GenerateBar />}
              <SaveBar />
              <button className="panel-btn panel-btn-secondary panel-btn-sm"
                onClick={() => generateReport("compliance")} disabled={!disp}>
                <ShieldCheck size={13} strokeWidth={1.5} style={{ display: "inline", verticalAlign: "middle", marginRight: 4 }} />Compliance Check
              </button>
            </div>
          </div>
        </div>

        {!hasData && !disp && <div className="panel-label" style={{ padding: 8 }}>Run "Generate from Codebase" to populate governance rules.</div>}

        <div style={{ overflowY: "auto", maxHeight: "calc(100vh - 300px)" }}>
          {hasData && Object.entries(byCategory).map(([cat, catRules]) => (
            <div key={cat} style={{ marginBottom: 12 }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-sm)", textTransform: "uppercase", letterSpacing: 1, color: "var(--text-secondary)", marginBottom: 6 }}>{cat}</div>
              {catRules.map(rule => (
                <div key={rule.id} className="panel-card" style={{ marginBottom: 6, padding: "12px 12px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                    <Badge label={rule.severity} color={SEV_COLOR[rule.severity]} />
                    <span style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{rule.name}</span>
                    <span className="panel-label" style={{ fontSize: "var(--font-size-xs)" }}>{rule.id}</span>
                  </div>
                  <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 4 }}>Description</div>
                  <textarea
                    value={rule.description}
                    onChange={e => patchRule(rule.id, "description", e.target.value)}
                    rows={2}
                    style={{
                      width: "100%", fontSize: "var(--font-size-sm)", padding: "4px 8px", resize: "vertical",
                      background: "var(--bg-tertiary)", color: "var(--text-primary)",
                      border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box", marginBottom: 6,
                    }}
                  />
                  {rule.check_fn_description && (
                    <>
                      <div className="panel-label" style={{ fontSize: "var(--font-size-sm)", marginBottom: 4 }}>Check Condition</div>
                      <textarea
                        value={rule.check_fn_description}
                        onChange={e => patchRule(rule.id, "check_fn_description", e.target.value)}
                        rows={2}
                        style={{
                          width: "100%", fontSize: "var(--font-size-sm)", padding: "4px 8px", resize: "vertical",
                          background: "var(--bg-tertiary)", color: "var(--text-primary)",
                          border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", boxSizing: "border-box",
                          fontStyle: "italic",
                        }}
                      />
                    </>
                  )}
                </div>
              ))}
            </div>
          ))}
        </div>

        {error && <div className="panel-error" style={{ marginTop: 8 }}>{error}</div>}
        <ReportPane />
      </>
    );
  };

  // ── Shell ─────────────────────────────────────────────────────────────────
  return (
    <div className="panel-container">
      <div className="panel-header">
        <h2 style={{ margin: 0, fontSize: "var(--font-size-xl)", fontWeight: 600, color: "var(--text-primary)" }}>
          Architecture Specification
          {disp && <span className="panel-label" style={{ marginLeft: 10, fontWeight: 400 }}>{disp.project_name}</span>}
        </h2>
        {dirty && <span style={{ fontSize: "var(--font-size-sm)", color: "var(--warning-color)", marginLeft: 8, display: "inline-flex", alignItems: "center", gap: 4 }}>
          <Circle size={7} strokeWidth={0} fill="var(--warning-color)" />unsaved changes
        </span>}
      </div>

      <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
        {(["togaf", "zachman", "c4", "adr", "governance"] as NavTab[]).map(t => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`}
            onClick={() => { setTab(t); setReport(""); setEditingCell(null); }}>
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
