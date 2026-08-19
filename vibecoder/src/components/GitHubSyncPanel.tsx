import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";

/** Mirrors `GitHubSyncStatus` in src-tauri/src/commands.rs. `branch`, `ahead`
 *  and `behind` are null when git could not answer — an untracked branch is
 *  not a branch that is level with its remote. */
interface GitHubSyncStatus {
 repo_url: string | null;
 branch: string | null;
 ahead: number | null;
 behind: number | null;
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
 // eslint-disable-next-line react-hooks/exhaustive-deps
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

 /** Commit/stage/push all live in the Source Control sidebar; this panel only
  *  ever links there so there is one place that writes to the repo. */
 const openSourceControl = () =>
   window.dispatchEvent(new CustomEvent("vibecoder:open-sidebar-tab", { detail: "git" }));

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
 const badgeStyle = (n: number, type: "ahead" | "behind") => ({
   padding: "2px 8px",
   borderRadius: "var(--radius-md)",
   fontSize: "var(--font-size-sm)",
   background: statusBadgeBg(n, type),
   color: statusBadgeFg(n, type),
 });
 /** Counts are null when the branch has no upstream — say that instead of 0/0. */
 const tracked = status?.ahead != null && status?.behind != null;

 return (
 <div className="panel-container">
 <div className="panel-header">
 <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
 <span style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>GitHub Remote</span>
 {status?.has_remote && (
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{status.repo_url?.replace("https://github.com/", "")}</span>
 )}
 </div>
 {status?.has_remote && (
 <div style={{ display: "flex", gap: "8px", marginTop: "8px", alignItems: "center" }}>
 {status.ahead != null && status.behind != null ? (
 <>
 <span style={badgeStyle(status.ahead, "ahead")}>↑ {status.ahead} ahead</span>
 <span style={badgeStyle(status.behind, "behind")}>↓ {status.behind} behind</span>
 </>
 ) : (
 <span style={{ ...badgeStyle(0, "ahead"), color: "var(--warning-color)", background: "var(--warning-bg)" }}>no upstream branch</span>
 )}
 {status.branch && <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>branch: {status.branch}</span>}
 </div>
 )}
 </div>

 {!tokenSaved && (
 <div style={{ padding: "12px 12px", background: "var(--warning-bg)", borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ fontSize: "var(--font-size-base)", marginBottom: "8px", color: "var(--warning-color)" }}>GitHub token required to list or create repositories</div>
 <div style={{ display: "flex", gap: "8px" }}>
 <input className="panel-input" style={{ flex: 1 }} type="password" placeholder="ghp_..." value={token} onChange={e => setToken(e.target.value)} />
 <button className="panel-btn panel-btn-primary" onClick={saveToken}>Save</button>
 </div>
 </div>
 )}

 <div className="panel-tab-bar">
 {(["sync", "repos", "create"] as const).map(t => (
 <button key={t} className={`panel-tab ${activeTab === t ? "active" : ""}`} onClick={() => { setActiveTab(t); if (t === "repos") listRepos(); }}>
 {t === "sync" ? "Remote" : t === "repos" ? "Repos" : "New Repo"}
 </button>
 ))}
 </div>

 {(error || success) && (
 <div className={error ? "panel-error" : "panel-section"} style={{ color: error ? "var(--error-color)" : "var(--success-color)", background: error ? "var(--error-bg)" : "var(--success-bg)" }}>
 {error || success}
 <button className="panel-btn" aria-label="Dismiss" style={{ float: "right", background: "none", border: "none", cursor: "pointer", color: "inherit", display: "inline-flex", alignItems: "center" }} onClick={() => { setError(null); setSuccess(null); }}><X size={14} /></button>
 </div>
 )}

 <div className="panel-body">
 {activeTab === "sync" && (
 <>
 {!status?.has_remote && (
 <div className="panel-empty">
 No remote configured. Create a repo or link an existing one.
 <button className="panel-btn panel-btn-primary" style={{ display: "block", margin: "12px auto 0" }} onClick={() => setActiveTab("create")}>Create Repository</button>
 </div>
 )}
 {status?.has_remote && (
 <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
 <div className="panel-section" style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
 Staging, commits, push and pull live in <strong>Source Control</strong> in the
 left sidebar — one place that writes to this repo. This panel covers the
 GitHub account side: token, repositories, and remote state.
 </div>
 <div style={{ display: "flex", gap: "8px" }}>
 <button className="panel-btn panel-btn-primary" style={{ flex: 1 }} onClick={openSourceControl}>Open Source Control</button>
 <button className="panel-btn panel-btn-secondary" onClick={loadStatus} disabled={loading} aria-label="Refresh remote status">⟳</button>
 </div>
 {!tracked && (
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
 {status.branch ? `${status.branch} tracks no remote branch` : "HEAD is not on a branch"} — push once from
 Source Control to set the upstream, then ahead/behind counts appear here.
 </div>
 )}
 {status.last_synced && <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Last synced: {status.last_synced}</div>}
 </div>
 )}
 </>
 )}

 {activeTab === "repos" && (
 <div>
 {repos.length === 0 && !loading && <div className="panel-empty">Click "Repos" tab to load your repositories</div>}
 {repos.map(r => (
 <div key={r.full_name} style={{ padding: "8px 12px", borderRadius: "var(--radius-xs-plus)", marginBottom: "4px", background: "var(--bg-secondary)" }}>
 <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span style={{ fontWeight: 600 }}> {r.name}</span>
 <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: "var(--radius-md)", background: r.private ? "var(--warning-bg)" : "var(--success-bg)", color: r.private ? "var(--warning-color)" : "var(--success-color)" }}>{r.private ? "Private" : "Public"}</span>
 </div>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: "2px" }}>branch: {r.default_branch} · {r.url}</div>
 </div>
 ))}
 </div>
 )}

 {activeTab === "create" && (
 <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
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
