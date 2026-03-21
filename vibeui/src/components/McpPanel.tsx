/**
 * McpPanel — Unified MCP panel combining Servers, Tool Registry, Directory, and Metrics.
 */
import { useState, useEffect, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface McpServer {
  name: string;
  command: string;
  args: string[];
  env: Record<string, string>;
}

interface McpToolInfo { name: string; description: string; }

interface OAuthForm {
  serverName: string; clientId: string; authUrl: string; tokenUrl: string;
  redirectUri: string; scopes: string; authCode: string;
  step: "config" | "code"; busy: boolean; msg: string | null;
}

interface ToolManifest {
  id: string; name: string; description: string; version: string;
  server_name: string; status: "loaded" | "unloaded" | "loading";
  size_kb: number; last_used: string | null; load_time_ms: number | null;
}

interface SearchResult {
  tool_id: string; name: string; description: string;
  server_name: string; relevance: number;
}

interface LazyMetrics {
  context_savings_pct: number; cache_hits: number; cache_misses: number;
  cache_hit_rate: number; avg_load_time_ms: number;
  load_times: { label: string; ms: number }[]; total_load_time_ms: number;
}

interface McpPlugin {
  id: string; name: string; author: string; description: string;
  category: string; rating: number; downloads: number; version: string;
  installed: boolean; updatable: boolean;
}

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 4px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--btn-primary-fg, var(--text-primary))" : "var(--text-primary)", marginRight: 4, fontWeight: active ? 600 : 400 });
const inputStyle: React.CSSProperties = { padding: "5px 8px", fontSize: "12px", background: "var(--bg-input, var(--bg-primary))", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", outline: "none", fontFamily: "var(--font-mono)", width: "100%", boxSizing: "border-box" };
const barBg: React.CSSProperties = { height: 8, borderRadius: 4, background: "var(--bg-tertiary)", overflow: "hidden" };
const barFill = (pct: number, color: string): React.CSSProperties => ({ height: "100%", width: `${Math.min(pct, 100)}%`, borderRadius: 4, background: color });
const badgeStyle = (v: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "#fff", background: v === "loaded" ? "var(--success-color)" : v === "loading" ? "var(--warning-color)" : "var(--text-muted)" });

const EMPTY_SERVER: McpServer = { name: "", command: "", args: [], env: {} };
const CATEGORIES = ["All", "File Systems", "Git", "Databases", "Cloud", "AI/ML", "Testing", "DevOps", "Communication"];

const renderStars = (r: number): string => "★".repeat(Math.floor(r)) + (r - Math.floor(r) >= 0.5 ? "½" : "") + "☆".repeat(5 - Math.floor(r) - (r - Math.floor(r) >= 0.5 ? 1 : 0));
const formatDl = (n: number): string => n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "servers" | "tools" | "directory" | "installed" | "metrics";

export function McpPanel() {
  const [tab, setTab] = useState<Tab>("servers");
  const [error, setError] = useState<string | null>(null);

  // ── Servers state ─────────────────────────────────────────────────────────
  const [servers, setServers] = useState<McpServer[]>([]);
  const [editing, setEditing] = useState<McpServer | null>(null);
  const [editIdx, setEditIdx] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<number | null>(null);
  const [testResult, setTestResult] = useState<Record<number, McpToolInfo[] | string>>({});
  const [confirmDelete, setConfirmDelete] = useState<number | null>(null);
  const [oauthForm, setOauthForm] = useState<OAuthForm | null>(null);
  const [tokenStatus, setTokenStatus] = useState<Record<string, boolean>>({});

  // ── Tools state ───────────────────────────────────────────────────────────
  const [manifests, setManifests] = useState<ToolManifest[]>([]);
  const [toolSearch, setToolSearch] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [metrics, setMetrics] = useState<LazyMetrics | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  // ── Directory state ───────────────────────────────────────────────────────
  const [plugins, setPlugins] = useState<McpPlugin[]>([]);
  const [dirSearch, setDirSearch] = useState("");
  const [catFilter, setCatFilter] = useState("All");
  const [dirLoading, setDirLoading] = useState(false);
  const [pluginAction, setPluginAction] = useState<string | null>(null);

  // ── Load data ─────────────────────────────────────────────────────────────
  useEffect(() => { loadServers(); }, []);

  async function loadServers() {
    try { setServers(await invoke<McpServer[]>("get_mcp_servers")); }
    catch (e) { setError(String(e)); }
  }

  useEffect(() => {
    let c = false;
    servers.forEach(srv => {
      invoke<{ connected: boolean; expired: boolean }>("get_mcp_token_status", { serverName: srv.name })
        .then(s => { if (!c) setTokenStatus(p => ({ ...p, [srv.name]: s.connected && !s.expired })); })
        .catch(() => { if (!c) setTokenStatus(p => ({ ...p, [srv.name]: false })); });
    });
    return () => { c = true; };
  }, [servers]);

  const fetchTools = useCallback(async () => {
    try { const r = await invoke<{ tools: ToolManifest[] }>("mcp_lazy_list_tools"); setManifests(r.tools ?? []); }
    catch (e) { setError(`Tools: ${e}`); }
  }, []);

  const fetchMetrics = useCallback(async () => {
    try { setMetrics(await invoke<LazyMetrics>("mcp_lazy_metrics")); }
    catch (e) { setError(`Metrics: ${e}`); }
  }, []);

  useEffect(() => { if (tab === "tools") { fetchTools(); } }, [tab, fetchTools]);
  useEffect(() => { if (tab === "metrics") { fetchTools(); fetchMetrics(); } }, [tab, fetchTools, fetchMetrics]);
  useEffect(() => {
    if (tab === "directory" && plugins.length === 0) {
      setDirLoading(true);
      invoke<{ plugins: McpPlugin[]; total: number }>("list_mcp_plugins")
        .then(r => setPlugins(r.plugins ?? []))
        .catch(e => setError(String(e)))
        .finally(() => setDirLoading(false));
    }
  }, [tab, plugins.length]);

  // ── Tool search ───────────────────────────────────────────────────────────
  useEffect(() => {
    if (!toolSearch.trim()) { setSearchResults([]); return; }
    let c = false;
    const t = setTimeout(async () => {
      try {
        const r = await invoke<{ results: SearchResult[] }>("mcp_lazy_search", { query: toolSearch });
        if (!c) setSearchResults(r.results ?? []);
      } catch (e) { if (!c) setError(`Search: ${e}`); }
    }, 200);
    return () => { c = true; clearTimeout(t); };
  }, [toolSearch]);

  // ── Server actions ────────────────────────────────────────────────────────
  async function saveServers(list: McpServer[]) {
    setSaving(true);
    try { await invoke("save_mcp_servers", { servers: list }); setServers(list); }
    catch (e) { setError(String(e)); }
    finally { setSaving(false); }
  }

  async function commitEdit() {
    if (!editing || !editing.name.trim() || !editing.command.trim()) return;
    const updated = [...servers];
    if (editIdx === null) updated.push({ ...editing }); else updated[editIdx] = { ...editing };
    await saveServers(updated);
    setEditing(null); setEditIdx(null);
  }

  async function testServer(idx: number) {
    setTesting(idx);
    try {
      const result = await invoke<McpToolInfo[]>("test_mcp_server", { server: servers[idx] });
      setTestResult(p => ({ ...p, [idx]: result }));
    }
    catch (e) { setTestResult(p => ({ ...p, [idx]: String(e) })); }
    finally { setTesting(null); }
  }

  // ── OAuth ─────────────────────────────────────────────────────────────────
  function startOAuth(name: string) { setOauthForm({ serverName: name, clientId: "", authUrl: "", tokenUrl: "", redirectUri: "http://localhost:7879/oauth/callback", scopes: "read", authCode: "", step: "config", busy: false, msg: null }); }

  async function initiateOAuth() {
    if (!oauthForm) return;
    setOauthForm(f => f && { ...f, busy: true, msg: null });
    try {
      await invoke("initiate_mcp_oauth", { serverName: oauthForm.serverName, clientId: oauthForm.clientId, authUrl: oauthForm.authUrl, redirectUri: oauthForm.redirectUri, scopes: oauthForm.scopes });
      setOauthForm(f => f && { ...f, busy: false, step: "code", msg: "Browser opened. Paste the authorization code below." });
    } catch (e) { setOauthForm(f => f && { ...f, busy: false, msg: `Error: ${e}` }); }
  }

  async function completeOAuth() {
    if (!oauthForm) return;
    setOauthForm(f => f && { ...f, busy: true, msg: null });
    try {
      await invoke("complete_mcp_oauth", { serverName: oauthForm.serverName, code: oauthForm.authCode, tokenUrl: oauthForm.tokenUrl, clientId: oauthForm.clientId, redirectUri: oauthForm.redirectUri });
      setTokenStatus(p => ({ ...p, [oauthForm.serverName]: true }));
      setOauthForm(null);
    } catch (e) { setOauthForm(f => f && { ...f, busy: false, msg: `Token exchange failed: ${e}` }); }
  }

  // ── Tool load/unload ──────────────────────────────────────────────────────
  async function toggleTool(id: string, status: string) {
    setActionLoading(id);
    try {
      if (status === "loaded") await invoke("mcp_lazy_unload_tool", { toolId: id });
      else {
        setManifests(p => p.map(m => m.id === id ? { ...m, status: "loading" as const } : m));
        await invoke("mcp_lazy_load_tool", { toolId: id });
      }
      await fetchTools(); await fetchMetrics();
    } catch (e) { setError(`Action: ${e}`); await fetchTools(); }
    finally { setActionLoading(null); }
  }

  // ── Plugin actions ────────────────────────────────────────────────────────
  async function installPlugin(id: string) {
    setPluginAction(id);
    try { await invoke<{ success: boolean; message: string }>("install_mcp_plugin", { id }); setPlugins(p => p.map(pl => pl.id === id ? { ...pl, installed: true } : pl)); }
    catch (e) { setError(String(e)); }
    finally { setPluginAction(null); }
  }

  async function uninstallPlugin(id: string) {
    setPluginAction(id);
    try { await invoke<{ success: boolean; message: string }>("uninstall_mcp_plugin", { id }); setPlugins(p => p.map(pl => pl.id === id ? { ...pl, installed: false } : pl)); }
    catch (e) { setError(String(e)); }
    finally { setPluginAction(null); }
  }

  // ── Derived ───────────────────────────────────────────────────────────────
  const loadedCount = useMemo(() => manifests.filter(m => m.status === "loaded").length, [manifests]);
  const installedCount = useMemo(() => plugins.filter(p => p.installed).length, [plugins]);
  const dirResults = useMemo(() => {
    let f = plugins;
    if (catFilter !== "All") f = f.filter(p => p.category === catFilter);
    if (dirSearch.trim()) { const q = dirSearch.toLowerCase(); f = f.filter(p => p.name.toLowerCase().includes(q) || p.description.toLowerCase().includes(q)); }
    return f;
  }, [plugins, dirSearch, catFilter]);

  // ── Render ────────────────────────────────────────────────────────────────
  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>MCP</h2>
      <p style={{ fontSize: 11, color: "var(--text-secondary)", margin: "0 0 10px" }}>
        Model Context Protocol — servers, tools, and plugins
      </p>

      {error && (
        <div style={{ fontSize: 12, color: "var(--error-color)", padding: "6px 8px", background: "rgba(220,50,50,0.15)", borderRadius: 4, marginBottom: 8 }}>
          {error} <button style={{ ...btnStyle, fontSize: 10, marginLeft: 8, padding: "2px 8px" }} onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      {/* Tab bar */}
      <div style={{ marginBottom: 12, display: "flex", flexWrap: "wrap", gap: 2 }} role="tablist">
        {(["servers", "tools", "directory", "installed", "metrics"] as Tab[]).map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabBtnStyle(tab === t)} onClick={() => setTab(t)}>
            {t === "servers" ? `Servers (${servers.length})` : t === "tools" ? `Tools (${manifests.length})` : t === "directory" ? `Directory (${plugins.length})` : t === "installed" ? `Installed (${plugins.filter(p => p.installed).length})` : "Metrics"}
          </button>
        ))}
      </div>

      {/* ── SERVERS TAB ──────────────────────────────────────────────────────── */}
      {tab === "servers" && (
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              Tools are injected as <code style={{ fontSize: 11 }}>mcp__&lt;server&gt;__&lt;tool&gt;</code>
            </span>
            <button onClick={() => { setEditing({ ...EMPTY_SERVER }); setEditIdx(null); }} style={{ ...btnStyle, background: "var(--accent-color)", fontSize: 11 }}>+ Add Server</button>
          </div>

          {servers.length === 0 && <div style={{ ...cardStyle, textAlign: "center", color: "var(--text-secondary)" }}>No MCP servers configured.</div>}

          {servers.map((srv, idx) => {
            const res = testResult[idx];
            const isTools = Array.isArray(res); const isErr = typeof res === "string";
            return (
              <div key={srv.name} style={cardStyle}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ fontSize: 13, fontWeight: 600, flex: 1 }}>
                    {srv.name}
                    {tokenStatus[srv.name] && <span style={{ marginLeft: 6, fontSize: 10, color: "var(--success-color)", background: "rgba(166,227,161,0.15)", padding: "1px 5px", borderRadius: 3 }}>OAuth</span>}
                  </span>
                  <button onClick={() => testServer(idx)} disabled={testing === idx} style={{ ...btnStyle, fontSize: 11, padding: "2px 8px" }}>{testing === idx ? "..." : "Test"}</button>
                  <button onClick={() => startOAuth(srv.name)} style={{ ...btnStyle, fontSize: 11, padding: "2px 8px" }}>OAuth</button>
                  <button onClick={() => { setEditing({ ...srv, args: [...srv.args] }); setEditIdx(idx); }} style={{ ...btnStyle, fontSize: 11, padding: "2px 8px" }}>Edit</button>
                  <button onClick={() => setConfirmDelete(idx)} style={{ ...btnStyle, fontSize: 11, padding: "2px 8px", borderColor: "var(--error-color)", color: "var(--error-color)" }}>✕</button>
                </div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", fontFamily: "var(--font-mono)", marginTop: 4 }}>$ {srv.command}{srv.args.length > 0 ? " " + srv.args.join(" ") : ""}</div>
                {isErr && <div style={{ fontSize: 11, color: "var(--error-color)", marginTop: 4 }}>{res}</div>}
                {isTools && res.length > 0 && (
                  <div style={{ marginTop: 4 }}>
                    <div style={{ fontSize: 10, color: "var(--text-secondary)", textTransform: "uppercase" }}>{res.length} tool{res.length !== 1 ? "s" : ""}</div>
                    {res.map(t => (
                      <div key={t.name} style={{ fontSize: 11, display: "flex", gap: 6 }}>
                        <code style={{ color: "var(--accent-color)", flexShrink: 0 }}>{t.name}</code>
                        <span style={{ color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{t.description}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* ── TOOLS TAB (Lazy Loading) ─────────────────────────────────────────── */}
      {tab === "tools" && (
        <div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>{loadedCount} / {manifests.length} tools loaded</span>
            <div style={{ ...barBg, minWidth: 120 }}><div style={barFill(manifests.length > 0 ? (loadedCount / manifests.length) * 100 : 0, "var(--info-color)")} /></div>
          </div>

          <div style={{ marginBottom: 10 }}>
            <input style={inputStyle} placeholder="Search tools by name or description..." value={toolSearch} onChange={e => setToolSearch(e.target.value)} />
          </div>

          {toolSearch.trim() ? (
            searchResults.length === 0 ? <div style={cardStyle}>No tools matching "{toolSearch}".</div> :
            searchResults.map(r => (
              <div key={r.tool_id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between" }}>
                <div><div style={{ fontWeight: 600 }}>{r.name}</div><div style={labelStyle}>{r.description}</div></div>
                <div style={{ textAlign: "right" }}><div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Relevance</div><div style={{ fontWeight: 600, color: "var(--accent-primary)" }}>{(r.relevance * 100).toFixed(0)}%</div></div>
              </div>
            ))
          ) : (
            manifests.map(m => (
              <div key={m.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontWeight: 600 }}>{m.name} <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{m.version}</span></div>
                  <div style={labelStyle}>{m.description}</div>
                  <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>
                    {m.size_kb} KB{m.server_name ? ` | ${m.server_name}` : ""}{m.load_time_ms != null ? ` | ${m.load_time_ms}ms` : ""}
                  </div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={badgeStyle(m.status)}>{m.status}</span>
                  <button style={{ ...btnStyle, opacity: actionLoading === m.id ? 0.6 : 1 }} disabled={actionLoading === m.id} onClick={() => toggleTool(m.id, m.status)}>
                    {actionLoading === m.id ? "..." : m.status === "loaded" ? "Unload" : "Load"}
                  </button>
                </div>
              </div>
            ))
          )}
          {manifests.length === 0 && !toolSearch.trim() && <div style={cardStyle}>No tools registered.</div>}
        </div>
      )}

      {/* ── DIRECTORY TAB ────────────────────────────────────────────────────── */}
      {tab === "directory" && (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
            <input style={{ ...inputStyle, flex: 1 }} value={dirSearch} onChange={e => setDirSearch(e.target.value)} placeholder="Search plugins..." />
            <select style={{ ...inputStyle, width: "auto" }} value={catFilter} onChange={e => setCatFilter(e.target.value)}>
              {CATEGORIES.map(c => <option key={c} value={c}>{c}</option>)}
            </select>
          </div>
          <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>{dirResults.length} plugins | {installedCount} installed</div>
          {dirLoading && <div style={cardStyle}>Loading...</div>}
          {dirResults.map(p => (
            <div key={p.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontWeight: 600 }}>{p.name} <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{p.version}</span></div>
                  <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>by {p.author} | {p.category}</div>
                  <div style={{ fontSize: 12, marginTop: 4 }}>{p.description}</div>
                  <div style={{ display: "flex", gap: 12, marginTop: 4, fontSize: 11 }}>
                    <span style={{ color: "var(--warning-color)" }}>{renderStars(p.rating)} {p.rating.toFixed(1)}</span>
                    <span style={{ color: "var(--text-secondary)" }}>{formatDl(p.downloads)} downloads</span>
                  </div>
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  {!p.installed && <button style={{ ...btnStyle, background: "var(--accent-primary)" }} onClick={() => installPlugin(p.id)} disabled={pluginAction === p.id}>{pluginAction === p.id ? "..." : "Install"}</button>}
                  {p.installed && <button style={{ ...btnStyle, borderColor: "var(--error-color)", color: "var(--error-color)" }} onClick={() => uninstallPlugin(p.id)} disabled={pluginAction === p.id}>{pluginAction === p.id ? "..." : "Uninstall"}</button>}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* ── INSTALLED TAB ────────────────────────────────────────────────────── */}
      {tab === "installed" && (
        <div>
          {(() => {
            const installed = plugins.filter(p => p.installed);
            if (installed.length === 0) {
              return (
                <div style={cardStyle}>
                  <div style={{ textAlign: "center", padding: "20px 0" }}>
                    <div style={{ fontSize: 14, marginBottom: 8 }}>No MCP plugins installed</div>
                    <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 12 }}>
                      Browse the Directory tab to find and install plugins.
                    </div>
                    <button style={{ ...btnStyle, background: "var(--accent-primary)" }} onClick={() => setTab("directory")}>
                      Browse Directory
                    </button>
                  </div>
                </div>
              );
            }
            return (
              <>
                {/* Summary bar */}
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
                  <div style={cardStyle}>
                    <div style={labelStyle}>Installed</div>
                    <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)" }}>{installed.length}</div>
                  </div>
                  <div style={cardStyle}>
                    <div style={labelStyle}>Updates Available</div>
                    <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: installed.some(p => p.updatable) ? "var(--warning-color)" : "var(--success-color)" }}>
                      {installed.filter(p => p.updatable).length}
                    </div>
                  </div>
                  <div style={cardStyle}>
                    <div style={labelStyle}>Categories</div>
                    <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)" }}>
                      {new Set(installed.map(p => p.category)).size}
                    </div>
                  </div>
                </div>

                {/* Update all button */}
                {installed.some(p => p.updatable) && (
                  <div style={{ marginBottom: 12 }}>
                    <button style={{ ...btnStyle, background: "var(--warning-color)", color: "var(--text-primary)" }}>
                      Update All ({installed.filter(p => p.updatable).length})
                    </button>
                  </div>
                )}

                {/* Installed plugin cards with details */}
                {installed.map(p => (
                  <div key={p.id} style={{ ...cardStyle, borderLeft: `3px solid ${p.updatable ? "var(--warning-color)" : "var(--success-color)"}` }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                      <div style={{ flex: 1 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                          <span style={{ fontWeight: 600, fontSize: 14 }}>{p.name}</span>
                          <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 8, background: "var(--bg-tertiary)", color: "var(--text-secondary)" }}>
                            v{p.version}
                          </span>
                          <span style={{
                            fontSize: 10, padding: "1px 6px", borderRadius: 8,
                            background: p.updatable ? "var(--warning-color)" : "var(--success-color)",
                            color: "#fff",
                          }}>
                            {p.updatable ? "Update available" : "Up to date"}
                          </span>
                        </div>
                        <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>
                          by {p.author} | {p.category}
                        </div>
                        <div style={{ fontSize: 12, marginTop: 4 }}>{p.description}</div>

                        {/* Plugin details */}
                        <div style={{ display: "flex", gap: 16, marginTop: 8, fontSize: 11 }}>
                          <span style={{ color: "var(--warning-color)" }}>{renderStars(p.rating)} {p.rating.toFixed(1)}</span>
                          <span style={{ color: "var(--text-secondary)" }}>{formatDl(p.downloads)} downloads</span>
                          <span style={{ color: "var(--text-secondary)" }}>ID: {p.id}</span>
                        </div>

                        {/* Config location hint */}
                        <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 6, fontFamily: "var(--font-mono)" }}>
                          Config: ~/.vibecli/mcp/{p.id}/config.json
                        </div>
                      </div>

                      {/* Action buttons */}
                      <div style={{ display: "flex", flexDirection: "column", gap: 4, minWidth: 80 }}>
                        {p.updatable && (
                          <button
                            style={{ ...btnStyle, background: "var(--warning-color)", color: "#fff", fontSize: 11 }}
                            onClick={() => installPlugin(p.id)}
                            disabled={pluginAction === p.id}
                          >
                            {pluginAction === p.id ? "..." : "Update"}
                          </button>
                        )}
                        <button
                          style={{ ...btnStyle, fontSize: 11 }}
                          onClick={() => {
                            // Switch to Tools tab filtered to this plugin's server
                            setTab("tools");
                          }}
                        >
                          View Tools
                        </button>
                        <button
                          style={{ ...btnStyle, borderColor: "var(--error-color)", color: "var(--error-color)", fontSize: 11 }}
                          onClick={() => uninstallPlugin(p.id)}
                          disabled={pluginAction === p.id}
                        >
                          {pluginAction === p.id ? "..." : "Uninstall"}
                        </button>
                      </div>
                    </div>
                  </div>
                ))}
              </>
            );
          })()}
        </div>
      )}

      {/* ── METRICS TAB ──────────────────────────────────────────────────────── */}
      {tab === "metrics" && metrics && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
            <div style={cardStyle}><div style={labelStyle}>Context Savings</div><div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--success-color)" }}>{metrics.context_savings_pct}%</div></div>
            <div style={cardStyle}><div style={labelStyle}>Cache Hits</div><div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)" }}>{metrics.cache_hits.toLocaleString()}</div></div>
            <div style={cardStyle}><div style={labelStyle}>Cache Misses</div><div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--error-color)" }}>{metrics.cache_misses}</div></div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>Cache Hit Rate</div>
            <div style={barBg}><div style={barFill(metrics.cache_hit_rate, "var(--success-color)")} /></div>
            <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>{metrics.cache_hit_rate.toFixed(1)}%</div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>Avg Load Time: {metrics.avg_load_time_ms}ms</div>
            {metrics.load_times.map(lt => (
              <div key={lt.label} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                <div style={{ width: 90, fontSize: 11 }}>{lt.label}</div>
                <div style={{ ...barBg, flex: 1 }}><div style={barFill((lt.ms / 60) * 100, "var(--info-color)")} /></div>
                <div style={{ width: 40, fontSize: 10, textAlign: "right" }}>{lt.ms}ms</div>
              </div>
            ))}
          </div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between" }}>
            <span>Tools loaded: {loadedCount} / {manifests.length}</span>
            <span>Total load time: {metrics.total_load_time_ms}ms</span>
          </div>
        </div>
      )}
      {tab === "metrics" && !metrics && <div style={cardStyle}>Loading metrics...</div>}

      {/* ── Modals ───────────────────────────────────────────────────────────── */}
      {editing && (
        <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
          <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 8, padding: 20, width: 360, display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{ fontSize: 13, fontWeight: 600 }}>{editIdx === null ? "Add MCP Server" : "Edit MCP Server"}</div>
            <label style={{ fontSize: 12, display: "flex", flexDirection: "column", gap: 4 }}>Name<input autoFocus type="text" value={editing.name} onChange={e => setEditing({ ...editing, name: e.target.value })} placeholder="e.g. github" style={inputStyle} /></label>
            <label style={{ fontSize: 12, display: "flex", flexDirection: "column", gap: 4 }}>Command<input type="text" value={editing.command} onChange={e => setEditing({ ...editing, command: e.target.value })} placeholder="npx @modelcontextprotocol/server-github" style={inputStyle} /></label>
            <label style={{ fontSize: 12, display: "flex", flexDirection: "column", gap: 4 }}>Args (space-separated)<input type="text" value={editing.args.join(" ")} onChange={e => setEditing({ ...editing, args: e.target.value ? e.target.value.split(" ") : [] })} placeholder="optional" style={inputStyle} /></label>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button onClick={() => { setEditing(null); setEditIdx(null); }} style={btnStyle}>Cancel</button>
              <button onClick={commitEdit} disabled={!editing.name.trim() || !editing.command.trim() || saving} style={{ ...btnStyle, background: "var(--accent-color)" }}>{saving ? "Saving..." : editIdx === null ? "Add" : "Save"}</button>
            </div>
          </div>
        </div>
      )}

      {oauthForm && (
        <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 110 }}>
          <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 8, padding: 20, width: 380, display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{ fontSize: 13, fontWeight: 600 }}>OAuth — {oauthForm.serverName}</div>
            {oauthForm.step === "config" ? (<>
              <label style={{ fontSize: 12, display: "flex", flexDirection: "column", gap: 3 }}>Client ID<input type="text" value={oauthForm.clientId} onChange={e => setOauthForm(f => f && { ...f, clientId: e.target.value })} style={inputStyle} /></label>
              <label style={{ fontSize: 12, display: "flex", flexDirection: "column", gap: 3 }}>Auth URL<input type="text" value={oauthForm.authUrl} onChange={e => setOauthForm(f => f && { ...f, authUrl: e.target.value })} style={inputStyle} /></label>
              <label style={{ fontSize: 12, display: "flex", flexDirection: "column", gap: 3 }}>Token URL<input type="text" value={oauthForm.tokenUrl} onChange={e => setOauthForm(f => f && { ...f, tokenUrl: e.target.value })} style={inputStyle} /></label>
              <label style={{ fontSize: 12, display: "flex", flexDirection: "column", gap: 3 }}>Scopes<input type="text" value={oauthForm.scopes} onChange={e => setOauthForm(f => f && { ...f, scopes: e.target.value })} style={inputStyle} /></label>
            </>) : (
              <label style={{ fontSize: 12, display: "flex", flexDirection: "column", gap: 3 }}>Authorization Code<input autoFocus type="text" value={oauthForm.authCode} onChange={e => setOauthForm(f => f && { ...f, authCode: e.target.value })} style={inputStyle} /></label>
            )}
            {oauthForm.msg && <div style={{ fontSize: 11, padding: "6px 8px", borderRadius: 4, background: oauthForm.msg.startsWith("Error") ? "rgba(220,50,50,0.15)" : "rgba(166,227,161,0.15)", color: oauthForm.msg.startsWith("Error") ? "var(--error-color)" : "var(--success-color)" }}>{oauthForm.msg}</div>}
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button onClick={() => setOauthForm(null)} style={btnStyle}>Cancel</button>
              {oauthForm.step === "config" ? (
                <button onClick={initiateOAuth} disabled={oauthForm.busy || !oauthForm.clientId || !oauthForm.authUrl} style={{ ...btnStyle, background: "var(--accent-color)" }}>{oauthForm.busy ? "Opening..." : "Open Browser"}</button>
              ) : (
                <button onClick={completeOAuth} disabled={oauthForm.busy || !oauthForm.authCode} style={{ ...btnStyle, background: "var(--success-color)", fontWeight: 600 }}>{oauthForm.busy ? "Exchanging..." : "Connect"}</button>
              )}
            </div>
          </div>
        </div>
      )}

      {confirmDelete !== null && (
        <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
          <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 8, padding: 20, maxWidth: 300, display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={{ fontSize: 13, fontWeight: 600 }}>Remove Server?</div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Remove <strong>{servers[confirmDelete]?.name}</strong>?</div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button onClick={() => setConfirmDelete(null)} style={btnStyle}>Cancel</button>
              <button onClick={async () => { await saveServers(servers.filter((_, i) => i !== confirmDelete)); setConfirmDelete(null); }} style={{ ...btnStyle, background: "var(--error-color)" }}>Remove</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
