import { useState, useEffect, useCallback, lazy, Suspense } from 'react';
import { gitStatusCode, gitStatusLabel } from '../lib/gitStatus';
import { MiddleTruncate } from './MiddleTruncate';
import { FolderOpen, AlertTriangle, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { ReviewPanel, ReviewControls, useCodeReview } from './ReviewPanel';
import { useToast } from '../hooks/useToast';
import { Toaster } from './Toaster';

/** The GitHub tabs (Remote / Actions / Triage) render inside this panel rather
 *  than in a second one on the right — lazily, so opening Source Control does
 *  not pay for three panels nobody asked for. */
const GitHubComposite = lazy(() =>
 import('./composite/GitHubComposite').then(m => ({ default: m.GitHubComposite })));

/** Which half of Source Control is showing. */
export type GitPanelView =
 | 'changes'
 | 'review'
 | 'tools'
 | 'github';

interface GitPanelProps {
 workspacePath: string | null;
 /** Open `filePath` in the diff view. The host fetches both sides from git —
  *  a unified diff cannot supply them, which is what this callback used to
  *  hand over. With `commit`, the comparison is that commit against its
  *  parent; without, HEAD against the working tree. */
 onCompareFile?: (filePath: string, commit?: string) => void;
 /** Provider name from the toolbar dropdown — forwarded to AI git commands so
  *  the commit-message generator (and friends) use the user's selected model
  *  instead of whichever provider happens to be active in the chat engine. */
 selectedProvider?: string;
 /** Controlled view. Omit to let the panel own it; pass both to drive it from
  *  outside (the Infrastructure sidebar's "GitHub Actions" entry does). */
 view?: GitPanelView;
 onViewChange?: (view: GitPanelView) => void;
}

interface GitStatus {
 branch: string;
 file_statuses: Record<string, string>;
}

/** Mirrors `GitHubSyncStatus` in src-tauri/src/commands.rs. `ahead`/`behind`
 *  are null when the branch tracks nothing — that is not "level with origin",
 *  so it renders as "no upstream" rather than 0/0. */
interface UpstreamStatus {
 repo_url: string | null;
 branch: string | null;
 ahead: number | null;
 behind: number | null;
 has_remote: boolean;
}

interface CommitInfo {
 hash: string;
 author: string;
 message: string;
 timestamp: number;
}

/** Mirrors `GitRepoSuggestion` in src-tauri/src/commands.rs. */
interface GitRepoSuggestion {
 in_repo: boolean;
 repo_root: string | null;
 should_suggest: boolean;
 declined: boolean;
 blocked_reason: string | null;
}

/** Longest commit message shown before it is clipped behind "more". */
const COMMIT_MESSAGE_CLAMP = 100;

/**
 * A commit message that does not take over the panel.
 *
 * Git messages are a short subject line and an arbitrarily long body, and this
 * list rendered the whole thing. One commit with a real body pushed "Files
 * Changed" and every file under it off the visible area, so the list you came
 * to the history for was unreachable without scrolling past prose.
 *
 * Clipped at the first line break or `COMMIT_MESSAGE_CLAMP` characters,
 * whichever comes first — the subject line is the part that identifies a
 * commit, and a body is exactly what a reader has not asked for yet. Nothing
 * is hidden without a way back: the toggle is only rendered when there is more
 * to show, so a short message has no dangling control.
 */
function CommitMessage({ text, fontSize }: { text: string; fontSize: number }) {
  const [expanded, setExpanded] = useState(false);
  const trimmed = text.trim();
  const firstBreak = trimmed.indexOf('\n');
  const subjectEnd = firstBreak === -1 ? trimmed.length : firstBreak;
  const cut = Math.min(subjectEnd, COMMIT_MESSAGE_CLAMP);
  const clipped = cut < trimmed.length;

  return (
    <div style={{ fontSize, marginTop: 4 }}>
      <span style={{ whiteSpace: expanded ? 'pre-wrap' : 'normal', wordBreak: 'break-word' }}>
        {expanded ? trimmed : trimmed.slice(0, cut)}
        {clipped && !expanded && '…'}
      </span>
      {clipped && (
        <button
          onClick={(e) => { e.stopPropagation(); setExpanded((v) => !v); }}
          aria-expanded={expanded}
          style={{
            marginLeft: 6, background: 'none', border: 'none', padding: 0,
            color: 'var(--accent-blue)', cursor: 'pointer', fontSize: 'var(--font-size-xs)',
          }}
        >
          {expanded ? 'less' : 'more'}
        </button>
      )}
    </div>
  );
}

export function GitPanel({ workspacePath, onCompareFile, selectedProvider, view: viewProp, onViewChange }: GitPanelProps) {
 const { toasts, toast, dismiss } = useToast();
 const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
 const [upstream, setUpstream] = useState<UpstreamStatus | null>(null);
 const [localView, setLocalView] = useState<GitPanelView>('changes');
 const view = viewProp ?? localView;
 const setView = (next: GitPanelView) => { setLocalView(next); onViewChange?.(next); };
 /* One review run, shared across two tabs: the controls that start it sit with
  * the changes on the Changes tab, the findings on the Review tab. Held here
  * because neither component can own state the other needs. */
 const review = useCodeReview(workspacePath, selectedProvider);
 const [commitMessage, setCommitMessage] = useState('');
 const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
 const [isLoading, setIsLoading] = useState(false);
 const [generatingMsg, setGeneratingMsg] = useState(false);
 const [branches, setBranches] = useState<string[]>([]);
 const [showHistory, setShowHistory] = useState(false);
 const [history, setHistory] = useState<CommitInfo[]>([]);
 const [selectedCommit, setSelectedCommit] = useState<CommitInfo | null>(null);
 const [commitFiles, setCommitFiles] = useState<string[]>([]);
 const [confirmDiscard, setConfirmDiscard] = useState<string | null>(null);
 const [branchTask, setBranchTask] = useState('');
 const [suggestingBranch, setSuggestingBranch] = useState(false);
 const [suggestedBranch, setSuggestedBranch] = useState<string | null>(null);
 const [changelog, setChangelog] = useState('');
 const [generatingChangelog, setGeneratingChangelog] = useState(false);
 const [changelogRef, setChangelogRef] = useState('HEAD~10');
 const [conflictText, setConflictText] = useState('');
 const [conflictFile, setConflictFile] = useState('');
 const [resolvingConflict, setResolvingConflict] = useState(false);
 const [conflictResolution, setConflictResolution] = useState('');
 const [gitError, setGitError] = useState<string | null>(null);
 const [gitUserName, setGitUserName] = useState('');
 const [gitUserEmail, setGitUserEmail] = useState('');
 const [gitCredUrl, setGitCredUrl] = useState('');
 const [gitCredUser, setGitCredUser] = useState('');
 const [gitCredToken, setGitCredToken] = useState('');
 const [sshAvailable, setSshAvailable] = useState(false);
 const [remoteUrl, setRemoteUrl] = useState('');
 const [repoSuggestion, setRepoSuggestion] = useState<GitRepoSuggestion | null>(null);
 const [initializingRepo, setInitializingRepo] = useState(false);

 const loadGitStatus = useCallback(async () => {
 try {
 const status = await invoke<GitStatus>('get_git_status');
 setGitStatus(status);
 setGitError(null);
 } catch (e) {
 const msg = String(e);
 setGitError(msg);
 }
 // Ahead/behind belongs next to the Push button, not in a second panel.
 if (!workspacePath) return;
 try {
 setUpstream(await invoke<UpstreamStatus>('get_github_sync_status', { workspacePath }));
 } catch {
 setUpstream(null);
 }
 }, [workspacePath]);

 useEffect(() => {
 if (workspacePath) {
 loadRepoSuggestion();
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
 }, [workspacePath, loadGitStatus]);

 // The embedded GitHub tabs send the user back here for anything that writes
 // to the repo. Both halves are inside this panel, so no shell round-trip.
 useEffect(() => {
 const handler = (e: Event) => {
 const next = (e as CustomEvent<unknown>).detail;
 if (next !== 'changes' && next !== 'github') return;
 setLocalView(next);
 onViewChange?.(next);
 };
 window.addEventListener('vibecoder:git-view', handler);
 return () => window.removeEventListener('vibecoder:git-view', handler);
 }, [onViewChange]);

 /** Asks the backend — which walks up to the enclosing repo — rather than
  *  inferring "no repo" from the text of a git error message. */
 const loadRepoSuggestion = async () => {
 if (!workspacePath) return;
 try {
 setRepoSuggestion(await invoke<GitRepoSuggestion>('git_repo_suggestion', { workspacePath }));
 } catch (e) {
 // Leave it null: the panel then falls back to reporting the git error
 // itself rather than claiming anything about version control.
 setRepoSuggestion(null);
 console.error('git_repo_suggestion failed', e);
 }
 };

 const handleInitRepo = async () => {
 if (!workspacePath) return;
 setInitializingRepo(true);
 try {
 const root = await invoke<string>('git_init_repo', { workspacePath });
 toast.success(`Created a git repository at ${root}`);
 await loadRepoSuggestion();
 await loadGitStatus();
 await loadBranches();
 } catch (e) {
 toast.error(`Could not create the repository: ${e}`);
 } finally {
 setInitializingRepo(false);
 }
 };

 const handleDismissRepoSuggestion = async () => {
 if (!workspacePath) return;
 try {
 await invoke('git_dismiss_repo_suggestion', { workspacePath });
 await loadRepoSuggestion();
 } catch (e) {
 toast.error(`Could not save that choice: ${e}`);
 }
 };

 const loadBranches = async () => {
 if (!workspacePath) return;
 try {
 const branchList = await invoke<string[]>('git_list_branches', { path: workspacePath });
 setBranches(branchList);
 } catch (e) {
 // A folder with no repository has no branches, and saying so as an error
 // toast on every open is noise, not information — the panel already
 // explains the situation properly.
 if (repoSuggestion?.in_repo !== false) {
 toast.error(`Failed to load branches: ${e}`);
 }
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
 onCompareFile(file, selectedCommit.hash);
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
 onCompareFile(file);
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
 const profileStr = localStorage.getItem('vibecoder-profile');
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

 /* Review, Changelog and Settings each get a tab of their own rather than a
  * collapsible section stacked under the changes list. As sections they shared
  * one scroll region with everything above them, so opening a review meant
  * scrolling past the working tree to read it and scrolling back to act on it —
  * and the three of them are the parts of this panel that most want the height:
  * a findings list, a generated changelog, and three groups of settings fields.
  *
  * Full-width `flex: 1` buttons stop working at five, so this is the house
  * `panel-tab-bar`. It does not wrap, so it scrolls sideways in a narrow panel
  * instead of squeezing the labels to nothing. */
 const viewSwitch = (
 <div
 role="tablist"
 aria-label="Source Control view"
 className="panel-tab-bar"
 style={{ marginBottom: '12px', overflowX: 'auto' }}
 >
 {([
 ['changes', 'Changes'],
 ['review', 'Review'],
 ['tools', 'Tools'],
 ['github', 'GitHub'],
 ] as const).map(([id, label]) => (
 <button
 key={id}
 role="tab"
 aria-selected={view === id}
 className={`panel-tab ${view === id ? 'active' : ''}`}
 style={{ flexShrink: 0 }}
 onClick={() => setView(id)}
 >
 {label}
 </button>
 ))}
 </div>
 );

 if (view === 'github') {
 return (
 <div className="panel-container" style={{ padding: '12px' }}>
 {viewSwitch}
 <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
 <Suspense fallback={<div style={{ padding: '12px', fontSize: '12px', color: 'var(--text-secondary)' }}>Loading GitHub…</div>}>
 <GitHubComposite workspacePath={workspacePath} provider={selectedProvider} />
 </Suspense>
 </div>
 </div>
 );
 }

 if (!gitStatus) {
 if (gitError) {
 // The backend is the authority on whether a repo encloses this folder;
 // matching on the error text only guesses, and guessed wrong for any
 // subfolder of a checkout. Fall back to the text match only when the
 // suggestion command itself failed.
 const isNotRepo = repoSuggestion
  ? !repoSuggestion.in_repo
  : gitError.toLowerCase().includes('not a git repository') || gitError.toLowerCase().includes('not found');
 return (
  <div className="panel-container" style={{ padding: '12px' }}>
  {viewSwitch}
  <div style={{ padding: '24px 16px', textAlign: 'center', color: 'var(--text-secondary)' }}>
  <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--text-secondary)" }}>{isNotRepo ? <FolderOpen size={28} strokeWidth={1.5} /> : <AlertTriangle size={28} strokeWidth={1.5} />}</div>
  <div style={{ fontSize: "var(--font-size-md)", fontWeight: 500, marginBottom: 6 }}>
   {isNotRepo ? 'No Git Repository' : 'Git Error'}
  </div>
  <div style={{ fontSize: "var(--font-size-base)", lineHeight: 1.6, marginBottom: 12 }}>
   {isNotRepo
    ? 'This folder is not tracked by git, so edits here have no history and no way back.'
    : gitError}
  </div>
  {isNotRepo && repoSuggestion?.blocked_reason && (
   <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', opacity: 0.7 }}>
    {repoSuggestion.blocked_reason}
   </div>
  )}
  {isNotRepo && !repoSuggestion?.blocked_reason && (
   <>
    <div style={{ display: 'flex', gap: 8, justifyContent: 'center', flexWrap: 'wrap' }}>
     <button
      className="panel-btn panel-btn-primary"
      onClick={handleInitRepo}
      disabled={initializingRepo}
     >
      {initializingRepo ? 'Initializing…' : 'Initialize repository'}
     </button>
     {repoSuggestion && !repoSuggestion.declined && (
      <button className="panel-btn" onClick={handleDismissRepoSuggestion}>
       Not now
      </button>
     )}
    </div>
    <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', opacity: 0.7, marginTop: 10 }}>
     {repoSuggestion?.declined
      ? 'You chose not to be asked again for this folder.'
      : <>Creates a local repository. You can publish it to GitHub from the <strong>GitHub</strong> tab of this panel afterwards.</>}
    </div>
   </>
  )}
  </div>
  </div>
 );
 }
 return (
 <div className="panel-container" style={{ padding: '12px' }}>
 {viewSwitch}
 <div className="empty-state">
 <p>Loading git status...</p>
 </div>
 </div>
 );
 }

 const changedFiles = Object.entries(gitStatus.file_statuses);

 /* ── Review / Changelog / Settings tabs ──────────────────────────────────
  * Each fills the panel below the tab bar. They sit after the repo checks
  * above deliberately: with no repository there is nothing to review, no log
  * to summarise and no config to write, so clicking one of these tabs lands on
  * the same "Initialize repository" screen rather than an empty form.
  *
  * Each body is wrapped in its own scroller. Without one they would size the
  * flex column and overflow the panel with no scrollbar — the squeeze that put
  * the three of them in a shared scroll region in the first place. */
 if (view === 'review') {
 return (
 <div className="panel-container" style={{ padding: '12px' }}>
 {viewSwitch}
 <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
 <ReviewPanel
 review={review}
 /* `line` is dropped deliberately: the diff view addresses a file, not a
   * position in one. Forwarding `onCompareFile` bare typechecked only while
   * its second parameter was unused — now that it means a commit, a review
   * finding's line number would have been read as a revision. */
 onOpenFile={onCompareFile ? (path) => onCompareFile(path) : undefined}
 />
 </div>
 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
 }

 /* One tab for everything that is neither the working tree nor a review of it:
  * suggesting a branch name, untangling a merge conflict, generating a
  * changelog, and the repository's own settings.
  *
  * Changelog and Settings had tabs of their own and were folded in here to cut
  * the bar from six tabs to four. They keep the panel's full height — the
  * problem being solved was competing with the changes list for one column,
  * not sharing a column with three short forms — and each section is labelled,
  * because a stack of unlabelled inputs is how the old collapsed layout read. */
 if (view === 'tools') {
 return (
 <div className="panel-container" style={{ padding: '12px' }}>
 {viewSwitch}
 <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 16 }}>

 <section>
 <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-primary)', fontWeight: 600, marginBottom: 6 }}>
 AI Branch Name
 </div>
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
 {suggestingBranch ? '…' : 'Suggest'}
 </button>
 </div>
 {suggestedBranch && (
 <div style={{ marginTop: 5, display: 'flex', alignItems: 'center', gap: 8, background: 'var(--bg-secondary)', padding: '4px 8px', borderRadius: "var(--radius-xs-plus)" }}>
 <code style={{ flex: 1, fontSize: "var(--font-size-sm)", color: 'var(--info-color)' }}>{suggestedBranch}</code>
 <button
 onClick={() => { navigator.clipboard.writeText(suggestedBranch).then(() => toast.success('Copied!')).catch(() => {}); }}
 style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', fontSize: "var(--font-size-xs)" }}
 >
 Copy
 </button>
 </div>
 )}
 </section>

 <section>
 <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-primary)', fontWeight: 600, marginBottom: 6 }}>
 Resolve Merge Conflict
 </div>
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
 {resolvingConflict ? 'Resolving…' : 'AI Resolve'}
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
 </section>

 <section>
 <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-primary)', fontWeight: 600, marginBottom: 6 }}>
 Generate Changelog
 </div>
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
 </section>

 <section>
 <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-primary)', fontWeight: 600, marginBottom: 6 }}>
 Git Settings
 </div>
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
 </section>

 </div>
 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
 }



 return (
 <div className="panel-container" style={{ padding: '12px' }}>
 {viewSwitch}
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

 <div style={{ flexShrink: 0, marginBottom: '12px', display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
 <button className="panel-btn btn-primary" onClick={handlePull} disabled={isLoading} style={{ fontSize: '12px', padding: '4px 8px' }}>
 Pull
 </button>
 <button className="panel-btn btn-primary" onClick={handlePush} disabled={isLoading} style={{ fontSize: '12px', padding: '4px 8px' }}>
 Push
 </button>
 <button className="panel-btn btn-secondary" onClick={handleShowHistory} style={{ fontSize: '12px', padding: '4px 8px' }}>
 History
 </button>
 {upstream?.has_remote && (
 <span
 title={upstream.repo_url ?? undefined}
 style={{ marginLeft: 'auto', alignSelf: 'center', fontSize: '11px', color: 'var(--text-secondary)' }}
 >
 {upstream.ahead != null && upstream.behind != null
 ? `↑ ${upstream.ahead} ↓ ${upstream.behind}`
 : 'no upstream'}
 </span>
 )}
 </div>

 {/* One scroll region for everything below the branch header.
  *
  * Every section used to be a direct flex child of `.panel-container`, and two
  * of them (this one, plus whichever div landed last — see the
  * `div:last-child` rule in App.css) claimed `flex: 1`. Expanding a section
  * then left the column out of room: the changes list, having
  * `overflow-y: auto`, resolved its min-height to 0 and vanished, taking the
  * file you were about to commit off the screen entirely. Sections flow inside
  * a single scroller, so an expanding one pushes rather than evicts.
  *
  * The three biggest of them have since moved to their own tabs, which removes
  * most of the pressure — but not the rule that caused it, so this stays.
  *
  * Block layout, deliberately: as a flex column its own children would shrink
  * to fit instead of overflowing, and the scrollbar would never appear — the
  * same squeeze in a new place. */}
 <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>

 {/* The list scrolls within a bounded height rather than growing without
   * limit: a 200-file working tree must not push Commit off the panel.
   *
   * Raised from 32vh with the move to tabs. The old cap was set when Review,
   * Changelog and Settings all sat below this list; with them gone, the space
   * they were reserving belongs to the list. Commit is still protected,
   * which is the only thing the cap was ever for. */}
 <div style={{ maxHeight: '50vh', overflowY: 'auto', marginBottom: '12px' }}>
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
 <CommitMessage text={selectedCommit.message} fontSize={12} />
 </div>
 <h4 style={{ fontSize: '11px', marginBottom: '8px', color: 'var(--text-secondary)' }}>Files Changed</h4>
 {commitFiles.map(file => (
 <div key={file} style={{ padding: '8px', background: 'var(--bg-secondary)', borderRadius: '4px', marginBottom: '4px', display: 'flex', gap: '8px', alignItems: 'center' }}>
 <MiddleTruncate text={file} style={{ fontSize: '11px', flex: 1, minWidth: 0 }} />
 <button onClick={() => handleCompareCommitFile(file)} style={{ background: 'none', border: 'none', color: 'var(--accent-blue)', cursor: 'pointer', fontSize: '10px', flex: 'none' }}>Diff</button>
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
 <CommitMessage text={commit.message} fontSize={11} />
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
 {/* `minWidth: 0` is what lets this shrink at all: a flex item defaults
     to `min-width: auto` and refuses to go below its content, so a long
     path used to push the status / Diff / discard controls out of the
     row rather than truncating. Middle-truncated because the end of a
     path is what tells two of them apart. */}
 <MiddleTruncate
 text={file}
 style={{ fontSize: '11px', flex: 1, minWidth: 0 }}
 />
 <span
 style={{
 fontSize: '10px',
 fontFamily: 'var(--font-mono, ui-monospace, monospace)',
 color: 'var(--text-secondary)',
 // Fixed width so the Diff and discard controls stay aligned
 // down the column whatever the code is.
 width: '1.1em',
 textAlign: 'center',
 flex: 'none',
 }}
 title={gitStatusLabel(status)}
 aria-label={gitStatusLabel(status)}
 >
 {gitStatusCode(status)}
 </span>
 <button
 onClick={() => handleCompare(file)}
 style={{
 background: 'none',
 border: 'none',
 color: 'var(--text-secondary)', /* Muted color */
 cursor: 'pointer',
 fontSize: '10px',
 padding: '2px 4px',
 // Never give up width: the controls are the column the eye
 // tracks down, and a row whose Diff sits 3px left of the one
 // above reads as misalignment even when nothing is wrong.
 flex: 'none',
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
 style={{ background: 'none', border: 'none', color: 'var(--text-danger)', cursor: 'pointer', padding: '2px 4px', display: 'flex', alignItems: 'center', flex: 'none' }}
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

 {/* ── Review ──
   * Starting a review belongs with the changes being reviewed; the findings
   * do not, so only the control is here and the report lands on the Review
   * tab. Pressing the button switches there, because a run whose output
   * appears on a tab you are not looking at reads as a button that did
   * nothing.
   *
   * Changelog, Settings and the AI git tools moved out wholesale — see the
   * `view` branches above. All of them were collapsible sections competing for
   * this one scroll region. */}
 <div style={{ borderTop: '1px solid var(--border-color)', paddingTop: 8, marginBottom: 10 }}>
 <ReviewControls
 review={review}
 workspacePath={workspacePath}
 onRun={() => setView('review')}
 />
 </div>


 {/* end of the scroll region */}
 </div>

 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
}
