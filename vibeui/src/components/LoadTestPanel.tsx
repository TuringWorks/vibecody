/**
 * LoadTestPanel — HTTP Load Tester.
 *
 * Sends N concurrent requests to any HTTP endpoint, shows p50/p90/p99
 * latency percentiles, requests/sec, success rate, and status code breakdown.
 * Live progress via `loadtest:progress` Tauri events.
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface LoadTestResult {
 total_requests: number;
 success: number;
 failed: number;
 duration_ms: number;
 requests_per_sec: number;
 avg_ms: number;
 min_ms: number;
 max_ms: number;
 p50_ms: number;
 p90_ms: number;
 p99_ms: number;
 status_codes: Record<string, number>;
}

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];

const PRESETS = [
 { label: "Light (10 reqs, c=2)", total: 10, concurrency: 2 },
 { label: "Medium (100 reqs, c=10)", total: 100, concurrency: 10 },
 { label: "Heavy (500 reqs, c=50)", total: 500, concurrency: 50 },
 { label: "Stress (1000 reqs, c=100)", total: 1000, concurrency: 100 },
];

function StatCard({ label, value, unit, color }: { label: string; value: string | number; unit?: string; color?: string }) {
 return (
 <div style={{ flex: 1, padding: "8px 10px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", textAlign: "center" }}>
 <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "monospace", color: color ?? "var(--text-primary)" }}>
 {typeof value === "number" ? value.toFixed(value < 10 ? 1 : 0) : value}
 {unit && <span style={{ fontSize: 11, fontWeight: 400, marginLeft: 2 }}>{unit}</span>}
 </div>
 <div style={{ fontSize: 9, color: "var(--text-muted)", fontWeight: 600, marginTop: 2 }}>{label}</div>
 </div>
 );
}

export function LoadTestPanel() {
 const [url, setUrl] = useState("https://jsonplaceholder.typicode.com/todos/1");
 const [method, setMethod] = useState("GET");
 const [body, setBody] = useState("");
 const [headersText, setHeadersText] = useState("{}");
 const [total, setTotal] = useState(100);
 const [concurrency, setConcurrency] = useState(10);
 const [result, setResult] = useState<LoadTestResult | null>(null);
 const [running, setRunning] = useState(false);
 const [progress, setProgress] = useState(0);
 const [error, setError] = useState<string | null>(null);
 const unlistenRef = useRef<(() => void) | null>(null);

 useEffect(() => () => { unlistenRef.current?.(); }, []);

 const parseHeaders = () => {
 try {
 const h = JSON.parse(headersText);
 return typeof h === "object" && h !== null ? h as Record<string, string> : undefined;
 } catch { return undefined; }
 };

 const run = async () => {
 if (!url || running) return;
 setRunning(true);
 setProgress(0);
 setResult(null);
 setError(null);

 unlistenRef.current?.();
 const unlisten = await listen<number>("loadtest:progress", (e) => {
 setProgress(e.payload);
 });
 unlistenRef.current = unlisten;

 try {
 const res = await invoke<LoadTestResult>("run_load_test", {
 url, method,
 body: body.trim() || null,
 headers: parseHeaders(),
 concurrency,
 total,
 });
 setResult(res);
 setProgress(total);
 } catch (e) {
 setError(String(e));
 } finally {
 setRunning(false);
 unlistenRef.current?.();
 unlistenRef.current = null;
 }
 };

 const progressPct = total > 0 ? Math.round((progress / total) * 100) : 0;
 const successRate = result ? Math.round((result.success / result.total_requests) * 100) : null;

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Header */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", alignItems: "center", gap: 8 }}>
 <span style={{ fontSize: 16 }}></span>
 <div style={{ fontSize: 13, fontWeight: 600 }}>HTTP Load Tester</div>
 </div>

 <div style={{ flex: 1, overflow: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
 {error && (
 <div style={{ padding: "6px 10px", background: "var(--error-bg, #2a1a1a)", color: "#f38ba8", borderRadius: 4, fontSize: 11 }}> {error}</div>
 )}

 {/* URL + method */}
 <div style={{ display: "flex", gap: 6 }}>
 <select
 value={method}
 onChange={(e) => setMethod(e.target.value)}
 style={{ padding: "6px 8px", fontSize: 11, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", fontWeight: 600 }}
 >
 {METHODS.map((m) => <option key={m}>{m}</option>)}
 </select>
 <input
 value={url}
 onChange={(e) => setUrl(e.target.value)}
 placeholder="https://api.example.com/endpoint"
 style={{ flex: 1, padding: "6px 10px", fontSize: 12, fontFamily: "monospace", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }}
 />
 </div>

 {/* Presets */}
 <div style={{ display: "flex", gap: 5, flexWrap: "wrap" }}>
 {PRESETS.map((p) => (
 <button
 key={p.label}
 onClick={() => { setTotal(p.total); setConcurrency(p.concurrency); }}
 style={{ padding: "3px 9px", fontSize: 10, borderRadius: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", color: "var(--text-muted)", cursor: "pointer" }}
 >
 {p.label}
 </button>
 ))}
 </div>

 {/* Config row */}
 <div style={{ display: "flex", gap: 8, alignItems: "flex-end" }}>
 {[
 { label: "Total Requests", value: total, setter: setTotal, min: 1, max: 10000 },
 { label: "Concurrency", value: concurrency, setter: setConcurrency, min: 1, max: 200 },
 ].map(({ label, value, setter, min, max }) => (
 <div key={label} style={{ display: "flex", flexDirection: "column", gap: 3, flex: 1 }}>
 <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>{label}</label>
 <input
 type="number" min={min} max={max} value={value}
 onChange={(e) => setter(Math.max(min, Math.min(max, Number(e.target.value))))}
 style={{ padding: "5px 8px", fontSize: 12, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }}
 />
 </div>
 ))}

 <button
 onClick={run}
 disabled={running || !url}
 style={{
 padding: "6px 20px", fontSize: 12, fontWeight: 700, alignSelf: "flex-end",
 background: running ? "var(--bg-secondary)" : "#6366f1",
 color: running ? "var(--text-muted)" : "#fff",
 border: "none", borderRadius: 4, cursor: running ? "not-allowed" : "pointer",
 height: 32,
 }}
 >
 {running ? ` ${progress}/${total}` : "Run"}
 </button>
 </div>

 {/* Optional body + headers */}
 {(method === "POST" || method === "PUT" || method === "PATCH") && (
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>Request Body</label>
 <textarea
 value={body}
 onChange={(e) => setBody(e.target.value)}
 rows={3}
 placeholder='{"key": "value"}'
 style={{ padding: "6px 10px", fontSize: 11, fontFamily: "monospace", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", resize: "vertical" }}
 />
 </div>
 )}

 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>Headers (JSON)</label>
 <input
 value={headersText}
 onChange={(e) => setHeadersText(e.target.value)}
 placeholder='{"Authorization": "Bearer TOKEN"}'
 style={{ padding: "5px 10px", fontSize: 11, fontFamily: "monospace", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", outline: "none" }}
 />
 </div>

 {/* Progress bar */}
 {running && (
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--text-muted)" }}>
 <span>Running… {progress}/{total}</span>
 <span>{progressPct}%</span>
 </div>
 <div style={{ height: 8, background: "var(--bg-secondary)", borderRadius: 4, overflow: "hidden", border: "1px solid var(--border-color)" }}>
 <div style={{ height: "100%", width: `${progressPct}%`, background: "#6366f1", borderRadius: 4, transition: "width 0.2s" }} />
 </div>
 </div>
 )}

 {/* Results */}
 {result && (
 <>
 {/* Summary row */}
 <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
 <StatCard label="Req/sec" value={result.requests_per_sec} color="#89b4fa" />
 <StatCard label="Avg" value={result.avg_ms} unit="ms" />
 <StatCard label="p50" value={result.p50_ms} unit="ms" />
 <StatCard label="p90" value={result.p90_ms} unit="ms" />
 <StatCard label="p99" value={result.p99_ms} unit="ms" />
 <StatCard label="Min/Max" value={`${result.min_ms}/${result.max_ms}`} unit="ms" />
 </div>

 {/* Success / failure */}
 <div style={{ display: "flex", gap: 6 }}>
 <div style={{ flex: 1, padding: "8px 10px", background: "rgba(166,227,161,0.1)", border: "1px solid #a6e3a1", borderRadius: 6, textAlign: "center" }}>
 <div style={{ fontSize: 18, fontWeight: 700, color: "#a6e3a1" }}>{result.success}</div>
 <div style={{ fontSize: 9, color: "#a6e3a1", fontWeight: 600 }}>SUCCESS ({successRate}%)</div>
 </div>
 {result.failed > 0 && (
 <div style={{ flex: 1, padding: "8px 10px", background: "rgba(243,139,168,0.1)", border: "1px solid #f38ba8", borderRadius: 6, textAlign: "center" }}>
 <div style={{ fontSize: 18, fontWeight: 700, color: "#f38ba8" }}>{result.failed}</div>
 <div style={{ fontSize: 9, color: "#f38ba8", fontWeight: 600 }}>FAILED ({100 - (successRate ?? 0)}%)</div>
 </div>
 )}
 <div style={{ flex: 1, padding: "8px 10px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, textAlign: "center" }}>
 <div style={{ fontSize: 18, fontWeight: 700 }}>{(result.duration_ms / 1000).toFixed(2)}s</div>
 <div style={{ fontSize: 9, color: "var(--text-muted)", fontWeight: 600 }}>TOTAL TIME</div>
 </div>
 </div>

 {/* Latency bar chart (visual) */}
 <div style={{ padding: "10px 12px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
 <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 8 }}>Latency Distribution</div>
 <div style={{ display: "flex", alignItems: "flex-end", gap: 4, height: 50 }}>
 {[
 { label: "avg", val: result.avg_ms },
 { label: "p50", val: result.p50_ms },
 { label: "p90", val: result.p90_ms },
 { label: "p99", val: result.p99_ms },
 { label: "max", val: result.max_ms },
 ].map(({ label, val }) => {
 const h = result.max_ms === 0 ? 4 : Math.max(4, Math.round((val / result.max_ms) * 46));
 const color = label === "p99" || label === "max" ? "#f38ba8"
 : label === "p90" ? "#f9e2af" : "#89b4fa";
 return (
 <div key={label} style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", gap: 2 }}>
 <span style={{ fontSize: 9, color: "var(--text-muted)" }}>{val}ms</span>
 <div style={{ width: "70%", height: h, background: color, borderRadius: "2px 2px 0 0" }} />
 <span style={{ fontSize: 9, color: "var(--text-muted)", fontWeight: 600 }}>{label}</span>
 </div>
 );
 })}
 </div>
 </div>

 {/* Status code breakdown */}
 <div>
 <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 6 }}>Status Codes</div>
 <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
 {Object.entries(result.status_codes)
 .sort(([a], [b]) =>Number(a) - Number(b))
 .map(([code, count]) => {
 const c = Number(code);
 const color = c === 0 ? "#f38ba8" : c < 300 ? "#a6e3a1" : c < 400 ? "#f9e2af" : "#f38ba8";
 return (
 <div key={code} style={{ padding: "5px 12px", borderRadius: 20, border: `1px solid ${color}`, background: `${color}22`, fontSize: 11 }}>
 <span style={{ color, fontWeight: 700 }}>{code === "0" ? "ERR" : code}</span>
 <span style={{ color: "var(--text-muted)", marginLeft: 6 }}>×{count}</span>
 </div>
 );
 })}
 </div>
 </div>
 </>
 )}
 </div>
 </div>
 );
}
