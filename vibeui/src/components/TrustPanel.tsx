import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

/* ── Types matching Tauri return shapes ─────────────────────────────── */

interface CostEntry {
  session_id: string;
  provider: string;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  cost_usd: number;
  timestamp_ms: number;
  task_hint: string | null;
}

interface CostMetrics {
  entries: CostEntry[];
  by_provider: { provider: string; total_cost_usd: number; total_tokens: number; call_count: number }[];
  total_cost_usd: number;
  total_tokens: number;
  budget_limit_usd: number | null;
  budget_remaining_usd: number | null;
}

interface TrustTraceEntry {
  session_id: string;
  timestamp: number;
  tool: string;
  success: boolean;
  duration_ms: number;
}

/* ── Derived types ──────────────────────────────────────────────────── */

interface ModelScore {
  model: string;
  provider: string;
  score: number;
  tasks: number;
  successes: number;
  failures: number;
  avgDurationMs: number;
}

interface TrustEvent {
  model: string;
  tool: string;
  success: boolean;
  delta: number;
  timestamp: number;
}

interface DomainScores {
  domain: string;
  scores: Record<string, number>;
}

/* ── Config keys for localStorage persistence ──────────────────────── */
const TRUST_CONFIG_KEY = "vibecody:trust-config";

interface TrustConfig {
  decayRate: number;
  recoveryRate: number;
  autoMergeThreshold: number;
  manualReviewThreshold: number;
}

const DEFAULT_CONFIG: TrustConfig = {
  decayRate: 5,
  recoveryRate: 10,
  autoMergeThreshold: 85,
  manualReviewThreshold: 50,
};

/* ── Helpers ────────────────────────────────────────────────────────── */

/** Map tool names to trust domains */
function toolDomain(tool: string): string {
  const t = tool.toLowerCase();
  if (t.includes("test") || t.includes("assert")) return "Testing";
  if (t.includes("edit") || t.includes("write") || t.includes("patch") || t.includes("refactor")) return "Code Generation";
  if (t.includes("fix") || t.includes("debug") || t.includes("diagnose")) return "Bug Fixing";
  if (t.includes("doc") || t.includes("readme") || t.includes("comment")) return "Documentation";
  if (t.includes("read") || t.includes("search") || t.includes("grep") || t.includes("glob")) return "Research";
  if (t.includes("bash") || t.includes("exec") || t.includes("run") || t.includes("shell")) return "Execution";
  return "Other";
}

function scoreColor(score: number): string {
  return score >= 80 ? "var(--success-color)" : score >= 50 ? "var(--warning-color)" : "var(--error-color)";
}

function timeAgo(ts: number): string {
  const diffSec = Math.floor((Date.now() / 1000) - ts);
  if (diffSec < 60) return `${diffSec}s ago`;
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)} min ago`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)} hr ago`;
  return `${Math.floor(diffSec / 86400)} days ago`;
}

/* ── Styles ─────────────────────────────────────────────────────────── */

const panelStyle: React.CSSProperties = {
  padding: 16, height: "100%", overflow: "auto",
  color: "var(--text-primary)", background: "var(--bg-primary)",
};
const headingStyle: React.CSSProperties = { fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 8, padding: 12, marginBottom: 8,
  border: "1px solid var(--border-color)",
};
const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px", cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent", border: "none", fontSize: 13, fontWeight: active ? 600 : 400,
});

/* ── Component ──────────────────────────────────────────────────────── */

export function TrustPanel() {
  const [tab, setTab] = useState("scores");
  const [loading, setLoading] = useState(true);
  const [config, setConfig] = useState<TrustConfig>(() => {
    try { return { ...DEFAULT_CONFIG, ...JSON.parse(localStorage.getItem(TRUST_CONFIG_KEY) || "{}") }; }
    catch { return DEFAULT_CONFIG; }
  });

  const [modelScores, setModelScores] = useState<ModelScore[]>([]);
  const [events, setEvents] = useState<TrustEvent[]>([]);
  const [domains, setDomains] = useState<DomainScores[]>([]);

  const updateConfig = (partial: Partial<TrustConfig>) => {
    const next = { ...config, ...partial };
    setConfig(next);
    localStorage.setItem(TRUST_CONFIG_KEY, JSON.stringify(next));
  };

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [costMetrics, traceEntries] = await Promise.all([
        invoke<CostMetrics>("get_cost_metrics"),
        invoke<TrustTraceEntry[]>("get_all_trace_entries"),
      ]);

      // ── Build provider→model mapping from cost data ──────────────
      const sessionProvider = new Map<string, { provider: string; model: string }>();
      for (const e of costMetrics.entries) {
        if (!sessionProvider.has(e.session_id)) {
          sessionProvider.set(e.session_id, { provider: e.provider, model: e.model });
        }
      }

      // ── Compute per-model scores ─────────────────────────────────
      // Score = weighted success rate with recency bias
      // Base: success_rate * 100, adjusted by recency (recent failures weigh more)
      const modelMap = new Map<string, {
        provider: string; model: string;
        successes: number; failures: number;
        totalDuration: number; count: number;
        sessions: Set<string>;
        recentEvents: TrustEvent[];
        domainStats: Map<string, { successes: number; failures: number }>;
      }>();

      for (const entry of traceEntries) {
        const info = sessionProvider.get(entry.session_id);
        const provider = info?.provider ?? "unknown";
        const model = info?.model ?? "unknown";
        const key = `${provider}/${model}`;

        let data = modelMap.get(key);
        if (!data) {
          data = {
            provider, model, successes: 0, failures: 0,
            totalDuration: 0, count: 0, sessions: new Set(),
            recentEvents: [], domainStats: new Map(),
          };
          modelMap.set(key, data);
        }

        if (entry.success) data.successes++; else data.failures++;
        data.totalDuration += entry.duration_ms;
        data.count++;
        data.sessions.add(entry.session_id);

        // Domain stats
        const domain = toolDomain(entry.tool);
        const ds = data.domainStats.get(domain) ?? { successes: 0, failures: 0 };
        if (entry.success) ds.successes++; else ds.failures++;
        data.domainStats.set(domain, ds);

        // Track recent events (keep latest 100 across all models)
        data.recentEvents.push({
          model: `${provider}/${model}`,
          tool: entry.tool,
          success: entry.success,
          delta: entry.success ? config.recoveryRate : -config.decayRate,
          timestamp: entry.timestamp,
        });
      }

      // Compute scores
      const scores: ModelScore[] = [];
      for (const [, data] of modelMap) {
        if (data.count === 0) continue;
        const total = data.successes + data.failures;
        const successRate = total > 0 ? data.successes / total : 0;
        // Score: base success rate * 100, clamped 0-100
        const score = Math.max(0, Math.min(100, Math.round(successRate * 100)));
        const displayName = data.model === "unknown" ? data.provider : `${data.provider}/${data.model}`;
        scores.push({
          model: displayName,
          provider: data.provider,
          score,
          tasks: data.sessions.size,
          successes: data.successes,
          failures: data.failures,
          avgDurationMs: data.count > 0 ? data.totalDuration / data.count : 0,
        });
      }
      scores.sort((a, b) => b.score - a.score);
      setModelScores(scores);

      // Flatten and sort all events, take latest 50
      const allEvents: TrustEvent[] = [];
      for (const [, data] of modelMap) {
        allEvents.push(...data.recentEvents);
      }
      allEvents.sort((a, b) => b.timestamp - a.timestamp);
      setEvents(allEvents.slice(0, 50));

      // Build domain scores
      const domainAgg = new Map<string, Map<string, { successes: number; failures: number }>>();
      for (const [, data] of modelMap) {
        const displayName = data.model === "unknown" ? data.provider : `${data.provider}/${data.model}`;
        for (const [domain, stats] of data.domainStats) {
          let dMap = domainAgg.get(domain);
          if (!dMap) { dMap = new Map(); domainAgg.set(domain, dMap); }
          const existing = dMap.get(displayName) ?? { successes: 0, failures: 0 };
          existing.successes += stats.successes;
          existing.failures += stats.failures;
          dMap.set(displayName, existing);
        }
      }
      const domainList: DomainScores[] = [];
      for (const [domain, models] of domainAgg) {
        const scoresObj: Record<string, number> = {};
        for (const [model, stats] of models) {
          const total = stats.successes + stats.failures;
          scoresObj[model] = total > 0 ? Math.round((stats.successes / total) * 100) : 0;
        }
        domainList.push({ domain, scores: scoresObj });
      }
      domainList.sort((a, b) => Object.keys(b.scores).length - Object.keys(a.scores).length);
      setDomains(domainList);

    } catch (e) {
      console.error("Trust data load error:", e);
    } finally {
      setLoading(false);
    }
  }, [config.decayRate, config.recoveryRate]);

  useEffect(() => { loadData(); }, [loadData]);

  if (loading) {
    return <div style={panelStyle}><h2 style={headingStyle}>Agent Trust Scoring</h2><div style={{ color: "var(--text-secondary)", padding: 20 }}>Loading trust data...</div></div>;
  }

  const noData = modelScores.length === 0;

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Agent Trust Scoring</h2>
      {noData && (
        <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center", padding: 24 }}>
          No trace or cost data yet. Use the Chat or Agent panels to generate activity, then scores will appear here.
        </div>
      )}
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["scores", "events", "domains", "config"].map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "scores" && !noData && (
        <div>
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 10 }}>
            Score = success rate across all traced tool calls. Green {"\u2265"}80, Yellow {"\u2265"}50, Red &lt;50.
          </div>
          {modelScores.map((s) => (
            <div key={s.model} style={{ ...cardStyle, display: "flex", alignItems: "center", gap: 12 }}>
              <div style={{ minWidth: 140, fontWeight: 600, fontSize: 13 }}>{s.model}</div>
              <div style={{ flex: 1, height: 8, borderRadius: 4, background: "var(--border-color)" }}>
                <div style={{ width: `${s.score}%`, height: 8, borderRadius: 4, background: scoreColor(s.score), transition: "width 0.3s" }} />
              </div>
              <span style={{ fontWeight: 600, fontSize: 13, color: scoreColor(s.score), minWidth: 36, textAlign: "right" }}>{s.score}</span>
              <span style={{ fontSize: 11, color: "var(--text-secondary)", minWidth: 60 }}>{s.tasks} tasks</span>
              <span style={{ fontSize: 10, color: "var(--text-secondary)", minWidth: 80 }}>
                {s.successes}ok / {s.failures}fail
              </span>
            </div>
          ))}
        </div>
      )}

      {tab === "events" && (
        <div>
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 10 }}>
            Latest 50 tool invocations across all models. Delta shows configured recovery (+{config.recoveryRate}) or decay (-{config.decayRate}) points.
          </div>
          {events.length === 0 ? (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>No events recorded yet</div>
          ) : events.map((e, i) => (
            <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{e.model}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 8 }}>{e.tool}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{timeAgo(e.timestamp)}</span>
                <span style={{
                  padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
                  background: e.success ? "#22c55e20" : "#ef444420",
                  color: e.success ? "var(--success-color)" : "var(--error-color)",
                }}>{e.success ? "success" : "failure"}</span>
                <span style={{ fontWeight: 600, fontSize: 12, color: e.delta >= 0 ? "var(--success-color)" : "var(--error-color)", minWidth: 28, textAlign: "right" }}>
                  {e.delta >= 0 ? "+" : ""}{e.delta}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "domains" && (
        <div>
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 10 }}>
            Per-domain success rates grouped by tool type (edit/write = Code Generation, test = Testing, fix/debug = Bug Fixing, etc.).
          </div>
          {domains.length === 0 ? (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", textAlign: "center" }}>No domain data yet</div>
          ) : domains.map((d) => (
            <div key={d.domain} style={cardStyle}>
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>{d.domain}</div>
              {Object.entries(d.scores)
                .sort(([, a], [, b]) => b - a)
                .map(([model, score]) => (
                <div key={model} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                  <span style={{ fontSize: 12, minWidth: 140, color: "var(--text-secondary)" }}>{model}</span>
                  <div style={{ flex: 1, height: 6, borderRadius: 3, background: "var(--border-color)" }}>
                    <div style={{ width: `${score}%`, height: 6, borderRadius: 3, background: scoreColor(score), transition: "width 0.3s" }} />
                  </div>
                  <span style={{ fontSize: 11, fontWeight: 600, color: scoreColor(score), minWidth: 28, textAlign: "right" }}>{score}</span>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 10 }}>
            Configure trust scoring parameters. Changes are saved and applied on next refresh.
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Decay Rate: {config.decayRate} pts/failure</div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>Points deducted per failed tool invocation</div>
            <input type="range" min={1} max={20} value={config.decayRate} onChange={(e) => updateConfig({ decayRate: Number(e.target.value) })} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Recovery Rate: {config.recoveryRate} pts/success</div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>Points awarded per successful tool invocation</div>
            <input type="range" min={1} max={25} value={config.recoveryRate} onChange={(e) => updateConfig({ recoveryRate: Number(e.target.value) })} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Auto-Merge Threshold: {config.autoMergeThreshold}</div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>Models scoring above this can auto-merge without review</div>
            <input type="range" min={50} max={100} value={config.autoMergeThreshold} onChange={(e) => updateConfig({ autoMergeThreshold: Number(e.target.value) })} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Manual Review Below: {config.manualReviewThreshold}</div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6 }}>Models scoring below this require manual review for all actions</div>
            <input type="range" min={10} max={80} value={config.manualReviewThreshold} onChange={(e) => updateConfig({ manualReviewThreshold: Number(e.target.value) })} style={{ width: "100%" }} />
          </div>
        </div>
      )}
    </div>
  );
}
