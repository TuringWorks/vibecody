/**
 * SelfImprovingSkillsPanel — Closed-loop skill learning dashboard.
 *
 * Displays per-skill health metrics, pending evolution proposals,
 * new skill drafts, and prune candidates. Lets users approve or
 * reject proposed changes before they are written to disk.
 */
import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Brain, TrendingUp, AlertTriangle, Plus, RefreshCw, ChevronDown, ChevronUp } from "lucide-react";

// ── Types ──────────────────────────────────────────────────────────────────────

interface SisStatus {
  total_activations: number;
  skills_tracked: number;
  thriving: number;
  struggling: number;
  critical: number;
  evolutions_pending: number;
  evolutions_applied: number;
  new_skills_drafted: number;
}

interface SkillMetrics {
  skill_name: string;
  total_activations: number;
  accepted: number;
  rejected: number;
  corrected: number;
  ignored: number;
  success_rate: number;
  last_activated: number;
  health: "Thriving" | "Healthy" | "Struggling" | "Critical" | "Insufficient";
}

interface SkillEvolution {
  id: string;
  kind: "RefineTriggers" | "RefineContent" | "AddExample" | "NewSkill" | "Prune";
  skill_name: string;
  rationale: string;
  proposed_content: string;
  confidence: number;
  auto_applicable: boolean;
  created_at: number;
  applied: boolean;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

function healthColor(h: SkillMetrics["health"]): string {
  switch (h) {
    case "Thriving":    return "var(--success-color, #34d399)";
    case "Healthy":     return "var(--accent-blue, #6c8cff)";
    case "Struggling":  return "var(--warning-color, #f5c542)";
    case "Critical":    return "var(--error-color, #ef4444)";
    default:            return "var(--text-secondary)";
  }
}

function kindLabel(k: SkillEvolution["kind"]): string {
  switch (k) {
    case "RefineTriggers": return "Refine Triggers";
    case "RefineContent":  return "Refine Content";
    case "AddExample":     return "Add Examples";
    case "NewSkill":       return "New Skill Draft";
    case "Prune":          return "Prune";
  }
}

function kindColor(k: SkillEvolution["kind"]): string {
  switch (k) {
    case "NewSkill":       return "var(--success-color, #34d399)";
    case "Prune":          return "var(--error-color, #ef4444)";
    case "RefineTriggers": return "var(--warning-color, #f5c542)";
    default:               return "var(--accent-blue, #6c8cff)";
  }
}

function pct(n: number): string {
  return `${Math.round(n * 100)}%`;
}

// ── Sub-components ─────────────────────────────────────────────────────────────

function StatusBar({ status }: { status: SisStatus }) {
  const cards = [
    { label: "Activations", value: status.total_activations, color: "var(--text-primary)" },
    { label: "Thriving",    value: status.thriving,           color: "var(--success-color, #34d399)" },
    { label: "Struggling",  value: status.struggling,         color: "var(--warning-color, #f5c542)" },
    { label: "Critical",    value: status.critical,           color: "var(--error-color, #ef4444)" },
    { label: "Pending",     value: status.evolutions_pending, color: "var(--accent-blue, #6c8cff)" },
    { label: "New Drafts",  value: status.new_skills_drafted, color: "var(--success-color, #34d399)" },
  ];
  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 8 }}>
      {cards.map(c => (
        <div key={c.label} className="panel-card" style={{ padding: "12px 12px", textAlign: "center" }}>
          <div style={{ fontSize: 20, fontWeight: 700, color: c.color }}>{c.value}</div>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 }}>{c.label}</div>
        </div>
      ))}
    </div>
  );
}

function MetricsTable({ metrics }: { metrics: SkillMetrics[] }) {
  if (metrics.length === 0) {
    return (
      <div className="panel-empty" style={{ padding: "24px 0" }}>
        No skill activations recorded yet. Skills are tracked automatically when the agent fires them.
      </div>
    );
  }
  const sorted = [...metrics].sort((a, b) => b.total_activations - a.total_activations);
  return (
    <div style={{ overflowX: "auto" }}>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
        <thead>
          <tr style={{ borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", textAlign: "left" }}>
            <th style={{ padding: "8px 8px" }}>Skill</th>
            <th style={{ padding: "8px 8px" }}>Health</th>
            <th style={{ padding: "8px 8px", textAlign: "right" }}>Activations</th>
            <th style={{ padding: "8px 8px", textAlign: "right" }}>Success</th>
            <th style={{ padding: "8px 8px", textAlign: "right" }}>Rejected</th>
            <th style={{ padding: "8px 8px", textAlign: "right" }}>Corrected</th>
          </tr>
        </thead>
        <tbody>
          {sorted.map(m => (
            <tr key={m.skill_name} style={{ borderBottom: "1px solid var(--border-color)" }}>
              <td style={{ padding: "8px 8px", fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{m.skill_name}</td>
              <td style={{ padding: "8px 8px" }}>
                <span style={{ fontSize: "var(--font-size-sm)", padding: "2px 8px", borderRadius: 3,
                  background: `color-mix(in srgb, ${healthColor(m.health)} 18%, transparent)`,
                  color: healthColor(m.health), fontWeight: 600 }}>
                  {m.health}
                </span>
              </td>
              <td style={{ padding: "8px 8px", textAlign: "right", color: "var(--text-secondary)" }}>{m.total_activations}</td>
              <td style={{ padding: "8px 8px", textAlign: "right", color: "var(--success-color, #34d399)", fontWeight: 600 }}>{pct(m.success_rate)}</td>
              <td style={{ padding: "8px 8px", textAlign: "right", color: "var(--error-color, #ef4444)" }}>{m.rejected}</td>
              <td style={{ padding: "8px 8px", textAlign: "right", color: "var(--warning-color, #f5c542)" }}>{m.corrected}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EvolutionCard({
  ev,
  onApply,
  onDismiss,
}: {
  ev: SkillEvolution;
  onApply: (id: string) => void;
  onDismiss: (id: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className="panel-card" style={{ marginBottom: 8 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ fontSize: "var(--font-size-sm)", padding: "2px 8px", borderRadius: 3,
          background: `color-mix(in srgb, ${kindColor(ev.kind)} 18%, transparent)`,
          color: kindColor(ev.kind), fontWeight: 600 }}>
          {kindLabel(ev.kind)}
        </span>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", color: "var(--text-primary)", flex: 1 }}>{ev.skill_name}</span>
        <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          {pct(ev.confidence)} confidence
          {ev.auto_applicable && (
            <span style={{ marginLeft: 6, color: "var(--success-color, #34d399)", fontWeight: 600 }}>auto</span>
          )}
        </span>
      </div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", margin: "8px 0" }}>{ev.rationale}</div>
      {ev.proposed_content && (
        <button
          onClick={() => setExpanded(v => !v)}
          style={{ fontSize: "var(--font-size-sm)", background: "none", border: "none", color: "var(--accent-blue, #6c8cff)", cursor: "pointer", padding: 0, marginBottom: 6 }}
        >
          {expanded ? <><ChevronUp size={10} /> Hide proposed content</> : <><ChevronDown size={10} /> View proposed content</>}
        </button>
      )}
      {expanded && ev.proposed_content && (
        <pre style={{ fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", background: "var(--bg-tertiary)",
          border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 8,
          overflowX: "auto", whiteSpace: "pre-wrap", wordBreak: "break-word", maxHeight: 200 }}>
          {ev.proposed_content}
        </pre>
      )}
      <div style={{ display: "flex", gap: 6, marginTop: 6 }}>
        <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)", padding: "3px 12px" }} onClick={() => onApply(ev.id)}>
          Apply
        </button>
        <button className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)", padding: "3px 12px" }} onClick={() => onDismiss(ev.id)}>
          Dismiss
        </button>
      </div>
    </div>
  );
}

// ── Main Panel ─────────────────────────────────────────────────────────────────

type Tab = "overview" | "metrics" | "evolutions" | "extract";

export function SelfImprovingSkillsPanel() {
  const [tab, setTab] = useState<Tab>("overview");
  const [status, setStatus] = useState<SisStatus | null>(null);
  const [metrics, setMetrics] = useState<SkillMetrics[]>([]);
  const [evolutions, setEvolutions] = useState<SkillEvolution[]>([]);
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [feedback, setFeedback] = useState("");

  // Extract form
  const [taskText, setTaskText] = useState("");
  const [responseText, setResponseText] = useState("");
  const [extractResult, setExtractResult] = useState<SkillEvolution | null>(null);

  const loadStatus = useCallback(async () => {
    try {
      const s = await invoke<SisStatus>("sis_status");
      setStatus(s);
    } catch (_) { /* ignore */ }
  }, []);

  const loadMetrics = useCallback(async () => {
    setLoading(true);
    try {
      const m = await invoke<SkillMetrics[]>("sis_get_metrics");
      setMetrics(m);
    } catch (_) { /* ignore */ }
    setLoading(false);
  }, []);

  const loadEvolutions = useCallback(async () => {
    setLoading(true);
    try {
      const evs = await invoke<SkillEvolution[]>("sis_propose_evolutions");
      setEvolutions(evs);
    } catch (_) { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  useEffect(() => {
    if (tab === "metrics") loadMetrics();
    if (tab === "evolutions") loadEvolutions();
  }, [tab, loadMetrics, loadEvolutions]);

  const applyEvolution = async (id: string) => {
    try {
      const msg = await invoke<string>("sis_apply_evolution", { evolutionId: id });
      setFeedback(msg);
      setEvolutions(prev => prev.filter(e => e.id !== id));
      loadStatus();
    } catch (e) {
      setFeedback(`Error: ${e}`);
    }
  };

  const dismissEvolution = (id: string) => {
    setDismissed(prev => new Set([...prev, id]));
  };

  const extractSkill = async () => {
    if (!taskText.trim() || !responseText.trim()) return;
    setLoading(true);
    setExtractResult(null);
    try {
      const ev = await invoke<SkillEvolution | null>("sis_extract_new_skill", {
        taskText,
        responseText,
        sessionId: `manual-${Date.now()}`,
      });
      setExtractResult(ev);
      if (ev) loadStatus();
    } catch (e) {
      setFeedback(`Error: ${e}`);
    }
    setLoading(false);
  };

  const visibleEvolutions = evolutions.filter(e => !dismissed.has(e.id) && !e.applied);

  const TABS: { id: Tab; label: string }[] = [
    { id: "overview",   label: "Overview" },
    { id: "metrics",    label: "Metrics" },
    { id: "evolutions", label: `Evolutions${visibleEvolutions.length ? ` (${visibleEvolutions.length})` : ""}` },
    { id: "extract",    label: "Extract Skill" },
  ];

  return (
    <div className="panel-container">
      <div className="panel-header">
        <Brain size={16} strokeWidth={1.5} style={{ color: "var(--accent-blue, #6c8cff)" }} />
        <h3>Self-Improving Skills</h3>
        <button className="panel-btn panel-btn-secondary" style={{ marginLeft: "auto", fontSize: "var(--font-size-sm)" }}
          onClick={() => { loadStatus(); if (tab === "metrics") loadMetrics(); if (tab === "evolutions") loadEvolutions(); }}>
          <RefreshCw size={12} /> Refresh
        </button>
      </div>

      <div className="panel-tab-bar">
        {TABS.map(t => (
          <button key={t.id} className={`panel-tab${tab === t.id ? " active" : ""}`} onClick={() => setTab(t.id)}>
            {t.label}
          </button>
        ))}
      </div>

      {feedback && (
        <div style={{ padding: "8px 16px", fontSize: "var(--font-size-base)", color: "var(--success-color, #34d399)",
          borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
          {feedback}
          <button onClick={() => setFeedback("")} style={{ marginLeft: 8, fontSize: "var(--font-size-sm)", background: "none",
            border: "none", color: "var(--text-secondary)", cursor: "pointer" }}>✕</button>
        </div>
      )}

      <div className="panel-body">
        {/* ── Overview ──────────────────────────────────────────────── */}
        {tab === "overview" && (
          <>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 12 }}>
              Skills learn from every agent interaction. Accepted responses reinforce a skill;
              rejected or corrected ones trigger evolution proposals.
            </div>
            {status ? <StatusBar status={status} /> : (
              <div className="panel-loading">Loading…</div>
            )}
            <div className="panel-card" style={{ marginTop: 12 }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 8, display: "flex", alignItems: "center", gap: 6 }}>
                <TrendingUp size={14} /> How the loop works
              </div>
              <ol style={{ margin: 0, paddingLeft: 18, fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: 1.9 }}>
                <li>Agent fires a skill on a task → activation recorded</li>
                <li>User accepts, rejects, or corrects the response → outcome recorded</li>
                <li>Engine aggregates outcomes into per-skill health metrics</li>
                <li>Struggling skills (&lt;55 % success) → trigger-refinement proposal</li>
                <li>Thriving skills with corrections → example-addition proposal</li>
                <li>Failing skills (&lt;25 % over 10+ uses) → prune proposal</li>
                <li>Sessions with no matching skill → <em>Extract Skill</em> tab drafts a new one</li>
              </ol>
            </div>
          </>
        )}

        {/* ── Metrics ───────────────────────────────────────────────── */}
        {tab === "metrics" && (
          loading ? <div className="panel-loading">Loading…</div>
          : <MetricsTable metrics={metrics} />
        )}

        {/* ── Evolutions ────────────────────────────────────────────── */}
        {tab === "evolutions" && (
          loading ? <div className="panel-loading">Analysing skills…</div>
          : visibleEvolutions.length === 0 ? (
            <div className="panel-empty">
              <AlertTriangle size={28} strokeWidth={1.5} style={{ color: "var(--text-muted)", marginBottom: 8 }} />
              <div style={{ fontWeight: 600 }}>No pending evolutions</div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>
                VibeCody will propose improvements once enough skill data is collected.
              </div>
            </div>
          ) : (
            <>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 10 }}>
                {visibleEvolutions.length} pending evolution{visibleEvolutions.length !== 1 ? "s" : ""}.
                Apply to write changes to the skill file, or dismiss to skip.
              </div>
              {visibleEvolutions.map(ev => (
                <EvolutionCard key={ev.id} ev={ev} onApply={applyEvolution} onDismiss={dismissEvolution} />
              ))}
            </>
          )
        )}

        {/* ── Extract Skill ─────────────────────────────────────────── */}
        {tab === "extract" && (
          <>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 10 }}>
              Paste a task and an accepted response to draft a new skill file from the interaction.
            </div>
            <label className="panel-label">Task text</label>
            <textarea
              value={taskText}
              onChange={e => setTaskText(e.target.value)}
              placeholder="What the user asked the agent to do…"
              rows={3}
              className="panel-input panel-textarea panel-input-full"
              style={{ resize: "vertical", marginBottom: 8 }}
            />
            <label className="panel-label">Accepted response / key content</label>
            <textarea
              value={responseText}
              onChange={e => setResponseText(e.target.value)}
              placeholder="The agent's accepted output or key patterns from the response…"
              rows={5}
              className="panel-input panel-textarea panel-input-full"
              style={{ resize: "vertical", marginBottom: 10 }}
            />
            <button
              className="panel-btn panel-btn-primary"
              onClick={extractSkill}
              disabled={loading || !taskText.trim() || !responseText.trim()}
              style={{ display: "flex", alignItems: "center", gap: 6 }}
            >
              <Plus size={13} />
              {loading ? "Extracting…" : "Extract New Skill"}
            </button>

            {extractResult && (
              <div className="panel-card" style={{ marginTop: 12 }}>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 6, color: "var(--success-color, #34d399)" }}>
                  Skill draft created: <code style={{ fontSize: "var(--font-size-base)" }}>{extractResult.skill_name}</code>
                </div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>{extractResult.rationale}</div>
                <pre style={{ fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", background: "var(--bg-tertiary)",
                  border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 8,
                  overflowX: "auto", whiteSpace: "pre-wrap", wordBreak: "break-word", maxHeight: 240 }}>
                  {extractResult.proposed_content}
                </pre>
                <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
                  <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)" }}
                    onClick={() => applyEvolution(extractResult.id)}>
                    Save to skills dir
                  </button>
                  <button className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}
                    onClick={() => setExtractResult(null)}>
                    Discard
                  </button>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default SelfImprovingSkillsPanel;
