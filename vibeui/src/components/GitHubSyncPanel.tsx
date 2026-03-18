import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GitHubSyncStatus {
 repo_url: string | null;
 branch: string;
 ahead: number;
 behind: number;
 has_remote: boolean;
 last_synced: string | null;
}

interface RepoInfo {
 name: string;
 full_name: string;
 private: boolean;
 default_branch: string;
 url: string;
}

export function GitHubSyncPanel({ workspacePath }: { workspacePath: string | null }) {
 const [status, setStatus] = useState<GitHubSyncStatus | null>(null);
 const [repos, setRepos] = useState<RepoInfo[]>([]);
 const [commitMsg, setCommitMsg] = useState("");
 const [newRepoName, setNewRepoName] = useState("");
 const [isPrivate, setIsPrivate] = useState(false);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [success, setSuccess] = useState<string | null>(null);
 const [activeTab, setActiveTab] = useState<"sync" | "repos" | "create">("sync");
 const [token, setToken] = useState("");
 const [tokenSaved, setTokenSaved] = useState(false);

 useEffect(() => {
 if (!workspacePath) return;
 loadStatus();
 checkToken();
 }, [workspacePath]);

 if (!workspacePath) {
 return <div className="empty-state"><p>Open a workspace folder to use GitHub sync.</p></div>;
 }

 const checkToken = async () => {
 try {
 const saved = await invoke<boolean>("has_github_token", { workspacePath });
 setTokenSaved(saved);
 } catch { /* ignore */ }
 };

 const loadStatus = async () => {
 try {
 const s = await invoke<GitHubSyncStatus>("get_github_sync_status", { workspacePath });
 setStatus(s);
 } catch { /* not a git repo or no remote */ }
 };

 const push = async () => {
 if (!commitMsg.trim()) { setError("Commit message required"); return; }
 setLoading(true);
 setError(null);
 try {
 await invoke("github_sync_push", { workspacePath, commitMessage: commitMsg });
 setSuccess(`Pushed: "${commitMsg}"`);
 setCommitMsg("");
 await loadStatus();
 } catch (e) { setError(String(e)); }
 finally { setLoading(false); }
 };

 const pull = async () => {
 setLoading(true);
 setError(null);
 try {
 await invoke("github_sync_pull", { workspacePath });
 setSuccess("Pulled latest changes from remote");
 await loadStatus();
 } catch (e) { setError(String(e)); }
 finally { setLoading(false); }
 };

 const createRepo = async () => {
 if (!newRepoName.trim()) { setError("Repository name required"); return; }
 setLoading(true);
 setError(null);
 try {
 const url = await invoke<string>("github_create_repo", { workspacePath, name: newRepoName, private: isPrivate });
 setSuccess(`Repository created: ${url}`);
 setNewRepoName("");
 setActiveTab("sync");
 await loadStatus();
 } catch (e) { setError(String(e)); }
 finally { setLoading(false); }
 };

 const listRepos = async () => {
 setLoading(true);
 try {
 const r = await invoke<RepoInfo[]>("list_github_repos", { workspacePath });
 setRepos(r);
 } catch (e) { setError(String(e)); }
 finally { setLoading(false); }
 };

 const saveToken = async () => {
 if (!token.trim()) return;
 setLoading(true);
 try {
 await invoke("save_github_token", { workspacePath, token });
 setTokenSaved(true);
 setToken("");
 setSuccess("GitHub token saved");
 await loadStatus();
 } catch (e) { setError(String(e)); }
 finally { setLoading(false); }
 };

 const s = {
 panel: { display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: "13px" } as React.CSSProperties,
 header: { padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" } as React.CSSProperties,
 tabs: { display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" } as React.CSSProperties,
 tab: (active: boolean): React.CSSProperties => ({ padding: "6px 14px", border: "none", cursor: "pointer", fontSize: "12px", background: "none", borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent", color: active ? "var(--text-primary)" : "var(--text-secondary)" }),
 content: { flex: 1, overflow: "auto", padding: "12px", display: "flex", flexDirection: "column", gap: "10px" } as React.CSSProperties,
 input: { width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)", padding: "6px 8px", borderRadius: "4px", fontSize: "12px", boxSizing: "border-box" as const } as React.CSSProperties,
 btn: (variant?: "danger" | "secondary"): React.CSSProperties => ({ padding: "6px 14px", background: variant === "danger" ? "var(--error-color)" : variant === "secondary" ? "var(--bg-secondary)" : "var(--accent-color)", color: variant === "secondary" ? "var(--text-primary)" : "white", border: variant === "secondary" ? "1px solid var(--border-color)" : "none", borderRadius: "4px", cursor: "pointer", fontSize: "12px" }),
 statusBadge: (n: number, type: "ahead" | "behind"): React.CSSProperties => ({ padding: "2px 8px", borderRadius: "10px", fontSize: "11px", background: n > 0 ? (type === "ahead" ? "var(--success-bg)" : "var(--error-bg)") : "var(--bg-secondary)", color: n > 0 ? (type === "ahead" ? "var(--success-color)" : "var(--error-color)") : "var(--text-secondary)" }),
 };

 return (
 <div style={s.panel}>
 <div style={s.header}>
 <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
 <span style={{ fontSize: "14px", fontWeight: 600 }}>GitHub Sync</span>
 {status?.has_remote && (
 <span style={{ fontSize: "11px", color: "var(--text-secondary)" }}>{status.repo_url?.replace("https://github.com/", "")}</span>
 )}
 </div>
 {status?.has_remote && (
 <div style={{ display: "flex", gap: "8px", marginTop: "6px" }}>
 <span style={s.statusBadge(status.ahead, "ahead")}>↑ {status.ahead} ahead</span>
 <span style={s.statusBadge(status.behind, "behind")}>↓ {status.behind} behind</span>
 <span style={{ fontSize: "11px", color: "var(--text-secondary)" }}>branch: {status.branch}</span>
 </div>
 )}
 </div>

 {!tokenSaved && (
 <div style={{ padding: "10px 12px", background: "var(--warning-bg)", borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ fontSize: "12px", marginBottom: "6px", color: "var(--warning-color)" }}>GITHUB_TOKEN required for sync</div>
 <div style={{ display: "flex", gap: "6px" }}>
 <input style={{ ...s.input, flex: 1 }} type="password" placeholder="ghp_..." value={token} onChange={e => setToken(e.target.value)} />
 <button style={s.btn()} onClick={saveToken}>Save</button>
 </div>
 </div>
 )}

 <div style={s.tabs}>
 {(["sync", "repos", "create"] as const).map(t => (
 <button key={t} style={s.tab(activeTab === t)} onClick={() => { setActiveTab(t); if (t === "repos") listRepos(); }}>
 {t === "sync" ? "Sync" : t === "repos" ? "Repos" : "New Repo"}
 </button>
 ))}
 </div>

 {(error || success) && (
 <div style={{ padding: "8px 12px", background: error ? "var(--error-bg)" : "var(--success-bg)", color: error ? "var(--error-color)" : "var(--success-color)", fontSize: "12px" }}>
 {error || success}
 <button aria-label="Dismiss" style={{ float: "right", background: "none", border: "none", cursor: "pointer", color: "inherit" }} onClick={() => { setError(null); setSuccess(null); }}>×</button>
 </div>
 )}

 <div style={s.content}>
 {activeTab === "sync" && (
 <>
 {!status?.has_remote && (
 <div style={{ color: "var(--text-secondary)", fontSize: "12px", textAlign: "center", marginTop: "20px" }}>
 No remote configured. Create a repo or link an existing one.
 <button style={{ ...s.btn(), display: "block", margin: "10px auto 0" }} onClick={() => setActiveTab("create")}>Create Repository</button>
 </div>
 )}
 {status?.has_remote && (
 <>
 <div>
 <label style={{ fontSize: "11px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Commit message</label>
 <textarea
 style={{ ...s.input, height: "60px", resize: "vertical", fontFamily: "inherit" }}
 placeholder="feat: add new feature"
 value={commitMsg}
 onChange={e => setCommitMsg(e.target.value)}
 />
 </div>
 <div style={{ display: "flex", gap: "8px" }}>
 <button style={{ ...s.btn(), flex: 1 }} onClick={push} disabled={loading || !commitMsg.trim()}>↑ Commit & Push</button>
 <button style={s.btn("secondary")} onClick={pull} disabled={loading}>↓ Pull</button>
 <button style={s.btn("secondary")} onClick={loadStatus} disabled={loading}>⟳</button>
 </div>
 {status.last_synced && <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>Last synced: {status.last_synced}</div>}
 </>
 )}
 </>
 )}

 {activeTab === "repos" && (
 <div>
 {repos.length === 0 && !loading && <div style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: "20px" }}>Click "Repos" tab to load your repositories</div>}
 {repos.map(r => (
 <div key={r.full_name} style={{ padding: "8px 10px", borderRadius: "4px", marginBottom: "4px", background: "var(--bg-secondary)" }}>
 <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span style={{ fontWeight: 600 }}> {r.name}</span>
 <span style={{ fontSize: "10px", padding: "2px 6px", borderRadius: "10px", background: r.private ? "var(--warning-bg)" : "var(--success-bg)", color: r.private ? "var(--warning-color)" : "var(--success-color)" }}>{r.private ? "Private" : "Public"}</span>
 </div>
 <div style={{ fontSize: "11px", color: "var(--text-secondary)", marginTop: "2px" }}>branch: {r.default_branch} · {r.url}</div>
 </div>
 ))}
 </div>
 )}

 {activeTab === "create" && (
 <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
 <div>
 <label style={{ fontSize: "11px", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Repository name</label>
 <input style={s.input} placeholder="my-project" value={newRepoName} onChange={e => setNewRepoName(e.target.value)} />
 </div>
 <label style={{ display: "flex", alignItems: "center", gap: "8px", cursor: "pointer", fontSize: "12px" }}>
 <input type="checkbox" checked={isPrivate} onChange={e => setIsPrivate(e.target.checked)} />
 Private repository
 </label>
 <button style={s.btn()} onClick={createRepo} disabled={loading || !newRepoName.trim()}>
 {loading ? "Creating..." : "Create & Push to GitHub"}
 </button>
 <p style={{ fontSize: "11px", color: "var(--text-secondary)", margin: 0 }}>Creates a new GitHub repository and pushes the current workspace to it.</p>
 </div>
 )}
 </div>
 </div>
 );
}
