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
import { ChevronDown } from "lucide-react";

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
 GET: { bg: "var(--info-bg)", color: "var(--info-color)" },
 POST: { bg: "var(--success-bg)", color: "var(--text-success)" },
 PUT: { bg: "var(--warning-bg)", color: "var(--text-warning)" },
 PATCH: { bg: "var(--warning-bg)", color: "var(--text-warning)" },
 DELETE: { bg: "var(--error-bg)", color: "var(--text-danger)" },
 HEAD: { bg: "var(--accent-bg)", color: "var(--text-accent)" },
 OPTIONS: { bg: "var(--success-bg)", color: "var(--text-success)" },
};

const methodStyle = (method: string) => {
 const c = METHOD_COLORS[method.toUpperCase()] ?? { bg: "var(--bg-tertiary)", color: "var(--text-primary)" };
 return { background: c.bg, color: c.color, padding: "2px 6px", borderRadius: 3, fontSize: 10, fontWeight: 700, fontFamily: "var(--font-mono)", flexShrink: 0 as const };
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
 <span style={{ color: "var(--text-accent)" }}>{key}</span>
 <span style={{ color: "var(--text-secondary)", marginLeft: 4 }}>{String(val.type ?? "any")}</span>
 {Boolean(val.description) && <span style={{ color: "var(--text-secondary)", marginLeft: 6, fontStyle: "italic" }}>— {String(val.description)}</span>}
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
 <span style={{ color: "var(--text-warning)" }}>array of</span>
 <SchemaView schema={schema.items as Record<string, unknown>} depth={depth + 1} />
 </div>
 );
 }
 return (
 <div style={{ marginLeft: indent, fontSize: 11, color: "var(--text-secondary)" }}>
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
 <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 3 }}>REQUEST BODY (JSON)</div>
 <textarea
 value={body}
 onChange={(e) => setBody(e.target.value)}
 style={{
 width: "100%", minHeight: 60, padding: 6,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", fontSize: 11,
 fontFamily: "var(--font-mono)", resize: "vertical", boxSizing: "border-box",
 }}
 />
 </div>
 )}
 <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
 <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
 {serverUrl.replace(/\/$/, "")}{endpoint.path}
 </span>
 <button
 onClick={run}
 disabled={loading}
 style={{
 marginLeft: "auto", padding: "4px 12px", fontSize: 11,
 background: "var(--accent-color)", color: "var(--text-primary)",
 border: "none", borderRadius: 4, cursor: loading ? "wait" : "pointer",
 flexShrink: 0,
 }}
 >
 {loading ? "" : "Send"}
 </button>
 </div>
 {response && (
 <pre style={{
 margin: 0, padding: 8, background: "var(--bg-primary)", color: "var(--text-primary)",
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
 background: open ? "var(--bg-selected)" : "var(--bg-secondary)",
 }}
 >
 <span style={methodStyle(endpoint.method)}>{endpoint.method.toUpperCase()}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, flex: 1 }}>{endpoint.path}</span>
 {op.summary && <span style={{ fontSize: 11, color: "var(--text-secondary)", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{op.summary}</span>}
 <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>{open ? "" : <ChevronDown size={12} />}</span>
 </div>

 {open && (
 <div style={{ padding: "10px 12px", borderTop: "1px solid var(--border-color)", background: "var(--bg-primary)", display: "flex", flexDirection: "column", gap: 10 }}>
 {op.description && <p style={{ margin: 0, fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>{op.description}</p>}

 {/* Parameters */}
 {op.parameters && op.parameters.length > 0 && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-secondary)", marginBottom: 4 }}>PARAMETERS</div>
 {op.parameters.map((p) => (
 <div key={p.name} style={{ display: "flex", gap: 8, fontSize: 11, lineHeight: 1.7 }}>
 <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-accent)", minWidth: 120 }}>{p.name}</span>
 <span style={{ color: "var(--text-secondary)", minWidth: 60 }}>{p.in}</span>
 {p.required && <span style={{ color: "var(--text-danger)", fontSize: 10 }}>required</span>}
 {p.description && <span style={{ color: "var(--text-secondary)" }}>{p.description}</span>}
 </div>
 ))}
 </div>
 )}

 {/* Request body */}
 {op.requestBody?.content && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-secondary)", marginBottom: 4 }}>REQUEST BODY</div>
 {Object.entries(op.requestBody.content).map(([ct, body]) => (
 <div key={ct}>
 <span style={{ fontSize: 10, color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>{ct}</span>
 {body.schema && <SchemaView schema={body.schema} />}
 </div>
 ))}
 </div>
 )}

 {/* Responses */}
 {op.responses && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-secondary)", marginBottom: 4 }}>RESPONSES</div>
 {Object.entries(op.responses).map(([code, resp]) => (
 <div key={code} style={{ display: "flex", gap: 8, fontSize: 11, lineHeight: 1.7 }}>
 <span style={{
 fontFamily: "var(--font-mono)", fontWeight: 700, minWidth: 40,
 color: code.startsWith("2") ? "var(--success-color)" : code.startsWith("4") ? "var(--warning-color)" : code.startsWith("5") ? "var(--error-color)" : "var(--text-primary)",
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
 color: tryIt ? "var(--text-primary)" : "var(--text-primary)", border: "1px solid var(--border-color)",
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

 return (
 <div className="panel-container">
 {/* Header: Load spec */}
 <div className="panel-header" style={{ padding: "10px 12px", flexDirection: "column", alignItems: "stretch" }}>
 <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
 {(["file", "url"] as const).map((s) => (
 <button
 key={s}
 onClick={() => setSource(s)}
 className={`panel-tab ${source === s ? "active" : ""}`}
 >
 {s === "file" ? "File" : "URL"}
 </button>
 ))}
 </div>

 {source === "file" ? (
 <div style={{ display: "flex", gap: 6 }}>
 <div style={{ position: "relative", flex: 1 }}>
 <input
 style={{ padding: "5px 8px", fontSize: 12, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", width: "100%", boxSizing: "border-box" }}
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
 style={{ padding: "5px 12px", fontSize: 12, background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer" }}
 >
 {loading ? "" : "Load"}
 </button>
 </div>
 ) : (
 <div style={{ display: "flex", gap: 6 }}>
 <input
 style={{ padding: "5px 8px", fontSize: 12, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", flex: 1 }}
 value={urlInput}
 onChange={(e) => setUrlInput(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && handleLoadUrl()}
 placeholder="http://localhost:3000/openapi.json"
 />
 <button
 onClick={handleLoadUrl}
 disabled={loading || !urlInput}
 style={{ padding: "5px 12px", fontSize: 12, background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer" }}
 >
 {loading ? "" : "Fetch"}
 </button>
 </div>
 )}

 {error && <div style={{ marginTop: 6, fontSize: 11, color: "var(--text-danger)" }}> {error}</div>}
 </div>

 {spec && (
 <>
 {/* Spec info bar */}
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", alignItems: "center", gap: 10 }}>
 <strong style={{ fontSize: 13 }}>{spec.info.title}</strong>
 <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>v{spec.info.version}</span>
 <span style={{ fontSize: 11, color: "var(--text-secondary)", marginLeft: "auto" }}>{endpoints.length} endpoints</span>
 </div>

 {/* Server URL */}
 <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", gap: 6, alignItems: "center" }}>
 <span style={{ fontSize: 11, color: "var(--text-secondary)", flexShrink: 0 }}>Base URL</span>
 <input
 style={{ padding: "5px 8px", fontSize: 11, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", flex: 1 }}
 value={serverUrl}
 onChange={(e) => setServerUrl(e.target.value)}
 />
 </div>

 {/* Filters */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", gap: 8, alignItems: "center" }}>
 <input
 style={{ padding: "5px 8px", fontSize: 12, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", flex: 1 }}
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
 ? { background: methodFilter === "ALL" ? "var(--accent-color)" : "var(--bg-secondary)", color: methodFilter === "ALL" ? "var(--text-primary)" : "var(--text-secondary)" }
 : { ...(methodFilter === m ? METHOD_COLORS[m] : { background: "var(--bg-secondary)", color: "var(--text-secondary)" }) }),
 }}
 >
 {m}
 </button>
 ))}
 </div>
 </div>

 {/* Endpoint list */}
 <div className="panel-body" style={{ padding: "8px 12px", overflow: "auto", display: "block" }}>
 {Object.entries(grouped).map(([tag, eps]) => (
 <div key={tag} style={{ marginBottom: 16 }}>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6 }}>
 {tag}
 </div>
 {eps.map((ep, i) => (
 <EndpointRow key={`${ep.method}-${ep.path}-${i}`} endpoint={ep} serverUrl={serverUrl} />
 ))}
 </div>
 ))}
 {filtered.length === 0 && (
 <div style={{ textAlign: "center", padding: "30px 0", color: "var(--text-secondary)", fontSize: 12 }}>
 No endpoints match your filter.
 </div>
 )}
 </div>
 </>
 )}

 {!spec && !loading && (
 <div className="panel-empty" style={{ gap: 10 }}>
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
