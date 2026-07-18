/**
 * SkillForgePanel — SkillLens (analyse) + SkillOpt (train) panel.
 *
 * Three views over the daemon's `/v1/skilllens/*` + `/v1/skillopt/*` routes
 * (proxied through Tauri commands):
 *   1. Catalog  — the shipped skills/*.md as a sortable table (no LLM).
 *   2. Lens     — score a skill: trigger-coverage + target-evolvability (LLM).
 *   3. Optimize — train a skill: launch a run, watch the val-curve, promote.
 *
 * Provider-agnostic (STRICT): every LLM call uses the provider+model selected
 * in the panel dropdown, populated from `useModelRegistry()` and seeded by the
 * toolbar `provider` prop. No hard-coded Anthropic. If no model is selected,
 * the Lens/Optimize tabs show a "select a model" empty state. Reference impl:
 * `GitPanel.tsx` + `SweBenchPanel.tsx`.
 *
 * Design: `vibecoder/design-system/README.md` — uses `panel-*` classes + CSS vars.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModelRegistry, PROVIDER_DEFAULT_MODEL } from "../hooks/useModelRegistry";

// ── Types ────────────────────────────────────────────────────────────────────

interface SkillRow {
  name: string;
  category: string;
  summary: string;
  source: string;
  trigger_coverage: number | null;
  extraction_efficacy: number | null;
  target_evolvability: number | null;
}

interface ScoreResult {
  skill: string;
  report: {
    trigger_coverage: number;
    extraction_efficacy: number | null;
    target_evolvability: number | null;
  };
  llm: { provider: string; model: string };
  tasks: number;
}

interface TrainReport {
  skill_name: string;
  epochs_run: number;
  best_val_score: number;
  val_curve: number[];
  accepted: number;
  rejected: number;
  final_tokens: number;
  spent_tokens: number;
  early_stopped: boolean;
  best_skill_md: string;
}

interface TrainJob {
  id: string;
  skill: string;
  llm: { provider: string; model: string };
  state: "running" | "done" | "failed" | "cancelled";
  report?: TrainReport;
  error?: string;
}

// ── Styles ───────────────────────────────────────────────────────────────────

const cardStyle: React.CSSProperties = {
  padding: "12px 16px",
  borderRadius: "var(--radius-md)",
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
};
const metricStyle: React.CSSProperties = {
  fontSize: "var(--font-size-2xl)",
  fontWeight: 700,
  color: "var(--text-primary)",
};
const labelStyle: React.CSSProperties = {
  fontSize: "var(--font-size-xs)",
  color: "var(--text-secondary)",
  textTransform: "uppercase",
  letterSpacing: "0.04em",
};
const barBg: React.CSSProperties = {
  height: 6,
  borderRadius: "var(--radius-xs-plus)",
  background: "var(--bg-tertiary)",
  overflow: "hidden",
};
const barFill = (pct: number, color: string): React.CSSProperties => ({
  height: "100%",
  width: `${Math.min(pct, 100)}%`,
  borderRadius: "var(--radius-xs-plus)",
  background: color,
});

// ── Component ────────────────────────────────────────────────────────────────

type Tab = "catalog" | "lens" | "optimize";

interface SkillForgePanelProps {
  /** Provider from the toolbar dropdown — seeds the in-panel provider picker. */
  provider?: string;
}

export function SkillForgePanel({ provider: providerProp }: SkillForgePanelProps) {
  const { providers, modelsForProvider } = useModelRegistry();
  const [tab, setTab] = useState<Tab>("catalog");

  // Provider/model — STRICT: every LLM call uses these, never a hard-coded default.
  const [selectedProvider, setSelectedProvider] = useState<string>(providerProp || "");
  const [selectedModel, setSelectedModel] = useState<string>("");
  useEffect(() => {
    if (providerProp && providerProp !== selectedProvider) setSelectedProvider(providerProp);
  }, [providerProp]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (!selectedProvider) return;
    const def = PROVIDER_DEFAULT_MODEL[selectedProvider];
    const first = modelsForProvider(selectedProvider)[0];
    setSelectedModel((cur) => cur || def || first || "");
  }, [selectedProvider]); // eslint-disable-line react-hooks/exhaustive-deps

  const availableModels = selectedProvider ? modelsForProvider(selectedProvider) : [];
  const modelSelected = !!selectedProvider && !!selectedModel;

  // Catalog
  const [skills, setSkills] = useState<SkillRow[]>([]);
  const [catalogLoading, setCatalogLoading] = useState(false);
  const [catalogError, setCatalogError] = useState<string | null>(null);

  const loadCatalog = useCallback(async () => {
    setCatalogLoading(true);
    setCatalogError(null);
    try {
      const res = await invoke<{ skills: SkillRow[] }>("skilllens_list_skills");
      setSkills(res.skills || []);
    } catch (e) {
      setCatalogError(String(e));
    } finally {
      setCatalogLoading(false);
    }
  }, []);

  useEffect(() => {
    loadCatalog();
  }, [loadCatalog]);

  // Lens
  const [lensSkill, setLensSkill] = useState<string>("");
  const [scoreResult, setScoreResult] = useState<ScoreResult | null>(null);
  const [scoring, setScoring] = useState(false);
  const [lensError, setLensError] = useState<string | null>(null);

  const runScore = useCallback(async () => {
    if (!lensSkill || !modelSelected) return;
    setScoring(true);
    setLensError(null);
    setScoreResult(null);
    try {
      const res = await invoke<ScoreResult>("skilllens_score", {
        skill: lensSkill,
        provider: selectedProvider,
        model: selectedModel,
      });
      setScoreResult(res);
    } catch (e) {
      setLensError(String(e));
    } finally {
      setScoring(false);
    }
  }, [lensSkill, modelSelected, selectedProvider, selectedModel]);

  // Optimize
  const [optSkill, setOptSkill] = useState<string>("");
  const [epochs, setEpochs] = useState<number>(8);
  const [valSplit, setValSplit] = useState<number>(0.3);
  const [textualLr, setTextualLr] = useState<number>(512);
  const [patience, setPatience] = useState<number>(3);
  const [seed, setSeed] = useState<number>(0);
  const [envKind, setEnvKind] = useState<"repo" | "static">("repo");
  const [envTasks, setEnvTasks] = useState<string>("");
  const [job, setJob] = useState<TrainJob | null>(null);
  const [training, setTraining] = useState(false);
  const [trainError, setTrainError] = useState<string | null>(null);
  const [showPromote, setShowPromote] = useState(false);
  const pollRef = useRef<number | null>(null);

  const stopPoll = () => {
    if (pollRef.current !== null) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
  };
  useEffect(() => () => stopPoll(), []);

  const pollStatus = useCallback(
    async (jobId: string) => {
      try {
        const res = await invoke<TrainJob>("skillopt_status", { jobId });
        setJob(res);
        if (res.state === "running") return;
        stopPoll();
      } catch (e) {
        stopPoll();
        setTrainError(String(e));
      }
    },
    [],
  );

  const launchTrain = useCallback(async () => {
    if (!optSkill || !modelSelected) return;
    setTraining(true);
    setTrainError(null);
    setJob(null);
    setShowPromote(false);
    try {
      const config = {
        epochs,
        val_split: valSplit,
        textual_lr: textualLr,
        patience,
        seed,
      };
      const res = await invoke<{ job_id: string }>("skillopt_train", {
        skill: optSkill,
        envKind,
        envTasks: envKind === "static" ? envTasks : null,
        config,
        provider: selectedProvider,
        model: selectedModel,
      });
      const jobId = res.job_id;
      setJob({ id: jobId, skill: optSkill, llm: { provider: selectedProvider, model: selectedModel }, state: "running" });
      stopPoll();
      pollRef.current = window.setInterval(() => pollStatus(jobId), 1500);
    } catch (e) {
      setTrainError(String(e));
    } finally {
      setTraining(false);
    }
  }, [optSkill, modelSelected, epochs, valSplit, textualLr, patience, seed, envKind, envTasks, selectedProvider, selectedModel, pollStatus]);

  const cancelTrain = useCallback(async () => {
    if (!job) return;
    try {
      await invoke("skillopt_cancel", { jobId: job.id });
      pollStatus(job.id);
    } catch (e) {
      setTrainError(String(e));
    }
  }, [job, pollStatus]);

  const promote = useCallback(async () => {
    if (!job?.report) return;
    try {
      await invoke("skillopt_promote", { skill: job.skill, content: job.report.best_skill_md });
      setShowPromote(false);
      alert(`Promoted ${job.skill} → <workspace>/.vibecli/skills/${job.skill}.opt.md (shipped skills/*.md untouched).`);
    } catch (e) {
      alert(`Promote failed: ${e}`);
    }
  }, [job]);

  // ── Render ─────────────────────────────────────────────────────────────────

  const providerPicker = (
    <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12, flexWrap: "wrap" }}>
      <label className="panel-label" style={{ margin: 0 }}>Provider</label>
      <select
        className="panel-select"
        value={selectedProvider}
        onChange={(e) => { setSelectedProvider(e.target.value); setSelectedModel(""); }}
      >
        <option value="">— select —</option>
        {providers.map((p) => <option key={p} value={p}>{p}</option>)}
      </select>
      <label className="panel-label" style={{ margin: 0 }}>Model</label>
      <select
        className="panel-select"
        value={selectedModel}
        onChange={(e) => setSelectedModel(e.target.value)}
        disabled={availableModels.length === 0}
      >
        <option value="">— select —</option>
        {availableModels.map((m) => <option key={m} value={m}>{m}</option>)}
      </select>
    </div>
  );

  const noModelBanner = !modelSelected && (
    <div className="panel-empty" style={{ padding: 12, marginBottom: 12 }}>
      Select a provider and model to run SkillLens scoring or SkillOpt training.
    </div>
  );

  return (
    <div className="panel-container">
      <div className="panel-tab-bar">
        {(["catalog", "lens", "optimize"] as Tab[]).map((t) => (
          <button
            key={t}
            className={`panel-tab${tab === t ? " active" : ""}`}
            onClick={() => setTab(t)}
          >
            {t === "catalog" ? "Catalog" : t === "lens" ? "Lens" : "Optimize"}
          </button>
        ))}
        <div style={{ flex: 1 }} />
        <button className="panel-btn" onClick={loadCatalog} disabled={catalogLoading}>
          {catalogLoading ? "Refreshing…" : "Refresh catalog"}
        </button>
      </div>

      {tab !== "catalog" && providerPicker}
      {tab !== "catalog" && noModelBanner}

      {tab === "catalog" && (
        <div>
          {catalogError && (
            <div className="panel-empty" style={{ padding: 12, color: "var(--error-color)" }}>
              {catalogError}
            </div>
          )}
          <p style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", margin: "4px 0 8px" }}>
            {skills.length} skills loaded from the bundled <code>skills/*.md</code> tree. No LLM required.
          </p>
          <table className="panel-table" style={{ width: "100%" }}>
            <thead>
              <tr>
                <th style={{ textAlign: "left" }}>Skill</th>
                <th>Category</th>
                <th>Coverage</th>
                <th>Evolvability</th>
                <th>Source</th>
              </tr>
            </thead>
            <tbody>
              {skills.slice(0, 200).map((s) => (
                <tr
                  key={s.name}
                  style={{ cursor: "pointer" }}
                  onClick={() => { setLensSkill(s.name); setOptSkill(s.name); setTab("lens"); }}
                  title="Click to open in Lens"
                >
                  <td>{s.name}</td>
                  <td style={{ color: "var(--text-secondary)" }}>{s.category}</td>
                  <td style={{ textAlign: "center" }}>{fmtPct(s.trigger_coverage)}</td>
                  <td style={{ textAlign: "center" }}>{fmtPct(s.target_evolvability)}</td>
                  <td style={{ color: "var(--text-secondary)", textAlign: "center" }}>{s.source}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {skills.length > 200 && (
            <p style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", marginTop: 8 }}>
              Showing first 200 of {skills.length}. Use Lens to inspect a specific skill by name.
            </p>
          )}
        </div>
      )}

      {tab === "lens" && (
        <div>
          <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12 }}>
            <label className="panel-label" style={{ margin: 0 }}>Skill</label>
            <input
              className="panel-input-full"
              style={{ flex: 1, minWidth: 200 }}
              list="skillforge-skill-list"
              value={lensSkill}
              onChange={(e) => setLensSkill(e.target.value)}
              placeholder="e.g. formal-verification"
            />
            <datalist id="skillforge-skill-list">
              {skills.map((s) => <option key={s.name} value={s.name} />)}
            </datalist>
            <button
              className="panel-btn-primary"
              onClick={runScore}
              disabled={!lensSkill || !modelSelected || scoring}
            >
              {scoring ? "Scoring…" : "Score"}
            </button>
          </div>
          {lensError && (
            <div className="panel-empty" style={{ padding: 12, color: "var(--error-color)" }}>{lensError}</div>
          )}
          {scoreResult && (
            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12 }}>
              <MetricCard label="Trigger Coverage" value={scoreResult.report.trigger_coverage} hint="deterministic · no LLM" color="var(--info-color)" />
              <MetricCard label="Target Evolvability" value={scoreResult.report.target_evolvability} hint={`held-out lift · ${scoreResult.llm.model}`} color="var(--success-color)" />
              <MetricCard label="Extraction Efficacy" value={scoreResult.report.extraction_efficacy} hint="needs a pool · Phase 4" color="var(--warning-color)" />
              <div style={{ gridColumn: "1 / -1", ...cardStyle }}>
                <div style={labelStyle}>Scored against</div>
                <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
                  {scoreResult.tasks} task(s) · provider <code>{scoreResult.llm.provider}</code> · model <code>{scoreResult.llm.model}</code>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {tab === "optimize" && (
        <div>
          <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12, flexWrap: "wrap" }}>
            <label className="panel-label" style={{ margin: 0 }}>Skill</label>
            <input
              className="panel-input-full"
              style={{ flex: 1, minWidth: 200 }}
              list="skillforge-skill-list"
              value={optSkill}
              onChange={(e) => setOptSkill(e.target.value)}
              placeholder="skill to train"
            />
            <select className="panel-select" value={envKind} onChange={(e) => setEnvKind(e.target.value as "repo" | "static")}>
              <option value="repo">Env: repo (catalog)</option>
              <option value="static">Env: static (JSONL)</option>
            </select>
          </div>
          {envKind === "static" && (
            <textarea
              className="panel-input-full"
              style={{ width: "100%", minHeight: 80, fontFamily: "var(--font-mono, monospace)", marginBottom: 12 }}
              placeholder={'One EvalTask per line, e.g. {"id":"t1","prompt":"...","grader":{"kind":"contains","value":"..."}}'}
              value={envTasks}
              onChange={(e) => setEnvTasks(e.target.value)}
            />
          )}
          <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 8, marginBottom: 12 }}>
            <NumField label="Epochs" value={epochs} onChange={setEpochs} />
            <NumField label="Val split" value={valSplit} onChange={setValSplit} step={0.05} />
            <NumField label="Textual LR" value={textualLr} onChange={setTextualLr} />
            <NumField label="Patience" value={patience} onChange={setPatience} />
            <NumField label="Seed" value={seed} onChange={setSeed} />
          </div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <button
              className="panel-btn-primary"
              onClick={launchTrain}
              disabled={!optSkill || !modelSelected || training}
            >
              {training ? "Launching…" : "Train"}
            </button>
            <button
              className="panel-btn"
              onClick={cancelTrain}
              disabled={!job || job.state !== "running"}
            >
              Cancel
            </button>
          </div>
          {trainError && (
            <div className="panel-empty" style={{ padding: 12, color: "var(--error-color)" }}>{trainError}</div>
          )}
          {job && <JobView job={job} onPromote={() => setShowPromote(true)} />}
          {job && showPromote && job.report && (
            <div style={{ ...cardStyle, marginTop: 12 }}>
              <div style={labelStyle}>Promote — writes {job.skill}.opt.md to the override dir (shipped skills/*.md untouched)</div>
              <p style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", margin: "6px 0" }}>
                Promoting writes the trained artifact to the per-workspace override dir
                (<code>&lt;workspace&gt;/.vibecli/skills/{job.skill}.opt.md</code>) so the shipped
                <code> skills/*.md</code> tree stays pristine. The live loader is not changed —
                swapping a promoted skill into the agent is a separate, deliberate action.
              </p>
              <div style={{ display: "flex", gap: 8 }}>
                <button className="panel-btn-primary" onClick={promote}>Promote {job.skill}.opt.md</button>
                <button className="panel-btn" onClick={() => setShowPromote(false)}>Dismiss</button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Sub-components ───────────────────────────────────────────────────────────

function MetricCard({ label, value, hint, color }: { label: string; value: number | null; hint: string; color: string }) {
  const pct = value == null ? 0 : value * 100;
  return (
    <div style={cardStyle}>
      <div style={labelStyle}>{label}</div>
      <div style={metricStyle}>{fmtPct(value)}</div>
      <div style={{ ...barBg, marginTop: 8 }}>
        <div style={barFill(pct, color)} />
      </div>
      <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 6 }}>{hint}</div>
    </div>
  );
}

function NumField({ label, value, onChange, step = 1 }: { label: string; value: number; onChange: (v: number) => void; step?: number }) {
  return (
    <div>
      <label className="panel-label">{label}</label>
      <input
        className="panel-input-full"
        type="number"
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  );
}

function JobView({ job, onPromote }: { job: TrainJob; onPromote: () => void }) {
  const stateColor: Record<string, string> = {
    running: "var(--info-color)",
    done: "var(--success-color)",
    failed: "var(--error-color)",
    cancelled: "var(--text-secondary)",
  };
  return (
    <div style={cardStyle}>
      <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 8 }}>
        <span style={{ fontWeight: 700 }}>{job.skill}</span>
        <span style={{ padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-primary)", background: stateColor[job.state] }}>
          {job.state}
        </span>
        <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
          {job.llm.provider} · {job.llm.model}
        </span>
      </div>
      {job.state === "failed" && job.error && (
        <div style={{ color: "var(--error-color)", fontSize: "var(--font-size-sm)" }}>{job.error}</div>
      )}
      {job.report && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8, marginBottom: 12 }}>
            <Stat label="Epochs run" value={String(job.report.epochs_run)} />
            <Stat label="Best val" value={job.report.best_val_score.toFixed(3)} />
            <Stat label="Accepted / Rejected" value={`${job.report.accepted} / ${job.report.rejected}`} />
            <Stat label="Spent tokens" value={fmtTokens(job.report.spent_tokens)} />
          </div>
          {job.report.early_stopped && (
            <p style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", margin: "0 0 8px" }}>
              Early-stopped — no held-out val gain for the configured patience window.
            </p>
          )}
          <div style={labelStyle}>Validation curve</div>
          <ValCurve curve={job.report.val_curve} />
          <details style={{ marginTop: 12 }}>
            <summary style={{ cursor: "pointer", color: "var(--text-secondary)" }}>Trained skill markdown ({job.report.best_skill_md.length} chars)</summary>
            <pre style={{ whiteSpace: "pre-wrap", fontSize: "var(--font-size-xs)", maxHeight: 300, overflow: "auto", background: "var(--bg-tertiary)", padding: 8, borderRadius: "var(--radius-sm)" }}>
              {job.report.best_skill_md}
            </pre>
          </details>
          {job.state === "done" && (
            <div style={{ marginTop: 12 }}>
              <button className="panel-btn-primary" onClick={onPromote}>Promote…</button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div style={labelStyle}>{label}</div>
      <div style={{ fontWeight: 600, color: "var(--text-primary)" }}>{value}</div>
    </div>
  );
}

function ValCurve({ curve }: { curve: number[] }) {
  if (curve.length === 0) {
    return <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>No epochs run.</div>;
  }
  const max = Math.max(...curve, 1);
  const w = 240;
  const h = 60;
  const step = curve.length > 1 ? w / (curve.length - 1) : 0;
  const points = curve
    .map((v, i) => `${(i * step).toFixed(1)},${(h - (v / max) * h).toFixed(1)}`)
    .join(" ");
  return (
    <svg width={w} height={h} style={{ background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)", marginTop: 4 }}>
      <polyline points={points} fill="none" stroke="var(--success-color)" strokeWidth={2} />
      {curve.map((v, i) => (
        <circle key={i} cx={i * step} cy={h - (v / max) * h} r={2} fill="var(--success-color)" />
      ))}
    </svg>
  );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function fmtPct(v: number | null): string {
  if (v == null) return "—";
  return `${(v * 100).toFixed(1)}%`;
}

function fmtTokens(n: number): string {
  if (n < 1000) return String(n);
  return `${(n / 1000).toFixed(1)}k`;
}