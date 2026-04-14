import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";

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

 const statusBadgeBg = (n: number, type: "ahead" | "behind") =>
   n > 0 ? (type === "ahead" ? "var(--success-bg)" : "var(--error-bg)") : "var(--bg-secondary)";
 const statusBadgeFg = (n: number, type: "ahead" | "behind") =>
   n > 0 ? (type === "ahead" ? "var(--success-color)" : "var(--error-color)") : "var(--text-secondary)";

 return (
 <div className="panel-container">
 <div className="panel-header">
 <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
 <span style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>GitHub Sync</span>
 {status?.has_remote && (
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{status.repo_url?.replace("https://github.com/", "")}</span>
 )}
 </div>
 {status?.has_remote && (
 <div style={{ display: "flex", gap: "8px", marginTop: "6px" }}>
 <span style={{ padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-sm)", background: statusBadgeBg(status.ahead, "ahead"), color: statusBadgeFg(status.ahead, "ahead") }}>↑ {status.ahead} ahead</span>
 <span style={{ padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-sm)", background: statusBadgeBg(status.behind, "behind"), color: statusBadgeFg(status.behind, "behind") }}>↓ {status.behind} behind</span>
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>branch: {status.branch}</span>
 </div>
 )}
 </div>

 {!tokenSaved && (
 <div style={{ padding: "10px 12px", background: "var(--warning-bg)", borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ fontSize: "var(--font-size-base)", marginBottom: "6px", color: "var(--warning-color)" }}>GITHUB_TOKEN required for sync</div>
 <div style={{ display: "flex", gap: "6px" }}>
 <input className="panel-input" style={{ flex: 1 }} type="password" placeholder="ghp_..." value={token} onChange={e => setToken(e.target.value)} />
 <button className="panel-btn panel-btn-primary" onClick={saveToken}>Save</button>
 </div>
 </div>
 )}

 <div className="panel-tab-bar">
 {(["sync", "repos", "create"] as const).map(t => (
 <button key={t} className={`panel-tab ${activeTab === t ? "active" : ""}`} onClick={() => { setActiveTab(t); if (t === "repos") listRepos(); }}>
 {t === "sync" ? "Sync" : t === "repos" ? "Repos" : "New Repo"}
 </button>
 ))}
 </div>

 {(error || success) && (
 <div className={error ? "panel-error" : "panel-section"} style={{ color: error ? "var(--error-color)" : "var(--success-color)", background: error ? "var(--error-bg)" : "var(--success-bg)" }}>
 {error || success}
 <button aria-label="Dismiss" style={{ float: "right", background: "none", border: "none", cursor: "pointer", color: "inherit", display: "inline-flex", alignItems: "center" }} onClick={() => { setError(null); setSuccess(null); }}><X size={14} /></button>
 </div>
 )}

 <div className="panel-body">
 {activeTab === "sync" && (
 <>
 {!status?.has_remote && (
 <div className="panel-empty">
 No remote configured. Create a repo or link an existing one.
 <button className="panel-btn panel-btn-primary" style={{ display: "block", margin: "10px auto 0" }} onClick={() => setActiveTab("create")}>Create Repository</button>
 </div>
 )}
 {status?.has_remote && (
 <>
 <div>
 <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Commit message</label>
 <textarea
 className="panel-textarea panel-input-full"
 style={{ height: "60px", resize: "vertical", fontFamily: "inherit" }}
 placeholder="feat: add new feature"
 value={commitMsg}
 onChange={e => setCommitMsg(e.target.value)}
 />
 </div>
 <div style={{ display: "flex", gap: "8px" }}>
 <button className="panel-btn panel-btn-primary" style={{ flex: 1 }} onClick={push} disabled={loading || !commitMsg.trim()}>↑ Commit & Push</button>
 <button className="panel-btn panel-btn-secondary" onClick={pull} disabled={loading}>↓ Pull</button>
 <button className="panel-btn panel-btn-secondary" onClick={loadStatus} disabled={loading}>⟳</button>
 </div>
 {status.last_synced && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Last synced: {status.last_synced}</div>}
 </>
 )}
 </>
 )}

 {activeTab === "repos" && (
 <div>
 {repos.length === 0 && !loading && <div className="panel-empty">Click "Repos" tab to load your repositories</div>}
 {repos.map(r => (
 <div key={r.full_name} style={{ padding: "8px 10px", borderRadius: "var(--radius-xs-plus)", marginBottom: "4px", background: "var(--bg-secondary)" }}>
 <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span style={{ fontWeight: 600 }}> {r.name}</span>
 <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 6px", borderRadius: "var(--radius-md)", background: r.private ? "var(--warning-bg)" : "var(--success-bg)", color: r.private ? "var(--warning-color)" : "var(--success-color)" }}>{r.private ? "Private" : "Public"}</span>
 </div>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: "2px" }}>branch: {r.default_branch} · {r.url}</div>
 </div>
 ))}
 </div>
 )}

 {activeTab === "create" && (
 <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
 <div>
 <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "block", marginBottom: "4px" }}>Repository name</label>
 <input className="panel-input panel-input-full" placeholder="my-project" value={newRepoName} onChange={e => setNewRepoName(e.target.value)} />
 </div>
 <label style={{ display: "flex", alignItems: "center", gap: "8px", cursor: "pointer", fontSize: "var(--font-size-base)" }}>
 <input type="checkbox" checked={isPrivate} onChange={e => setIsPrivate(e.target.checked)} />
 Private repository
 </label>
 <button className="panel-btn panel-btn-primary" onClick={createRepo} disabled={loading || !newRepoName.trim()}>
 {loading ? "Creating..." : "Create & Push to GitHub"}
 </button>
 <p style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", margin: 0 }}>Creates a new GitHub repository and pushes the current workspace to it.</p>
 </div>
 )}
 </div>
 </div>
 );
}
