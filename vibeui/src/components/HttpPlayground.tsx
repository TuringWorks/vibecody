import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Header { key: string; value: string; }

interface HttpResponseData {
 status: number;
 status_text: string;
 headers: Header[];
 body: string;
 duration_ms: number;
}

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

const statusColor = (code: number) => {
 if (code < 300) return "var(--success-color)";
 if (code < 400) return "var(--info-color)";
 if (code < 500) return "var(--warning-color)";
 return "var(--error-color)";
};

const isJsonLike = (body: string) => body.trimStart().startsWith("{") || body.trimStart().startsWith("[");

function tryPrettyJson(body: string) {
 try { return JSON.stringify(JSON.parse(body), null, 2); } catch { return null; }
}

export function HttpPlayground({ workspacePath }: { workspacePath: string | null }) {
 const [method, setMethod] = useState("GET");
 const [url, setUrl] = useState("http://localhost:3000/");
 const [headers, setHeaders] = useState<Header[]>([{ key: "Content-Type", value: "application/json" }]);
 const [body, setBody] = useState("");
 const [response, setResponse] = useState<HttpResponseData | null>(null);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [resTab, setResTab] = useState<"body" | "headers">("body");
 const [prettyBody, setPrettyBody] = useState(true);
 const [endpoints, setEndpoints] = useState<string[]>([]);
 const [endpointsLoading, setEndpointsLoading] = useState(false);

 const showBody = ["POST", "PUT", "PATCH"].includes(method);

 const addHeader = () => setHeaders(h => [...h, { key: "", value: "" }]);
 const removeHeader = (i: number) => setHeaders(h => h.filter((_, idx) => idx !== i));
 const updateHeader = (i: number, field: "key" | "value", val: string) =>
 setHeaders(h => h.map((row, idx) => idx === i ? { ...row, [field]: val } : row));

 const handleSend = async () => {
 if (!url.trim()) return;
 setLoading(true);
 setError(null);
 setResponse(null);
 try {
 const activeHeaders = headers.filter(h => h.key.trim());
 const r = await invoke<HttpResponseData>("send_http_request", {
 method,
 url: url.trim(),
 headers: activeHeaders,
 body: showBody && body.trim() ? body.trim() : null,
 });
 setResponse(r);
 setResTab("body");
 } catch (e: unknown) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const handleDiscover = async () => {
 if (!workspacePath) return;
 setEndpointsLoading(true);
 try {
 const found = await invoke<string[]>("discover_api_endpoints", { workspace: workspacePath });
 setEndpoints(found);
 } catch {
 setEndpoints([]);
 } finally {
 setEndpointsLoading(false);
 }
 };

 const handleKey = (e: React.KeyboardEvent) => {
 if ((e.ctrlKey || e.metaKey) && e.key === "Enter") handleSend();
 };

 const displayBody = (() => {
 if (!response) return "";
 if (prettyBody && isJsonLike(response.body)) {
 return tryPrettyJson(response.body) ?? response.body;
 }
 return response.body;
 })();

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "10px", fontFamily: "monospace", fontSize: "13px" }}>
 <div style={{ fontWeight: "bold" }}>HTTP Playground</div>

 {/* URL bar */}
 <div style={{ display: "flex", gap: "6px" }}>
 <select
 value={method}
 onChange={e => setMethod(e.target.value)}
 style={{ background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "4px 6px", fontFamily: "inherit", fontSize: "13px", width: "90px" }}
 >
 {METHODS.map(m => <option key={m} value={m}>{m}</option>)}
 </select>
 <input
 value={url}
 onChange={e => setUrl(e.target.value)}
 onKeyDown={handleKey}
 placeholder="https://api.example.com/endpoint"
 style={{ flex: 1, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "4px 8px", fontFamily: "inherit", fontSize: "13px" }}
 />
 <button
 onClick={handleSend}
 disabled={loading || !url.trim()}
 style={{ background: loading ? "var(--bg-secondary)" : "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "4px", padding: "4px 16px", cursor: loading ? "default" : "pointer" }}
 >
 {loading ? "" : "Send"}
 </button>
 </div>

 {/* Quick-launch local URLs */}
 <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
 {["localhost:3000", "localhost:5173", "localhost:8080", "localhost:4000", "localhost:7878"].map(h => (
 <button
 key={h}
 onClick={() => setUrl(`http://${h}/`)}
 style={{ background: "var(--bg-secondary)", color: "var(--text-muted)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "2px 8px", cursor: "pointer", fontSize: "11px" }}
 >
 {h}
 </button>
 ))}
 {workspacePath && (
 <button
 onClick={handleDiscover}
 disabled={endpointsLoading}
 style={{ marginLeft: "auto", background: "var(--bg-secondary)", color: "var(--text-muted)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "2px 8px", cursor: "pointer", fontSize: "11px" }}
 >
 {endpointsLoading ? "…" : "Discover routes"}
 </button>
 )}
 </div>

 {/* Discovered routes */}
 {endpoints.length > 0 && (
 <div style={{ background: "var(--bg-secondary)", borderRadius: "4px", padding: "8px", maxHeight: "100px", overflowY: "auto" }}>
 <div style={{ color: "var(--text-muted)", fontSize: "11px", marginBottom: "4px" }}>Discovered routes ({endpoints.length})</div>
 {endpoints.map((ep, i) => (
 <div
 key={i}
 onClick={() => setUrl(ep)}
 style={{ fontSize: "11px", color: "var(--text-secondary)", cursor: "pointer", padding: "1px 0", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
 title={ep}
 >
 {ep}
 </div>
 ))}
 </div>
 )}

 {/* Headers */}
 <div>
 <div style={{ display: "flex", alignItems: "center", marginBottom: "4px" }}>
 <span style={{ color: "var(--text-muted)", fontSize: "11px" }}>Headers</span>
 <button onClick={addHeader} style={{ marginLeft: "8px", background: "none", color: "var(--accent-color)", border: "none", cursor: "pointer", fontSize: "12px", padding: "0 4px" }}>+</button>
 </div>
 <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
 {headers.map((h, i) => (
 <div key={i} style={{ display: "flex", gap: "4px" }}>
 <input
 value={h.key}
 onChange={e => updateHeader(i, "key", e.target.value)}
 placeholder="Key"
 style={{ flex: 1, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "3px 6px", fontFamily: "inherit", fontSize: "12px" }}
 />
 <input
 value={h.value}
 onChange={e => updateHeader(i, "value", e.target.value)}
 placeholder="Value"
 style={{ flex: 2, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "3px 6px", fontFamily: "inherit", fontSize: "12px" }}
 />
 <button onClick={() => removeHeader(i)} style={{ background: "none", color: "var(--error-color)", border: "none", cursor: "pointer", padding: "0 6px", fontSize: "14px" }}>×</button>
 </div>
 ))}
 </div>
 </div>

 {/* Request body */}
 {showBody && (
 <textarea
 value={body}
 onChange={e => setBody(e.target.value)}
 placeholder='{"key": "value"}'
 rows={4}
 style={{ resize: "vertical", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", padding: "6px 8px", fontFamily: "inherit", fontSize: "12px" }}
 />
 )}

 {error && (
 <div style={{ color: "var(--error-color)", fontSize: "12px" }}>{error}</div>
 )}

 {/* Response panel */}
 {response && (
 <div style={{ flex: 1, display: "flex", flexDirection: "column", border: "1px solid var(--border-color)", borderRadius: "6px", overflow: "hidden", minHeight: "150px" }}>
 {/* Status bar */}
 <div style={{ padding: "6px 12px", background: "var(--bg-secondary)", display: "flex", alignItems: "center", gap: "10px" }}>
 <span style={{ fontWeight: "bold", color: statusColor(response.status), fontSize: "15px" }}>
 {response.status}
 </span>
 <span style={{ color: "var(--text-secondary)" }}>{response.status_text}</span>
 <span style={{ marginLeft: "auto", color: "var(--text-muted)", fontSize: "11px" }}>
 {response.duration_ms}ms
 </span>
 {/* Tabs */}
 <div style={{ display: "flex", gap: "4px" }}>
 {(["body", "headers"] as const).map(t => (
 <button
 key={t}
 onClick={() => setResTab(t)}
 style={{
 background: resTab === t ? "var(--accent-color)" : "transparent",
 color: resTab === t ? "var(--text-primary)" : "var(--text-muted)",
 border: "none", borderRadius: "4px", padding: "2px 8px", cursor: "pointer", fontSize: "11px",
 }}
 >
 {t === "body" ? `Body (${response.body.length}B)` : `Headers (${response.headers.length})`}
 </button>
 ))}
 {resTab === "body" && isJsonLike(response.body) && (
 <button
 onClick={() => setPrettyBody(p => !p)}
 style={{ background: prettyBody ? "rgba(255,255,255,0.08)" : "transparent", color: "var(--text-muted)", border: "none", borderRadius: "4px", padding: "2px 6px", cursor: "pointer", fontSize: "11px" }}
 >
 {prettyBody ? "Raw" : "Pretty"}
 </button>
 )}
 </div>
 </div>
 {/* Content */}
 <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px" }}>
 {resTab === "body" ? (
 <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontSize: "12px", fontFamily: "inherit" }}>
 {displayBody || <span style={{ color: "var(--text-muted)" }}>(empty body)</span>}
 </pre>
 ) : (
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "12px" }}>
 <tbody>
 {response.headers.map((h, i) => (
 <tr key={i} style={{ borderBottom: "1px solid var(--bg-secondary)" }}>
 <td style={{ padding: "3px 8px 3px 0", color: "var(--text-muted)", whiteSpace: "nowrap", verticalAlign: "top" }}>{h.key}</td>
 <td style={{ padding: "3px 0", color: "var(--text-secondary)", wordBreak: "break-all" }}>{h.value}</td>
 </tr>
 ))}
 </tbody>
 </table>
 )}
 </div>
 </div>
 )}

 {!response && !loading && (
 <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", textAlign: "center" }}>
 Build a request above and click Send<br />(Ctrl+Enter also works)
 </div>
 )}
 </div>
 );
}
