import { useState, useCallback, useMemo, useEffect, Fragment } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FolderOpen, RefreshCw, File, Folder, ChevronRight, ChevronDown, ChevronUp,
  Shield, FilePlus, FolderPlus, X, Check,
} from "lucide-react";
import { AIChat } from "./AIChat";
import Modal from "./Modal";
import { useToast } from "../hooks/useToast";
import { useModelRegistry, getDefaultProvider } from "../hooks/useModelRegistry";

// ── Types ─────────────────────────────────────────────────────────────────────

interface FileEntry {
  path: string;
  name: string;
  is_directory: boolean;
  size?: number;
}

interface GitStatus {
  file_statuses: Record<string, string>;
}

interface PendingWrite {
  id: number;
  path: string;
  content: string;
  lines: number;
}

interface PreviewState {
  path: string;
  name: string;
  content: string;
}

interface ContextMenuState {
  x: number;
  y: number;
  entry: FileEntry; // null entries are not represented; use null state to hide
}

export interface SandboxChatPanelProps {
  provider?: string;
  availableProviders?: string[];
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatFileTree(entries: FileEntry[], sandboxPath: string): string {
  if (!entries.length) return "(empty)";
  const lines = entries
    .slice(0, 80) // limit context size
    .map((e) => `${e.is_directory ? "📁" : "📄"} ${e.path.replace(sandboxPath + "/", "")}`);
  if (entries.length > 80) lines.push(`… and ${entries.length - 80} more`);
  return lines.join("\n");
}

// Pure builder for the sandbox system prompt. Exported for unit tests.
// The wording here is load-bearing: weak local models will produce
// analysis-only responses ("here are the issues I'd fix…") unless told
// explicitly that action via the tags is mandatory and that an agent
// loop will continue the turn after each tool call.
export function buildSandboxSystemPrompt(
  sandboxPath: string,
  entries: FileEntry[],
): string {
  const fileList = formatFileTree(entries, sandboxPath);
  return [
    `## Sandbox Access (active)`,
    `Folder: ${sandboxPath}`,
    ``,
    `You are an autonomous coding agent with full read/write access to this folder.`,
    ``,
    `RULES:`,
    `- Tasks like "add tests", "fix bug", "refactor", "increase coverage" require ACTION, not analysis.`,
    `  Use the tags below to read and write files. Do NOT respond with a summary, plan, or list of`,
    `  recommendations describing what you would do — DO it.`,
    `- Always read the relevant files before writing. Do not invent file contents.`,
    `- After tool tags execute, you will be re-invoked with the output. Continue the task across`,
    `  turns until done. End with a brief summary only after all writes are complete.`,
    ``,
    `TAGS (the only way to take action):`,
    `- <read_file path="${sandboxPath}/relative/path" />`,
    `- <write_file path="${sandboxPath}/relative/path">…full file contents…</write_file>`,
    `- Relative filenames also work (e.g. \`app.py\`); writes are applied automatically.`,
    ``,
    `**Current sandbox contents:**`,
    fileList,
  ].join("\n");
}

const sepOf = (p: string) => (p.includes("\\") ? "\\" : "/");
const joinPath = (dir: string, name: string) => {
  const sep = sepOf(dir);
  return (dir.endsWith(sep) ? dir : dir + sep) + name;
};
const parentOf = (p: string) => {
  const sep = sepOf(p);
  const i = p.lastIndexOf(sep);
  return i <= 0 ? sep : p.substring(0, i);
};

// ── Component ─────────────────────────────────────────────────────────────────

export function SandboxChatPanel({ provider: initialProvider, availableProviders }: SandboxChatPanelProps) {
  const { toast } = useToast();
  const { providers } = useModelRegistry();
  const effectiveProviders = availableProviders ?? providers;
  const [provider, setProvider] = useState(initialProvider ?? effectiveProviders[0] ?? getDefaultProvider());

  useEffect(() => {
    if (initialProvider) setProvider(initialProvider);
  }, [initialProvider]);

  // ── Core sandbox state ────────────────────────────────────────────────────
  const [sandboxPath, setSandboxPath] = useState<string | null>(null);
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loadingDir, setLoadingDir] = useState(false);
  const [dirError, setDirError] = useState<string | null>(null);
  const [autoApprove, setAutoApprove] = useState(true);
  const [writeLog, setWriteLog] = useState<string[]>([]);

  // ── Tree explorer state (VS Code-style) ──────────────────────────────────
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());
  const [dirContents, setDirContents] = useState<Map<string, FileEntry[]>>(new Map());

  // ── Side surfaces ─────────────────────────────────────────────────────────
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [preview, setPreview] = useState<PreviewState | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [pendingWrites, setPendingWrites] = useState<PendingWrite[]>([]);
  const [pendingTagInput, setPendingTagInput] = useState<string | undefined>(undefined);
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
  const [inputModal, setInputModal] = useState<{
    title: string;
    placeholder: string;
    initialValue?: string;
    onConfirm: (value: string) => void;
  } | null>(null);

  // ── Stable sandbox session ID for Watch sync ──────────────────────────────
  // Derive a deterministic session ID from the sandbox path using FNV-1a hash,
  // so messages are persisted to sessions.db and visible on the Watch.
  const sandboxSessionId = useMemo<string | undefined>(() => {
    if (!sandboxPath) return undefined;
    let h = 0x811c9dc5;
    for (let i = 0; i < sandboxPath.length; i++) {
      h ^= sandboxPath.charCodeAt(i);
      h = Math.imul(h, 0x01000193) >>> 0;
    }
    return `sbx-${h.toString(16).padStart(8, "0")}`;
  }, [sandboxPath]);

  // Notify daemon so Watch can navigate to this sandbox session
  useEffect(() => {
    invoke("watch_set_sandbox_chat_session", { sessionId: sandboxSessionId ?? null }).catch(() => {});
    return () => {
      // Clear sandbox session when panel unmounts
      invoke("watch_set_sandbox_chat_session", { sessionId: null }).catch(() => {});
    };
  }, [sandboxSessionId]);

  // ── Directory loading ─────────────────────────────────────────────────────

  const fetchGitFor = useCallback((path: string) => {
    invoke<GitStatus>("get_git_status_for_path", { path })
      .then(setGitStatus)
      .catch(() => setGitStatus(null));
  }, []);

  const loadRoot = useCallback(async (path: string) => {
    setLoadingDir(true);
    setDirError(null);
    try {
      const result = await invoke<FileEntry[]>("list_directory_sandbox", { path });
      setEntries(result);
      setDirContents(new Map([[path, result]]));
      setExpandedDirs(new Set([path]));
      fetchGitFor(path);
    } catch (e) {
      setDirError(String(e));
    } finally {
      setLoadingDir(false);
    }
  }, [fetchGitFor]);

  const ensureDirContents = useCallback(
    async (path: string): Promise<FileEntry[] | null> => {
      const cached = dirContents.get(path);
      if (cached) return cached;
      try {
        const result = await invoke<FileEntry[]>("list_directory_sandbox", { path });
        setDirContents(prev => {
          const next = new Map(prev);
          next.set(path, result);
          return next;
        });
        return result;
      } catch (e) {
        console.error("Failed to list sandbox dir:", e);
        return null;
      }
    },
    [dirContents],
  );

  const refreshDir = useCallback(async (path: string) => {
    try {
      const result = await invoke<FileEntry[]>("list_directory_sandbox", { path });
      setDirContents(prev => {
        const next = new Map(prev);
        next.set(path, result);
        return next;
      });
      if (path === sandboxPath) setEntries(result);
    } catch (e) {
      console.error("Failed to refresh:", e);
    }
    if (sandboxPath) fetchGitFor(sandboxPath);
  }, [sandboxPath, fetchGitFor]);

  const toggleDir = useCallback(async (path: string) => {
    if (expandedDirs.has(path)) {
      setExpandedDirs(prev => {
        const next = new Set(prev);
        next.delete(path);
        return next;
      });
      return;
    }
    if (!dirContents.has(path)) {
      const loaded = await ensureDirContents(path);
      if (loaded === null) {
        toast.error("Failed to open directory");
        return;
      }
    }
    setExpandedDirs(prev => new Set(prev).add(path));
  }, [expandedDirs, dirContents, ensureDirContents, toast]);

  const collapseAll = useCallback(() => {
    setExpandedDirs(sandboxPath ? new Set([sandboxPath]) : new Set());
  }, [sandboxPath]);

  // Walk from a file up to the sandbox root, lazy-load each segment, and
  // mark them all expanded so the file row is visible in the tree.
  const revealFile = useCallback(async (filePath: string) => {
    if (!sandboxPath) return;
    const sep = sepOf(sandboxPath);
    const root = sandboxPath.endsWith(sep) ? sandboxPath.slice(0, -1) : sandboxPath;
    if (!filePath.startsWith(root + sep)) return;
    const relative = filePath.slice(root.length + 1);
    const parts = relative.split(sep);
    parts.pop(); // drop filename
    const chain: string[] = [root];
    let cur = root;
    for (const part of parts) {
      cur = cur + sep + part;
      chain.push(cur);
    }
    for (const dir of chain) await ensureDirContents(dir);
    setExpandedDirs(prev => {
      const next = new Set(prev);
      for (const dir of chain) next.add(dir);
      return next;
    });
  }, [sandboxPath, ensureDirContents]);

  // ── Toolbar handlers ─────────────────────────────────────────────────────

  const handlePickFolder = async () => {
    const selected = await open({ directory: true, multiple: false, title: "Select Sandbox Folder" });
    if (typeof selected === "string") {
      setSandboxPath(selected);
      setWriteLog([]);
      setPendingWrites([]);
      loadRoot(selected);
    }
  };

  const handleRefresh = () => {
    if (sandboxPath) loadRoot(sandboxPath);
  };

  // ── Mutations (modal-driven for inputs, browser confirm for delete) ──────

  const handleNewFile = (parentDir?: string) => {
    const target = parentDir ?? sandboxPath;
    if (!target) {
      toast.warn("Pick a sandbox folder first");
      return;
    }
    setInputModal({
      title: `New File in ${target.split(/[/\\]/).filter(Boolean).pop() || target}`,
      placeholder: "filename.ext",
      onConfirm: async (name) => {
        setInputModal(null);
        if (!name) return;
        const path = joinPath(target, name);
        try {
          await invoke("write_file_sandbox", { path, content: "" });
          await refreshDir(target);
          setExpandedDirs(prev => new Set(prev).add(target));
          // Auto-preview the newly-created (empty) file so the user can start typing
          // immediately, matching VS Code behavior.
          setPreview({ path, name, content: "" });
        } catch (e) {
          toast.error(`Create failed: ${e}`);
        }
      },
    });
  };

  const handleNewFolder = (parentDir?: string) => {
    const target = parentDir ?? sandboxPath;
    if (!target) {
      toast.warn("Pick a sandbox folder first");
      return;
    }
    setInputModal({
      title: `New Folder in ${target.split(/[/\\]/).filter(Boolean).pop() || target}`,
      placeholder: "folder-name",
      onConfirm: async (name) => {
        setInputModal(null);
        if (!name) return;
        const path = joinPath(target, name);
        try {
          await invoke("create_directory_sandbox", { path });
          await refreshDir(target);
          setExpandedDirs(prev => new Set(prev).add(target));
        } catch (e) {
          toast.error(`Create failed: ${e}`);
        }
      },
    });
  };

  const handleRename = (entry: FileEntry) => {
    setContextMenu(null);
    setInputModal({
      title: `Rename ${entry.name}`,
      placeholder: entry.name,
      initialValue: entry.name,
      onConfirm: async (newName) => {
        setInputModal(null);
        if (!newName || newName === entry.name) return;
        try {
          await invoke("rename_path_sandbox", { path: entry.path, newName });
          await refreshDir(parentOf(entry.path));
        } catch (e) {
          toast.error(`Rename failed: ${e}`);
        }
      },
    });
  };

  const handleDelete = (entry: FileEntry) => {
    setContextMenu(null);
    if (!confirm(`Delete ${entry.name}? This cannot be undone.`)) return;
    (async () => {
      try {
        await invoke("delete_path_sandbox", { path: entry.path });
        await refreshDir(parentOf(entry.path));
        setDirContents(prev => {
          const next = new Map(prev);
          next.delete(entry.path);
          return next;
        });
        setExpandedDirs(prev => {
          if (!prev.has(entry.path)) return prev;
          const next = new Set(prev);
          next.delete(entry.path);
          return next;
        });
        if (preview?.path === entry.path) setPreview(null);
      } catch (e) {
        toast.error(`Delete failed: ${e}`);
      }
    })();
  };

  const handleCopyPath = (entry: FileEntry) => {
    setContextMenu(null);
    navigator.clipboard.writeText(entry.path).then(
      () => toast.info("Path copied to clipboard"),
      () => toast.error("Failed to copy path"),
    );
  };

  // ── File click: preview (left) or insert tag (shift+left) ────────────────

  const handleEntryClick = async (e: React.MouseEvent, entry: FileEntry) => {
    if (entry.is_directory) {
      toggleDir(entry.path);
      return;
    }
    if (e.shiftKey) {
      setPendingTagInput(`<read_file path="${entry.path}"/>`);
      return;
    }
    setPreview({ path: entry.path, name: entry.name, content: "" });
    setPreviewLoading(true);
    try {
      const content = await invoke<string>("read_file_sandbox", { path: entry.path });
      setPreview({ path: entry.path, name: entry.name, content });
    } catch (err) {
      setPreview({ path: entry.path, name: entry.name, content: `Failed to read file:\n${err}` });
    } finally {
      setPreviewLoading(false);
    }
  };

  // ── Pending writes (queue when auto-approve is off) ──────────────────────

  const handlePendingWrite = useCallback(async (path: string, content: string) => {
    if (!sandboxPath) return;
    const sep = sepOf(sandboxPath);
    const resolvedPath = path.includes("/") || path.includes("\\")
      ? path
      : `${sandboxPath.endsWith(sep) ? sandboxPath.slice(0, -1) : sandboxPath}${sep}${path}`;
    const isInsideSandbox = resolvedPath.startsWith(sandboxPath);
    if (!isInsideSandbox) {
      toast.warn(`Write outside sandbox ignored: ${path}`);
      return;
    }

    if (autoApprove) {
      try {
        await invoke("write_file_sandbox", { path: resolvedPath, content });
        setWriteLog(prev => [...prev.slice(-19), resolvedPath]);
        await refreshDir(parentOf(resolvedPath));
        revealFile(resolvedPath);
      } catch (e) {
        toast.error(`Sandbox write failed: ${e}`);
      }
      return;
    }

    // Queue for approval
    setPendingWrites(prev => [
      ...prev,
      {
        id: Date.now() + Math.random(),
        path: resolvedPath,
        content,
        lines: content === "" ? 0 : content.split("\n").length,
      },
    ]);
  }, [sandboxPath, autoApprove, toast, refreshDir, revealFile]);

  const approveWrite = useCallback(async (id: number) => {
    const pending = pendingWrites.find(p => p.id === id);
    if (!pending) return;
    try {
      await invoke("write_file_sandbox", { path: pending.path, content: pending.content });
      setPendingWrites(prev => prev.filter(p => p.id !== id));
      setWriteLog(prev => [...prev.slice(-19), pending.path]);
      await refreshDir(parentOf(pending.path));
      revealFile(pending.path);
    } catch (e) {
      toast.error(`Write failed: ${e}`);
    }
  }, [pendingWrites, refreshDir, revealFile, toast]);

  const rejectWrite = useCallback((id: number) => {
    setPendingWrites(prev => prev.filter(p => p.id !== id));
  }, []);

  const approveAllWrites = useCallback(async () => {
    // Snapshot ids before iterating because each call mutates the list.
    const ids = pendingWrites.map(p => p.id);
    for (const id of ids) await approveWrite(id);
  }, [pendingWrites, approveWrite]);

  // ── pinnedMemory — sandbox system prompt ──────────────────────────────────

  const pinnedMemory = useMemo(() => {
    if (!sandboxPath) return undefined;
    return buildSandboxSystemPrompt(sandboxPath, entries);
  }, [sandboxPath, entries]);

  // ── Git status lookup for a single row ───────────────────────────────────

  const gitStatusFor = useCallback((entryPath: string): string | undefined => {
    if (!gitStatus) return undefined;
    const found = Object.entries(gitStatus.file_statuses).find(([p]) => entryPath.endsWith(p));
    if (!found) return undefined;
    const status = found[1];
    return status.charAt(0); // M / A / D / ?
  }, [gitStatus]);

  // ── Dismiss context menu on outside click ────────────────────────────────
  useEffect(() => {
    if (!contextMenu) return;
    const handler = () => setContextMenu(null);
    window.addEventListener("click", handler);
    return () => window.removeEventListener("click", handler);
  }, [contextMenu]);

  // ── Recursive tree renderer ──────────────────────────────────────────────

  const renderRow = (entry: FileEntry, depth: number) => {
    const isExpanded = entry.is_directory && expandedDirs.has(entry.path);
    const isActivePreview = preview?.path === entry.path;
    const children = isExpanded ? dirContents.get(entry.path) : undefined;
    const gitChar = gitStatusFor(entry.path);
    return (
      <Fragment key={entry.path}>
        <div
          role="button"
          tabIndex={0}
          onClick={(e) => handleEntryClick(e, entry)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleEntryClick(e as unknown as React.MouseEvent, entry);
          }}
          onContextMenu={(e) => {
            e.preventDefault();
            setContextMenu({ x: e.clientX, y: e.clientY, entry });
          }}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            padding: "2px 8px",
            paddingLeft: 8 + depth * 14,
            cursor: "pointer",
            fontSize: "var(--font-size-base)",
            color: "var(--text-primary)",
            userSelect: "none",
            background: isActivePreview ? "var(--bg-elevated)" : "transparent",
            boxShadow: isActivePreview ? "inset 2px 0 0 var(--accent-primary, #2196f3)" : "none",
          }}
          className="tree-row-hover"
          title={entry.path}
        >
          {entry.is_directory
            ? (isExpanded
                ? <ChevronDown size={12} style={{ flexShrink: 0 }} />
                : <ChevronRight size={12} style={{ flexShrink: 0 }} />)
            : <span style={{ width: 12, flexShrink: 0 }} />}
          {entry.is_directory
            ? <Folder size={12} style={{ color: "var(--warning-color)", flexShrink: 0 }} />
            : <File size={12} style={{ color: "var(--text-muted)", flexShrink: 0 }} />}
          <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
            {entry.name}
          </span>
          {gitChar && (
            <span style={{ fontSize: 10, color: "var(--git-modified, #e2c08d)", flexShrink: 0 }}>
              {gitChar}
            </span>
          )}
        </div>
        {children && children.map(child => renderRow(child, depth + 1))}
      </Fragment>
    );
  };

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="panel-container">
      {/* Toolbar */}
      <div className="panel-header">
        <Shield size={14} style={{ color: sandboxPath ? "var(--success-color)" : "var(--text-muted)" }} />
        <span style={{ fontSize: "var(--font-size-base)", fontWeight: 500, color: "var(--text-primary)" }}>Sandbox</span>
        {sandboxPath ? (
          <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={sandboxPath}>
            {sandboxPath}
          </span>
        ) : (
          <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", flex: 1 }}>No folder selected</span>
        )}
        <button onClick={handlePickFolder} title="Pick sandbox folder" className="panel-btn panel-btn-secondary panel-btn-xs" style={{ display: "flex", alignItems: "center", gap: 4, whiteSpace: "nowrap" }}>
          <FolderOpen size={12} /> {sandboxPath ? "Change" : "Open Folder"}
        </button>
        {sandboxPath && (
          <>
            <button className="panel-btn" onClick={() => handleNewFile()} title="New File at sandbox root" style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 2 }}>
              <FilePlus size={13} />
            </button>
            <button className="panel-btn" onClick={() => handleNewFolder()} title="New Folder at sandbox root" style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 2 }}>
              <FolderPlus size={13} />
            </button>
            <button className="panel-btn" onClick={collapseAll} title="Collapse All" style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 2 }}>
              <ChevronUp size={13} />
            </button>
            <button className="panel-btn" onClick={handleRefresh} title="Refresh file tree" disabled={loadingDir} style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 2 }}>
              <RefreshCw size={13} style={{ animation: loadingDir ? "spin 1s linear infinite" : "none" }} />
            </button>
          </>
        )}
      </div>

      {/* Body: split between file tree and chat */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {/* File tree (only when sandbox is set) */}
        {sandboxPath && (
          <div style={{ width: 200, minWidth: 160, maxWidth: 260, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", overflow: "hidden" }}>
            <div style={{ padding: "8px 12px", fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
              FILES
            </div>
            <div style={{ flex: 1, overflowY: "auto", padding: "4px 0" }}>
              {dirError && (
                <div style={{ padding: "8px 12px", fontSize: "var(--font-size-sm)", color: "var(--error-color)" }}>{dirError}</div>
              )}
              {entries.map(entry => renderRow(entry, 0))}
            </div>

            {/* Pending writes drawer (only visible when auto-approve is off and queue is non-empty) */}
            {pendingWrites.length > 0 && (
              <div style={{ borderTop: "1px solid var(--border-color)", padding: "8px 12px", flexShrink: 0, maxHeight: 180, overflowY: "auto" }}>
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 6 }}>
                  <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", fontWeight: 600 }}>
                    PENDING WRITES ({pendingWrites.length})
                  </div>
                  <button
                    onClick={approveAllWrites}
                    title="Approve all pending writes"
                    style={{ background: "var(--success-color, #4caf50)", color: "#fff", border: "none", borderRadius: 3, padding: "2px 6px", fontSize: "var(--font-size-xs)", cursor: "pointer" }}
                  >
                    Approve all
                  </button>
                </div>
                {pendingWrites.map((p) => {
                  const filename = p.path.split(/[/\\]/).pop() || p.path;
                  return (
                    <div key={p.id} style={{ display: "flex", alignItems: "center", gap: 4, padding: "3px 0" }}>
                      <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-primary)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={p.path}>
                        {filename} <span style={{ color: "var(--text-muted)" }}>({p.lines} lines)</span>
                      </span>
                      <button
                        onClick={() => approveWrite(p.id)}
                        title="Approve this write"
                        style={{ background: "none", border: "none", cursor: "pointer", color: "var(--success-color, #4caf50)", padding: 2, display: "flex" }}
                      >
                        <Check size={12} />
                      </button>
                      <button
                        onClick={() => rejectWrite(p.id)}
                        title="Reject this write"
                        style={{ background: "none", border: "none", cursor: "pointer", color: "var(--error-color, #f44336)", padding: 2, display: "flex" }}
                      >
                        <X size={12} />
                      </button>
                    </div>
                  );
                })}
              </div>
            )}

            {writeLog.length > 0 && (
              <div style={{ borderTop: "1px solid var(--border-color)", padding: "8px 12px", flexShrink: 0 }}>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginBottom: 4 }}>RECENT WRITES</div>
                {writeLog.slice(-4).map((p, i) => (
                  <div key={i} style={{ fontSize: "var(--font-size-xs)", color: "var(--success-color)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={p}>
                    ✓ {p.split(/[/\\]/).pop()}
                  </div>
                ))}
              </div>
            )}
            {/* Auto-approve toggle */}
            <div style={{ borderTop: "1px solid var(--border-color)", padding: "8px 12px", flexShrink: 0, display: "flex", alignItems: "center", gap: 6 }}>
              <input type="checkbox" id="sandbox-auto-approve" checked={autoApprove} onChange={(e) => setAutoApprove(e.target.checked)} style={{ cursor: "pointer" }} />
              <label htmlFor="sandbox-auto-approve" style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", cursor: "pointer" }}>
                Auto-write files
              </label>
            </div>
          </div>
        )}

        {/* Chat */}
        <div style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }}>
          {!sandboxPath ? (
            <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 12, padding: 24, textAlign: "center" }}>
              <Shield size={32} style={{ color: "var(--text-muted)", opacity: 0.5 }} />
              <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 500 }}>No sandbox folder selected</div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", maxWidth: 280 }}>
                Pick a folder to give the AI full read/write access. It can create, edit, and delete files freely within that folder.
              </div>
              <button className="panel-btn" onClick={handlePickFolder} style={{ background: "var(--accent)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm-alt)", padding: "8px 16px", cursor: "pointer", fontSize: "var(--font-size-md)", display: "flex", alignItems: "center", gap: 6 }}>
                <FolderOpen size={14} /> Open Sandbox Folder
              </button>
            </div>
          ) : (
            <AIChat
              provider={provider}
              availableProviders={effectiveProviders}
              onProviderChange={setProvider}
              pinnedMemory={pinnedMemory}
              context={`Sandbox: ${sandboxPath}`}
              onPendingWrite={handlePendingWrite}
              pendingInput={pendingTagInput}
              onPendingInputConsumed={() => setPendingTagInput(undefined)}
              sessionId={sandboxSessionId}
              sessionTitle={sandboxPath ? `Sandbox: ${sandboxPath.split("/").pop()}` : "Sandbox"}
            />
          )}
        </div>
      </div>

      {/* File preview modal (read-only) */}
      {preview && (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="sandbox-preview-title"
          onClick={() => setPreview(null)}
          onKeyDown={(e) => { if (e.key === "Escape") setPreview(null); }}
          style={{
            position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)",
            display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1500,
          }}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{
              background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
              borderRadius: 6, width: "min(900px, 90vw)", height: "min(600px, 80vh)",
              display: "flex", flexDirection: "column", overflow: "hidden",
            }}
          >
            <div style={{
              display: "flex", alignItems: "center", gap: 8, padding: "10px 14px",
              borderBottom: "1px solid var(--border-color)", flexShrink: 0,
            }}>
              <File size={14} style={{ color: "var(--text-muted)" }} />
              <span id="sandbox-preview-title" style={{ fontSize: "var(--font-size-base)", fontWeight: 500, color: "var(--text-primary)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={preview.path}>
                {preview.name}
              </span>
              <button
                onClick={() => setPreview(null)}
                aria-label="Close preview"
                style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 4, display: "flex" }}
              >
                <X size={16} />
              </button>
            </div>
            <pre style={{
              flex: 1, margin: 0, padding: 14, overflow: "auto",
              fontFamily: "var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace)",
              fontSize: 12, lineHeight: 1.5, color: "var(--text-primary)",
              background: "var(--bg-primary)", whiteSpace: "pre", tabSize: 4,
            }}>
              {previewLoading ? "Loading…" : preview.content}
            </pre>
            <div style={{
              padding: "6px 14px", borderTop: "1px solid var(--border-color)",
              fontSize: "var(--font-size-xs)", color: "var(--text-muted)", flexShrink: 0,
            }}>
              Read-only preview · Shift+click any file row to insert <code>&lt;read_file&gt;</code> into the chat instead.
            </div>
          </div>
        </div>
      )}

      {/* Context menu */}
      {contextMenu && (
        <div
          onClick={(e) => e.stopPropagation()}
          style={{
            position: "fixed", top: contextMenu.y, left: contextMenu.x,
            background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
            borderRadius: 4, padding: "4px 0", zIndex: 1000,
            boxShadow: "0 2px 5px rgba(0,0,0,0.2)", minWidth: 160,
          }}
        >
          {contextMenu.entry.is_directory && (
            <>
              <CtxItem label="New File" onClick={() => handleNewFile(contextMenu.entry.path)} />
              <CtxItem label="New Folder" onClick={() => handleNewFolder(contextMenu.entry.path)} />
              <CtxSep />
            </>
          )}
          <CtxItem label="Copy Path" onClick={() => handleCopyPath(contextMenu.entry)} />
          {/* Sandbox root can't be renamed or deleted from the explorer. */}
          {contextMenu.entry.path !== sandboxPath && (
            <>
              <CtxItem label="Rename" onClick={() => handleRename(contextMenu.entry)} />
              <CtxItem label="Delete" danger onClick={() => handleDelete(contextMenu.entry)} />
            </>
          )}
        </div>
      )}

      {/* Shared input modal */}
      <Modal
        isOpen={!!inputModal}
        title={inputModal?.title ?? ""}
        placeholder={inputModal?.placeholder}
        initialValue={inputModal?.initialValue}
        onConfirm={(v) => inputModal?.onConfirm(v)}
        onCancel={() => setInputModal(null)}
      />
    </div>
  );
}

// ── Small helpers ─────────────────────────────────────────────────────────────

function CtxItem({ label, onClick, danger }: { label: string; onClick: () => void; danger?: boolean }) {
  return (
    <div
      role="menuitem"
      onClick={(e) => { e.stopPropagation(); onClick(); }}
      onMouseEnter={(e) => (e.currentTarget.style.background = "var(--bg-tertiary)")}
      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
      style={{
        padding: "8px 12px",
        cursor: "pointer",
        fontSize: 13,
        color: danger ? "var(--text-danger, #ff4d4f)" : "var(--text-primary)",
      }}
    >
      {label}
    </div>
  );
}

function CtxSep() {
  return <div style={{ height: 1, background: "var(--border-color)", margin: "4px 0" }} />;
}
