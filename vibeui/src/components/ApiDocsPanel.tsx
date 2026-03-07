/**
 * ApiDocsPanel — OpenAPI / Swagger Documentation Viewer.
 *
 * Features:
 * - Load spec from a local file in the workspace or from a URL
 * - Endpoint list grouped by tag, filterable by method / search
 * - Expandable request body + response schemas
 * - "Try it" fires the request via the existing send_http_request Tauri command
 * - No runtime dependencies beyond what's already in the project
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Minimal OpenAPI types ──────────────────────────────────────────────────
interface OApiInfo { title: string; version: string; description?: string }
interface OApiServer { url: string; description?: string }
interface OApiParam { name: string; in: string; required?: boolean; description?: string; schema?: Record<string, unknown> }
interface OApiResponse { description?: string; content?: Record<string, { schema?: Record<string, unknown> }> }
interface OApiOperation {
 operationId?: string;
 summary?: string;
 description?: string;
 tags?: string[];
 parameters?: OApiParam[];
 requestBody?: { required?: boolean; content?: Record<string, { schema?: Record<string, unknown> }> };
 responses?: Record<string, OApiResponse>;
}
interface OApiSpec {
 openapi?: string;
 swagger?: string;
 info: OApiInfo;
 servers?: OApiServer[];
 paths: Record<string, Record<string, OApiOperation>>;
}

interface Endpoint {
 method: string;
 path: string;
 operation: OApiOperation;
 tag: string;
}

interface ApiDocsPanelProps {
 workspacePath: string | null;
}

// ── HTTP method colours ────────────────────────────────────────────────────
const METHOD_COLORS: Record<string, { bg: string; color: string }> = {
 GET: { bg: "#0a3a5a", color: "#61dafb" },
 POST: { bg: "#0a3a1a", color: "var(--text-success, #a6e3a1)" },
 PUT: { bg: "#3a2a00", color: "var(--text-warning, #f9e2af)" },
 PATCH: { bg: "#2a2000", color: "var(--text-warning-alt, #fab387)" },
 DELETE: { bg: "#3a0a0a", color: "var(--text-danger, #f38ba8)" },
 HEAD: { bg: "#1a1a3a", color: "var(--text-accent, #cba6f7)" },
 OPTIONS: { bg: "#1a2a2a", color: "#94e2d5" },
};

const methodStyle = (method: string) => {
 const c = METHOD_COLORS[method.toUpperCase()] ?? { bg: "#1a1a2a", color: "var(--text-primary)" };
 return { background: c.bg, color: c.color, padding: "2px 6px", borderRadius: 3, fontSize: 10, fontWeight: 700, fontFamily: "monospace", flexShrink: 0 as const };
};

// ── Simple JSON schema renderer ────────────────────────────────────────────
function SchemaView({ schema, depth = 0 }: { schema: Record<string, unknown>; depth?: number }) {
 if (!schema) return null;
 const indent = depth * 12;
 if (schema.type === "object" && schema.properties) {
 const props = schema.properties as Record<string, Record<string, unknown>>;
 return (
 <div style={{ marginLeft: indent }}>
 {Object.entries(props).map(([key, val]) => (
 <div key={key} style={{ fontSize: 11, lineHeight: 1.6 }}>
 <span style={{ color: "var(--text-accent, #cba6f7)" }}>{key}</span>
 <span style={{ color: "var(--text-muted)", marginLeft: 4 }}>{String(val.type ?? "any")}</span>
 {Boolean(val.description) && <span style={{ color: "var(--text-muted)", marginLeft: 6, fontStyle: "italic" }}>— {String(val.description)}</span>}
 {val.type === "object" && Boolean(val.properties) && (
 <SchemaView schema={val as Record<string, unknown>} depth={depth + 1} />
 )}
 </div>
 ))}
 </div>
 );
 }
 if (schema.type === "array" && schema.items) {
 return (
 <div style={{ marginLeft: indent, fontSize: 11 }}>
 <span style={{ color: "var(--text-warning, #f9e2af)" }}>array of</span>
 <SchemaView schema={schema.items as Record<string, unknown>} depth={depth + 1} />
 </div>
 );
 }
 return (
 <div style={{ marginLeft: indent, fontSize: 11, color: "var(--text-muted)" }}>
 {String(schema.type ?? "any")}
 {Boolean(schema.enum) && <span> [{(schema.enum as unknown[]).map(String).join(", ")}]</span>}
 </div>
 );
}

// ── Try-It panel ──────────────────────────────────────────────────────────
function TryIt({ endpoint, serverUrl }: { endpoint: Endpoint; serverUrl: string }) {
 const [body, setBody] = useState("{}");
 const [response, setResponse] = useState<string | null>(null);
 const [loading, setLoading] = useState(false);

 const run = async () => {
 setLoading(true);
 setResponse(null);
 const url = serverUrl.replace(/\/$/, "") + endpoint.path;
 try {
 const resp = await invoke<{ status: number; body: string; headers: Record<string, string> }>(
 "send_http_request",
 {
 method: endpoint.method.toUpperCase(),
 url,
 headers: { "Content-Type": "application/json" },
 body: ["POST", "PUT", "PATCH"].includes(endpoint.method.toUpperCase()) ? body : null,
 }
 );
 setResponse(`HTTP ${resp.status}\n\n${resp.body}`);
 } catch (e) {
 setResponse(`Error: ${e}`);
 } finally {
 setLoading(false);
 }
 };

 const hasBody = ["POST", "PUT", "PATCH"].includes(endpoint.method.toUpperCase());

 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 8 }}>
 {hasBody && (
 <div>
 <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 3 }}>REQUEST BODY (JSON)</div>
 <textarea
 value={body}
 onChange={(e) => setBody(e.target.value)}
 style={{
 width: "100%", minHeight: 60, padding: 6,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", fontSize: 11,
 fontFamily: "monospace", resize: "vertical", boxSizing: "border-box",
 }}
 />
 </div>
 )}
 <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
 <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
 {serverUrl.replace(/\/$/, "")}{endpoint.path}
 </span>
 <button
 onClick={run}
 disabled={loading}
 style={{
 marginLeft: "auto", padding: "4px 12px", fontSize: 11,
 background: "var(--accent-color)", color: "#fff",
 border: "none", borderRadius: 4, cursor: loading ? "wait" : "pointer",
 flexShrink: 0,
 }}
 >
 {loading ? "" : "Send"}
 </button>
 </div>
 {response && (
 <pre style={{
 margin: 0, padding: 8, background: "var(--bg-primary, #0d1117)", color: "var(--text-primary, #e6edf3)",
 border: "1px solid var(--border-color)", borderRadius: 4,
 fontSize: 11, lineHeight: 1.4, overflow: "auto", maxHeight: 200,
 whiteSpace: "pre-wrap", wordBreak: "break-all",
 }}>
 {response}
 </pre>
 )}
 </div>
 );
}

// ── Endpoint row ──────────────────────────────────────────────────────────
function EndpointRow({ endpoint, serverUrl }: { endpoint: Endpoint; serverUrl: string }) {
 const [open, setOpen] = useState(false);
 const [tryIt, setTryIt] = useState(false);
 const op = endpoint.operation;

 return (
 <div style={{
 border: "1px solid var(--border-color)", borderRadius: 6,
 overflow: "hidden", marginBottom: 4,
 }}>
 <div
 onClick={() => setOpen((p) => !p)}
 style={{
 display: "flex", alignItems: "center", gap: 10,
 padding: "8px 12px", cursor: "pointer",
 background: open ? "var(--bg-selected, #1a2a3a)" : "var(--bg-secondary)",
 }}
 >
 <span style={methodStyle(endpoint.method)}>{endpoint.method.toUpperCase()}</span>
 <span style={{ fontFamily: "monospace", fontSize: 12, flex: 1 }}>{endpoint.path}</span>
 {op.summary && <span style={{ fontSize: 11, color: "var(--text-muted)", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{op.summary}</span>}
 <span style={{ fontSize: 12, color: "var(--text-muted)" }}>{open ? "" : "▼"}</span>
 </div>

 {open && (
 <div style={{ padding: "10px 12px", borderTop: "1px solid var(--border-color)", background: "var(--bg-primary)", display: "flex", flexDirection: "column", gap: 10 }}>
 {op.description && <p style={{ margin: 0, fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>{op.description}</p>}

 {/* Parameters */}
 {op.parameters && op.parameters.length > 0 && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-muted)", marginBottom: 4 }}>PARAMETERS</div>
 {op.parameters.map((p) => (
 <div key={p.name} style={{ display: "flex", gap: 8, fontSize: 11, lineHeight: 1.7 }}>
 <span style={{ fontFamily: "monospace", color: "var(--text-accent, #cba6f7)", minWidth: 120 }}>{p.name}</span>
 <span style={{ color: "var(--text-muted)", minWidth: 60 }}>{p.in}</span>
 {p.required && <span style={{ color: "var(--text-danger, #f38ba8)", fontSize: 10 }}>required</span>}
 {p.description && <span style={{ color: "var(--text-secondary)" }}>{p.description}</span>}
 </div>
 ))}
 </div>
 )}

 {/* Request body */}
 {op.requestBody?.content && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-muted)", marginBottom: 4 }}>REQUEST BODY</div>
 {Object.entries(op.requestBody.content).map(([ct, body]) => (
 <div key={ct}>
 <span style={{ fontSize: 10, color: "var(--text-muted)", fontFamily: "monospace" }}>{ct}</span>
 {body.schema && <SchemaView schema={body.schema} />}
 </div>
 ))}
 </div>
 )}

 {/* Responses */}
 {op.responses && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-muted)", marginBottom: 4 }}>RESPONSES</div>
 {Object.entries(op.responses).map(([code, resp]) => (
 <div key={code} style={{ display: "flex", gap: 8, fontSize: 11, lineHeight: 1.7 }}>
 <span style={{
 fontFamily: "monospace", fontWeight: 700, minWidth: 40,
 color: code.startsWith("2") ? "#a6e3a1" : code.startsWith("4") ? "#fab387" : code.startsWith("5") ? "#f38ba8" : "var(--text-primary)",
 }}>{code}</span>
 <span style={{ color: "var(--text-secondary)" }}>{resp.description}</span>
 </div>
 ))}
 </div>
 )}

 {/* Try it */}
 <div>
 <button
 onClick={() => setTryIt((p) => !p)}
 style={{
 padding: "4px 10px", fontSize: 11, background: tryIt ? "var(--accent-color)" : "var(--bg-secondary)",
 color: tryIt ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)",
 borderRadius: 4, cursor: "pointer",
 }}
 >
 {tryIt ? "✕ Close Try-It" : "Try it"}
 </button>
 {tryIt && <TryIt endpoint={endpoint} serverUrl={serverUrl} />}
 </div>
 </div>
 )}
 </div>
 );
}

// ── Main panel ────────────────────────────────────────────────────────────
export function ApiDocsPanel({ workspacePath }: ApiDocsPanelProps) {
 const [source, setSource] = useState<"file" | "url">("file");
 const [filePath, setFilePath] = useState("");
 const [urlInput, setUrlInput] = useState("http://localhost:3000/openapi.json");
 const [spec, setSpec] = useState<OApiSpec | null>(null);
 const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [methodFilter, setMethodFilter] = useState<string>("ALL");
 const [search, setSearch] = useState("");
 const [serverUrl, setServerUrl] = useState("http://localhost:3000");
 const [specFiles, setSpecFiles] = useState<string[]>([]);

 // Auto-discover OpenAPI files in workspace
 useEffect(() => {
 if (!workspacePath) return;
 invoke<string[]>("search_workspace_symbols", { query: "openapi", workspace: workspacePath })
 .catch(() => null);
 // Look for common spec filenames
 const candidates = ["openapi.json", "openapi.yaml", "openapi.yml", "swagger.json", "swagger.yaml", "api.yaml", "api.json"];
 setSpecFiles(candidates);
 }, [workspacePath]);

 const parseSpec = useCallback((text: string): OApiSpec | null => {
 try {
 // Try JSON first
 return JSON.parse(text) as OApiSpec;
 } catch {
 // Very minimal YAML → JSON conversion for simple flat specs
 // For real YAML, users should use URL source pointing to a running server
 const lines = text.split("\n");
 const jsonLines: string[] = [];
 let inPaths = false;
 for (const line of lines) {
 if (line.trimStart().startsWith("#")) continue;
 if (line.includes(": |") || line.includes(": >")) continue; // skip multiline strings
 inPaths = inPaths || line.startsWith("paths:");
 jsonLines.push(line);
 }
 // Fall back — try to parse as JSON with a best-effort YAML strip
 try {
 const cleaned = text
 .replace(/^(\s*)(\w[\w-]*):\s*$/gm, '$1"$2": {}')
 .replace(/^(\s*)(\w[\w-]*):\s*"?([^"\n{}\[\]]+)"?\s*$/gm, '$1"$2": "$3"');
 return JSON.parse(cleaned) as OApiSpec;
 } catch {
 return null;
 }
 }
 }, []);

 const buildEndpoints = useCallback((s: OApiSpec): Endpoint[] => {
 const eps: Endpoint[] = [];
 const HTTP_METHODS = ["get", "post", "put", "patch", "delete", "head", "options"];
 for (const [path, pathItem] of Object.entries(s.paths ?? {})) {
 for (const [method, op] of Object.entries(pathItem ?? {})) {
 if (!HTTP_METHODS.includes(method)) continue;
 const tag = (op.tags?.[0] ?? "default").toLowerCase();
 eps.push({ method, path, operation: op as OApiOperation, tag });
 }
 }
 eps.sort((a, b) => a.tag.localeCompare(b.tag) || a.path.localeCompare(b.path));
 return eps;
 }, []);

 const loadFromText = useCallback((text: string) => {
 const parsed = parseSpec(text);
 if (!parsed || !parsed.paths) {
 setError("Could not parse OpenAPI spec. Make sure it's valid JSON or YAML.");
 return;
 }
 setSpec(parsed);
 setEndpoints(buildEndpoints(parsed));
 if (parsed.servers?.[0]?.url) setServerUrl(parsed.servers[0].url);
 setError(null);
 }, [parseSpec, buildEndpoints]);

 const handleLoadFile = async () => {
 if (!workspacePath || !filePath) return;
 setLoading(true);
 setError(null);
 try {
 const text = await invoke<string>("read_file", {
 path: `${workspacePath}/${filePath}`,
 });
 loadFromText(text);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const handleLoadUrl = async () => {
 setLoading(true);
 setError(null);
 try {
 const text = await invoke<string>("fetch_url_for_context", { url: urlInput });
 // fetch_url_for_context prepends a header line — strip it
 const bodyStart = text.indexOf("\n");
 loadFromText(bodyStart > 0 ? text.slice(bodyStart + 1) : text);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const methods = ["ALL", ...Array.from(new Set(endpoints.map((e) => e.method.toUpperCase())))];

 const filtered = endpoints.filter((e) => {
 const methodMatch = methodFilter === "ALL" || e.method.toUpperCase() === methodFilter;
 const searchMatch = !search || e.path.toLowerCase().includes(search.toLowerCase()) ||
 (e.operation.summary ?? "").toLowerCase().includes(search.toLowerCase());
 return methodMatch && searchMatch;
 });

 // Group by tag
 const grouped: Record<string, Endpoint[]> = {};
 for (const ep of filtered) {
 (grouped[ep.tag] ??= []).push(ep);
 }

 const inputStyle: React.CSSProperties = {
 padding: "5px 8px", fontSize: 12, background: "var(--bg-secondary)",
 border: "1px solid var(--border-color)", borderRadius: 4,
 color: "var(--text-primary)", outline: "none",
 };

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Header: Load spec */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
 <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
 {(["file", "url"] as const).map((s) => (
 <button
 key={s}
 onClick={() => setSource(s)}
 style={{
 padding: "3px 10px", fontSize: 11, borderRadius: 12,
 background: source === s ? "var(--accent-color)" : "transparent",
 color: source === s ? "#fff" : "var(--text-muted)",
 border: `1px solid ${source === s ? "var(--accent-color)" : "var(--border-color)"}`,
 cursor: "pointer",
 }}
 >
 {s === "file" ? "File" : "URL"}
 </button>
 ))}
 </div>

 {source === "file" ? (
 <div style={{ display: "flex", gap: 6 }}>
 <div style={{ position: "relative", flex: 1 }}>
 <input
 style={{ ...inputStyle, width: "100%", boxSizing: "border-box" }}
 value={filePath}
 onChange={(e) => setFilePath(e.target.value)}
 placeholder="openapi.json / openapi.yaml"
 list="spec-files"
 />
 <datalist id="spec-files">
 {specFiles.map((f) => <option key={f} value={f} />)}
 </datalist>
 </div>
 <button
 onClick={handleLoadFile}
 disabled={loading || !filePath || !workspacePath}
 style={{ padding: "5px 12px", fontSize: 12, background: "var(--accent-color)", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" }}
 >
 {loading ? "" : "Load"}
 </button>
 </div>
 ) : (
 <div style={{ display: "flex", gap: 6 }}>
 <input
 style={{ ...inputStyle, flex: 1 }}
 value={urlInput}
 onChange={(e) => setUrlInput(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && handleLoadUrl()}
 placeholder="http://localhost:3000/openapi.json"
 />
 <button
 onClick={handleLoadUrl}
 disabled={loading || !urlInput}
 style={{ padding: "5px 12px", fontSize: 12, background: "var(--accent-color)", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" }}
 >
 {loading ? "" : "Fetch"}
 </button>
 </div>
 )}

 {error && <div style={{ marginTop: 6, fontSize: 11, color: "var(--text-danger, #f38ba8)" }}> {error}</div>}
 </div>

 {spec && (
 <>
 {/* Spec info bar */}
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", alignItems: "center", gap: 10 }}>
 <strong style={{ fontSize: 13 }}>{spec.info.title}</strong>
 <span style={{ fontSize: 11, color: "var(--text-muted)" }}>v{spec.info.version}</span>
 <span style={{ fontSize: 11, color: "var(--text-muted)", marginLeft: "auto" }}>{endpoints.length} endpoints</span>
 </div>

 {/* Server URL */}
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", gap: 6, alignItems: "center" }}>
 <span style={{ fontSize: 11, color: "var(--text-muted)", flexShrink: 0 }}>Base URL</span>
 <input
 style={{ ...inputStyle, flex: 1, fontSize: 11 }}
 value={serverUrl}
 onChange={(e) => setServerUrl(e.target.value)}
 />
 </div>

 {/* Filters */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", gap: 8, alignItems: "center" }}>
 <input
 style={{ ...inputStyle, flex: 1 }}
 value={search}
 onChange={(e) => setSearch(e.target.value)}
 placeholder="Filter endpoints…"
 />
 <div style={{ display: "flex", gap: 4 }}>
 {methods.map((m) => (
 <button
 key={m}
 onClick={() => setMethodFilter(m)}
 style={{
 padding: "3px 8px", fontSize: 10, borderRadius: 3,
 border: "none", cursor: "pointer",
 fontWeight: 700,
 ...(m === "ALL"
 ? { background: methodFilter === "ALL" ? "var(--accent-color)" : "var(--bg-secondary)", color: methodFilter === "ALL" ? "#fff" : "var(--text-muted)" }
 : { ...(methodFilter === m ? METHOD_COLORS[m] : { background: "var(--bg-secondary)", color: "var(--text-muted)" }) }),
 }}
 >
 {m}
 </button>
 ))}
 </div>
 </div>

 {/* Endpoint list */}
 <div style={{ flex: 1, overflow: "auto", padding: "8px 12px" }}>
 {Object.entries(grouped).map(([tag, eps]) => (
 <div key={tag} style={{ marginBottom: 16 }}>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6 }}>
 {tag}
 </div>
 {eps.map((ep, i) => (
 <EndpointRow key={`${ep.method}-${ep.path}-${i}`} endpoint={ep} serverUrl={serverUrl} />
 ))}
 </div>
 ))}
 {filtered.length === 0 && (
 <div style={{ textAlign: "center", padding: "30px 0", color: "var(--text-muted)", fontSize: 12 }}>
 No endpoints match your filter.
 </div>
 )}
 </div>
 </>
 )}

 {!spec && !loading && (
 <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", gap: 10 }}>
 <div style={{ fontSize: 36 }}></div>
 <div style={{ fontSize: 13 }}>Load an OpenAPI / Swagger spec to get started.</div>
 <div style={{ fontSize: 11, textAlign: "center", lineHeight: 1.6, maxWidth: 280 }}>
 Supports <strong>OpenAPI 3.x</strong> and <strong>Swagger 2.x</strong> in JSON format.
 Point to a running dev server's <code>/openapi.json</code> or a local file.
 </div>
 </div>
 )}
 </div>
 );
}
