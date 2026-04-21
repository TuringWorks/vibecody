import { useState, useEffect } from 'react';
import { FolderOpen, AlertTriangle, X, ChevronDown, ChevronRight } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { ReviewPanel } from './ReviewPanel';
import { useToast } from '../hooks/useToast';
import { Toaster } from './Toaster';

interface GitPanelProps {
 workspacePath: string | null;
 onCompareFile?: (filePath: string, diff: string) => void;
 /** Provider name from the toolbar dropdown — forwarded to AI git commands so
  *  the commit-message generator (and friends) use the user's selected model
  *  instead of whichever provider happens to be active in the chat engine. */
 selectedProvider?: string;
}

interface GitStatus {
 branch: string;
 file_statuses: Record<string, string>;
}

interface CommitInfo {
 hash: string;
 author: string;
 message: string;
 timestamp: number;
}

export function GitPanel({ workspacePath, onCompareFile, selectedProvider }: GitPanelProps) {
 const { toasts, toast, dismiss } = useToast();
 const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
 const [commitMessage, setCommitMessage] = useState('');
 const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
 const [isLoading, setIsLoading] = useState(false);
 const [generatingMsg, setGeneratingMsg] = useState(false);
 const [branches, setBranches] = useState<string[]>([]);
 const [showHistory, setShowHistory] = useState(false);
 const [history, setHistory] = useState<CommitInfo[]>([]);
 const [selectedCommit, setSelectedCommit] = useState<CommitInfo | null>(null);
 const [commitFiles, setCommitFiles] = useState<string[]>([]);
 const [showReview, setShowReview] = useState(false);
 const [confirmDiscard, setConfirmDiscard] = useState<string | null>(null);
 const [branchTask, setBranchTask] = useState('');
 const [suggestingBranch, setSuggestingBranch] = useState(false);
 const [suggestedBranch, setSuggestedBranch] = useState<string | null>(null);
 const [showChangelog, setShowChangelog] = useState(false);
 const [changelog, setChangelog] = useState('');
 const [generatingChangelog, setGeneratingChangelog] = useState(false);
 const [changelogRef, setChangelogRef] = useState('HEAD~10');
 const [showConflictModal, setShowConflictModal] = useState(false);
 const [conflictText, setConflictText] = useState('');
 const [conflictFile, setConflictFile] = useState('');
 const [resolvingConflict, setResolvingConflict] = useState(false);
 const [conflictResolution, setConflictResolution] = useState('');
 const [gitError, setGitError] = useState<string | null>(null);
 const [showGitSettings, setShowGitSettings] = useState(false);
 const [gitUserName, setGitUserName] = useState('');
 const [gitUserEmail, setGitUserEmail] = useState('');
 const [gitCredUrl, setGitCredUrl] = useState('');
 const [gitCredUser, setGitCredUser] = useState('');
 const [gitCredToken, setGitCredToken] = useState('');
 const [sshAvailable, setSshAvailable] = useState(false);
 const [remoteUrl, setRemoteUrl] = useState('');

 useEffect(() => {
 if (workspacePath) {
 loadGitStatus();
 loadBranches();
 loadGitConfig();
 }
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [workspacePath]);

 // Auto-refresh git status every 30 seconds
 useEffect(() => {
 if (!workspacePath) return;
 const id = setInterval(loadGitStatus, 30_000);
 return () => clearInterval(id);
 }, [workspacePath]);

 const loadGitStatus = async () => {
 try {
 const status = await invoke<GitStatus>('get_git_status');
 setGitStatus(status);
 setGitError(null);
 } catch (e) {
 const msg = String(e);
 setGitError(msg);
 }
 };

 const loadBranches = async () => {
 if (!workspacePath) return;
 try {
 const branchList = await invoke<string[]>('git_list_branches', { path: workspacePath });
 setBranches(branchList);
 } catch (e) {
 toast.error(`Failed to load branches: ${e}`);
 }
 };

 const handleSwitchBranch = async (branch: string) => {
 if (!workspacePath) return;
 setIsLoading(true);
 try {
 await invoke('git_switch_branch', { path: workspacePath, branch });
 await loadGitStatus();
 toast.success(`Switched to branch: ${branch}`);
 } catch (e) {
 toast.error(`Failed to switch branch: ${e}`);
 } finally {
 setIsLoading(false);
 }
 };

 const handleShowHistory = async () => {
 if (!workspacePath) return;
 setShowHistory(!showHistory);
 if (!showHistory) {
 try {
 const commits = await invoke<CommitInfo[]>('git_get_history', { path: workspacePath, limit: 50 });
 setHistory(commits);
 } catch (e) {
 toast.error(`Failed to load history: ${e}`);
 }
 }
 };

 const handleSelectCommit = async (commit: CommitInfo) => {
 setSelectedCommit(commit);
 setCommitFiles([]);
 if (!workspacePath) return;
 try {
 const files = await invoke<string[]>('git_get_commit_files', {
 path: workspacePath,
 hash: commit.hash,
 });
 setCommitFiles(files);
 } catch (e) {
 toast.error(`Failed to get commit files: ${e}`);
 }
 };

 const handleCompareCommitFile = async (file: string) => {
 if (!workspacePath || !selectedCommit || !onCompareFile) return;
 try {
 const diff = await invoke<string>('git_diff', { path: workspacePath, filePath: file });
 onCompareFile(file, diff);
 } catch (e) {
 toast.error(`Failed to get diff: ${e}`);
 }
 };

 const handleDiscardChanges = async (file: string) => {
 if (!workspacePath) return;
 setConfirmDiscard(null);

 setIsLoading(true);
 try {
 await invoke('git_discard_changes', { path: workspacePath, filePath: file });
 await loadGitStatus();
 toast.success('Changes discarded');
 } catch (e) {
 toast.error(`Failed to discard changes: ${e}`);
 } finally {
 setIsLoading(false);
 }
 };

 const handleSuggestBranch = async () => {
 if (!branchTask.trim()) return;
 setSuggestingBranch(true);
 setSuggestedBranch(null);
 try {
 const name = await invoke<string>('suggest_branch_name', {
 taskDescription: branchTask,
 provider: selectedProvider || null,
 });
 setSuggestedBranch(name);
 } catch (e) {
 toast.error(`Branch suggestion failed: ${e}`);
 } finally {
 setSuggestingBranch(false);
 }
 };

 const handleGenerateChangelog = async () => {
 if (!workspacePath) return;
 setGeneratingChangelog(true);
 setChangelog('');
 try {
 const result = await invoke<string>('generate_changelog', {
 workspace: workspacePath,
 sinceRef: changelogRef || null,
 provider: selectedProvider || null,
 });
 setChangelog(result);
 } catch (e) {
 toast.error(`Changelog generation failed: ${e}`);
 } finally {
 setGeneratingChangelog(false);
 }
 };

 const handleResolveConflict = async () => {
 if (!workspacePath || !conflictText.trim()) return;
 setResolvingConflict(true);
 try {
 const resolved = await invoke<string>('resolve_merge_conflict', {
 filePath: conflictFile,
 conflictText,
 provider: selectedProvider || null,
 });
 setConflictResolution(resolved);
 } catch (e) {
 toast.error(`Conflict resolution failed: ${e}`);
 } finally {
 setResolvingConflict(false);
 }
 };

 const handleCompare = async (file: string) => {
 if (!workspacePath || !onCompareFile) return;
 try {
 const diff = await invoke<string>('git_diff', { path: workspacePath, filePath: file });
 onCompareFile(file, diff);
 } catch (e) {
 toast.error(`Failed to get diff: ${e}`);
 }
 };

 const handleGenerateMsg = async () => {
 setGeneratingMsg(true);
 try {
 const msg = await invoke<string>('generate_commit_message', {
 files: selectedFiles.length > 0 ? selectedFiles : null,
 // Honour the toolbar's model dropdown so the generator uses whatever
 // provider the user has selected (not the chat engine's default).
 provider: selectedProvider || null,
 });
 setCommitMessage(msg);
 } catch (e) {
 toast.error(`AI commit message failed: ${e}`);
 } finally {
 setGeneratingMsg(false);
 }
 };

 const handleCommit = async () => {
 if (!workspacePath || !commitMessage || selectedFiles.length === 0) return;

 setIsLoading(true);
 try {
 // Read profile for git author fallback
 const profileStr = localStorage.getItem('vibeui-profile');
 const profile = profileStr ? JSON.parse(profileStr) : {};
 await invoke('git_commit', {
 path: workspacePath,
 message: commitMessage,
 files: selectedFiles,
 authorName: profile.displayName || null,
 authorEmail: profile.email || null,
 });
 setCommitMessage('');
 setSelectedFiles([]);
 await loadGitStatus();
 toast.success('Committed successfully!');
 } catch (e) {
 toast.error(`Failed to commit: ${e}`);
 } finally {
 setIsLoading(false);
 }
 };

 const handlePush = async () => {
 if (!workspacePath || !gitStatus) return;

 setIsLoading(true);
 try {
 await invoke('git_push', {
 path: workspacePath,
 remote: 'origin',
 branch: gitStatus.branch,
 });
 toast.success('Pushed successfully!');
 } catch (e) {
 toast.error(`Failed to push: ${e}`);
 } finally {
 setIsLoading(false);
 }
 };

 const handlePull = async () => {
 if (!workspacePath || !gitStatus) return;

 setIsLoading(true);
 try {
 await invoke('git_pull', {
 path: workspacePath,
 remote: 'origin',
 branch: gitStatus.branch,
 });
 await loadGitStatus();
 toast.success('Pulled successfully!');
 } catch (e) {
 toast.error(`Failed to pull: ${e}`);
 } finally {
 setIsLoading(false);
 }
 };

 const loadGitConfig = async () => {
 if (!workspacePath) return;
 try {
 const config = await invoke<{ user_name: string; user_email: string; remote_url: string; ssh_available: boolean }>('get_git_config', { path: workspacePath });
 setGitUserName(config.user_name);
 setGitUserEmail(config.user_email);
 setRemoteUrl(config.remote_url);
 setSshAvailable(config.ssh_available);
 } catch {
 // Git config may not be available
 }
 };

 const saveGitConfig = async () => {
 if (!workspacePath) return;
 try {
 await invoke('set_git_config', { path: workspacePath, userName: gitUserName, userEmail: gitUserEmail });
 toast.success('Git config saved');
 } catch (e) {
 toast.error(`Failed to save git config: ${e}`);
 }
 };

 const saveGitCredentials = async () => {
 if (!gitCredUrl || !gitCredUser || !gitCredToken) return;
 try {
 await invoke('store_git_credentials', { url: gitCredUrl, username: gitCredUser, token: gitCredToken });
 toast.success('Credentials stored');
 setGitCredToken('');
 } catch (e) {
 toast.error(`Failed to store credentials: ${e}`);
 }
 };

 const toggleFileSelection = (file: string) => {
 setSelectedFiles(prev =>
 prev.includes(file)
 ? prev.filter(f => f !== file)
 : [...prev, file]
 );
 };

 const toggleSelectAll = (allFiles: string[]) => {
 if (selectedFiles.length === allFiles.length) {
 setSelectedFiles([]);
 } else {
 setSelectedFiles([...allFiles]);
 }
 };

 if (!workspacePath) {
 return (
 <div className="empty-state">
 <p>No workspace folder open</p>
 </div>
 );
 }

 if (!gitStatus) {
 if (gitError) {
 const isNotRepo = gitError.toLowerCase().includes('not a git repository') || gitError.toLowerCase().includes('not found');
 return (
  <div style={{ padding: '24px 16px', textAlign: 'center', color: 'var(--text-secondary)' }}>
  <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--text-secondary)" }}>{isNotRepo ? <FolderOpen size={28} strokeWidth={1.5} /> : <AlertTriangle size={28} strokeWidth={1.5} />}</div>
  <div style={{ fontSize: "var(--font-size-md)", fontWeight: 500, marginBottom: 6 }}>
   {isNotRepo ? 'No Git Repository' : 'Git Error'}
  </div>
  <div style={{ fontSize: "var(--font-size-base)", lineHeight: 1.6, marginBottom: 12 }}>
   {isNotRepo
    ? 'This folder is not a git repository.'
    : gitError}
  </div>
  {isNotRepo && (
   <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', opacity: 0.7 }}>
    Run <code style={{ fontSize: "var(--font-size-xs)" }}>git init</code> in the terminal to initialize one.
   </div>
  )}
  </div>
 );
 }
 return (
 <div className="empty-state">
 <p>Loading git status...</p>
 </div>
 );
 }

 const changedFiles = Object.entries(gitStatus.file_statuses);

 return (
 <div className="panel-container" style={{ padding: '12px' }}>
 <div style={{ marginBottom: '12px', display: 'flex', alignItems: 'center', gap: '8px' }}>
 <strong>Branch:</strong>
 <select
 value={gitStatus.branch}
 onChange={(e) => handleSwitchBranch(e.target.value)}
 disabled={isLoading}
 className="panel-select"
 style={{ flex: 1 }}
 >
 {branches.map(branch => (
 <option key={branch} value={branch}>{branch}</option>
 ))}
 </select>
 </div>

 <div style={{ marginBottom: '12px', display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
 <button className="panel-btn btn-primary" onClick={handlePull} disabled={isLoading} style={{ fontSize: '12px', padding: '4px 8px' }}>
 Pull
 </button>
 <button className="panel-btn btn-primary" onClick={handlePush} disabled={isLoading} style={{ fontSize: '12px', padding: '4px 8px' }}>
 Push
 </button>
 <button className="panel-btn btn-secondary" onClick={handleShowHistory} style={{ fontSize: '12px', padding: '4px 8px' }}>
 History
 </button>
 </div>

 <div style={{ flex: 1, overflowY: 'auto', marginBottom: '12px' }}>
 {showHistory ? (
 <div>
 <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
 <h3 style={{ fontSize: '13px' }}>Commit History</h3>
 <button aria-label="Close history" onClick={() => setShowHistory(false)} style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', display: 'flex', alignItems: 'center' }}><X size={16} /></button>
 </div>
 {selectedCommit ? (
 <div>
 <button onClick={() => setSelectedCommit(null)} style={{ background: 'none', border: 'none', color: 'var(--accent-blue)', cursor: 'pointer', fontSize: '11px', marginBottom: '8px' }}>← Back to commits</button>
 <div style={{ padding: '8px', background: 'var(--bg-tertiary)', borderRadius: '4px', marginBottom: '8px' }}>
 <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>{selectedCommit.hash.substring(0, 7)} • {selectedCommit.author}</div>
 <div style={{ fontSize: '12px', marginTop: '4px' }}>{selectedCommit.message}</div>
 </div>
 <h4 style={{ fontSize: '11px', marginBottom: '8px', color: 'var(--text-secondary)' }}>Files Changed</h4>
 {commitFiles.map(file => (
 <div key={file} style={{ padding: '8px', background: 'var(--bg-secondary)', borderRadius: '4px', marginBottom: '4px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
 <span style={{ fontSize: '11px' }}>{file}</span>
 <button onClick={() => handleCompareCommitFile(file)} style={{ background: 'none', border: 'none', color: 'var(--accent-blue)', cursor: 'pointer', fontSize: '10px' }}>Diff</button>
 </div>
 ))}
 </div>
 ) : (
 history.map(commit => (
 <div role="button" tabIndex={0}
 key={commit.hash}
 onClick={() => handleSelectCommit(commit)}
 style={{
 padding: '8px',
 marginBottom: '8px',
 background: 'var(--bg-tertiary)',
 borderRadius: '4px',
 cursor: 'pointer',
 }}
 onMouseEnter={(e) => e.currentTarget.style.background = 'var(--bg-secondary)'}
 onMouseLeave={(e) => e.currentTarget.style.background = 'var(--bg-tertiary)'}
 >
 <div style={{ fontSize: '10px', color: 'var(--text-secondary)', marginBottom: '2px' }}>
 {commit.hash.substring(0, 7)} • {commit.author} • {new Date(commit.timestamp * 1000).toLocaleDateString()}
 </div>
 <div style={{ fontSize: '11px' }}>{commit.message}</div>
 </div>
 ))
 )}
 </div>
 ) : (
 <div>
 <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '8px' }}>
 {changedFiles.length > 0 && (
 <input
 type="checkbox"
 checked={selectedFiles.length === changedFiles.length && changedFiles.length > 0}
 ref={(el) => { if (el) el.indeterminate = selectedFiles.length > 0 && selectedFiles.length < changedFiles.length; }}
 onChange={() => toggleSelectAll(changedFiles.map(([f]) => f))}
 title={selectedFiles.length === changedFiles.length ? 'Deselect all' : 'Select all'}
 />
 )}
 <h3 style={{ fontSize: '13px', margin: 0 }}>Changes</h3>
 {changedFiles.length > 0 && (
 <span style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>
 {selectedFiles.length}/{changedFiles.length}
 </span>
 )}
 </div>
 {changedFiles.length === 0 ? (
 <p style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>No changes</p>
 ) : (
 changedFiles.map(([file, status]) => (
 <div
 key={file}
 style={{
 padding: '8px',
 background: selectedFiles.includes(file) ? 'var(--bg-tertiary)' : 'transparent',
 borderRadius: '4px',
 marginBottom: '4px',
 display: 'flex',
 alignItems: 'center',
 gap: '8px',
 }}
 >
 <input
 type="checkbox"
 checked={selectedFiles.includes(file)}
 onChange={() => toggleFileSelection(file)}
 />
 <span style={{ fontSize: '11px', flex: 1 }}>{file}</span>
 <span style={{ fontSize: '9px', color: 'var(--text-secondary)' }}>{status}</span>
 <button
 onClick={() => handleCompare(file)}
 style={{
 background: 'none',
 border: 'none',
 color: 'var(--text-secondary)', /* Muted color */
 cursor: 'pointer',
 fontSize: '10px',
 padding: '2px 4px',
 }}
 title="Compare"
 >
 Diff
 </button>
 {confirmDiscard === file ? (
 <>
 <span style={{ fontSize: '10px', color: 'var(--text-danger)' }}>Discard?</span>
 <button
 onClick={() => handleDiscardChanges(file)}
 style={{ background: 'none', border: 'none', color: 'var(--text-danger)', cursor: 'pointer', fontSize: '10px', padding: '2px 4px', fontWeight: 600 }}
 >
 Yes
 </button>
 <button
 onClick={() => setConfirmDiscard(null)}
 style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', fontSize: '10px', padding: '2px 4px' }}
 >
 No
 </button>
 </>
 ) : (
 <button
 onClick={() => setConfirmDiscard(file)}
 style={{ background: 'none', border: 'none', color: 'var(--text-danger)', cursor: 'pointer', padding: '2px 4px', display: 'flex', alignItems: 'center' }}
 title="Discard changes"
 >
 <X size={10} />
 </button>
 )}
 </div>
 ))
 )}
 </div>
 )}
 </div>

 <div>
 <div style={{ position: 'relative' }}>
 <textarea
 value={commitMessage}
 onChange={(e) => setCommitMessage(e.target.value)}
 placeholder="Commit message..."
 className="panel-input panel-textarea panel-input-full"
 style={{ minHeight: '50px', paddingRight: '64px', marginBottom: '8px', fontFamily: 'inherit' }}
 />
 <button className="panel-btn"
 onClick={handleGenerateMsg}
 disabled={generatingMsg}
 title="Generate commit message with AI"
 style={{
 position: 'absolute', top: '4px', right: '4px',
 padding: '2px 8px', fontSize: '10px', fontWeight: 600,
 background: generatingMsg ? 'var(--bg-secondary)' : 'var(--accent-bg)',
 color: generatingMsg ? 'var(--text-secondary)' : 'var(--accent-color)',
 border: '1px solid var(--border-color)', borderRadius: '3px',
 cursor: generatingMsg ? 'not-allowed' : 'pointer',
 }}
 >
 {generatingMsg ? '…' : ' AI'}
 </button>
 </div>
 <button
 className="panel-btn btn-primary"
 onClick={handleCommit}
 disabled={isLoading || !commitMessage || selectedFiles.length === 0}
 style={{ width: '100%', fontSize: '12px' }}
 >
 Commit ({selectedFiles.length} files)
 </button>
 </div>

 {/* ── Code Review section ── */}
 <div style={{ borderTop: '1px solid var(--border-color)', paddingTop: 8 }}>
 <button
 onClick={() => setShowReview(!showReview)}
 style={{
 width: '100%', textAlign: 'left', padding: '8px 8px',
 background: showReview ? 'var(--bg-tertiary)' : 'transparent',
 border: 'none', borderRadius: "var(--radius-xs-plus)", cursor: 'pointer',
 color: 'var(--text-primary)', fontSize: "var(--font-size-base)",
 display: 'flex', alignItems: 'center', gap: 6,
 }}
 >
 {showReview && <ChevronDown size={12} />}
 <span>Code Review</span>
 </button>
 {showReview && (
 <div style={{ marginTop: 8, height: 420, borderRadius: "var(--radius-sm)", overflow: 'hidden', background: 'var(--bg-secondary)' }}>
 <ReviewPanel
 workspacePath={workspacePath}
 onOpenFile={onCompareFile ? (path) => {
 invoke<string>('git_diff', { path: workspacePath, filePath: path })
 .then((diff) => onCompareFile(path, diff))
 .catch(console.error);
 } : undefined}
 />
 </div>
 )}
 </div>
 {/* ── AI Git Tools section ── */}
 <div style={{ borderTop: '1px solid var(--border-color)', paddingTop: 8 }}>
 {/* Branch Name Suggester */}
 <div style={{ marginBottom: 10 }}>
 <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginBottom: 4 }}>AI Branch Name</div>
 <div style={{ display: 'flex', gap: 6 }}>
 <input
 value={branchTask}
 onChange={e => setBranchTask(e.target.value)}
 onKeyDown={e => e.key === 'Enter' && handleSuggestBranch()}
 placeholder="Describe the task…"
 style={{ flex: 1, background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', fontFamily: 'inherit', fontSize: "var(--font-size-sm)" }}
 />
 <button className="panel-btn"
 onClick={handleSuggestBranch}
 disabled={suggestingBranch || !branchTask.trim()}
 style={{ background: 'var(--accent-bg)', color: 'var(--accent-color)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', cursor: 'pointer', fontSize: "var(--font-size-sm)" }}
 >
 {suggestingBranch ? '…' : ''}
 </button>
 </div>
 {suggestedBranch && (
 <div style={{ marginTop: 5, display: 'flex', alignItems: 'center', gap: 8, background: 'var(--bg-secondary)', padding: '4px 8px', borderRadius: "var(--radius-xs-plus)" }}>
 <code style={{ flex: 1, fontSize: "var(--font-size-sm)", color: 'var(--info-color)' }}>{suggestedBranch}</code>
 <button
 onClick={() => { navigator.clipboard.writeText(suggestedBranch).then(() => toast.success('Copied!')).catch(() => {}); }}
 style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', fontSize: "var(--font-size-xs)" }}
 >
 
 </button>
 </div>
 )}
 </div>

 {/* Changelog Generator */}
 <div style={{ marginBottom: 10 }}>
 <button
 onClick={() => setShowChangelog(c => !c)}
 style={{ width: '100%', textAlign: 'left', padding: '4px 0', background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--text-primary)', fontSize: "var(--font-size-base)", display: 'flex', alignItems: 'center', gap: 6 }}
 >
 {showChangelog && <ChevronDown size={12} />}
 <span>Generate Changelog</span>
 </button>
 {showChangelog && (
 <div style={{ marginTop: 6 }}>
 <div style={{ display: 'flex', gap: 6, marginBottom: 6 }}>
 <input
 value={changelogRef}
 onChange={e => setChangelogRef(e.target.value)}
 placeholder="since (e.g. HEAD~10 or v1.2.0)"
 style={{ flex: 1, background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', fontFamily: 'inherit', fontSize: "var(--font-size-sm)" }}
 />
 <button className="panel-btn"
 onClick={handleGenerateChangelog}
 disabled={generatingChangelog}
 style={{ background: 'var(--accent-bg)', color: 'var(--accent-color)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', cursor: 'pointer', fontSize: "var(--font-size-sm)" }}
 >
 {generatingChangelog ? '…' : ' Generate'}
 </button>
 </div>
 {changelog && (
 <div style={{ position: 'relative' }}>
 <textarea
 value={changelog}
 onChange={e => setChangelog(e.target.value)}
 rows={8}
 style={{ width: '100%', background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: 6, fontFamily: 'inherit', fontSize: "var(--font-size-sm)", boxSizing: 'border-box' }}
 />
 <button
 onClick={() => { navigator.clipboard.writeText(changelog).then(() => toast.success('Copied!')).catch(() => {}); }}
 style={{ position: 'absolute', top: 4, right: 4, background: 'var(--bg-secondary)', border: '1px solid var(--border-color)', borderRadius: 3, padding: '2px 8px', cursor: 'pointer', fontSize: "var(--font-size-xs)", color: 'var(--text-secondary)' }}
 >
 
 </button>
 </div>
 )}
 </div>
 )}
 </div>

 {/* Merge Conflict Resolver */}
 <div>
 <button
 onClick={() => setShowConflictModal(c => !c)}
 style={{ width: '100%', textAlign: 'left', padding: '4px 0', background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--text-primary)', fontSize: "var(--font-size-base)", display: 'flex', alignItems: 'center', gap: 6 }}
 >
 {showConflictModal && <ChevronDown size={12} />}
 <span>Resolve Merge Conflict</span>
 </button>
 {showConflictModal && (
 <div style={{ marginTop: 6 }}>
 <input
 value={conflictFile}
 onChange={e => setConflictFile(e.target.value)}
 placeholder="File path (e.g. src/main.rs)"
 style={{ width: '100%', background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', fontFamily: 'inherit', fontSize: "var(--font-size-sm)", marginBottom: 5, boxSizing: 'border-box' }}
 />
 <textarea
 value={conflictText}
 onChange={e => setConflictText(e.target.value)}
 placeholder="Paste the conflict block here (<<<<<<< HEAD ... ======= ... >>>>>>> branch)..."
 rows={6}
 style={{ width: '100%', background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: 6, fontFamily: 'inherit', fontSize: "var(--font-size-sm)", marginBottom: 5, boxSizing: 'border-box' }}
 />
 <button className="panel-btn"
 onClick={handleResolveConflict}
 disabled={resolvingConflict || !conflictText.trim()}
 style={{ width: '100%', background: 'var(--accent-bg)', color: 'var(--accent-color)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '4px 0', cursor: 'pointer', fontSize: "var(--font-size-sm)", marginBottom: 5 }}
 >
 {resolvingConflict ? ' Resolving…' : ' AI Resolve'}
 </button>
 {conflictResolution && (
 <div style={{ position: 'relative' }}>
 <textarea
 value={conflictResolution}
 onChange={e => setConflictResolution(e.target.value)}
 rows={8}
 style={{ width: '100%', background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: 6, fontFamily: 'inherit', fontSize: "var(--font-size-sm)", boxSizing: 'border-box' }}
 />
 <button
 onClick={() => { navigator.clipboard.writeText(conflictResolution).then(() => toast.success('Copied!')).catch(() => {}); }}
 style={{ position: 'absolute', top: 4, right: 4, background: 'var(--bg-secondary)', border: '1px solid var(--border-color)', borderRadius: 3, padding: '2px 8px', cursor: 'pointer', fontSize: "var(--font-size-xs)", color: 'var(--text-secondary)' }}
 >
 Copy resolution
 </button>
 </div>
 )}
 </div>
 )}
 </div>
 </div>

 {/* ── Git Settings section ── */}
 <div style={{ borderTop: '1px solid var(--border-color)', paddingTop: 8 }}>
 <button
 onClick={() => setShowGitSettings(!showGitSettings)}
 style={{
 width: '100%', textAlign: 'left', padding: '8px 8px',
 background: showGitSettings ? 'var(--bg-tertiary)' : 'transparent',
 border: 'none', borderRadius: "var(--radius-xs-plus)", cursor: 'pointer',
 color: 'var(--text-primary)', fontSize: "var(--font-size-base)",
 display: 'flex', alignItems: 'center', gap: 6,
 }}
 >
 {showGitSettings ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
 <span>Git Settings</span>
 {sshAvailable && <span style={{ fontSize: 9, padding: '1px 4px', borderRadius: 3, background: 'var(--success-bg)', color: 'var(--success-color)' }}>SSH</span>}
 </button>
 {showGitSettings && (
 <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 10 }}>
 {/* User identity */}
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginBottom: 4 }}>User Identity</div>
 <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
 <input
 value={gitUserName}
 onChange={e => setGitUserName(e.target.value)}
 placeholder="User name"
 style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', fontSize: "var(--font-size-sm)", fontFamily: 'inherit' }}
 />
 <input
 value={gitUserEmail}
 onChange={e => setGitUserEmail(e.target.value)}
 placeholder="Email"
 style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', fontSize: "var(--font-size-sm)", fontFamily: 'inherit' }}
 />
 <button className="panel-btn"
 onClick={saveGitConfig}
 disabled={!gitUserName && !gitUserEmail}
 style={{ alignSelf: 'flex-start', background: 'var(--accent-bg)', color: 'var(--accent-color)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', cursor: 'pointer', fontSize: "var(--font-size-sm)" }}
 >
 Save Identity
 </button>
 </div>
 </div>

 {/* Remote & SSH info */}
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginBottom: 4 }}>Remote</div>
 <div style={{ fontSize: "var(--font-size-sm)", padding: '4px 8px', background: 'var(--bg-tertiary)', borderRadius: "var(--radius-xs-plus)", wordBreak: 'break-all' }}>
 {remoteUrl || 'No remote configured'}
 </div>
 <div style={{ marginTop: 4, fontSize: "var(--font-size-xs)", color: sshAvailable ? 'var(--success-color)' : 'var(--text-secondary)' }}>
 {remoteUrl.startsWith('git@') ? 'Using SSH' : sshAvailable ? 'SSH keys detected — switch remote to SSH for passwordless auth' : 'No SSH keys found — use HTTPS with credentials below'}
 </div>
 </div>

 {/* Credentials for HTTPS */}
 {!remoteUrl.startsWith('git@') && (
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginBottom: 4 }}>HTTPS Credentials</div>
 <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
 <input
 value={gitCredUrl}
 onChange={e => setGitCredUrl(e.target.value)}
 placeholder="Repository URL (e.g. https://github.com/user/repo)"
 style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', fontSize: "var(--font-size-sm)", fontFamily: 'inherit' }}
 />
 <input
 value={gitCredUser}
 onChange={e => setGitCredUser(e.target.value)}
 placeholder="Username"
 style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', fontSize: "var(--font-size-sm)", fontFamily: 'inherit' }}
 />
 <input
 type="password"
 value={gitCredToken}
 onChange={e => setGitCredToken(e.target.value)}
 placeholder="Personal access token / password"
 style={{ background: 'var(--bg-tertiary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', fontSize: "var(--font-size-sm)", fontFamily: 'inherit' }}
 />
 <button className="panel-btn"
 onClick={saveGitCredentials}
 disabled={!gitCredUrl || !gitCredUser || !gitCredToken}
 style={{ alignSelf: 'flex-start', background: 'var(--accent-bg)', color: 'var(--accent-color)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)", padding: '3px 8px', cursor: 'pointer', fontSize: "var(--font-size-sm)" }}
 >
 Store Credentials
 </button>
 <div style={{ fontSize: "var(--font-size-xs)", color: 'var(--text-secondary)' }}>
 Stored via git credential-store. Use a personal access token instead of password.
 </div>
 </div>
 </div>
 )}
 </div>
 )}
 </div>

 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
}
