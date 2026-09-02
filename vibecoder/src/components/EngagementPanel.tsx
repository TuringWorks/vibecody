/**
 * EngagementPanel — the delivery spine for a client engagement.
 *
 * VibeCoder already ships the tools that produce every artifact a four-phase
 * engagement promises. What was missing was the object that says which of them
 * *this* engagement has actually produced, and whether the phase they belong to
 * may be closed. This panel is the face of `vibecli`'s `/engagements/*` routes.
 *
 * Three rules it inherits from the daemon and must not soften on the way to the
 * screen:
 *
 *  1. `n/a` and `0%` are different claims. A phase with no in-scope work shows
 *     a dash, never a zero, because a zero reads as "measured, and bad".
 *  2. Unmeasured is its own state. A gate nobody judged is rendered distinctly
 *     from one that was judged and failed — same as `vibecli --eval`.
 *  3. Every deliverable names the panel that produces it. That is the whole
 *     navigational claim: 300+ panels are useless if the operator has to guess
 *     which one applies to "threat model".
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  Briefcase,
  CircleCheck,
  CircleDashed,
  CircleHelp,
  CircleX,
  FileText,
  MinusCircle,
  Paperclip,
  Plus,
  RefreshCw,
} from "lucide-react";
// Daemon routes are behind `require_auth`; a plain fetch() 401s. See daemonFetch.ts.
import { daemonFetch } from "../lib/daemonFetch";

// ── Types (mirror crates' `engagement.rs` serde shapes) ─────────────────────

type PhaseId = "discover" | "prove" | "build" | "operate";

type DeliverableStatus =
  | "not_started"
  | "in_progress"
  | "ready"
  | "accepted"
  | "waived";

type GateVerdict = "not_measured" | "pending" | "pass" | "fail" | "waived";

interface Engagement {
  id: string;
  name: string;
  client: string;
  workspace_path: string | null;
  status: "draft" | "active" | "paused" | "closed";
  current_phase: PhaseId;
  summary: string;
  created_at: number;
  updated_at: number;
}

interface Deliverable {
  id: string;
  engagement_id: string;
  phase: PhaseId;
  key: string;
  title: string;
  description: string;
  status: DeliverableStatus;
  owner: string | null;
  tool_hint: string | null;
  notes: string;
  evidence_count: number;
}

interface Gate {
  id: string;
  engagement_id: string;
  phase: PhaseId;
  title: string;
  criterion: string;
  measurement: string;
  observed: string | null;
  verdict: GateVerdict;
  rationale: string;
  decided_by: string | null;
  decided_at: number | null;
}

interface Blocker {
  kind:
    | "deliverable_outstanding"
    | "deliverable_without_evidence"
    | "gate_failed"
    | "gate_pending"
    | "gate_not_measured";
  subject: string;
  detail: string;
}

interface PhaseReadiness {
  phase: PhaseId;
  title: string;
  /** null for Discover — the engagement model publishes no duration for it. */
  cadence: string | null;
  deliverables: {
    total: number;
    not_started: number;
    in_progress: number;
    ready: number;
    accepted: number;
    waived: number;
    claimed_without_evidence: number;
  };
  gates: {
    total: number;
    not_measured: number;
    pending: number;
    pass: number;
    fail: number;
    waived: number;
  };
  /** null means "no in-scope work" — render a dash, never 0%. */
  completion: number | null;
  blockers: Blocker[];
  can_exit: boolean;
}

interface EngagementReport {
  engagement: Engagement;
  phases: PhaseReadiness[];
  generated_at: number;
}

interface Evidence {
  id: string;
  deliverable_id: string;
  kind: "file" | "url" | "run" | "metric" | "note";
  label: string;
  reference: string;
  captured_at: number;
}

interface EngagementPanelProps {
  /** URL of the vibecli daemon (default: http://localhost:7878) */
  daemonUrl?: string;
  workspacePath?: string | null;
}

// ── Presentation constants ──────────────────────────────────────────────────

const PHASE_ORDER: PhaseId[] = ["discover", "prove", "build", "operate"];

const DELIVERABLE_STATUSES: DeliverableStatus[] = [
  "not_started",
  "in_progress",
  "ready",
  "accepted",
  "waived",
];

const STATUS_LABEL: Record<DeliverableStatus, string> = {
  not_started: "Not started",
  in_progress: "In progress",
  ready: "Ready",
  accepted: "Accepted",
  waived: "Waived",
};

const STATUS_COLOR: Record<DeliverableStatus, string> = {
  not_started: "var(--text-secondary)",
  in_progress: "var(--text-warning)",
  ready: "var(--text-info, var(--text-primary))",
  accepted: "var(--text-success)",
  waived: "var(--text-secondary)",
};

const VERDICTS: GateVerdict[] = [
  "not_measured",
  "pending",
  "pass",
  "fail",
  "waived",
];

/**
 * Verdict presentation. `not_measured` and `fail` are deliberately different
 * icons *and* different colours — collapsing them is the exact mistake that
 * turns "nobody looked" into "the work is broken".
 */
const VERDICT_META: Record<
  GateVerdict,
  { label: string; color: string; icon: React.ReactNode }
> = {
  not_measured: {
    label: "Not measured",
    color: "var(--text-secondary)",
    icon: <CircleHelp size={13} strokeWidth={1.5} />,
  },
  pending: {
    label: "Pending",
    color: "var(--text-warning)",
    icon: <CircleDashed size={13} strokeWidth={1.5} />,
  },
  pass: {
    label: "Pass",
    color: "var(--text-success)",
    icon: <CircleCheck size={13} strokeWidth={1.5} />,
  },
  fail: {
    label: "Fail",
    color: "var(--text-danger)",
    icon: <CircleX size={13} strokeWidth={1.5} />,
  },
  waived: {
    label: "Waived",
    color: "var(--text-secondary)",
    icon: <MinusCircle size={13} strokeWidth={1.5} />,
  },
};

/**
 * Deliverable `tool_hint` → the VibeCoder tab that owns the producing panel.
 *
 * The daemon names the *component*; the shell navigates by *tab id*. A hint
 * with no entry here is not an error — the hint is still shown as text, it
 * simply is not clickable. That is better than sending the operator to a tab
 * that does not contain what they were promised.
 */
const TOOL_TAB: Record<string, string> = {
  ArchitectureSpecPanel: "architecture",
  DepsPanel: "code-analysis",
  SmartDepsPanel: "code-analysis",
  CodeMetricsPanel: "code-analysis",
  DocumentIngestPanel: "ai-context",
  SpecPanel: "planning",
  PlanDocumentPanel: "planning",
  SecurityPosturePanel: "security",
  SecurityReviewPanel: "security",
  CompliancePanel: "security",
  DeployPanel: "build-deploy",
  BuildPanel: "build-deploy",
  ArenaPanel: "ai-playground",
  CostPanel: "billing",
  CounselPanel: "ai-teams",
  K8sPanel: "containers",
  CicdPanel: "ci-cd",
  CoveragePanel: "testing",
  TraceDashboard: "observability",
  HealthMonitorPanel: "system-monitor",
  TeamGovernancePanel: "enterprise-governance",
  TeamOnboardingPanel: "enterprise-governance",
  CompanyDashboardPanel: "company",
  CompanyPortabilityPanel: "company",
  CollabPanel: "collaboration",
  EngagementPanel: "engagement",
};

/** `n/a` and `0%` are different claims. This is the only place that decides. */
function formatCompletion(value: number | null): string {
  if (value === null) return "n/a";
  return `${Math.round(value * 100)}%`;
}

const DAEMON_DEFAULT = "http://localhost:7878";

// ── Panel ───────────────────────────────────────────────────────────────────

export function EngagementPanel({
  daemonUrl = DAEMON_DEFAULT,
  workspacePath = null,
}: EngagementPanelProps) {
  const [engagements, setEngagements] = useState<Engagement[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [report, setReport] = useState<EngagementReport | null>(null);
  const [deliverables, setDeliverables] = useState<Deliverable[]>([]);
  const [gates, setGates] = useState<Gate[]>([]);
  const [activePhase, setActivePhase] = useState<PhaseId>("discover");
  const [evidenceFor, setEvidenceFor] = useState<string | null>(null);
  const [evidence, setEvidence] = useState<Evidence[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newClient, setNewClient] = useState("");

  const request = useCallback(
    async (path: string, init?: RequestInit) => {
      const res = await daemonFetch(`${daemonUrl}${path}`, init);
      if (!res.ok) {
        // Surface the daemon's own message. A generic "request failed" hides
        // exactly the validation errors this API exists to raise — "a passing
        // gate must record what was observed" is the point, not noise.
        let detail = `HTTP ${res.status}`;
        try {
          const body = await res.json();
          if (body && typeof body.error === "string") detail = body.error;
        } catch {
          /* non-JSON body; keep the status line */
        }
        throw new Error(detail);
      }
      return res;
    },
    [daemonUrl]
  );

  const loadEngagements = useCallback(async () => {
    try {
      const res = await request("/engagements");
      const data = await res.json();
      setEngagements(data.engagements ?? []);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [request]);

  const loadSelected = useCallback(
    async (id: string) => {
      setLoading(true);
      try {
        const [reportRes, delivRes, gatesRes] = await Promise.all([
          request(`/engagements/${id}`),
          request(`/engagements/${id}/deliverables`),
          request(`/engagements/${id}/gates`),
        ]);
        const reportBody = await reportRes.json();
        const delivBody = await delivRes.json();
        const gatesBody = await gatesRes.json();
        setReport(reportBody.report ?? null);
        setDeliverables(delivBody.deliverables ?? []);
        setGates(gatesBody.gates ?? []);
        if (reportBody.report?.engagement?.current_phase) {
          setActivePhase(reportBody.report.engagement.current_phase);
        }
        setError(null);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    [request]
  );

  useEffect(() => {
    loadEngagements();
  }, [loadEngagements]);

  useEffect(() => {
    if (selectedId) loadSelected(selectedId);
  }, [selectedId, loadSelected]);

  const createEngagement = async () => {
    if (!newName.trim()) return;
    setCreating(true);
    try {
      const res = await request("/engagements", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: newName.trim(),
          client: newClient.trim(),
          workspace_path: workspacePath,
          summary: "",
        }),
      });
      const body = await res.json();
      setNewName("");
      setNewClient("");
      await loadEngagements();
      if (body.engagement?.id) setSelectedId(body.engagement.id);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setCreating(false);
    }
  };

  const setDeliverableStatus = async (
    d: Deliverable,
    status: DeliverableStatus
  ) => {
    if (!selectedId) return;
    try {
      await request(`/engagements/${selectedId}/deliverables/${d.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ status }),
      });
      await loadSelected(selectedId);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const judgeGate = async (g: Gate, verdict: GateVerdict) => {
    if (!selectedId) return;
    // The daemon rejects a pass with nothing observed. Ask here rather than
    // letting the operator hit a 400 they cannot act on.
    let observed = g.observed ?? "";
    if (verdict === "pass") {
      const answer = window.prompt(
        `What was observed?\n\nMeasured by: ${g.measurement}`,
        observed
      );
      if (answer === null) return;
      observed = answer;
    }
    try {
      await request(`/engagements/${selectedId}/gates/${g.id}/judge`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          verdict,
          observed: observed.trim() === "" ? null : observed,
          rationale: g.rationale,
        }),
      });
      await loadSelected(selectedId);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const openEvidence = async (d: Deliverable) => {
    if (!selectedId) return;
    if (evidenceFor === d.id) {
      setEvidenceFor(null);
      return;
    }
    try {
      const res = await request(
        `/engagements/${selectedId}/deliverables/${d.id}/evidence`
      );
      const body = await res.json();
      setEvidence(body.evidence ?? []);
      setEvidenceFor(d.id);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const attachEvidence = async (d: Deliverable) => {
    if (!selectedId) return;
    const reference = window.prompt(
      "Evidence reference — a file path, URL, run id, or measured value:"
    );
    if (!reference || !reference.trim()) return;
    const label = window.prompt("Label for this evidence:", d.title) ?? "";
    const kind = reference.startsWith("http")
      ? "url"
      : reference.includes("/") || reference.includes("\\")
        ? "file"
        : "note";
    try {
      await request(
        `/engagements/${selectedId}/deliverables/${d.id}/evidence`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ kind, label, reference: reference.trim() }),
        }
      );
      await loadSelected(selectedId);
      const res = await request(
        `/engagements/${selectedId}/deliverables/${d.id}/evidence`
      );
      const body = await res.json();
      setEvidence(body.evidence ?? []);
      setEvidenceFor(d.id);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const advance = async (force: boolean) => {
    if (!selectedId) return;
    try {
      const res = await request(`/engagements/${selectedId}/advance`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ force }),
      });
      const body = await res.json();
      const outcome = body.outcome;
      if (!outcome?.advanced) {
        setError(outcome?.reason ?? "Phase cannot be closed yet.");
      } else {
        setError(
          outcome.forced
            ? `${outcome.reason} The override is recorded on the engagement.`
            : null
        );
      }
      await loadSelected(selectedId);
      await loadEngagements();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const openInTab = (toolHint: string) => {
    const tab = TOOL_TAB[toolHint];
    if (!tab) return;
    window.dispatchEvent(
      new CustomEvent("vibecoder:open-tab", { detail: tab })
    );
  };

  const phaseReadiness = useMemo(
    () => report?.phases.find((p) => p.phase === activePhase) ?? null,
    [report, activePhase]
  );

  const phaseDeliverables = useMemo(
    () => deliverables.filter((d) => d.phase === activePhase),
    [deliverables, activePhase]
  );

  const phaseGates = useMemo(
    () => gates.filter((g) => g.phase === activePhase),
    [gates, activePhase]
  );

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>
          <Briefcase
            size={14}
            strokeWidth={1.5}
            style={{ verticalAlign: "-2px", marginRight: 6 }}
          />
          Engagements
        </h3>
        <button
          className="panel-btn panel-btn-secondary panel-btn-xs"
          onClick={() => {
            loadEngagements();
            if (selectedId) loadSelected(selectedId);
          }}
          title="Reload from the daemon"
        >
          <RefreshCw size={12} strokeWidth={1.5} /> Refresh
        </button>
      </div>

      <div className="panel-body">
        {error && (
          <div className="panel-error" role="alert">
            {error}
          </div>
        )}

        {/* Engagement selector + creation */}
        <div className="panel-card" style={{ marginBottom: 12 }}>
          <div
            style={{
              display: "flex",
              gap: 8,
              alignItems: "center",
              flexWrap: "wrap",
            }}
          >
            <select
              className="panel-select"
              value={selectedId ?? ""}
              onChange={(e) => setSelectedId(e.target.value || null)}
              aria-label="Select engagement"
            >
              <option value="">— select an engagement —</option>
              {engagements.map((e) => (
                <option key={e.id} value={e.id}>
                  {e.name}
                  {e.client ? ` · ${e.client}` : ""} ({e.status})
                </option>
              ))}
            </select>
            {selectedId && (
              <>
                <a
                  className="panel-btn panel-btn-secondary panel-btn-xs"
                  href={`${daemonUrl}/engagements/${selectedId}/report.md`}
                  target="_blank"
                  rel="noreferrer"
                  title="Status report (markdown)"
                >
                  <FileText size={12} strokeWidth={1.5} /> Report
                </a>
                <a
                  className="panel-btn panel-btn-secondary panel-btn-xs"
                  href={`${daemonUrl}/engagements/${selectedId}/handover.md`}
                  target="_blank"
                  rel="noreferrer"
                  title="Handover pack (markdown)"
                >
                  <FileText size={12} strokeWidth={1.5} /> Handover
                </a>
              </>
            )}
          </div>

          <div
            style={{
              display: "flex",
              gap: 8,
              alignItems: "center",
              marginTop: 8,
              flexWrap: "wrap",
            }}
          >
            <input
              className="panel-input"
              placeholder="New engagement name"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
            <input
              className="panel-input"
              placeholder="Client"
              value={newClient}
              onChange={(e) => setNewClient(e.target.value)}
            />
            <button
              className="panel-btn panel-btn-primary panel-btn-xs"
              onClick={createEngagement}
              disabled={creating || !newName.trim()}
            >
              <Plus size={12} strokeWidth={1.5} />{" "}
              {creating ? "Creating…" : "Create"}
            </button>
          </div>
          <div
            style={{
              fontSize: 11,
              color: "var(--text-secondary)",
              marginTop: 6,
            }}
          >
            A new engagement is seeded with all four phases, every promised
            deliverable, and its gates — unmeasured until someone measures them.
          </div>
        </div>

        {!selectedId && (
          <div style={{ color: "var(--text-secondary)", fontSize: 12 }}>
            Select or create an engagement to see its phase board.
          </div>
        )}

        {selectedId && report && (
          <>
            {/* Phase summary strip */}
            <div
              role="group"
              aria-label="Phase readiness"
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))",
                gap: 8,
                marginBottom: 12,
              }}
            >
              {PHASE_ORDER.map((pid, i) => {
                const p = report.phases.find((x) => x.phase === pid);
                if (!p) return null;
                const isCurrent = report.engagement.current_phase === pid;
                const isActive = activePhase === pid;
                return (
                  <button
                    key={pid}
                    onClick={() => setActivePhase(pid)}
                    aria-pressed={isActive}
                    style={{
                      textAlign: "left",
                      padding: "8px 10px",
                      borderRadius: 6,
                      cursor: "pointer",
                      background: isActive
                        ? "var(--bg-secondary)"
                        : "transparent",
                      border: `1px solid ${
                        isCurrent ? "var(--accent-green)" : "var(--border-color)"
                      }`,
                      color: "var(--text-primary)",
                    }}
                  >
                    <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>
                      {`0${i + 1}`}
                      {/* Discover publishes no cadence; a dash, not a guess. */}
                      {p.cadence ? ` · ${p.cadence}` : " · —"}
                    </div>
                    <div style={{ fontSize: 12, fontWeight: 600 }}>{p.title}</div>
                    <div
                      style={{
                        fontSize: 11,
                        color: "var(--text-secondary)",
                        marginTop: 2,
                      }}
                    >
                      {formatCompletion(p.completion)} accepted ·{" "}
                      <span
                        style={{
                          color: p.can_exit
                            ? "var(--text-success)"
                            : "var(--text-warning)",
                        }}
                      >
                        {p.can_exit
                          ? "ready"
                          : `${p.blockers.length} blocker${
                              p.blockers.length === 1 ? "" : "s"
                            }`}
                      </span>
                    </div>
                  </button>
                );
              })}
            </div>

            {loading && (
              <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                Loading…
              </div>
            )}

            {phaseReadiness && (
              <>
                {/* Phase actions */}
                <div
                  style={{
                    display: "flex",
                    gap: 8,
                    alignItems: "center",
                    marginBottom: 10,
                    flexWrap: "wrap",
                  }}
                >
                  <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                    Gates: {phaseReadiness.gates.pass} pass ·{" "}
                    {phaseReadiness.gates.fail} fail ·{" "}
                    {phaseReadiness.gates.pending} pending ·{" "}
                    {phaseReadiness.gates.not_measured} not measured ·{" "}
                    {phaseReadiness.gates.waived} waived
                  </span>
                  {report.engagement.current_phase === activePhase && (
                    <>
                      <button
                        className="panel-btn panel-btn-primary panel-btn-xs"
                        onClick={() => advance(false)}
                        disabled={!phaseReadiness.can_exit}
                        title={
                          phaseReadiness.can_exit
                            ? "Close this phase and move to the next"
                            : "Blockers outstanding"
                        }
                      >
                        Close phase
                      </button>
                      {!phaseReadiness.can_exit && (
                        <button
                          className="panel-btn panel-btn-danger panel-btn-xs"
                          onClick={() => advance(true)}
                          title="Advance with blockers outstanding — recorded as an override"
                        >
                          Advance anyway
                        </button>
                      )}
                    </>
                  )}
                </div>

                {/* Blockers — the honest part */}
                {phaseReadiness.blockers.length > 0 && (
                  <details
                    className="panel-card"
                    style={{ marginBottom: 12 }}
                    open={false}
                  >
                    <summary style={{ cursor: "pointer", fontSize: 12 }}>
                      {phaseReadiness.blockers.length} blocker
                      {phaseReadiness.blockers.length === 1 ? "" : "s"} in{" "}
                      {phaseReadiness.title}
                    </summary>
                    <ul
                      style={{
                        margin: "8px 0 0",
                        paddingLeft: 18,
                        fontSize: 11,
                        color: "var(--text-secondary)",
                      }}
                    >
                      {phaseReadiness.blockers.map((b, i) => (
                        <li key={`${b.kind}-${b.subject}-${i}`}>{b.detail}</li>
                      ))}
                    </ul>
                  </details>
                )}

                {/* Deliverables */}
                <h4 style={{ fontSize: 12, margin: "0 0 6px" }}>Deliverables</h4>
                {phaseDeliverables.length === 0 && (
                  <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                    No deliverables defined for this phase.
                  </div>
                )}
                {phaseDeliverables.map((d) => (
                  <div
                    key={d.id}
                    className="panel-card"
                    style={{ marginBottom: 6, padding: "8px 10px" }}
                  >
                    <div
                      style={{
                        display: "flex",
                        gap: 8,
                        alignItems: "center",
                        flexWrap: "wrap",
                      }}
                    >
                      <span
                        style={{
                          fontSize: 12,
                          fontWeight: 600,
                          color: STATUS_COLOR[d.status],
                        }}
                      >
                        {d.title}
                      </span>
                      <select
                        className="panel-select"
                        value={d.status}
                        onChange={(e) =>
                          setDeliverableStatus(
                            d,
                            e.target.value as DeliverableStatus
                          )
                        }
                        aria-label={`Status of ${d.title}`}
                        style={{ fontSize: 11 }}
                      >
                        {DELIVERABLE_STATUSES.map((s) => (
                          <option key={s} value={s}>
                            {STATUS_LABEL[s]}
                          </option>
                        ))}
                      </select>
                      <button
                        className="panel-btn panel-btn-secondary panel-btn-xs"
                        onClick={() => openEvidence(d)}
                        title="Show attached evidence"
                      >
                        <Paperclip size={11} strokeWidth={1.5} />{" "}
                        {d.evidence_count}
                      </button>
                      <button
                        className="panel-btn panel-btn-secondary panel-btn-xs"
                        onClick={() => attachEvidence(d)}
                      >
                        Attach
                      </button>
                      {d.tool_hint && (
                        <button
                          className="panel-btn panel-btn-secondary panel-btn-xs"
                          onClick={() => openInTab(d.tool_hint as string)}
                          disabled={!TOOL_TAB[d.tool_hint]}
                          title={
                            TOOL_TAB[d.tool_hint]
                              ? `Open ${d.tool_hint}`
                              : `${d.tool_hint} — no tab mapping`
                          }
                        >
                          {d.tool_hint}
                        </button>
                      )}
                      {(d.status === "ready" || d.status === "accepted") &&
                        d.evidence_count === 0 && (
                          <span
                            style={{
                              fontSize: 10,
                              color: "var(--text-danger)",
                            }}
                            title="Claimed done with nothing behind it"
                          >
                            no evidence
                          </span>
                        )}
                    </div>
                    <div
                      style={{
                        fontSize: 11,
                        color: "var(--text-secondary)",
                        marginTop: 4,
                      }}
                    >
                      {d.description}
                    </div>
                    {evidenceFor === d.id && (
                      <ul
                        style={{
                          margin: "6px 0 0",
                          paddingLeft: 18,
                          fontSize: 11,
                        }}
                      >
                        {evidence.length === 0 && (
                          <li style={{ color: "var(--text-secondary)" }}>
                            Nothing attached.
                          </li>
                        )}
                        {evidence.map((x) => (
                          <li key={x.id}>
                            <code>{x.kind}</code> {x.label} — {x.reference}
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                ))}

                {/* Gates */}
                <h4 style={{ fontSize: 12, margin: "14px 0 6px" }}>
                  Gates — agreed before the phase runs
                </h4>
                {phaseGates.length === 0 && (
                  <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                    No gates defined for this phase.
                  </div>
                )}
                {phaseGates.map((g) => {
                  const meta = VERDICT_META[g.verdict];
                  return (
                    <div
                      key={g.id}
                      className="panel-card"
                      style={{ marginBottom: 6, padding: "8px 10px" }}
                    >
                      <div
                        style={{
                          display: "flex",
                          gap: 8,
                          alignItems: "center",
                          flexWrap: "wrap",
                        }}
                      >
                        <span style={{ color: meta.color, display: "flex" }}>
                          {meta.icon}
                        </span>
                        <span style={{ fontSize: 12, fontWeight: 600 }}>
                          {g.title}
                        </span>
                        <select
                          className="panel-select"
                          value={g.verdict}
                          onChange={(e) =>
                            judgeGate(g, e.target.value as GateVerdict)
                          }
                          aria-label={`Verdict for ${g.title}`}
                          style={{ fontSize: 11 }}
                        >
                          {VERDICTS.map((v) => (
                            <option key={v} value={v}>
                              {VERDICT_META[v].label}
                            </option>
                          ))}
                        </select>
                      </div>
                      <div
                        style={{
                          fontSize: 11,
                          color: "var(--text-secondary)",
                          marginTop: 4,
                        }}
                      >
                        <div>{g.criterion}</div>
                        <div style={{ marginTop: 2 }}>
                          <strong>Measured by:</strong> {g.measurement}
                        </div>
                        <div style={{ marginTop: 2 }}>
                          <strong>Observed:</strong>{" "}
                          {/* Absent stays absent — no zero, no placeholder. */}
                          {g.observed ?? <em>not recorded</em>}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default EngagementPanel;
