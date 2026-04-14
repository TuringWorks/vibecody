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
 <div style={{ flex: 1, padding: "8px 10px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", textAlign: "center" }}>
 <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: color ?? "var(--text-primary)" }}>
 {typeof value === "number" ? value.toFixed(value < 10 ? 1 : 0) : value}
 {unit && <span style={{ fontSize: "var(--font-size-sm)", fontWeight: 400, marginLeft: 2 }}>{unit}</span>}
 </div>
 <div style={{ fontSize: 9, color: "var(--text-secondary)", fontWeight: 600, marginTop: 2 }}>{label}</div>
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
 const cancelRef = useRef(false);
 const taskIdRef = useRef(0);

 useEffect(() => () => { unlistenRef.current?.(); }, []);

 const parseHeaders = () => {
 try {
 const h = JSON.parse(headersText);
 return typeof h === "object" && h !== null ? h as Record<string, string> : undefined;
 } catch { return undefined; }
 };

 const handleSuspend = () => {
 cancelRef.current = true;
 setRunning(false);
 setError("Load test suspended by user.");
 };

 const run = async () => {
 if (!url || running) return;
 cancelRef.current = false;
 taskIdRef.current += 1;
 const thisId = taskIdRef.current;
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
 if (cancelRef.current || taskIdRef.current !== thisId) return;
 setResult(res);
 setProgress(total);
 } catch (e) {
 if (cancelRef.current || taskIdRef.current !== thisId) return;
 setError(String(e));
 } finally {
 if (!cancelRef.current && taskIdRef.current === thisId) {
 setRunning(false);
 }
 unlistenRef.current?.();
 unlistenRef.current = null;
 }
 };

 const progressPct = total > 0 ? Math.round((progress / total) * 100) : 0;
 const successRate = result ? Math.round((result.success / result.total_requests) * 100) : null;

 return (
 <div className="panel-container">
 {/* Header */}
 <div className="panel-header">
 <span style={{ fontSize: 16 }}></span>
 <h3>HTTP Load Tester</h3>
 </div>

 <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 {error && (
 <div style={{ padding: "6px 10px", background: "var(--error-bg)", color: "var(--error-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-sm)" }}> {error}</div>
 )}

 {/* URL + method */}
 <div style={{ display: "flex", gap: 6 }}>
 <select
 value={method}
 onChange={(e) => setMethod(e.target.value)}
 style={{ padding: "6px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none", fontWeight: 600 }}
 >
 {METHODS.map((m) => <option key={m}>{m}</option>)}
 </select>
 <input
 value={url}
 onChange={(e) => setUrl(e.target.value)}
 placeholder="https://api.example.com/endpoint"
 style={{ flex: 1, padding: "6px 10px", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }}
 />
 </div>

 {/* Presets */}
 <div style={{ display: "flex", gap: 5, flexWrap: "wrap" }}>
 {PRESETS.map((p) => (
 <button
 key={p.label}
 onClick={() => { setTotal(p.total); setConcurrency(p.concurrency); }}
 style={{ padding: "3px 9px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-md)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", color: "var(--text-secondary)", cursor: "pointer" }}
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
 <label style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)" }}>{label}</label>
 <input
 type="number" min={min} max={max} value={value}
 onChange={(e) => setter(Math.max(min, Math.min(max, Number(e.target.value))))}
 style={{ padding: "5px 8px", fontSize: "var(--font-size-base)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }}
 />
 </div>
 ))}

 {running ? (
 <button
 onClick={handleSuspend}
 style={{
 padding: "6px 20px", fontSize: "var(--font-size-base)", fontWeight: 700, alignSelf: "flex-end",
 background: "var(--error-color)",
 color: "var(--btn-primary-fg)",
 border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer",
 height: 32,
 }}
 >
 Suspend ({progress}/{total})
 </button>
 ) : (
 <button
 onClick={run}
 disabled={!url}
 style={{
 padding: "6px 20px", fontSize: "var(--font-size-base)", fontWeight: 700, alignSelf: "flex-end",
 background: "var(--accent-color)",
 color: "var(--btn-primary-fg)",
 border: "none", borderRadius: "var(--radius-xs-plus)", cursor: !url ? "not-allowed" : "pointer",
 height: 32,
 }}
 >
 Run
 </button>
 )}
 </div>

 {/* Optional body + headers */}
 {(method === "POST" || method === "PUT" || method === "PATCH") && (
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)" }}>Request Body</label>
 <textarea
 value={body}
 onChange={(e) => setBody(e.target.value)}
 rows={3}
 placeholder='{"key": "value"}'
 style={{ padding: "6px 10px", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none", resize: "vertical" }}
 />
 </div>
 )}

 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)" }}>Headers (JSON)</label>
 <input
 value={headersText}
 onChange={(e) => setHeadersText(e.target.value)}
 placeholder='{"Authorization": "Bearer TOKEN"}'
 style={{ padding: "5px 10px", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-secondary)", outline: "none" }}
 />
 </div>

 {/* Progress bar */}
 {running && (
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
 <span>Running… {progress}/{total}</span>
 <span>{progressPct}%</span>
 </div>
 <div style={{ height: 8, background: "var(--bg-secondary)", borderRadius: "var(--radius-xs-plus)", overflow: "hidden", border: "1px solid var(--border-color)" }}>
 <div style={{ height: "100%", width: `${progressPct}%`, background: "var(--accent-color)", borderRadius: "var(--radius-xs-plus)", transition: "width 0.2s" }} />
 </div>
 </div>
 )}

 {/* Results */}
 {result && (
 <>
 {/* Summary row */}
 <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
 <StatCard label="Req/sec" value={result.requests_per_sec} color="var(--accent-color)" />
 <StatCard label="Avg" value={result.avg_ms} unit="ms" />
 <StatCard label="p50" value={result.p50_ms} unit="ms" />
 <StatCard label="p90" value={result.p90_ms} unit="ms" />
 <StatCard label="p99" value={result.p99_ms} unit="ms" />
 <StatCard label="Min/Max" value={`${result.min_ms}/${result.max_ms}`} unit="ms" />
 </div>

 {/* Success / failure */}
 <div style={{ display: "flex", gap: 6 }}>
 <div style={{ flex: 1, padding: "8px 10px", background: "color-mix(in srgb, var(--accent-green) 10%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-sm)", textAlign: "center" }}>
 <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--success-color)" }}>{result.success}</div>
 <div style={{ fontSize: 9, color: "var(--success-color)", fontWeight: 600 }}>SUCCESS ({successRate}%)</div>
 </div>
 {result.failed > 0 && (
 <div style={{ flex: 1, padding: "8px 10px", background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", border: "1px solid var(--error-color)", borderRadius: "var(--radius-sm)", textAlign: "center" }}>
 <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--error-color)" }}>{result.failed}</div>
 <div style={{ fontSize: 9, color: "var(--error-color)", fontWeight: 600 }}>FAILED ({100 - (successRate ?? 0)}%)</div>
 </div>
 )}
 <div style={{ flex: 1, padding: "8px 10px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", textAlign: "center" }}>
 <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{(result.duration_ms / 1000).toFixed(2)}s</div>
 <div style={{ fontSize: 9, color: "var(--text-secondary)", fontWeight: 600 }}>TOTAL TIME</div>
 </div>
 </div>

 {/* Latency bar chart (visual) */}
 <div style={{ padding: "10px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
 <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, marginBottom: 8 }}>Latency Distribution</div>
 <div style={{ display: "flex", alignItems: "flex-end", gap: 4, height: 50 }}>
 {[
 { label: "avg", val: result.avg_ms },
 { label: "p50", val: result.p50_ms },
 { label: "p90", val: result.p90_ms },
 { label: "p99", val: result.p99_ms },
 { label: "max", val: result.max_ms },
 ].map(({ label, val }) => {
 const h = result.max_ms === 0 ? 4 : Math.max(4, Math.round((val / result.max_ms) * 46));
 const color = label === "p99" || label === "max" ? "var(--error-color)"
 : label === "p90" ? "var(--warning-color)" : "var(--accent-color)";
 return (
 <div key={label} style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", gap: 2 }}>
 <span style={{ fontSize: 9, color: "var(--text-secondary)" }}>{val}ms</span>
 <div style={{ width: "70%", height: h, background: color, borderRadius: "2px 2px 0 0" }} />
 <span style={{ fontSize: 9, color: "var(--text-secondary)", fontWeight: 600 }}>{label}</span>
 </div>
 );
 })}
 </div>
 </div>

 {/* Status code breakdown */}
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, marginBottom: 6 }}>Status Codes</div>
 <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
 {Object.entries(result.status_codes)
 .sort(([a], [b]) =>Number(a) - Number(b))
 .map(([code, count]) => {
 const c = Number(code);
 const color = c === 0 ? "var(--error-color)" : c < 300 ? "var(--success-color)" : c < 400 ? "var(--warning-color)" : "var(--error-color)";
 return (
 <div key={code} style={{ padding: "5px 12px", borderRadius: 20, border: `1px solid ${color}`, background: `${color}22`, fontSize: "var(--font-size-sm)" }}>
 <span style={{ color, fontWeight: 700 }}>{code === "0" ? "ERR" : code}</span>
 <span style={{ color: "var(--text-secondary)", marginLeft: 6 }}>×{count}</span>
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
