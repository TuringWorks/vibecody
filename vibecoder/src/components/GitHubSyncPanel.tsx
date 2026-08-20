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
 const [tokenSaved, setTokenSaved] = useState(false);

 useEffect(() => {
 if (!workspacePath) return;
 loadStatus();
 checkToken();
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [workspacePath]);

 // Settings owns the token; re-read when it is saved or cleared there.
 useEffect(() => {
 const onChanged = (e: Event) => {
 const detail = (e as CustomEvent<{ field?: string }>).detail;
 if (detail?.field === "github_token") checkToken();
 };
 window.addEventListener("vibecoder:integration-token-changed", onChanged);
 return () => window.removeEventListener("vibecoder:integration-token-changed", onChanged);
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [workspacePath]);

 if (!workspacePath) {
 return <div className="empty-state"><p>Open a workspace folder to use the GitHub remote panel.</p></div>;
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

 /** Staging, commits, push and pull are the Changes half of this same panel —
  *  one place that writes to the repo. */
 const openChanges = () =>
   window.dispatchEvent(new CustomEvent("vibecoder:git-view", { detail: "changes" }));

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

 /** The token lives in Settings → Integrations → Infrastructure, encrypted in
  *  the profile store. This panel reads whether one is set and links there. */
 const openTokenSettings = () =>
   window.dispatchEvent(new CustomEvent("vibecoder:open-settings", {
     detail: { section: "integrations", category: "infra" },
   }));

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
 <div style={{ padding: "12px", background: "var(--warning-bg)", borderBottom: "1px solid var(--border-color)", display: "flex", alignItems: "center", gap: "12px", flexWrap: "wrap" }}>
 <div style={{ flex: 1, minWidth: "220px", fontSize: "var(--font-size-base)", color: "var(--warning-color)" }}>
 No GitHub token set — listing and creating repositories needs one.
 </div>
 <button className="panel-btn panel-btn-primary" onClick={openTokenSettings}>Add token in Settings</button>
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
 Staging, commits, push and pull are in the <strong>Changes</strong> tab — one
 place that writes to this repo. This tab covers the GitHub account side:
 repositories and remote state. The access token lives in Settings →
 Integrations → Infrastructure.
 </div>
 <div style={{ display: "flex", gap: "8px" }}>
 <button className="panel-btn panel-btn-primary" style={{ flex: 1 }} onClick={openChanges}>Go to Changes</button>
 <button className="panel-btn panel-btn-secondary" onClick={loadStatus} disabled={loading} aria-label="Refresh remote status">⟳</button>
 </div>
 {!tracked && (
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
 {status.branch ? `${status.branch} tracks no remote branch` : "HEAD is not on a branch"} — push once from
 the Changes tab to set the upstream, then ahead/behind counts appear here.
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
