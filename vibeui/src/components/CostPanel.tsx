import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

interface ProviderCostSummary {
 provider: string;
 total_cost_usd: number;
 total_tokens: number;
 call_count: number;
}

interface CostMetrics {
 entries: CostEntry[];
 by_provider: ProviderCostSummary[];
 total_cost_usd: number;
 total_tokens: number;
 budget_limit_usd: number | null;
 budget_remaining_usd: number | null;
}

const fmt = (usd: number) =>
 usd < 0.001 ? "<$0.001" : `$${usd.toFixed(4)}`;

const fmtTokens = (n: number) =>
 n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` :
 n >= 1_000 ? `${(n / 1_000).toFixed(1)}K` : String(n);

const fmtTime = (ms: number) => {
 const d = new Date(ms);
 return d.toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
};

const budgetColor = (remaining: number, limit: number) => {
 const pct = remaining / limit;
 if (pct > 0.5) return "var(--success-color)";
 if (pct > 0.2) return "var(--warning-color)";
 return "var(--error-color)";
};

export function CostPanel() {
 const [metrics, setMetrics] = useState<CostMetrics | null>(null);
 const [loading, setLoading] = useState(false);
 const [budgetInput, setBudgetInput] = useState("");
 const [savingBudget, setSavingBudget] = useState(false);
 const [clearing, setClearing] = useState(false);
 const [showAll, setShowAll] = useState(false);

 const load = useCallback(async () => {
 setLoading(true);
 try {
 const m = await invoke<CostMetrics>("get_cost_metrics");
 setMetrics(m);
 setBudgetInput(m.budget_limit_usd != null ? String(m.budget_limit_usd) : "");
 } catch {
 /* ignore */
 } finally {
 setLoading(false);
 }
 }, []);

 useEffect(() => { load(); }, [load]);

 const handleSetBudget = async () => {
 setSavingBudget(true);
 const limit = budgetInput.trim() ? parseFloat(budgetInput) : null;
 try {
 await invoke("set_cost_limit", { limitUsd: limit });
 await load();
 } finally {
 setSavingBudget(false);
 }
 };

 const handleClear = async () => {
 if (!confirm("Clear all cost history? This cannot be undone.")) return;
 setClearing(true);
 try {
 await invoke("clear_cost_history");
 await load();
 } finally {
 setClearing(false);
 }
 };

 const visibleEntries = showAll
 ? (metrics?.entries ?? [])
 : (metrics?.entries ?? []).slice(0, 20);

 const providerCosts = metrics?.by_provider?.map(p => p.total_cost_usd) ?? [];
 const maxProviderCost = providerCosts.length > 0 ? Math.max(...providerCosts) : 0;

 return (
 <div className="panel-container">
 <div className="panel-header">
   <h3>Cost &amp; Performance Observatory</h3>
 </div>

 <div className="panel-body">
 {loading && !metrics && (
 <div className="panel-loading">Loading…</div>
 )}

 {metrics && (
 <>
 {/* Summary row */}
 <div style={{ display: "flex", gap: "12px", flexWrap: "wrap", marginBottom: "14px" }}>
 {[
 { label: "Total Cost", value: fmt(metrics.total_cost_usd) },
 { label: "Total Tokens", value: fmtTokens(metrics.total_tokens) },
 { label: "AI Calls", value: String(metrics.entries.length) },
 ].map(({ label, value }) => (
 <div key={label} style={{ background: "var(--bg-secondary)", padding: "8px 14px", borderRadius: "6px", textAlign: "center", minWidth: "100px" }}>
 <div style={{ fontSize: "18px", fontWeight: "bold", color: "var(--accent-color)" }}>{value}</div>
 <div style={{ fontSize: "11px", color: "var(--text-secondary)", marginTop: "2px" }}>{label}</div>
 </div>
 ))}
 </div>

 {/* Budget */}
 <div style={{ background: "var(--bg-secondary)", padding: "10px", borderRadius: "6px", marginBottom: "14px" }}>
 <div style={{ fontSize: "11px", color: "var(--text-secondary)", marginBottom: "6px" }}>Monthly Budget Limit</div>
 <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
 <span style={{ color: "var(--text-secondary)" }}>$</span>
 <input
 type="number"
 min="0"
 step="0.5"
 value={budgetInput}
 onChange={e => setBudgetInput(e.target.value)}
 placeholder="e.g. 10.00 (blank = no limit)"
 style={{ flex: 1, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "4px 8px", fontFamily: "inherit", fontSize: "12px" }}
 />
 <button
 onClick={handleSetBudget}
 disabled={savingBudget}
 style={{ background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "4px", padding: "4px 12px", cursor: "pointer", fontSize: "12px" }}
 >
 {savingBudget ? "…" : "Save"}
 </button>
 </div>
 {metrics.budget_limit_usd != null && metrics.budget_remaining_usd != null && (
 <div style={{ marginTop: "8px" }}>
 <div style={{ display: "flex", justifyContent: "space-between", fontSize: "11px", marginBottom: "3px" }}>
 <span style={{ color: "var(--text-secondary)" }}>Used: {fmt(metrics.total_cost_usd)} / {fmt(metrics.budget_limit_usd)}</span>
 <span style={{ color: budgetColor(metrics.budget_remaining_usd, metrics.budget_limit_usd) }}>
 {fmt(metrics.budget_remaining_usd)} remaining
 </span>
 </div>
 <div style={{ background: "var(--bg-primary)", borderRadius: "3px", height: "5px", overflow: "hidden" }}>
 <div style={{
 background: budgetColor(metrics.budget_remaining_usd, metrics.budget_limit_usd),
 width: `${Math.min(100, (metrics.total_cost_usd / metrics.budget_limit_usd) * 100)}%`,
 height: "100%",
 transition: "width 0.4s",
 }} />
 </div>
 </div>
 )}
 </div>

 {/* By provider */}
 {metrics.by_provider.length > 0 && (
 <div style={{ marginBottom: "14px" }}>
 <div style={{ fontSize: "11px", color: "var(--text-secondary)", marginBottom: "6px" }}>Cost by Provider</div>
 <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
 {metrics.by_provider.map(p => (
 <div key={p.provider} style={{ display: "flex", alignItems: "center", gap: "8px" }}>
 <span style={{ minWidth: "70px", color: "var(--text-secondary)", fontSize: "12px" }}>{p.provider}</span>
 <div style={{ flex: 1, background: "var(--bg-secondary)", borderRadius: "2px", height: "8px", overflow: "hidden" }}>
 <div style={{
 background: "var(--accent-color)",
 width: maxProviderCost > 0 ? `${(p.total_cost_usd / maxProviderCost) * 100}%` : "0%",
 height: "100%",
 }} />
 </div>
 <span style={{ minWidth: "60px", textAlign: "right", color: "var(--text-secondary)", fontSize: "12px" }}>{fmt(p.total_cost_usd)}</span>
 <span style={{ minWidth: "70px", textAlign: "right", color: "var(--text-secondary)", fontSize: "11px" }}>{fmtTokens(p.total_tokens)} tok</span>
 <span style={{ minWidth: "50px", textAlign: "right", color: "var(--text-secondary)", fontSize: "11px" }}>{p.call_count}×</span>
 </div>
 ))}
 </div>
 </div>
 )}

 {/* Recent calls */}
 {metrics.entries.length > 0 && (
 <div>
 <div style={{ display: "flex", alignItems: "center", marginBottom: "6px" }}>
 <span style={{ fontSize: "11px", color: "var(--text-secondary)" }}>Recent Calls</span>
 <button
 onClick={handleClear}
 disabled={clearing}
 style={{ marginLeft: "auto", background: "none", color: "var(--error-color)", border: "none", cursor: "pointer", fontSize: "11px", padding: "0" }}
 >
 {clearing ? "…" : "Clear history"}
 </button>
 </div>
 <div style={{ display: "flex", flexDirection: "column", gap: "3px" }}>
 {visibleEntries.map((e, i) => (
 <div key={i} style={{ background: "var(--bg-secondary)", padding: "5px 8px", borderRadius: "4px", display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
 <span style={{ color: "var(--text-secondary)", fontSize: "10px", minWidth: "110px" }}>{fmtTime(e.timestamp_ms)}</span>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px" }}>{e.provider}</span>
 <span style={{ color: "var(--text-secondary)", fontSize: "10px" }}>{e.model}</span>
 {e.task_hint && <span style={{ color: "var(--text-secondary)", fontSize: "10px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: "160px" }}>{e.task_hint}</span>}
 <span style={{ marginLeft: "auto", color: "var(--text-secondary)", fontSize: "11px" }}>{fmtTokens(e.prompt_tokens + e.completion_tokens)} tok</span>
 <span style={{ color: e.cost_usd > 0 ? "var(--accent-color)" : "var(--text-secondary)", fontSize: "11px", minWidth: "60px", textAlign: "right" }}>{fmt(e.cost_usd)}</span>
 </div>
 ))}
 </div>
 {metrics.entries.length > 20 && (
 <button
 onClick={() => setShowAll(s => !s)}
 style={{ marginTop: "8px", background: "none", color: "var(--accent-color)", border: "none", cursor: "pointer", fontSize: "12px", padding: "0" }}
 >
 {showAll ? "Show less" : `Show all ${metrics.entries.length} calls`}
 </button>
 )}
 </div>
 )}

 {metrics.entries.length === 0 && (
 <div className="panel-empty">
 No cost records yet.<br />
 Costs are recorded automatically when using AI chat and agent features.
 </div>
 )}
 </>
 )}
 </div>
 </div>
 );
}
