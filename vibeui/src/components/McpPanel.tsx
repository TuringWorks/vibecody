import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface McpServer {
 name: string;
 command: string;
 args: string[];
 env: Record<string, string>;
}

interface McpToolInfo {
 name: string;
 description: string;
}

interface OAuthForm {
 serverName: string;
 clientId: string;
 authUrl: string;
 tokenUrl: string;
 redirectUri: string;
 scopes: string;
 authCode: string;
 step: "config" | "code";
 busy: boolean;
 msg: string | null;
}

const EMPTY_SERVER: McpServer = { name: "", command: "", args: [], env: {} };

export function McpPanel() {
 const [servers, setServers] = useState<McpServer[]>([]);
 const [editing, setEditing] = useState<McpServer | null>(null);
 const [editIdx, setEditIdx] = useState<number | null>(null);
 const [saving, setSaving] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [testing, setTesting] = useState<number | null>(null);
 const [testResult, setTestResult] = useState<Record<number, McpToolInfo[] | string>>({});
 const [confirmDelete, setConfirmDelete] = useState<number | null>(null);
 const [oauthForm, setOauthForm] = useState<OAuthForm | null>(null);
 const [tokenStatus, setTokenStatus] = useState<Record<string, boolean>>({});

 useEffect(() => {
 loadServers();
 }, []);

 // Load token status for all servers whenever the list changes
 useEffect(() => {
 let cancelled = false;
 servers.forEach((srv) => {
 invoke<{ connected: boolean; expired: boolean }>("get_mcp_token_status", { serverName: srv.name })
 .then((s) => { if (!cancelled) setTokenStatus((prev) => ({ ...prev, [srv.name]: s.connected && !s.expired })); })
 .catch(() => { if (!cancelled) setTokenStatus((prev) => ({ ...prev, [srv.name]: false })); });
 });
 return () => { cancelled = true; };
 }, [servers]);

 function startOAuth(serverName: string) {
 setOauthForm({
 serverName,
 clientId: "",
 authUrl: "",
 tokenUrl: "",
 redirectUri: "http://localhost:7879/oauth/callback",
 scopes: "read",
 authCode: "",
 step: "config",
 busy: false,
 msg: null,
 });
 }

 async function initiateOAuth() {
 if (!oauthForm) return;
 setOauthForm((f) => f && { ...f, busy: true, msg: null });
 try {
 await invoke("initiate_mcp_oauth", {
 serverName: oauthForm.serverName,
 clientId: oauthForm.clientId,
 authUrl: oauthForm.authUrl,
 redirectUri: oauthForm.redirectUri,
 scopes: oauthForm.scopes,
 });
 setOauthForm((f) => f && { ...f, busy: false, step: "code", msg: "Browser opened. Paste the authorization code below." });
 } catch (e) {
 setOauthForm((f) => f && { ...f, busy: false, msg: `Error: ${e}` });
 }
 }

 async function completeOAuth() {
 if (!oauthForm) return;
 setOauthForm((f) => f && { ...f, busy: true, msg: null });
 try {
 await invoke("complete_mcp_oauth", {
 serverName: oauthForm.serverName,
 code: oauthForm.authCode,
 tokenUrl: oauthForm.tokenUrl,
 clientId: oauthForm.clientId,
 redirectUri: oauthForm.redirectUri,
 });
 setTokenStatus((prev) => ({ ...prev, [oauthForm.serverName]: true }));
 setOauthForm(null);
 } catch (e) {
 setOauthForm((f) => f && { ...f, busy: false, msg: `Token exchange failed: ${e}` });
 }
 }

 async function loadServers() {
 setError(null);
 try {
 const list = await invoke<McpServer[]>("get_mcp_servers");
 setServers(list);
 } catch (e) {
 setError(String(e));
 }
 }

 async function save(list: McpServer[]) {
 setSaving(true);
 setError(null);
 try {
 await invoke("save_mcp_servers", { servers: list });
 setServers(list);
 } catch (e) {
 setError(String(e));
 } finally {
 setSaving(false);
 }
 }

 function startAdd() {
 setEditing({ ...EMPTY_SERVER });
 setEditIdx(null);
 }

 function startEdit(idx: number) {
 setEditing({ ...servers[idx], args: [...servers[idx].args] });
 setEditIdx(idx);
 }

 async function commitEdit() {
 if (!editing || !editing.name.trim() || !editing.command.trim()) return;
 const updated = [...servers];
 if (editIdx === null) {
 updated.push({ ...editing });
 } else {
 updated[editIdx] = { ...editing };
 }
 await save(updated);
 setEditing(null);
 setEditIdx(null);
 }

 async function deleteServer(idx: number) {
 const updated = servers.filter((_, i) => i !== idx);
 await save(updated);
 setConfirmDelete(null);
 // Clear any test result for deleted server
 setTestResult((prev) => {
 const next = { ...prev };
 delete next[idx];
 return next;
 });
 }

 async function testServer(idx: number) {
 setTesting(idx);
 setTestResult((prev) => ({ ...prev, [idx]: [] }));
 try {
 const tools = await invoke<McpToolInfo[]>("test_mcp_server", { server: servers[idx] });
 setTestResult((prev) => ({ ...prev, [idx]: tools }));
 } catch (e) {
 setTestResult((prev) => ({ ...prev, [idx]: String(e) }));
 } finally {
 setTesting(null);
 }
 }

 const result = (idx: number) => testResult[idx];

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "10px", fontFamily: "var(--font-mono, monospace)", position: "relative" }}>
 {/* Header */}
 <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
 <span style={{ fontWeight: 600, fontSize: "14px" }}>MCP Servers</span>
 <button
 onClick={startAdd}
 style={{ marginLeft: "auto", padding: "4px 10px", fontSize: "12px", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "4px", cursor: "pointer" }}
 >
 + Add Server
 </button>
 </div>

 <p style={{ fontSize: "12px", color: "var(--text-secondary)", margin: 0 }}>
 Configure external MCP servers. Their tools are injected into the agent context as{" "}
 <code style={{ fontSize: "11px" }}>mcp__&lt;server&gt;__&lt;tool&gt;</code>.
 </p>

 {error && (
 <div style={{ fontSize: "12px", color: "var(--error-color)", padding: "6px 8px", background: "rgba(220,50,50,0.15)", borderRadius: "4px" }}>
 {error}
 </div>
 )}

 {/* Server list */}
 <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: "8px" }}>
 {servers.length === 0 && (
 <div style={{ fontSize: "12px", color: "var(--text-secondary)", textAlign: "center", padding: "32px 16px" }}>
 No MCP servers configured.<br />
 <span style={{ opacity: 0.7 }}>Click "+ Add Server" to add one.</span>
 </div>
 )}

 {servers.map((srv, idx) => {
 const res = result(idx);
 const isTools = Array.isArray(res);
 const isErr = typeof res === "string";
 return (
 <div
 key={srv.name}
 style={{
 border: "1px solid var(--border-color)",
 borderRadius: "6px",
 padding: "10px 12px",
 background: "var(--bg-secondary)",
 display: "flex",
 flexDirection: "column",
 gap: "6px",
 }}
 >
 {/* Server header row */}
 <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
 <span style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-primary)", flex: 1 }}>
 {srv.name}
 {tokenStatus[srv.name] && (
 <span style={{ marginLeft: 6, fontSize: "10px", color: "var(--text-success, #a6e3a1)", background: "rgba(166,227,161,0.15)", padding: "1px 5px", borderRadius: 3 }}>
 OAuth
 </span>
 )}
 </span>
 <button
 onClick={() => testServer(idx)}
 disabled={testing === idx}
 style={{ padding: "2px 8px", fontSize: "11px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "3px", color: "var(--text-primary)", cursor: "pointer" }}
 >
 {testing === idx ? "Testing…" : "Test"}
 </button>
 <button
 onClick={() => startOAuth(srv.name)}
 style={{ padding: "2px 8px", fontSize: "11px", background: tokenStatus[srv.name] ? "rgba(166,227,161,0.15)" : "var(--bg-tertiary)", border: `1px solid ${tokenStatus[srv.name] ? "var(--success-color)" : "var(--border-color)"}`, borderRadius: "3px", color: tokenStatus[srv.name] ? "var(--success-color)" : "var(--text-primary)", cursor: "pointer" }}
 title="Connect via OAuth"
 >
 OAuth
 </button>
 <button
 onClick={() => startEdit(idx)}
 style={{ padding: "2px 8px", fontSize: "11px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "3px", color: "var(--text-primary)", cursor: "pointer" }}
 >
 Edit
 </button>
 <button
 onClick={() => setConfirmDelete(idx)}
 style={{ padding: "2px 8px", fontSize: "11px", background: "transparent", border: "1px solid var(--error-color)", borderRadius: "3px", color: "var(--error-color)", cursor: "pointer" }}
 >
 ✕
 </button>
 </div>

 {/* Command */}
 <div style={{ fontSize: "11px", color: "var(--text-secondary)", fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
 $ {srv.command}{srv.args.length > 0 ? " " + srv.args.join(" ") : ""}
 </div>

 {/* Tool test results */}
 {isErr && (
 <div style={{ fontSize: "11px", color: "var(--error-color)", padding: "4px 6px", background: "rgba(220,50,50,0.1)", borderRadius: "3px" }}>
 {res}
 </div>
 )}
 {isTools && res.length === 0 && (
 <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>No tools exposed.</div>
 )}
 {isTools && res.length > 0 && (
 <div style={{ display: "flex", flexDirection: "column", gap: "2px", marginTop: "2px" }}>
 <div style={{ fontSize: "10px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
 {res.length} tool{res.length !== 1 ? "s" : ""}
 </div>
 {res.map((t) => (
 <div key={t.name} style={{ fontSize: "11px", display: "flex", gap: "6px" }}>
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

 {/* Edit / Add form */}
 {editing && (
 <div style={{
 position: "absolute",
 inset: 0,
 background: "rgba(0,0,0,0.5)",
 display: "flex",
 alignItems: "center",
 justifyContent: "center",
 zIndex: 100,
 }}>
 <div style={{
 background: "var(--bg-secondary)",
 border: "1px solid var(--border-color)",
 borderRadius: "8px",
 padding: "20px",
 width: "360px",
 display: "flex",
 flexDirection: "column",
 gap: "10px",
 }}>
 <div style={{ fontSize: "13px", fontWeight: 600 }}>
 {editIdx === null ? "Add MCP Server" : "Edit MCP Server"}
 </div>

 <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "4px" }}>
 Name
 <input
 autoFocus
 type="text"
 value={editing.name}
 onChange={(e) => setEditing({ ...editing, name: e.target.value })}
 placeholder="e.g. github"
 style={inputStyle}
 />
 </label>

 <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "4px" }}>
 Command
 <input
 type="text"
 value={editing.command}
 onChange={(e) => setEditing({ ...editing, command: e.target.value })}
 placeholder="e.g. npx @modelcontextprotocol/server-github"
 style={inputStyle}
 />
 </label>

 <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "4px" }}>
 Extra args (space-separated)
 <input
 type="text"
 value={editing.args.join(" ")}
 onChange={(e) => setEditing({ ...editing, args: e.target.value ? e.target.value.split(" ") : [] })}
 placeholder="optional"
 style={inputStyle}
 />
 </label>

 <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end", marginTop: "4px" }}>
 <button
 onClick={() => { setEditing(null); setEditIdx(null); }}
 style={{ padding: "6px 14px", fontSize: "12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}
 >
 Cancel
 </button>
 <button
 onClick={commitEdit}
 disabled={!editing.name.trim() || !editing.command.trim() || saving}
 style={{ padding: "6px 14px", fontSize: "12px", background: "var(--accent-color)", border: "none", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}
 >
 {saving ? "Saving…" : editIdx === null ? "Add" : "Save"}
 </button>
 </div>
 </div>
 </div>
 )}

 {/* OAuth Setup Modal */}
 {oauthForm && (
 <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.6)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 110 }}>
 <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "8px", padding: "20px", width: "380px", display: "flex", flexDirection: "column", gap: "10px" }}>
 <div style={{ fontSize: "13px", fontWeight: 600 }}>
 OAuth Setup — {oauthForm.serverName}
 </div>
 {oauthForm.step === "config" ? (
 <>
 <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "3px" }}>
 Client ID
 <input type="text" value={oauthForm.clientId} onChange={(e) => setOauthForm((f) => f && { ...f, clientId: e.target.value })} placeholder="your-client-id" style={inputStyle} />
 </label>
 <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "3px" }}>
 Authorization URL
 <input type="text" value={oauthForm.authUrl} onChange={(e) => setOauthForm((f) => f && { ...f, authUrl: e.target.value })} placeholder="https://provider.example.com/oauth/authorize" style={inputStyle} />
 </label>
 <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "3px" }}>
 Token URL
 <input type="text" value={oauthForm.tokenUrl} onChange={(e) => setOauthForm((f) => f && { ...f, tokenUrl: e.target.value })} placeholder="https://provider.example.com/oauth/token" style={inputStyle} />
 </label>
 <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "3px" }}>
 Scopes (space-separated)
 <input type="text" value={oauthForm.scopes} onChange={(e) => setOauthForm((f) => f && { ...f, scopes: e.target.value })} placeholder="read write" style={inputStyle} />
 </label>
 </>
 ) : (
 <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "3px" }}>
 Authorization Code
 <input autoFocus type="text" value={oauthForm.authCode} onChange={(e) => setOauthForm((f) => f && { ...f, authCode: e.target.value })} placeholder="Paste the code from your browser" style={inputStyle} />
 </label>
 )}
 {oauthForm.msg && (
 <div style={{ fontSize: "11px", padding: "6px 8px", borderRadius: "4px", background: oauthForm.msg.startsWith("Error") ? "rgba(220,50,50,0.15)" : "rgba(166,227,161,0.15)", color: oauthForm.msg.startsWith("Error") ? "var(--error-color)" : "var(--success-color)" }}>
 {oauthForm.msg}
 </div>
 )}
 <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end", marginTop: "4px" }}>
 <button onClick={() => setOauthForm(null)} style={{ padding: "6px 14px", fontSize: "12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}>
 Cancel
 </button>
 {oauthForm.step === "config" ? (
 <button onClick={initiateOAuth} disabled={oauthForm.busy || !oauthForm.clientId || !oauthForm.authUrl} style={{ padding: "6px 14px", fontSize: "12px", background: "var(--accent-color)", border: "none", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}>
 {oauthForm.busy ? "Opening…" : "Open Browser"}
 </button>
 ) : (
 <button onClick={completeOAuth} disabled={oauthForm.busy || !oauthForm.authCode} style={{ padding: "6px 14px", fontSize: "12px", background: "var(--success-color)", border: "none", borderRadius: "4px", color: "var(--bg-tertiary)", cursor: "pointer", fontWeight: 600 }}>
 {oauthForm.busy ? "Exchanging…" : "Connect"}
 </button>
 )}
 </div>
 </div>
 </div>
 )}

 {/* Confirm delete modal */}
 {confirmDelete !== null && (
 <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
 <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "8px", padding: "20px", maxWidth: "300px", width: "90%", display: "flex", flexDirection: "column", gap: "12px" }}>
 <div style={{ fontSize: "13px", fontWeight: 600 }}>Remove Server?</div>
 <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
 Remove <strong>{servers[confirmDelete]?.name}</strong> from the list?
 </div>
 <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
 <button onClick={() => setConfirmDelete(null)}
 style={{ padding: "6px 14px", fontSize: "12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}>
 Cancel
 </button>
 <button onClick={() => deleteServer(confirmDelete)}
 style={{ padding: "6px 14px", fontSize: "12px", background: "var(--error-color)", border: "none", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}>
 Remove
 </button>
 </div>
 </div>
 </div>
 )}
 </div>
 );
}

const inputStyle: React.CSSProperties = {
 padding: "5px 8px",
 fontSize: "12px",
 background: "var(--bg-input, var(--bg-primary))",
 border: "1px solid var(--border-color)",
 borderRadius: "4px",
 color: "var(--text-primary)",
 outline: "none",
 fontFamily: "monospace",
};
