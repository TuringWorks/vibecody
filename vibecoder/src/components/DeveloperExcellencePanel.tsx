/**
 * DeveloperExcellencePanel — delivery performance and practice maturity for the
 * workspace, measured rather than asserted.
 *
 * This panel is the face of `vibecli`'s `/devex/*` routes. It replaces the one
 * thing VibeCoder used to get wrong about engineering metrics: the IDP
 * scorecard awarded every service 8/10 for deploy frequency and 7/10 for lead
 * time from constants in `commands.rs`, labelled "simulated DORA-style
 * metrics". A director reading that was being told their delivery performance
 * by a number nobody measured.
 *
 * Three rules it inherits from the daemon and must not soften on the way to the
 * screen:
 *
 *  1. **An unmeasurable metric is not zero.** It renders in its own section,
 *     with the reason it could not be measured and the concrete change that
 *     would make it measurable. A tile showing `0` for a metric nobody
 *     instrumented is a lie the dashboard tells on the operator's behalf.
 *  2. **Every value carries its proxy and its sample size.** "2.0
 *     deployments/week" from two observations and from two hundred are
 *     different claims.
 *  3. **Detected maturity is capped at 'defined'.** A file proves a practice
 *     exists, not that it is followed. The top level is attested by people, and
 *     this panel says so on screen rather than in a tooltip.
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  Activity,
  CircleHelp,
  ClipboardCheck,
  FileText,
  Gauge,
  RefreshCw,
  UserPlus,
  Users,
} from "lucide-react";
// Daemon routes are behind `require_auth`; a plain fetch() 401s. See daemonFetch.ts.
import { daemonFetch } from "../lib/daemonFetch";

// ── Types (mirror `devex_metrics.rs` serde shapes) ──────────────────────────

type Band = "elite" | "high" | "medium" | "low";

interface Percentile {
  label: string;
  value: number;
}

interface Measure {
  value: number;
  unit: string;
  band: Band;
  sample_size: number;
  proxy: string;
  percentiles?: Percentile[];
}

interface Unmeasured {
  metric: string;
  reason: string;
  to_measure_this: string;
}

interface Deployment {
  name: string;
  commit: string;
  at: number;
  followed_by_remediation: boolean;
}

interface DoraReport {
  repo: string;
  window_days: number;
  since: number;
  generated_at: number;
  release_marker: string;
  release_marker_description: string;
  band_source: string;
  deployment_frequency?: Measure;
  lead_time_for_changes?: Measure;
  change_failure_rate?: Measure;
  time_to_restore?: Measure;
  unmeasured: Unmeasured[];
  deployments: Deployment[];
  commits_in_window: number;
  authors_in_window: number;
  notes: string[];
}

interface Signal {
  name: string;
  found: boolean;
  path?: string;
}

interface PracticeResult {
  key: string;
  title: string;
  pillar: string;
  signals: Signal[];
  found: number;
  expected: number;
  level: number;
  level_name: string;
  next_step: string;
  /**
   * A known limit of this practice's detection. Rendered beside the missing
   * signals, never in a footnote: "missing: test directory" on a repository
   * with thousands of inline Rust tests reads as a finding until the caveat is
   * in the same glance.
   */
  detection_caveat?: string;
}

interface PracticesReport {
  workspace: string;
  generated_at: number;
  practices: PracticeResult[];
  mean_level: number;
  max_detectable_level: number;
  scope_note: string;
}

interface NewContributor {
  author: string;
  first_commit_at: number;
  hours_to_second_commit?: number;
  commits_in_window: number;
}

interface OnboardingReport {
  repo: string;
  window_days: number;
  readiness: Signal[];
  readiness_found: number;
  readiness_expected: number;
  new_contributors: NewContributor[];
  not_measured: Unmeasured[];
  notes: string[];
}

interface Scorecard {
  dora: DoraReport;
  practices: PracticesReport;
  dora_coverage: number;
  delivery_grade?: string;
  headline: string;
}

// ── SPACE ────────────────────────────────────────────────────────────────────

interface SpaceMeasure {
  name: string;
  value: number;
  unit: string;
  source: string;
  sample_size: number;
  caveat?: string;
}

interface SpaceDimensionResult {
  dimension: string;
  key: string;
  title: string;
  measures: SpaceMeasure[];
  unmeasured: Unmeasured[];
}

interface SpaceReport {
  repo: string;
  window_days: number;
  dimensions: SpaceDimensionResult[];
  dimensions_measured: number;
  /** False when Performance has no measure — nothing says whether what shipped worked. */
  outcome_signal: boolean;
  scope_note: string;
}

export interface DeveloperExcellencePanelProps {
  workspacePath?: string | null;
  daemonUrl?: string;
}

const DAEMON_DEFAULT = "http://localhost:7878";

type TabId = "delivery" | "space" | "practices" | "onboarding";

const TABS: ReadonlyArray<{ id: TabId; label: string; icon: React.ReactNode }> = [
  { id: "delivery", label: "Delivery (DORA)", icon: <Gauge size={12} strokeWidth={1.5} /> },
  { id: "space", label: "Experience (SPACE)", icon: <Users size={12} strokeWidth={1.5} /> },
  { id: "practices", label: "Practices", icon: <ClipboardCheck size={12} strokeWidth={1.5} /> },
  { id: "onboarding", label: "Onboarding", icon: <UserPlus size={12} strokeWidth={1.5} /> },
];

/** Window presets. 90 days is the daemon's own default. */
const WINDOWS: ReadonlyArray<number> = [30, 90, 180, 365];

/** Human label for a DORA key, so the payload's snake_case never reaches the eye. */
const METRIC_LABELS: Readonly<Record<string, string>> = {
  deployment_frequency: "Deployment frequency",
  lead_time_for_changes: "Lead time for changes",
  change_failure_rate: "Change failure rate",
  time_to_restore: "Time to restore",
  time_to_first_commit: "Time to first commit",
  tooling_satisfaction: "Tooling satisfaction",
  delivery_stability: "Delivery stability",
  review_latency: "Review latency",
  pipeline_wait: "Pipeline wait",
  uninterrupted_focus_hours: "Uninterrupted focus hours",
  file_co_ownership: "File co-ownership",
};

function metricLabel(key: string): string {
  return METRIC_LABELS[key] ?? key;
}

/**
 * Colour for a band. Deliberately not a red/green pair: the bands are a
 * position in an industry distribution, not a pass/fail, and painting "low"
 * red invites exactly the league-table reading the metrics must not get.
 */
function bandColor(band: Band): string {
  switch (band) {
    case "elite":
      return "var(--accent-success, #3fb950)";
    case "high":
      return "var(--accent-primary, #7c6aef)";
    case "medium":
      return "var(--accent-warning, #d29922)";
    case "low":
      return "var(--text-secondary, #8b949e)";
    default: {
      // Exhaustiveness: a new band added to the daemon must be handled here
      // rather than falling through to an arbitrary colour.
      const never: never = band;
      return never;
    }
  }
}

/** `1.5` → `1.5`, `1.50` → `1.5`, but keep two places where they carry meaning. */
function formatValue(value: number): string {
  if (!Number.isFinite(value)) return "—";
  if (Math.abs(value) >= 100) return value.toFixed(0);
  if (Math.abs(value) >= 10) return value.toFixed(1);
  return value.toFixed(2);
}

function formatDate(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleDateString();
}

// ── Panel ───────────────────────────────────────────────────────────────────

export function DeveloperExcellencePanel({
  workspacePath = null,
  daemonUrl = DAEMON_DEFAULT,
}: DeveloperExcellencePanelProps) {
  const [tab, setTab] = useState<TabId>("delivery");
  const [windowDays, setWindowDays] = useState<number>(90);
  const [marker, setMarker] = useState<"tags" | "merges">("tags");
  const [scorecard, setScorecard] = useState<Scorecard | null>(null);
  const [onboarding, setOnboarding] = useState<OnboardingReport | null>(null);
  const [space, setSpace] = useState<SpaceReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exportDoc, setExportDoc] = useState<string | null>(null);

  const query = useMemo(() => {
    if (!workspacePath) return null;
    const params = new URLSearchParams({
      path: workspacePath,
      window: String(windowDays),
      marker,
    });
    return params.toString();
  }, [workspacePath, windowDays, marker]);

  const request = useCallback(
    async (path: string) => {
      const res = await daemonFetch(`${daemonUrl}${path}`);
      if (!res.ok) {
        // Surface the daemon's own message. The /devex routes answer 400 with
        // the specific reason — "`path` is required", "unknown marker" — and a
        // generic "request failed" would hide the only useful part.
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

  const load = useCallback(async () => {
    if (!query) return;
    setLoading(true);
    setError(null);
    try {
      const [scRes, obRes, spRes] = await Promise.all([
        request(`/devex/scorecard?${query}`),
        request(`/devex/onboarding?${query}`),
        request(`/devex/space?${query}`),
      ]);
      const scBody = await scRes.json();
      const obBody = await obRes.json();
      const spBody = await spRes.json();
      setScorecard(scBody.scorecard ?? null);
      setOnboarding(obBody.onboarding ?? null);
      setSpace(spBody.space ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      // The previous run's numbers are cleared on failure. Leaving a stale
      // scorecard on screen next to a fresh window selector would attribute
      // old measurements to a window they did not come from.
      setScorecard(null);
      setOnboarding(null);
      setSpace(null);
    } finally {
      setLoading(false);
    }
  }, [query, request]);

  useEffect(() => {
    void load();
  }, [load]);

  const showReport = useCallback(async () => {
    if (!query) return;
    try {
      const res = await request(`/devex/scorecard.md?${query}`);
      setExportDoc(await res.text());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [query, request]);

  const showSurvey = useCallback(async () => {
    try {
      const res = await request(`/devex/survey.md`);
      setExportDoc(await res.text());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [request]);

  const measured: ReadonlyArray<[string, Measure]> = useMemo(() => {
    const d = scorecard?.dora;
    if (!d) return [];
    return (
      [
        ["deployment_frequency", d.deployment_frequency],
        ["lead_time_for_changes", d.lead_time_for_changes],
        ["change_failure_rate", d.change_failure_rate],
        ["time_to_restore", d.time_to_restore],
      ] as ReadonlyArray<[string, Measure | undefined]>
    ).flatMap(([k, m]) => (m ? [[k, m] as [string, Measure]] : []));
  }, [scorecard]);

  if (!workspacePath) {
    return (
      <div className="panel-container">
        <div className="panel-header">
          <h3>
            <Activity size={14} strokeWidth={1.5} style={{ verticalAlign: "-2px", marginRight: 6 }} />
            Developer Excellence
          </h3>
        </div>
        <div className="panel-body">
          <div className="panel-empty">
            Open a workspace to measure it. This panel will not guess a
            directory and label the result yours.
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>
          <Activity size={14} strokeWidth={1.5} style={{ verticalAlign: "-2px", marginRight: 6 }} />
          Developer Excellence
        </h3>
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <select
            className="panel-select panel-select-xs"
            value={windowDays}
            onChange={(e) => setWindowDays(Number(e.target.value))}
            aria-label="Measurement window"
            title="Measurement window"
          >
            {WINDOWS.map((d) => (
              <option key={d} value={d}>
                {d} days
              </option>
            ))}
          </select>
          <select
            className="panel-select panel-select-xs"
            value={marker}
            onChange={(e) => setMarker(e.target.value === "merges" ? "merges" : "tags")}
            aria-label="How deployments are identified"
            title="How deployments are identified"
          >
            <option value="tags">Deploys = version tags</option>
            <option value="merges">Deploys = branch merges</option>
          </select>
          <button
            className="panel-btn panel-btn-secondary panel-btn-xs"
            onClick={showReport}
            title="The scorecard as a markdown briefing"
          >
            <FileText size={12} strokeWidth={1.5} /> Report
          </button>
          <button
            className="panel-btn panel-btn-secondary panel-btn-xs"
            onClick={() => void load()}
            disabled={loading}
            title="Re-measure from the daemon"
          >
            <RefreshCw size={12} strokeWidth={1.5} /> {loading ? "Measuring…" : "Refresh"}
          </button>
        </div>
      </div>

      <div className="panel-body">
        {error && (
          <div className="panel-error" role="alert">
            {error}
          </div>
        )}

        {scorecard && (
          <div className="panel-card" style={{ marginBottom: 12 }}>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>{scorecard.headline}</div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
              DORA coverage {Math.round(scorecard.dora_coverage * 100)}% ·{" "}
              {scorecard.dora.commits_in_window} commits ·{" "}
              {scorecard.dora.authors_in_window} authors ·{" "}
              deployments identified by {scorecard.dora.release_marker_description}
            </div>
          </div>
        )}

        <div className="panel-tabs" style={{ display: "flex", gap: 6, marginBottom: 10 }}>
          {TABS.map((t) => (
            <button
              key={t.id}
              className={`panel-btn panel-btn-xs ${tab === t.id ? "panel-btn-primary" : "panel-btn-secondary"}`}
              onClick={() => setTab(t.id)}
            >
              {t.icon} {t.label}
            </button>
          ))}
        </div>

        {tab === "delivery" && scorecard && (
          <>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: 8 }}>
              {measured.map(([key, m]) => (
                <div key={key} className="panel-card">
                  <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{metricLabel(key)}</div>
                  <div style={{ fontSize: 20, fontWeight: 600, color: bandColor(m.band) }}>
                    {formatValue(m.value)}{" "}
                    <span style={{ fontSize: 11, fontWeight: 400, color: "var(--text-secondary)" }}>
                      {m.unit}
                    </span>
                  </div>
                  <div style={{ fontSize: 11 }}>
                    <span style={{ color: bandColor(m.band), textTransform: "capitalize" }}>{m.band}</span>
                    <span style={{ color: "var(--text-secondary)" }}> · n={m.sample_size}</span>
                  </div>
                  {m.percentiles && m.percentiles.length > 0 && (
                    <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                      {m.percentiles.map((p) => `${p.label} ${formatValue(p.value)}`).join(" · ")}
                    </div>
                  )}
                  <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }} title={m.proxy}>
                    proxy: {m.proxy}
                  </div>
                </div>
              ))}
            </div>

            {scorecard.dora.unmeasured.length > 0 && (
              <div className="panel-card" style={{ marginTop: 12 }}>
                <div style={{ fontWeight: 600, marginBottom: 6 }}>
                  <CircleHelp size={12} strokeWidth={1.5} style={{ verticalAlign: "-2px", marginRight: 4 }} />
                  Not measured — and why
                </div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
                  These are absent, not zero. A metric with no signal is a gap in
                  instrumentation, not a result.
                </div>
                {scorecard.dora.unmeasured.map((u) => (
                  <div key={u.metric} style={{ marginBottom: 8 }}>
                    <div style={{ fontWeight: 600, fontSize: 12 }}>{metricLabel(u.metric)}</div>
                    <div style={{ fontSize: 11 }}>{u.reason}</div>
                    <div style={{ fontSize: 11, color: "var(--accent-primary)" }}>
                      To measure it: {u.to_measure_this}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {scorecard.dora.deployments.length > 0 && (
              <div className="panel-card" style={{ marginTop: 12 }}>
                <div style={{ fontWeight: 600, marginBottom: 6 }}>
                  Deployments in window ({scorecard.dora.deployments.length})
                </div>
                <div style={{ maxHeight: 200, overflowY: "auto", fontSize: 11 }}>
                  {scorecard.dora.deployments
                    .slice()
                    .reverse()
                    .map((d) => (
                      <div
                        key={`${d.commit}-${d.at}`}
                        style={{ display: "flex", justifyContent: "space-between", padding: "2px 0" }}
                      >
                        <span style={{ fontFamily: "var(--font-mono, monospace)" }}>{d.name}</span>
                        <span style={{ color: "var(--text-secondary)" }}>
                          {formatDate(d.at)}
                          {d.followed_by_remediation ? " · remediated" : ""}
                        </span>
                      </div>
                    ))}
                </div>
              </div>
            )}

            {scorecard.dora.notes.length > 0 && (
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 10 }}>
                {scorecard.dora.notes.map((n) => (
                  <div key={n}>note: {n}</div>
                ))}
              </div>
            )}

            <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 10 }}>
              {scorecard.dora.band_source}
            </div>
          </>
        )}

        {tab === "space" && space && (
          <>
            {!space.outcome_signal && (
              <div className="panel-error" role="alert" style={{ marginBottom: 8 }}>
                No outcome signal: nothing here says whether what shipped worked, because no DORA
                stability metric could be computed. Activity and Collaboration describe volume and
                shape — read without an outcome they are not a picture of productivity. Fix the
                Delivery tab&apos;s unmeasured block first.
              </div>
            )}
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
              {space.scope_note}
            </div>
            {space.dimensions.map((d) => (
              <div key={d.key} className="panel-card" style={{ marginBottom: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
                  <span style={{ fontWeight: 600 }}>{d.title}</span>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                    {d.measures.length === 0
                      ? "no measure from this repository"
                      : `${d.measures.length} measure${d.measures.length === 1 ? "" : "s"}`}
                  </span>
                </div>
                {d.measures.map((m) => (
                  <div key={m.name} style={{ fontSize: 11, marginTop: 6 }}>
                    <div>
                      <strong>{formatValue(m.value)}</strong>{" "}
                      <span style={{ color: "var(--text-secondary)" }}>{m.unit}</span> — {m.name}
                    </div>
                    <div style={{ color: "var(--text-secondary)", fontSize: 10 }}>
                      source: {m.source} · n={m.sample_size}
                    </div>
                    {m.caveat && (
                      <div style={{ color: "var(--text-secondary)", fontSize: 10, fontStyle: "italic" }}>
                        {m.caveat}
                      </div>
                    )}
                  </div>
                ))}
                {d.unmeasured.map((u) => (
                  <div key={u.metric} style={{ fontSize: 11, marginTop: 6 }}>
                    <div style={{ fontWeight: 600 }}>
                      <CircleHelp size={11} strokeWidth={1.5} style={{ verticalAlign: "-1px", marginRight: 4 }} />
                      {metricLabel(u.metric)} — not measured here
                    </div>
                    <div>{u.reason}</div>
                    <div style={{ color: "var(--accent-primary)" }}>To measure it: {u.to_measure_this}</div>
                  </div>
                ))}
              </div>
            ))}
            <button
              className="panel-btn panel-btn-secondary panel-btn-xs"
              onClick={showSurvey}
              title="The quarterly experience-survey instrument"
            >
              <FileText size={12} strokeWidth={1.5} /> Survey instrument
            </button>
          </>
        )}

        {tab === "practices" && scorecard && (
          <>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>
              {scorecard.practices.scope_note}
            </div>
            {scorecard.practices.practices.map((p) => (
              <div key={p.key} className="panel-card" style={{ marginBottom: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
                  <span style={{ fontWeight: 600 }}>{p.title}</span>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                    level {p.level}/{scorecard.practices.max_detectable_level} ({p.level_name}) ·{" "}
                    {p.found}/{p.expected} signals
                  </span>
                </div>
                <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>{p.pillar}</div>
                <div style={{ fontSize: 11, marginTop: 4 }}>
                  {p.signals.map((s) => (
                    <div key={s.name} style={{ color: s.found ? "var(--text-primary)" : "var(--text-secondary)" }}>
                      {s.found ? "✓" : "○"} {s.name}
                      {s.path ? ` — ${s.path}` : ""}
                    </div>
                  ))}
                </div>
                {p.detection_caveat && (
                  <div
                    style={{
                      fontSize: 11,
                      color: "var(--text-secondary)",
                      marginTop: 4,
                      fontStyle: "italic",
                    }}
                  >
                    What this cannot see: {p.detection_caveat}
                  </div>
                )}
                {p.found < p.expected && (
                  <div style={{ fontSize: 11, color: "var(--accent-primary)", marginTop: 4 }}>{p.next_step}</div>
                )}
              </div>
            ))}
          </>
        )}

        {tab === "onboarding" && onboarding && (
          <>
            <div className="panel-card" style={{ marginBottom: 8 }}>
              <div style={{ fontWeight: 600 }}>
                Bootstrap readiness — {onboarding.readiness_found}/{onboarding.readiness_expected} signals
              </div>
              <div style={{ fontSize: 11, marginTop: 4 }}>
                {onboarding.readiness.map((s) => (
                  <div key={s.name} style={{ color: s.found ? "var(--text-primary)" : "var(--text-secondary)" }}>
                    {s.found ? "✓" : "○"} {s.name}
                    {s.path ? ` — ${s.path}` : ""}
                  </div>
                ))}
              </div>
              {onboarding.notes.map((n) => (
                <div key={n} style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 6 }}>
                  {n}
                </div>
              ))}
            </div>

            {onboarding.not_measured.map((u) => (
              <div key={u.metric} className="panel-card" style={{ marginBottom: 8 }}>
                <div style={{ fontWeight: 600, fontSize: 12 }}>
                  <CircleHelp size={12} strokeWidth={1.5} style={{ verticalAlign: "-2px", marginRight: 4 }} />
                  {metricLabel(u.metric)} — not measured
                </div>
                <div style={{ fontSize: 11 }}>{u.reason}</div>
                <div style={{ fontSize: 11, color: "var(--accent-primary)" }}>
                  To measure it: {u.to_measure_this}
                </div>
              </div>
            ))}

            <div className="panel-card">
              <div style={{ fontWeight: 600, marginBottom: 6 }}>
                First-time contributors in the last {onboarding.window_days} days (
                {onboarding.new_contributors.length})
              </div>
              {onboarding.new_contributors.length === 0 ? (
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                  Nobody committed here for the first time in this window.
                </div>
              ) : (
                <div style={{ maxHeight: 240, overflowY: "auto", fontSize: 11 }}>
                  {onboarding.new_contributors.map((c) => (
                    <div
                      key={c.author}
                      style={{ display: "flex", justifyContent: "space-between", padding: "2px 0" }}
                    >
                      <span>{c.author}</span>
                      <span style={{ color: "var(--text-secondary)" }}>
                        {formatDate(c.first_commit_at)} · {c.commits_in_window} commits ·{" "}
                        {c.hours_to_second_commit === undefined
                          ? "no second commit yet"
                          : `${formatValue(c.hours_to_second_commit)}h to second`}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </>
        )}

        {exportDoc !== null && (
          <div className="panel-card" style={{ marginTop: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
              <span style={{ fontWeight: 600 }}>Scorecard briefing (markdown)</span>
              <span style={{ display: "flex", gap: 6 }}>
                <button
                  className="panel-btn panel-btn-secondary panel-btn-xs"
                  onClick={() => void navigator.clipboard?.writeText(exportDoc)}
                >
                  Copy
                </button>
                <button
                  className="panel-btn panel-btn-secondary panel-btn-xs"
                  onClick={() => setExportDoc(null)}
                >
                  Close
                </button>
              </span>
            </div>
            <pre
              style={{
                maxHeight: 320,
                overflow: "auto",
                fontSize: 11,
                whiteSpace: "pre-wrap",
                fontFamily: "var(--font-mono, monospace)",
              }}
            >
              {exportDoc}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}

export default DeveloperExcellencePanel;
