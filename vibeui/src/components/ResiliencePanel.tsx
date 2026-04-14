import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ───────────────────────────────────────────────────────────────────

interface ProviderHealth {
  name: string;
  score: number;
  successRate: number;
  avgLatencyMs: number;
  totalCalls: number;
  recentFailures: number;
}

interface CircuitBreakerState {
  state: "PROGRESS" | "STALLED" | "SPINNING" | "DEGRADED" | "BLOCKED";
  rotations: number;
  maxRotations: number;
  stepsSinceFileChange: number;
  recoveryProbing: boolean;
}

interface FailureRecord {
  timestampMs: number;
  category: string;
  provider: string | null;
  errorMessage: string;
  context: string | null;
}

interface FailurePattern {
  category: string;
  count: number;
  provider: string | null;
  isRecurring: boolean;
}

interface ResilienceConfig {
  retryMaxAttempts: number;
  retryJitterEnabled: boolean;
  cbStallThreshold: number;
  cbSpinThreshold: number;
  cbMaxRotations: number;
  cbRecoveryCooldownSecs: number;
  healthAwareFailover: boolean;
  failureJournalEnabled: boolean;
}

const DEFAULT_CONFIG: ResilienceConfig = {
  retryMaxAttempts: 5,
  retryJitterEnabled: true,
  cbStallThreshold: 5,
  cbSpinThreshold: 3,
  cbMaxRotations: 3,
  cbRecoveryCooldownSecs: 30,
  healthAwareFailover: true,
  failureJournalEnabled: true,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

const card: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)" };
const heading: React.CSSProperties = { fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 8, color: "var(--text-primary)" };
const label: React.CSSProperties = { fontSize: "var(--font-size-base)", color: "var(--text-secondary)" };
const tbl: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-md)" };
const th: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontSize: "var(--font-size-base)", fontWeight: 500 };
const td: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };

function badge(color: string): React.CSSProperties {
  return { display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-sm)", fontWeight: 600, background: color + "22", color };
}

function HealthBar({ score }: { score: number }) {
  const color = score > 0.8 ? "var(--accent-green)" : score > 0.5 ? "var(--accent-gold)" : "var(--accent-rose)";
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <div style={{ height: 6, borderRadius: 3, background: "var(--bg-tertiary)", width: 80, overflow: "hidden" }}>
        <div style={{ height: "100%", width: `${Math.round(score * 100)}%`, background: color, borderRadius: 3 }} />
      </div>
      <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600, color }}>{(score * 100).toFixed(0)}%</span>
    </div>
  );
}

function StateBadge({ state }: { state: string }) {
  const colors: Record<string, string> = {
    PROGRESS: "var(--accent-green)",
    STALLED: "var(--accent-gold)",
    SPINNING: "var(--accent-gold)",
    DEGRADED: "var(--accent-rose)",
    BLOCKED: "var(--accent-rose)",
  };
  return <span style={badge(colors[state] || "var(--text-secondary)")}>{state}</span>;
}

function CategoryBadge({ category }: { category: string }) {
  const colors: Record<string, string> = {
    RateLimit: "var(--accent-gold)",
    Timeout: "var(--accent-purple)",
    ServerError: "var(--accent-rose)",
    AuthError: "var(--error-color)",
    NetworkError: "var(--info-color)",
    InvalidResponse: "#795548",
    StreamInterrupted: "#607d8b",
    Unknown: "var(--text-secondary)",
  };
  return <span style={badge(colors[category] || "var(--text-secondary)")}>{category}</span>;
}

function timeAgo(ms: number): string {
  const secs = Math.floor((Date.now() - ms) / 1000);
  if (secs < 60) return `${secs}s ago`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
  return `${Math.floor(secs / 3600)}h ago`;
}

// ── Main Panel ──────────────────────────────────────────────────────────────

export function ResiliencePanel() {
  const [tab, setTab] = useState<"health" | "circuit" | "journal" | "config">("health");
  const [providers, setProviders] = useState<ProviderHealth[]>([]);
  const [cb, setCb] = useState<CircuitBreakerState>({ state: "PROGRESS", rotations: 0, maxRotations: 3, stepsSinceFileChange: 0, recoveryProbing: false });
  const [failures, setFailures] = useState<FailureRecord[]>([]);
  const [patterns, setPatterns] = useState<FailurePattern[]>([]);
  const [config, setConfig] = useState<ResilienceConfig>(DEFAULT_CONFIG);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => { loadAll(); }, []);

  // Reload data when tab changes to keep it fresh
  useEffect(() => {
    if (tab === "health") loadHealth();
    if (tab === "circuit") loadCircuit();
    if (tab === "journal") loadJournal();
    if (tab === "config") loadConfig();
  }, [tab]);

  async function loadAll() {
    setLoading(true);
    try {
      await Promise.all([loadHealth(), loadCircuit(), loadJournal(), loadConfig()]);
    } finally {
      setLoading(false);
    }
  }

  async function loadHealth() {
    try {
      const data = await invoke<ProviderHealth[]>("get_provider_health");
      setProviders(data);
    } catch { /* keep current */ }
  }

  async function loadCircuit() {
    try {
      const data = await invoke<CircuitBreakerState>("get_circuit_breaker_state");
      setCb(data);
    } catch { /* keep current */ }
  }

  async function loadJournal() {
    try {
      const [f, p] = await Promise.all([
        invoke<FailureRecord[]>("get_failure_records", { limit: 50 }),
        invoke<FailurePattern[]>("get_failure_patterns"),
      ]);
      setFailures(f);
      setPatterns(p);
    } catch { /* keep current */ }
  }

  async function loadConfig() {
    try {
      const data = await invoke<ResilienceConfig>("get_resilience_config");
      setConfig(data);
    } catch { /* keep default */ }
  }

  async function handleSaveConfig() {
    setSaving(true);
    try {
      await invoke("save_resilience_config", { config });
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-tab-bar">
        {(["health", "circuit", "journal", "config"] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`panel-tab ${tab === t ? "active" : ""}`}
          >
            {t === "health" ? "Provider Health" : t === "circuit" ? "Circuit Breaker" : t === "journal" ? "Failure Journal" : "Config"}
          </button>
        ))}
      </div>

      {loading && (
        <div className="panel-loading">Loading resilience data...</div>
      )}

      {/* Provider Health */}
      {!loading && tab === "health" && (
        <div>
          <div style={heading}>Provider Health Scores</div>
          {providers.length === 0 ? (
            <div style={{ ...card, textAlign: "center", color: "var(--text-secondary)" }}>
              No provider health data yet. Health scores are recorded as AI providers are used.
            </div>
          ) : (
            <div style={{ ...card, padding: 0, overflow: "hidden" }}>
              <table style={tbl}>
                <thead>
                  <tr>
                    <th style={th}>Provider</th>
                    <th style={th}>Health</th>
                    <th style={th}>Success Rate</th>
                    <th style={th}>Avg Latency</th>
                    <th style={th}>Calls</th>
                    <th style={th}>Failures</th>
                  </tr>
                </thead>
                <tbody>
                  {providers.map((p) => (
                    <tr key={p.name}>
                      <td style={td}><strong>{p.name}</strong></td>
                      <td style={td}><HealthBar score={p.score} /></td>
                      <td style={td}>{(p.successRate * 100).toFixed(1)}%</td>
                      <td style={td}>{p.avgLatencyMs.toFixed(0)} ms</td>
                      <td style={td}>{p.totalCalls}</td>
                      <td style={td}>{p.recentFailures}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          <div style={{ ...label, marginTop: 8 }}>
            Health score = success_rate x 0.7 + latency_factor x 0.3 — higher-scoring providers are tried first in failover.
          </div>
        </div>
      )}

      {/* Circuit Breaker */}
      {!loading && tab === "circuit" && (
        <div>
          <div style={heading}>Circuit Breaker Status</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
            <div style={card}>
              <div style={label}>State</div>
              <div style={{ marginTop: 4 }}><StateBadge state={cb.state} /></div>
            </div>
            <div style={card}>
              <div style={label}>Rotations</div>
              <div style={{ fontSize: 20, fontWeight: 700, marginTop: 4 }}>{cb.rotations} / {cb.maxRotations}</div>
            </div>
            <div style={card}>
              <div style={label}>Steps Since File Change</div>
              <div style={{ fontSize: 20, fontWeight: 700, marginTop: 4 }}>{cb.stepsSinceFileChange}</div>
            </div>
          </div>
          <div style={{ ...card, marginTop: 12 }}>
            <div style={heading}>Recovery Policy</div>
            <div style={{ display: "flex", gap: 24 }}>
              <div>
                <span style={label}>Probing: </span>
                <span style={badge(cb.recoveryProbing ? "var(--accent-green)" : "var(--text-secondary)")}>{cb.recoveryProbing ? "Active" : "Idle"}</span>
              </div>
              <div>
                <span style={label}>Cooldown: </span>{config.cbRecoveryCooldownSecs}s
              </div>
            </div>
            <div style={{ ...label, marginTop: 8 }}>
              After cooldown, the circuit breaker enters half-open state and probes with live calls. Two consecutive successes restore Progress.
            </div>
          </div>
        </div>
      )}

      {/* Failure Journal */}
      {!loading && tab === "journal" && (
        <div>
          <div style={heading}>Detected Patterns</div>
          {patterns.length === 0 ? (
            <div style={label}>No recurring patterns detected.</div>
          ) : (
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
              {patterns.map((p, i) => (
                <div key={i} style={{ ...card, display: "flex", alignItems: "center", gap: 8 }}>
                  <CategoryBadge category={p.category} />
                  <span>{p.provider || "all"}</span>
                  <span style={{ fontWeight: 600 }}>x{p.count}</span>
                  {p.isRecurring && <span style={badge("var(--accent-rose)")}>recurring</span>}
                </div>
              ))}
            </div>
          )}

          <div style={heading}>Recent Failures</div>
          {failures.length === 0 ? (
            <div style={{ ...card, textAlign: "center", color: "var(--text-secondary)" }}>
              No failures recorded. Failures are logged when AI provider calls fail.
            </div>
          ) : (
            <div style={{ ...card, padding: 0, overflow: "hidden" }}>
              <table style={tbl}>
                <thead>
                  <tr>
                    <th style={th}>When</th>
                    <th style={th}>Category</th>
                    <th style={th}>Provider</th>
                    <th style={th}>Error</th>
                    <th style={th}>Context</th>
                  </tr>
                </thead>
                <tbody>
                  {failures.map((f, i) => (
                    <tr key={i}>
                      <td style={td}>{timeAgo(f.timestampMs)}</td>
                      <td style={td}><CategoryBadge category={f.category} /></td>
                      <td style={td}>{f.provider || "—"}</td>
                      <td style={{ ...td, maxWidth: 300, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.errorMessage}</td>
                      <td style={td}>{f.context || "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* Config */}
      {!loading && tab === "config" && (
        <div>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div>
              <div style={heading}>Resilience Configuration</div>
              <div style={label}>Values from <code>[resilience]</code> in ~/.vibecli/config.toml</div>
            </div>
            <button
              onClick={handleSaveConfig}
              disabled={saving}
              className={`panel-btn ${saving ? "panel-btn-secondary" : "panel-btn-primary"}`}
            >
              {saving ? "Saving..." : "Save Config"}
            </button>
          </div>
          <div style={{ ...card, marginTop: 8 }}>
            <table style={tbl}>
              <thead>
                <tr>
                  <th style={th}>Setting</th>
                  <th style={th}>Value</th>
                  <th style={th}>Description</th>
                </tr>
              </thead>
              <tbody>
                <tr><td style={td}><code>retry_max_attempts</code></td><td style={td}><input type="number" value={config.retryMaxAttempts} onChange={(e) => setConfig({ ...config, retryMaxAttempts: +e.target.value })} style={{ width: 60, padding: 2, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 3 }} /></td><td style={td}>Max retry attempts per LLM call</td></tr>
                <tr><td style={td}><code>retry_jitter_enabled</code></td><td style={td}><input type="checkbox" checked={config.retryJitterEnabled} onChange={(e) => setConfig({ ...config, retryJitterEnabled: e.target.checked })} /></td><td style={td}>+/-25% jitter on backoff to prevent thundering herd</td></tr>
                <tr><td style={td}><code>cb_stall_threshold</code></td><td style={td}><input type="number" value={config.cbStallThreshold} onChange={(e) => setConfig({ ...config, cbStallThreshold: +e.target.value })} style={{ width: 60, padding: 2, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 3 }} /></td><td style={td}>Steps without file changes before STALLED</td></tr>
                <tr><td style={td}><code>cb_spin_threshold</code></td><td style={td}><input type="number" value={config.cbSpinThreshold} onChange={(e) => setConfig({ ...config, cbSpinThreshold: +e.target.value })} style={{ width: 60, padding: 2, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 3 }} /></td><td style={td}>Repeated errors before SPINNING</td></tr>
                <tr><td style={td}><code>cb_max_rotations</code></td><td style={td}><input type="number" value={config.cbMaxRotations} onChange={(e) => setConfig({ ...config, cbMaxRotations: +e.target.value })} style={{ width: 60, padding: 2, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 3 }} /></td><td style={td}>Max approach rotations before BLOCKED</td></tr>
                <tr><td style={td}><code>cb_recovery_cooldown_secs</code></td><td style={td}><input type="number" value={config.cbRecoveryCooldownSecs} onChange={(e) => setConfig({ ...config, cbRecoveryCooldownSecs: +e.target.value })} style={{ width: 60, padding: 2, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 3 }} /></td><td style={td}>Cooldown before half-open recovery probe</td></tr>
                <tr><td style={td}><code>health_aware_failover</code></td><td style={td}><input type="checkbox" checked={config.healthAwareFailover} onChange={(e) => setConfig({ ...config, healthAwareFailover: e.target.checked })} /></td><td style={td}>Sort providers by health score in failover</td></tr>
                <tr><td style={td}><code>failure_journal_enabled</code></td><td style={td}><input type="checkbox" checked={config.failureJournalEnabled} onChange={(e) => setConfig({ ...config, failureJournalEnabled: e.target.checked })} /></td><td style={td}>Persist failures to ~/.vibecli/failure_journal.jsonl</td></tr>
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
