import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ReviewPanel } from './ReviewPanel';
import { useToast } from '../hooks/useToast';
import { Toaster } from './Toaster';

interface GitPanelProps {
    workspacePath: string | null;
    onCompareFile?: (filePath: string, diff: string) => void;
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

export function GitPanel({ workspacePath, onCompareFile }: GitPanelProps) {
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

    useEffect(() => {
        if (workspacePath) {
            loadGitStatus();
            loadBranches();
        }
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
        } catch (e) {
            toast.error(`Failed to load git status: ${e}`);
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
            const msg = await invoke<string>('generate_commit_message');
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
            await invoke('git_commit', {
                path: workspacePath,
                message: commitMessage,
                files: selectedFiles,
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

    const toggleFileSelection = (file: string) => {
        setSelectedFiles(prev =>
            prev.includes(file)
                ? prev.filter(f => f !== file)
                : [...prev, file]
        );
    };

    if (!workspacePath) {
        return (
            <div className="empty-state">
                <p>No workspace folder open</p>
            </div>
        );
    }

    if (!gitStatus) {
        return (
            <div className="empty-state">
                <p>Loading git status...</p>
            </div>
        );
    }

    const changedFiles = Object.entries(gitStatus.file_statuses);

    return (
        <div style={{ padding: '10px', display: 'flex', flexDirection: 'column', height: '100%' }}>
            <div style={{ marginBottom: '10px', display: 'flex', alignItems: 'center', gap: '8px' }}>
                <strong>Branch:</strong>
                <select
                    value={gitStatus.branch}
                    onChange={(e) => handleSwitchBranch(e.target.value)}
                    disabled={isLoading}
                    style={{
                        flex: 1,
                        padding: '4px 8px',
                        background: 'var(--bg-tertiary)',
                        border: '1px solid var(--border-color)',
                        color: 'var(--text-primary)',
                        borderRadius: '4px',
                        fontSize: '12px',
                    }}
                >
                    {branches.map(branch => (
                        <option key={branch} value={branch}>{branch}</option>
                    ))}
                </select>
            </div>

            <div style={{ marginBottom: '10px', display: 'flex', gap: '5px', flexWrap: 'wrap' }}>
                <button className="btn-primary" onClick={handlePull} disabled={isLoading} style={{ fontSize: '12px', padding: '4px 8px' }}>
                    Pull
                </button>
                <button className="btn-primary" onClick={handlePush} disabled={isLoading} style={{ fontSize: '12px', padding: '4px 8px' }}>
                    Push
                </button>
                <button className="btn-secondary" onClick={handleShowHistory} style={{ fontSize: '12px', padding: '4px 8px' }}>
                    History
                </button>
            </div>

            <div style={{ flex: 1, overflowY: 'auto', marginBottom: '10px' }}>
                {showHistory ? (
                    <div>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                            <h3 style={{ fontSize: '13px' }}>Commit History</h3>
                            <button onClick={() => setShowHistory(false)} style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', fontSize: '16px' }}>×</button>
                        </div>
                        {selectedCommit ? (
                            <div>
                                <button onClick={() => setSelectedCommit(null)} style={{ background: 'none', border: 'none', color: 'var(--accent-blue)', cursor: 'pointer', fontSize: '11px', marginBottom: '8px' }}>← Back to commits</button>
                                <div style={{ padding: '8px', background: 'var(--bg-tertiary)', borderRadius: '4px', marginBottom: '8px' }}>
                                    <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>{selectedCommit.hash.substring(0, 7)} • {selectedCommit.author}</div>
                                    <div style={{ fontSize: '12px', marginTop: '4px' }}>{selectedCommit.message}</div>
                                </div>
                                <h4 style={{ fontSize: '11px', marginBottom: '6px', color: 'var(--text-secondary)' }}>Files Changed</h4>
                                {commitFiles.map(file => (
                                    <div key={file} style={{ padding: '6px', background: 'var(--bg-secondary)', borderRadius: '4px', marginBottom: '4px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                                        <span style={{ fontSize: '11px' }}>{file}</span>
                                        <button onClick={() => handleCompareCommitFile(file)} style={{ background: 'none', border: 'none', color: 'var(--accent-blue)', cursor: 'pointer', fontSize: '10px' }}>Diff</button>
                                    </div>
                                ))}
                            </div>
                        ) : (
                            history.map(commit => (
                                <div
                                    key={commit.hash}
                                    onClick={() => handleSelectCommit(commit)}
                                    style={{
                                        padding: '8px',
                                        marginBottom: '6px',
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
                        <h3 style={{ fontSize: '13px', marginBottom: '8px' }}>Changes</h3>
                        {changedFiles.length === 0 ? (
                            <p style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>No changes</p>
                        ) : (
                            changedFiles.map(([file, status]) => (
                                <div
                                    key={file}
                                    style={{
                                        padding: '6px',
                                        background: selectedFiles.includes(file) ? 'var(--bg-tertiary)' : 'transparent',
                                        borderRadius: '4px',
                                        marginBottom: '4px',
                                        display: 'flex',
                                        alignItems: 'center',
                                        gap: '6px',
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
                                            <span style={{ fontSize: '10px', color: 'var(--text-danger, #ff4d4f)' }}>Discard?</span>
                                            <button
                                                onClick={() => handleDiscardChanges(file)}
                                                style={{ background: 'none', border: 'none', color: 'var(--text-danger, #ff4d4f)', cursor: 'pointer', fontSize: '10px', padding: '2px 4px', fontWeight: 600 }}
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
                                            style={{ background: 'none', border: 'none', color: 'var(--text-danger, #ff4d4f)', cursor: 'pointer', fontSize: '10px', padding: '2px 4px' }}
                                            title="Discard changes"
                                        >
                                            ✕
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
                        style={{
                            width: '100%',
                            minHeight: '50px',
                            padding: '6px',
                            paddingRight: '60px',
                            background: 'var(--bg-tertiary)',
                            border: '1px solid var(--border-color)',
                            color: 'var(--text-primary)',
                            borderRadius: '4px',
                            marginBottom: '6px',
                            fontFamily: 'inherit',
                            fontSize: '12px',
                            boxSizing: 'border-box',
                        }}
                    />
                    <button
                        onClick={handleGenerateMsg}
                        disabled={generatingMsg}
                        title="Generate commit message with AI"
                        style={{
                            position: 'absolute', top: '4px', right: '4px',
                            padding: '2px 7px', fontSize: '10px', fontWeight: 600,
                            background: generatingMsg ? 'var(--bg-secondary)' : 'rgba(99,102,241,0.2)',
                            color: generatingMsg ? 'var(--text-secondary)' : '#a5b4fc',
                            border: '1px solid rgba(99,102,241,0.4)', borderRadius: '3px',
                            cursor: generatingMsg ? 'not-allowed' : 'pointer',
                        }}
                    >
                        {generatingMsg ? '…' : '✨ AI'}
                    </button>
                </div>
                <button
                    className="btn-primary"
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
                        width: '100%', textAlign: 'left', padding: '6px 8px',
                        background: showReview ? 'var(--bg-tertiary)' : 'transparent',
                        border: 'none', borderRadius: 4, cursor: 'pointer',
                        color: 'var(--text-primary)', fontSize: 12,
                        display: 'flex', alignItems: 'center', gap: 6,
                    }}
                >
                    <span>{showReview ? '▼' : '▶'}</span>
                    <span>🔍 Code Review</span>
                </button>
                {showReview && (
                    <div style={{ marginTop: 8, height: 420, borderRadius: 6, overflow: 'hidden', background: 'var(--bg-secondary)' }}>
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
            <Toaster toasts={toasts} onDismiss={dismiss} />
        </div>
    );
}
