import { useState, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, RefreshCw, File, Folder, ChevronRight, ChevronDown, Shield } from "lucide-react";
import { AIChat } from "./AIChat";
import { useModelRegistry } from "../hooks/useModelRegistry";

// ── Types ─────────────────────────────────────────────────────────────────────

interface FileEntry {
  path: string;
  name: string;
  is_directory: boolean;
  size?: number;
}

interface TreeNode {
  entry: FileEntry;
  children: TreeNode[];
  expanded: boolean;
}

export interface SandboxChatPanelProps {
  provider?: string;
  availableProviders?: string[];
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function buildTree(entries: FileEntry[]): TreeNode[] {
  return entries.map((e) => ({ entry: e, children: [], expanded: false }));
}

function formatFileTree(entries: FileEntry[], sandboxPath: string): string {
  if (!entries.length) return "(empty)";
  const lines = entries
    .slice(0, 80) // limit context size
    .map((e) => `${e.is_directory ? "📁" : "📄"} ${e.path.replace(sandboxPath + "/", "")}`);
  if (entries.length > 80) lines.push(`… and ${entries.length - 80} more`);
  return lines.join("\n");
}

// ── Component ─────────────────────────────────────────────────────────────────

export function SandboxChatPanel({ provider: initialProvider, availableProviders }: SandboxChatPanelProps) {
  const { providers } = useModelRegistry();
  const effectiveProviders = availableProviders ?? providers;
  const [provider, setProvider] = useState(initialProvider ?? effectiveProviders[0] ?? "claude");

  const [sandboxPath, setSandboxPath] = useState<string | null>(null);
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [tree, setTree] = useState<TreeNode[]>([]);
  const [loadingDir, setLoadingDir] = useState(false);
  const [dirError, setDirError] = useState<string | null>(null);
  const [autoApprove, setAutoApprove] = useState(true);
  const [writeLog, setWriteLog] = useState<string[]>([]);

  // ── Directory loading ──────────────────────────────────────────────────────

  const loadDirectory = useCallback(async (path: string) => {
    setLoadingDir(true);
    setDirError(null);
    try {
      const result = await invoke<FileEntry[]>("list_directory_sandbox", { path });
      setEntries(result);
      setTree(buildTree(result));
    } catch (e) {
      setDirError(String(e));
    } finally {
      setLoadingDir(false);
    }
  }, []);

  const handlePickFolder = async () => {
    const selected = await open({ directory: true, multiple: false, title: "Select Sandbox Folder" });
    if (typeof selected === "string") {
      setSandboxPath(selected);
      setWriteLog([]);
      loadDirectory(selected);
    }
  };

  const handleRefresh = () => { if (sandboxPath) loadDirectory(sandboxPath); };

  // ── Tree expand/collapse ───────────────────────────────────────────────────

  const toggleNode = useCallback(async (node: TreeNode) => {
    if (!node.entry.is_directory) return;
    const nowExpanded = !node.expanded;
    if (nowExpanded && node.children.length === 0) {
      try {
        const children = await invoke<FileEntry[]>("list_directory_sandbox", { path: node.entry.path });
        node.children = buildTree(children);
      } catch {
        // ignore errors for subdirs
      }
    }
    node.expanded = nowExpanded;
    setTree((prev) => [...prev]); // force re-render
  }, []);

  // ── onPendingWrite — auto-approve for sandbox paths ───────────────────────

  const handlePendingWrite = useCallback(async (path: string, content: string) => {
    if (!sandboxPath) return;
    const isInsideSandbox = path.startsWith(sandboxPath) || !path.includes("/");
    const resolvedPath = path.includes("/") ? path : `${sandboxPath}/${path}`;

    if (autoApprove && isInsideSandbox) {
      try {
        await invoke("write_file_sandbox", { path: resolvedPath, content });
        setWriteLog((prev) => [...prev.slice(-19), resolvedPath]);
        if (sandboxPath) loadDirectory(sandboxPath);
      } catch (e) {
        console.error("Sandbox write failed:", e);
      }
    } else {
      // Outside sandbox or auto-approve off — show a note
      console.warn("[SandboxChat] Write outside sandbox ignored:", path);
    }
  }, [sandboxPath, autoApprove, loadDirectory]);

  // ── pinnedMemory — sandbox system prompt ──────────────────────────────────

  const pinnedMemory = useMemo(() => {
    if (!sandboxPath) return undefined;
    const fileList = formatFileTree(entries, sandboxPath);
    return [
      `## Sandbox Access`,
      `You have full read/write permissions inside the sandbox folder: **${sandboxPath}**`,
      ``,
      `**Capabilities:**`,
      `- Read any file: use \`<read_file path="${sandboxPath}/filename" />\``,
      `- Write/create files: use \`<write_file path="${sandboxPath}/filename">content</write_file>\``,
      `- All writes are applied automatically — no confirmation needed`,
      `- Use relative filenames (e.g. \`app.py\`) or full paths`,
      ``,
      `**Current sandbox contents:**`,
      fileList,
    ].join("\n");
  }, [sandboxPath, entries]);

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
          <button onClick={handleRefresh} title="Refresh file tree" disabled={loadingDir} style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 2 }}>
            <RefreshCw size={13} style={{ animation: loadingDir ? "spin 1s linear infinite" : "none" }} />
          </button>
        )}
      </div>

      {/* Body: split between file tree and chat */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {/* File tree (only when sandbox is set) */}
        {sandboxPath && (
          <div style={{ width: 200, minWidth: 160, maxWidth: 260, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", overflow: "hidden" }}>
            <div style={{ padding: "6px 10px", fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
              FILES
            </div>
            <div style={{ flex: 1, overflowY: "auto", padding: "4px 0" }}>
              {dirError && (
                <div style={{ padding: "8px 10px", fontSize: "var(--font-size-sm)", color: "var(--error-color)" }}>{dirError}</div>
              )}
              {tree.map((node) => (
                <TreeNodeRow key={node.entry.path} node={node} depth={0} onToggle={toggleNode} />
              ))}
            </div>
            {writeLog.length > 0 && (
              <div style={{ borderTop: "1px solid var(--border-color)", padding: "6px 10px", flexShrink: 0 }}>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginBottom: 4 }}>RECENT WRITES</div>
                {writeLog.slice(-4).map((p, i) => (
                  <div key={i} style={{ fontSize: "var(--font-size-xs)", color: "var(--success-color)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={p}>
                    ✓ {p.split("/").pop()}
                  </div>
                ))}
              </div>
            )}
            {/* Auto-approve toggle */}
            <div style={{ borderTop: "1px solid var(--border-color)", padding: "6px 10px", flexShrink: 0, display: "flex", alignItems: "center", gap: 6 }}>
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
              <button onClick={handlePickFolder} style={{ background: "var(--accent)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm-alt)", padding: "8px 16px", cursor: "pointer", fontSize: "var(--font-size-md)", display: "flex", alignItems: "center", gap: 6 }}>
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
            />
          )}
        </div>
      </div>
    </div>
  );
}

// ── Tree node row ─────────────────────────────────────────────────────────────

function TreeNodeRow({ node, depth, onToggle }: { node: TreeNode; depth: number; onToggle: (n: TreeNode) => void }) {
  return (
    <>
      <div
        onClick={() => onToggle(node)}
        style={{ display: "flex", alignItems: "center", gap: 4, padding: "2px 8px", paddingLeft: 8 + depth * 14, cursor: node.entry.is_directory ? "pointer" : "default", fontSize: "var(--font-size-base)", color: "var(--text-primary)", userSelect: "none" }}
        className="tree-row-hover"
      >
        {node.entry.is_directory
          ? (node.expanded ? <ChevronDown size={12} style={{ flexShrink: 0 }} /> : <ChevronRight size={12} style={{ flexShrink: 0 }} />)
          : <span style={{ width: 12, flexShrink: 0 }} />}
        {node.entry.is_directory
          ? <Folder size={12} style={{ color: "#f97b22", flexShrink: 0 }} />
          : <File size={12} style={{ color: "var(--text-muted)", flexShrink: 0 }} />}
        <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{node.entry.name}</span>
      </div>
      {node.expanded && node.children.map((child) => (
        <TreeNodeRow key={child.entry.path} node={child} depth={depth + 1} onToggle={onToggle} />
      ))}
    </>
  );
}
