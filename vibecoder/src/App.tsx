import React, { useState, useEffect, useRef, useCallback } from "react";
import { useToast } from "./hooks/useToast";
import { useNotifications } from "./hooks/useNotifications";
import { useApiKeyMonitor } from "./hooks/useApiKeyMonitor";
import { useDaemonMonitor } from "./hooks/useDaemonMonitor";
import { probeAndCacheDefaultProvider, PROVIDER_DEFAULT_MODEL } from "./hooks/useModelRegistry";
import { registerGhostText, type GhostTextHandle } from "./lib/ghostText";
import { Toaster } from "./components/Toaster";
import { NotificationCenter } from "./components/NotificationCenter";
import Editor, { DiffEditor, OnMount } from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { DiffCompleteModal } from "./components/DiffCompleteModal";
import { Terminal } from "./components/Terminal";
import { BrowserPanel } from "./components/BrowserPanel";
import { detectLanguage, getFileIcon } from "./utils/fileUtils";
import { createLspBridge, fileUri, type LspBridge, type LspLanguageSupport } from "./lib/lsp";
import { LspStatus } from "./components/LspStatus";
import { EFFORT_LEVELS, type EffortLevel, getSelectedEffort, setSelectedEffort, effortLabel } from "./utils/effort";
import { ImageViewer, isImageFile } from "./components/ImageViewer";
import { DocumentViewer, isDocumentFile } from "./components/DocumentViewer";
import "./App.css";
import { ThemeToggle } from "./components/ThemeToggle";
import { CommandPalette, Command } from "./components/CommandPalette";
import Modal from "./components/Modal";
import { GitPanel } from "./components/GitPanel";
import { MarkdownPreview } from "./components/MarkdownPreview";
import { HtmlPreview } from "./components/HtmlPreview";
import { DrawioPreview } from "./components/DrawioPreview";
import { Icon } from "./components/Icon";
import "./ActivityBar.css";
import { ExtensionManager } from "./extensions/ExtensionManager";
// Import worker using Vite's syntax
import ExtensionHostWorker from "./extensions/ExtensionHost?worker";
import { useCollab } from "./hooks/useCollab";
import { flowContext } from "./utils/FlowContext";
import { OnboardingTour } from "./components/OnboardingTour";
import { GroupedTabBar } from "./components/GroupedTabBar";
import { MenuBar, MenuGroup } from "./components/MenuBar";
import "./components/GroupedTabBar.css";
import { PanelHost } from "./components/LazyPanels";
import { useEditorTheme } from "./hooks/useEditorTheme";
import { SettingsPanel } from "./components/SettingsPanel";
import { TaintedConfirmationModal } from "./components/TaintedConfirmationModal";
import { ALL_TABS } from "./constants/tabGroups";
import { globalShortcuts, editorShortcuts, renderShortcut } from "./constants/shortcuts";
import { TAB_META, DEFAULT_TAB_META } from "./constants/tabMeta";

interface FileEntry {
  path: string;
  name: string;
  is_directory: boolean;
  size?: number;
}

interface SearchResult {
  path: string;
  line_number: number;
  line_content: string;
}

interface GitStatus {
  branch: string;
  file_statuses: Record<string, string>; // path -> status
}

/* Monaco structural types. The LSP wire types and every conversion between
   them live in `lib/lsp.ts`; App only drives the document lifecycle. */
type MonacoEditor = Parameters<OnMount>[0];

interface OpenFile {
  path: string;
  content: string;
  language: string;
  isDirty: boolean;
  /** When true, the file is an image and content is base64-encoded binary data */
  isImage?: boolean;
  /** When true, the file is a document (PDF/EPUB) and content is base64-encoded binary data */
  isDocument?: boolean;
  /** Base64-encoded binary data for images and documents */
  base64Data?: string;
}

/** Drives ⌘ vs Ctrl in the shortcut labels. Module scope: it cannot change
 *  during a session, so recomputing it per render bought nothing. */
const isMacPlatform = /Mac/.test(navigator.userAgent);

function App() {
  const { toasts, toast, dismiss } = useToast();
  const { notifications, unreadCount, add: addNotification, markRead, markAllRead, dismiss: dismissNotification } = useNotifications();
  useApiKeyMonitor({ toast, addNotification, osNotifications: true });
  useDaemonMonitor({ toast, addNotification });
  const { themeName: editorTheme, defineTheme: defineEditorTheme } = useEditorTheme();
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const [workspaceFolders, setWorkspaceFolders] = useState<string[]>([]);
  const [recentWorkspaces, setRecentWorkspaces] = useState<string[]>([]);
  const [files, setFiles] = useState<FileEntry[]>([]);
  // VS Code-style tree explorer: which dirs are expanded, and a per-dir
  // children cache so we can render the tree without re-fetching.
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());
  const [dirContents, setDirContents] = useState<Map<string, FileEntry[]>>(new Map());
  const [aiProviders, setAiProviders] = useState<string[]>([]);
  const [selectedProvider, setSelectedProvider] = useState<string>("");
  // Per-request reasoning/compute effort tier (gap C5). Persisted via the
  // effort util so any LLM-calling panel can read the current selection.
  const [selectedEffort, setSelectedEffortState] = useState<EffortLevel>(() => getSelectedEffort());
  const [showSidebar, setShowSidebar] = useState(true);
  const [activeSidebarTab, setActiveSidebarTab] = useState<"explorer" | "search" | "git" | "testing" | "project" | "infra" | "ai" | "security">("explorer");
  const [showAIChat, setShowAIChat] = useState(false);
  const [aiPanelTab, setAiPanelTab] = useState("chat");
  // /goal slash command — seed forwarded into the Goals panel's New Goal modal.
  const [newGoalSeed, setNewGoalSeed] = useState<string | null>(null);
  const [panelsMaximized, setPanelsMaximized] = useState(false);
  const [showEditorArea, setShowEditorArea] = useState(true);
  const [showFilterBar, setShowFilterBar] = useState(true);
  const [showTerminal, setShowTerminal] = useState(false);
  const [bottomTab, setBottomTab] = useState<"terminal" | "browser">("terminal");
  /**
   * Where the Terminal/Browser panel sits: the full-width strip along the
   * bottom, or the centre, filling the editor's region.
   *
   * Persisted — a dock position the user has chosen is a workspace preference,
   * and having it snap back to the bottom on every launch would make the
   * feature not worth using.
   */
  const [panelDock, setPanelDock] = useState<"bottom" | "center">(
    () => (localStorage.getItem("vibecoder-panel-dock") === "center" ? "center" : "bottom"),
  );
  const [showCommandPalette, setShowCommandPalette] = useState(false);
  const [showTour, setShowTour] = useState(() => !localStorage.getItem('vibecoder-onboarding-complete'));
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [appVersion, setAppVersion] = useState("0.0.0");

  const completeTour = useCallback(() => {
    localStorage.setItem('vibecoder-onboarding-complete', 'true');
    setShowTour(false);
  }, []);

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [modalConfig, setModalConfig] = useState<{
    title: string;
    placeholder: string;
    onConfirm: (value: string) => void;
  }>({ title: '', placeholder: '', onConfirm: () => { } });
  const [currentDirectory, setCurrentDirectory] = useState<string | null>(null);
  // Undo strip — shown after an AI write for up to 30 s so the user can revert.
  const [lastApply, setLastApply] = useState<{
    path: string; filename: string; original: string; written: string;
  } | null>(null);
  const lastApplyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Collab (CRDT multiplayer)
  const collab = useCollab();

  // Search state
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  // Git state
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);

  // Context Menu
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; file: FileEntry } | null>(null);
  const [pendingDeleteFile, setPendingDeleteFile] = useState<{ name: string; path: string } | null>(null);

  // Resizable Panes State
  const [sidebarWidth, setSidebarWidth] = useState(250);
  const [terminalHeight, setTerminalHeight] = useState(200);
  const [aiPanelWidth, setAiPanelWidth] = useState(480);
  const [isResizing, setIsResizing] = useState<'sidebar' | 'terminal' | 'aipanel' | null>(null);

  // Preview State
  const [showMarkdownPreview, setShowMarkdownPreview] = useState(false);
  const [showHtmlPreview, setShowHtmlPreview] = useState(false);
  const [showSvgPreview, setShowSvgPreview] = useState(false);
  const [showDrawioPreview, setShowDrawioPreview] = useState(false);

  // Git Diff View State
  const [gitDiffView, setGitDiffView] = useState<{ file: string; original: string; modified: string } | null>(null);

  // Extension Manager
  const extensionManagerRef = useRef<ExtensionManager | null>(null);

  // Ref so editor-mount callbacks always see the current provider
  const selectedProviderRef = useRef<string>(selectedProvider);
  useEffect(() => {
    selectedProviderRef.current = selectedProvider;
  }, [selectedProvider]);

  // The toolbar selects a provider; the model is that provider's registry
  // default, the same resolution every other panel uses. Sent explicitly with
  // each AI-editing request so the backend never has to guess one — and never
  // re-points the shared chat engine to find out.
  const selectedModel = PROVIDER_DEFAULT_MODEL[selectedProvider] ?? "";
  const selectedModelRef = useRef<string>(selectedModel);
  selectedModelRef.current = selectedModel;

  // Listen for file-tree refresh requests from child panels (e.g. Screenshot to App)
  useEffect(() => {
    const handler = () => { if (currentDirectory) loadDirectory(currentDirectory); };
    window.addEventListener("vibecoder:refresh-files", handler);
    return () => window.removeEventListener("vibecoder:refresh-files", handler);
  });

  // Derived state for active file
  const activeFile = openFiles.find(f => f.path === activeFilePath);
  const editorContent = activeFile?.content || "";
  const editorLanguage = activeFile?.language || "typescript";
  const currentFile = activeFilePath; // Alias for backward compatibility in some checks

  useEffect(() => {
    // Load available AI providers
    const refreshProviders = (rawProviders: string[]) => {
      // Dedup: provider display names ("Ollama (llama3.2)") are used as React
      // keys in the toolbar/select; duplicates from the backend would warn.
      const providers = [...new Set(rawProviders)];
      setAiProviders(providers);
      if (providers.length > 0 && !selectedProvider) {
        const defaultProvider = providers.find(p => p.startsWith("Ollama")) || providers[0];
        setSelectedProvider(defaultProvider);
      }
    };
    invoke<string[]>("get_available_ai_providers")
      .then(refreshProviders)
      .catch(console.error);

    // Probe embedded-daemon reachability and cache for next session's default.
    probeAndCacheDefaultProvider();

    // Listen for provider updates from Settings panel (API key changes)
    const onProvidersUpdated = (e: Event) => {
      const providers = [...new Set((e as CustomEvent<string[]>).detail)];
      setAiProviders(providers);
      // If current selection is no longer valid, pick the first available
      if (providers.length > 0 && !providers.includes(selectedProvider)) {
        setSelectedProvider(providers[0]);
      }
    };
    window.addEventListener("vibecoder:providers-updated", onProvidersUpdated);

    // Load workspace folders
    invoke<string[]>("get_workspace_folders")
      .then(setWorkspaceFolders)
      .catch(console.error);

    // Load recent workspaces — surfaced in the empty-state so users
    // can re-open without re-picking via the system folder dialog.
    invoke<string[]>("list_recent_workspaces")
      .then(setRecentWorkspaces)
      .catch(() => { /* file may not exist on first launch */ });

    // Load app version from Tauri
    getVersion().then(setAppVersion).catch(() => {});

    // Initialize Extension Manager
    const manager = new ExtensionManager({
      showInformationMessage: (message) => {
        window.lastExtensionMessage = message;
      },
      showErrorMessage: (message) => {
        console.error(`[Extension Error] ${message}`);
        window.lastExtensionMessage = `Error: ${message}`;
      },
    });

    try {
      const worker = new ExtensionHostWorker();
      manager.setWorker(worker);
      extensionManagerRef.current = manager;
      window.extensionManager = manager;
      // Extension Manager initialized
    } catch (e) {
      toast.error(`Failed to initialize extension worker: ${e}`);
    }

    return () => {
      window.removeEventListener("vibecoder:providers-updated", onProvidersUpdated);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Global keyboard shortcuts
  useEffect(() => {
    const AI_TABS = ALL_TABS.slice(0, 9);
    const handleKeyDown = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      // Cmd+K — command palette
      if (mod && e.key === 'k') {
        e.preventDefault();
        setShowCommandPalette(true);
      }
      // Cmd+B — toggle sidebar
      if (mod && e.key === 'b') {
        e.preventDefault();
        setShowSidebar(prev => !prev);
      }
      // Cmd+J — toggle AI panel
      if (mod && !e.shiftKey && e.key === 'j') {
        e.preventDefault();
        setShowAIChat(prev => !prev);
      }
      // Cmd+` — toggle terminal
      if (mod && e.key === '`') {
        e.preventDefault();
        setShowTerminal(prev => !prev);
      }
      // Cmd+Shift+P — command palette (VS Code alias)
      if (mod && e.shiftKey && e.key === 'P') {
        e.preventDefault();
        setShowCommandPalette(true);
      }
      // Cmd+P — command palette. Both the docs site and this app's README have
      // advertised this since before it existed, so to anyone following either
      // it read as a shortcut that broke rather than one never implemented.
      if (mod && !e.shiftKey && e.key === 'p') {
        e.preventDefault();
        setShowCommandPalette(true);
      }
      // Cmd+1..9 — switch AI tab
      if (mod && !e.shiftKey && e.key >= '1' && e.key <= '9') {
        const idx = parseInt(e.key) - 1;
        if (idx < AI_TABS.length) {
          e.preventDefault();
          setShowAIChat(true);
          setAiPanelTab(AI_TABS[idx]);
        }
      }
      // Cmd+Shift+E — focus explorer
      if (mod && e.shiftKey && e.key === 'E') {
        e.preventDefault();
        setActiveSidebarTab('explorer');
        setShowSidebar(true);
      }
      // Cmd+Shift+G — focus git
      if (mod && e.shiftKey && e.key === 'G') {
        e.preventDefault();
        setActiveSidebarTab('git');
        setShowSidebar(true);
      }
      // Cmd+O — open folder
      if (mod && !e.shiftKey && e.key === 'o') {
        e.preventDefault();
        openFolder();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Open a known folder path — shared between the system picker
   * (openFolder) and the recents list. Returns true on success. */
  const openWorkspacePath = async (path: string): Promise<boolean> => {
    try {
      await invoke("set_workspace_folder", { path });
      setWorkspaceFolders([path]);
      setOpenFiles([]);
      setActiveFilePath(null);
      // Results are paths inside the folder we just left — clicking one would
      // reopen a file that is no longer in scope.
      setSearchQuery("");
      setSearchResults([]);
      loadDirectory(path);
      localStorage.setItem("vibecoder_workspace", path);
      window.dispatchEvent(new CustomEvent("vibecoder:workspace-changed", { detail: path }));
      // Refresh recents so the just-opened path bubbles to the top.
      invoke<string[]>("list_recent_workspaces")
        .then(setRecentWorkspaces)
        .catch(() => {});
      return true;
    } catch (error) {
      toast.error(`Failed to open folder: ${error}`);
      return false;
    }
  };

  const openFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Open Folder",
      });
      if (selected && typeof selected === 'string') {
        await openWorkspacePath(selected);
      }
    } catch (error) {
      console.error("Failed to open folder:", error);
      toast.error(`Failed to open folder: ${error}`);
    }
  };

  const removeRecentWorkspace = async (path: string) => {
    try {
      await invoke("remove_recent_workspace", { path });
      setRecentWorkspaces(prev => prev.filter(p => p !== path));
    } catch (e) {
      toast.error(`Failed to remove from recents: ${e}`);
    }
  };

  const loadDirectory = async (path: string) => {
    try {
      const entries = await invoke<FileEntry[]>("list_directory", { path });
      setFiles(entries);
      setCurrentDirectory(path);
      // Seed the tree-explorer cache and auto-expand the new root so its
      // children show without an extra click.
      setDirContents(prev => {
        const next = new Map(prev);
        next.set(path, entries);
        return next;
      });
      setExpandedDirs(new Set([path]));
      // Fetch git status when directory loads
      fetchGitStatus();
    } catch (error) {
      console.error("Failed to load directory:", error);
    }
  };

  // Lazy-load a directory's children for the tree explorer. Returns the
  // entries (also writing them into the cache) so callers can chain.
  const ensureDirContents = async (path: string): Promise<FileEntry[] | null> => {
    const cached = dirContents.get(path);
    if (cached) return cached;
    try {
      const entries = await invoke<FileEntry[]>("list_directory", { path });
      setDirContents(prev => {
        const next = new Map(prev);
        next.set(path, entries);
        return next;
      });
      return entries;
    } catch (e) {
      console.error("Failed to load directory:", e);
      return null;
    }
  };

  const toggleDir = async (path: string) => {
    if (expandedDirs.has(path)) {
      setExpandedDirs(prev => {
        const next = new Set(prev);
        next.delete(path);
        return next;
      });
      return;
    }
    const loaded = await ensureDirContents(path);
    if (loaded === null) {
      toast.error("Failed to open directory");
      return;
    }
    setExpandedDirs(prev => {
      const next = new Set(prev);
      next.add(path);
      return next;
    });
  };

  const collapseAll = () => {
    setExpandedDirs(currentDirectory ? new Set([currentDirectory]) : new Set());
  };

  // Auto-reveal: when a file becomes active (from any source — palette,
  // search, git diff), expand the chain of parents so the file is visible
  // in the tree. No-op if the file isn't under the open workspace.
  const revealFile = async (filePath: string) => {
    if (!currentDirectory) return;
    const sep = currentDirectory.includes("\\") ? "\\" : "/";
    const root = currentDirectory.endsWith(sep) ? currentDirectory.slice(0, -1) : currentDirectory;
    if (!filePath.startsWith(root + sep) && filePath !== root) return;
    const relative = filePath.slice(root.length + 1);
    const parts = relative.split(sep);
    parts.pop(); // drop filename
    const chain: string[] = [root];
    let cur = root;
    for (const part of parts) {
      cur = cur + sep + part;
      chain.push(cur);
    }
    // Lazy-load every dir on the chain. ensureDirContents is a no-op when
    // already cached, so this is cheap on a hot path.
    for (const dir of chain) {
      await ensureDirContents(dir);
    }
    setExpandedDirs(prev => {
      const next = new Set(prev);
      for (const dir of chain) next.add(dir);
      return next;
    });
  };

  const fetchGitStatus = async () => {
    try {
      const status = await invoke<GitStatus>("get_git_status");
      setGitStatus(status);
    } catch (_error) {
      // Not a git repo or git not available — expected in some workspaces
      setGitStatus(null);
    }
  };

  const getFileColor = (path: string) => {
    if (!gitStatus) return "var(--text-primary)";
    // Normalize path for comparison (simple check)
    const status = Object.entries(gitStatus.file_statuses).find(([p, _]) => path.endsWith(p));
    if (!status) return "var(--text-primary)";

    switch (status[1]) {
      case "Modified": return "var(--git-modified)";
      case "New": return "var(--git-added)";
      case "Deleted": return "var(--git-deleted)";
      case "Ignored": return "var(--git-ignored)";
      case "Conflicted": return "var(--git-conflicted)";
      default: return "var(--text-primary)";
    }
  };

  const openFile = async (path: string) => {
    // VS Code "reveal in explorer": expand parents in the tree no matter
    // how the open was triggered (palette, search, git diff, recents).
    revealFile(path);
    // Check if already open
    if (openFiles.some(f => f.path === path)) {
      setActiveFilePath(path);
      return;
    }

    try {
      const filename = path.split('/').pop() || path.split('\\').pop() || '';

      // ── Image files → read as base64 binary ────────────────────
      if (isImageFile(filename)) {
        // Always get base64 for raster images
        const base64Data = await invoke<string>("read_file_base64", { path });

        setOpenFiles(prev => [...prev, {
          path,
          content: `[Image: ${filename}]`,
          language: 'plaintext',
          isDirty: false,
          isImage: true,
          base64Data,
        }]);
        setActiveFilePath(path);
        invoke("track_flow_event", { kind: "file_open", data: path }).catch(() => {});
        return;
      }

      // ── Document files (PDF, EPUB) → read as base64 binary ─────
      if (isDocumentFile(filename)) {
        const base64Data = await invoke<string>("read_file_base64", { path });

        setOpenFiles(prev => [...prev, {
          path,
          content: `[Document: ${filename}]`,
          language: 'plaintext',
          isDirty: false,
          isDocument: true,
          base64Data,
        }]);
        setActiveFilePath(path);
        invoke("track_flow_event", { kind: "file_open", data: path }).catch(() => {});
        return;
      }

      // ── Text files → normal Monaco flow ─────────────────────────
      const content = await invoke<string>("read_file", { path });
      const language = detectLanguage(filename);

      setOpenFiles(prev => [...prev, {
        path,
        content,
        language,
        isDirty: false
      }]);
      setActiveFilePath(path);

      // Phase 3: Flow tracking
      invoke("track_flow_event", { kind: "file_open", data: path }).catch(() => {});

      // IntelliSense: register the document with its language server. The
      // bridge starts the server on demand and registers Monaco providers for
      // this language the first time we see it.
      lspBridgeRef.current?.openDocument(path, language, content);
    } catch (error) {
      console.error("Failed to open file:", error);
      toast.error("Failed to open file: " + error);
    }
  };

  const closeFile = (path: string, e?: React.MouseEvent) => {
    e?.stopPropagation(); // Prevent tab selection

    const newOpenFiles = openFiles.filter(f => f.path !== path);
    setOpenFiles(newOpenFiles);

    if (activeFilePath === path) {
      // Switch to the last opened file or null
      const lastFile = newOpenFiles[newOpenFiles.length - 1];
      setActiveFilePath(lastFile ? lastFile.path : null);
    }

    // Release the document on the language server and drop its markers.
    lspBridgeRef.current?.closeDocument(path);
  };

  const saveFile = async () => {
    if (!activeFilePath || !activeFile) return;
    try {
      await invoke("write_file", { path: activeFilePath, content: activeFile.content });

      // Update dirty state
      setOpenFiles(prev => prev.map(f =>
        f.path === activeFilePath ? { ...f, isDirty: false } : f
      ));
      // Some servers only run the full analysis on save (clangd, jdtls).
      lspBridgeRef.current?.saveDocument(activeFilePath);
    } catch (error) {
      console.error("Failed to save file:", error);
      toast.error("Failed to save file: " + error);
    }
    // Refresh git status after save
    fetchGitStatus();
  };

  const handleEditorChange = (value: string | undefined) => {
    if (value !== undefined && activeFilePath) {
      setOpenFiles(prev => prev.map(f =>
        f.path === activeFilePath ? { ...f, content: value, isDirty: true } : f
      ));
      // Push the edit to the language server (debounced inside the bridge, and
      // flushed before any completion request). Without this the server keeps
      // answering against the text as it was when the file was opened, so
      // nothing you just typed can be completed.
      lspBridgeRef.current?.changeDocument(activeFilePath, value);
      // Phase 3: Flow tracking (fire-and-forget)
      invoke("track_flow_event", { kind: "file_edit", data: activeFilePath }).catch(() => {});
    }
  };

  const cursorUpdateTimeoutRef = useRef<number | null>(null);

  // DiffComplete (⌘.) state — explicit-trigger, diff-output AI edit
  const [diffComplete, setDiffComplete] = useState<{
    filePath: string;
    language: string;
    originalContent: string;
    selectionText: string;
    selectionStartLine: number; // 1-based
    selectionEndLine: number;
  } | null>(null);
  const editorRef = useRef<MonacoEditor | null>(null);
  // Ref mirror of currentDirectory for use inside async callbacks that outlive a render
  const currentDirectoryRef = useRef(currentDirectory);
  currentDirectoryRef.current = currentDirectory;

  // ── IntelliSense ──────────────────────────────────────────────────────────
  // The bridge is created once, on editor mount, and lives as long as the
  // editor. Providers registered at mount close over these refs rather than
  // render values: `workspaceFolders` is still empty when the editor first
  // mounts for anyone who opens a file before a folder, and a captured empty
  // root disables IntelliSense for the rest of the session.
  const workspaceFoldersRef = useRef(workspaceFolders);
  workspaceFoldersRef.current = workspaceFolders;
  // `onMount` fires once, so anything it closes over is frozen at the render
  // that first showed the editor. Cursor sync and ⌘. read the active file
  // through this ref, or they keep acting on whichever tab was open first.
  const activeFilePathRef = useRef(activeFilePath);
  activeFilePathRef.current = activeFilePath;
  const lspBridgeRef = useRef<LspBridge | null>(null);
  const lspNoticesRef = useRef(new Set<string>());
  const ghostTextRef = useRef<GhostTextHandle | null>(null);

  /** Tell the user once per language why IntelliSense is quiet. */
  const reportLspUnavailable = useCallback((support: LspLanguageSupport) => {
    if (lspNoticesRef.current.has(support.language)) return;
    lspNoticesRef.current.add(support.language);
    if (support.state === "unconfigured") return; // no server exists; not news
    toast.warn(`No IntelliSense for ${support.language}: ${support.detail}`);
  }, [toast]);

  const monacoRef = useRef<Parameters<OnMount>[1] | null>(null);

  const lspBridge = useCallback((monaco: Parameters<OnMount>[1]): LspBridge => {
    const existing = lspBridgeRef.current;
    if (existing) return existing;
    const bridge = createLspBridge(monaco, {
      invoke: <T,>(command: string, args?: Record<string, unknown>) =>
        invoke<T>(command, args),
      getWorkspaceRoot: () => workspaceFoldersRef.current[0] ?? "",
      onLanguageUnavailable: reportLspUnavailable,
    });
    lspBridgeRef.current = bridge;
    return bridge;
  }, [reportLspUnavailable]);

  useEffect(() => () => {
    lspBridgeRef.current?.dispose();
    lspBridgeRef.current = null;
    // Registered on "*", so leaking it would leave a dead provider consulted
    // on every keystroke for the rest of the process.
    ghostTextRef.current?.dispose();
    ghostTextRef.current = null;
  }, []);

  // Now that each file gets its own Monaco model (see the editor's `path`
  // prop), something has to dispose them: nothing does it when a tab closes,
  // so a long session would hold a tokenized copy of every file ever opened.
  // Reconcile after each change to the tab set, never touching the model the
  // editor is currently showing or the `inmemory://` models other panels own.
  useEffect(() => {
    const monaco = monacoRef.current;
    if (!monaco) return;
    const open = new Set(openFiles.map((file) => fileUri(file.path)));
    const showing = editorRef.current?.getModel()?.uri.toString();
    type Model = { uri: { toString(): string }; dispose(): void };
    (monaco.editor.getModels() as Model[]).forEach((model) => {
      const uri = model.uri.toString();
      if (!uri.startsWith("file://") || uri === showing || open.has(uri)) return;
      model.dispose();
    });
  }, [openFiles]);

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;

    // Register VibeCoder theme with Monaco so the editor matches the app theme
    defineEditorTheme(monaco);

    // Grammars for the languages Monaco lacks are registered in
    // `monaco-setup.ts`, at module load — doing it here would be too late for
    // the first file of a session, whose model is created before `onMount`.

    // ── Cmd+. : DiffComplete (diff-mode AI edit) ──
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.Period,
      () => {
        const model = editor.getModel();
        if (!model) return;
        const selection = editor.getSelection();
        const hasSelection = selection && !selection.isEmpty();
        const selectedText = hasSelection ? model.getValueInRange(selection) : "";
        setDiffComplete({
          filePath: activeFilePathRef.current ?? "",
          language: model.getLanguageId(),
          originalContent: model.getValue(),
          selectionText: selectedText,
          selectionStartLine: hasSelection ? selection.startLineNumber : 0,
          selectionEndLine: hasSelection ? selection.endLineNumber : 0,
        });
      }
    );

    // ── Ctrl+Space (⌥\ on mac too): ghost text ──
    // The provider answers only Monaco's `Explicit` trigger kind, so this
    // chord is the sole path to a suggestion — typing never produces one.
    // See `lib/ghostText.ts` for why that gate is the whole design.
    const ghost = registerGhostText(monaco, {
      invoke: <T,>(command: string, args?: Record<string, unknown>) =>
        invoke<T>(command, args),
      getProvider: () => selectedProviderRef.current,
      getModel: () => selectedModelRef.current,
      getFilePath: () => activeFilePathRef.current ?? "",
      onError: (message) => toast.warn(message),
      // The cap lives in `vibe_ai::ghost`; don't restate the number here, it
      // would go stale silently. The backend reports *that* it clipped.
      onTruncated: () =>
        toast.info("Suggestion was clipped — accept it and re-trigger for more."),
    });
    ghostTextRef.current = ghost;
    editor.addCommand(
      monaco.KeyMod.Alt | monaco.KeyCode.Backslash,
      () => ghost.trigger(editor),
    );

    // IntelliSense: completion, hover, go-to-definition, signature help and
    // diagnostics, all driven from `lib/lsp.ts`. Providers are registered
    // lazily, per language, the first time a file of that language is opened —
    // so we know the server's real trigger characters, and we never register
    // providers for a language with no server installed.
    lspBridge(monaco);

    editor.onDidChangeCursorSelection(() => {
      if (!activeFilePathRef.current) return;

      if (cursorUpdateTimeoutRef.current) {
        window.clearTimeout(cursorUpdateTimeoutRef.current);
      }

      cursorUpdateTimeoutRef.current = window.setTimeout(() => {
        const path = activeFilePathRef.current;
        const selections = editor.getSelections();
        if (!path || !selections) return;

        const cursors = selections.map((sel) => ({
          position: { line: sel.positionLineNumber - 1, column: sel.positionColumn - 1 },
          selection: {
            start: { line: sel.selectionStartLineNumber - 1, column: sel.selectionStartColumn - 1 },
            end: { line: sel.positionLineNumber - 1, column: sel.positionColumn - 1 }
          }
        }));

        invoke("update_cursors", { path, cursors })
          .catch(() => { /* best-effort: cursor sync failures are non-critical */ });
      }, 100); // Debounce 100ms
    });
  };

  const handlePendingWrite = async (path: string, content: string) => {
    // If it's an image/binary, do not attempt to string-diff it. The DiffReviewPanel
    // will crash attempting to layout a 5MB base64 string with break-all.
    if (isImageFile(path)) {
      try {
        await invoke("write_file", { path, content });
        const dir = currentDirectoryRef.current;
        if (dir) loadDirectory(dir);

        const language = detectLanguage(path);
        setOpenFiles((prev) => {
          const exists = prev.some((f) => f.path === path);
          if (exists) return prev.map((f) =>
            f.path === path ? { ...f, content, isDirty: false, isImage: true, base64Data: content } : f
          );
          return [...prev, { path, content, language, isDirty: false, isImage: true, base64Data: content }];
        });
        setActiveFilePath(path);
      } catch (err) {
        console.error("Failed to automatically write image file:", err);
      } finally {
        setTimeout(() => window.dispatchEvent(new Event("vibecoder:diff-resolved")), 100);
      }
      return;
    }

    // AI writes land on disk directly. The accept/reject overlay that used to
    // gate every write is gone: the same information is already in the Source
    // Control diff, and the gate had to be dismissed even when the model
    // proposed content identical to the file, which showed an empty panel over
    // "0/0 hunks" that could do nothing but be cancelled.
    //
    // The undo strip below is the safety net — one click restores `original`.
    try {
      // Read the pre-write content first: once the file is overwritten there is
      // nothing left to undo *to*.
      let original = "";
      try {
        original = await invoke<string>("read_file", { path });
      } catch (_e) {
        // File might not exist yet — a new file, so undo restores emptiness.
      }

      await invoke("write_file", { path, content });

      const dir = currentDirectoryRef.current;
      if (dir) loadDirectory(dir);

      if (lastApplyTimerRef.current) clearTimeout(lastApplyTimerRef.current);
      setLastApply({
        path,
        filename: path.split("/").pop() ?? path,
        original,
        written: content,
      });
      lastApplyTimerRef.current = setTimeout(() => setLastApply(null), 30_000);

      // Keep the open buffer and the language server in step with disk. An
      // AI-applied edit is still an edit: without this, completion, hover and
      // diagnostics keep answering from the pre-write text.
      const language = detectLanguage(path);
      setOpenFiles((prev) => {
        const exists = prev.some((f) => f.path === path);
        if (exists) {
          return prev.map((f) =>
            f.path === path ? { ...f, content, isDirty: false } : f
          );
        }
        return [...prev, { path, content, language, isDirty: false }];
      });
      setActiveFilePath(path);
      lspBridgeRef.current?.openDocument(path, language, content);
    } catch (error) {
      console.error("Failed to write AI edit:", error);
    } finally {
      // AIChat waits on this to release the next queued file — it must fire on
      // every path, including failure, or a failed write stalls the queue.
      window.dispatchEvent(new Event("vibecoder:diff-resolved"));
    }
  };

  // Diff accept/reject is handled inline in the DiffReviewPanel onApply callback.

  // Keyboard shortcut for save (Cmd+S / Ctrl+S)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        saveFile();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentFile, editorContent]);

  // Keyboard shortcut: Cmd/Ctrl+Shift+M to maximize/restore panels, Escape to restore
  useEffect(() => {
    const handleMaximize = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'M') {
        e.preventDefault();
        if (!showAIChat) { setShowAIChat(true); }
        setPanelsMaximized(prev => !prev);
      }
      if (e.key === 'Escape' && panelsMaximized) {
        setPanelsMaximized(false);
      }
    };
    window.addEventListener('keydown', handleMaximize);
    return () => window.removeEventListener('keydown', handleMaximize);
  }, [showAIChat, panelsMaximized]);

  // Refresh a directory's children in the tree cache after a mutation
  // (new file, new folder, delete, rename) so the tree updates in place
  // without collapsing other branches.
  const refreshDir = async (path: string) => {
    try {
      const entries = await invoke<FileEntry[]>("list_directory", { path });
      setDirContents(prev => {
        const next = new Map(prev);
        next.set(path, entries);
        return next;
      });
      // Keep the top-level `files` array in sync when the workspace root
      // is the one being refreshed (covers the toolbar buttons).
      if (path === currentDirectory) setFiles(entries);
    } catch (e) {
      console.error("Failed to refresh directory:", e);
    }
    fetchGitStatus();
  };

  const handleNewFile = (parentDir?: string) => {
    const targetDir = parentDir ?? currentDirectory;
    if (!targetDir) {
      toast.warn("Please open a folder first.");
      return;
    }

    setModalConfig({
      title: 'Create New File',
      placeholder: 'Enter file name (e.g., main.rs)',
      onConfirm: async (name) => {
        setModalOpen(false);
        if (!name) return;

        // Fix path construction to avoid issues
        const separator = targetDir.includes('\\') ? '\\' : '/';
        const cleanDir = targetDir.endsWith(separator) ? targetDir : targetDir + separator;
        const path = cleanDir + name;

        try {
          await invoke("write_file", { path, content: "" });
          await refreshDir(targetDir);
          // Make sure the parent shows the new file
          setExpandedDirs(prev => {
            const next = new Set(prev);
            next.add(targetDir);
            return next;
          });
          // Optionally open the new file
          openFile(path);
        } catch (error) {
          console.error("Failed to create file:", error);
          toast.error("Failed to create file: " + error);
        }
      }
    });
    setModalOpen(true);
  };

  const handleNewFolder = (parentDir?: string) => {
    const targetDir = parentDir ?? currentDirectory;
    if (!targetDir) {
      toast.warn("Please open a folder first.");
      return;
    }

    setModalConfig({
      title: 'Create New Folder',
      placeholder: 'Enter folder name',
      onConfirm: async (name) => {
        setModalOpen(false);
        if (!name) return;

        const separator = targetDir.includes('\\') ? '\\' : '/';
        const cleanDir = targetDir.endsWith(separator) ? targetDir : targetDir + separator;
        const path = cleanDir + name;

        try {
          await invoke("create_directory", { path });
          await refreshDir(targetDir);
          setExpandedDirs(prev => {
            const next = new Set(prev);
            next.add(targetDir);
            return next;
          });
        } catch (error) {
          console.error("Failed to create folder:", error);
          toast.error("Failed to create folder: " + error);
        }
      }
    });
    setModalOpen(true);
  };

  const handleSearch = async () => {
    if (!searchQuery.trim()) return;
    setIsSearching(true);
    setSearchResults([]);
    try {
      const results = await invoke<SearchResult[]>("search_files", {
        query: searchQuery,
        caseSensitive: false
      });
      setSearchResults(results);
    } catch (error) {
      console.error("Search failed:", error);
      toast.error("Search failed: " + error);
    } finally {
      setIsSearching(false);
    }
  };

  const handleSearchResultClick = async (result: SearchResult) => {
    await openFile(result.path);
    // Scroll Monaco editor to the matching line once the file is open
    if (result.line_number && editorRef.current) {
      editorRef.current.revealLineInCenter(result.line_number);
      editorRef.current.setPosition({ lineNumber: result.line_number, column: 1 });
    }
  };

  const handlePanelOpenFile = async (path: string, line?: number) => {
    await openFile(path);
    // Small delay to let Monaco mount/switch to the new file
    setTimeout(() => {
      if (line && editorRef.current) {
        editorRef.current.revealLineInCenter(line);
        editorRef.current.setPosition({ lineNumber: line, column: 1 });
        editorRef.current.focus();
      }
    }, 100);
  };

  // Platform-aware modifier keys for shortcut display
  const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.userAgent);
  const modKey = isMac ? '⌘' : 'Ctrl+';
  const shiftMod = isMac ? '⇧' : 'Shift+';

  // Define commands for command palette
  const commands: Command[] = [
    // File operations
    {
      id: 'file.openFolder',
      label: 'Open Folder',
      category: 'File',
      icon: <Icon name="folder-open" size={16} />,
      shortcut: modKey + 'O',
      action: openFolder,
    },
    {
      id: 'file.save',
      label: 'Save File',
      category: 'File',
      icon: <Icon name="save" size={16} />,
      shortcut: modKey + 'S',
      action: saveFile,
    },
    {
      id: 'file.createFile',
      label: 'Create New File',
      category: 'File',
      icon: <Icon name="file-plus" size={16} />,
      action: handleNewFile,
    },
    {
      id: 'file.createFolder',
      label: 'Create New Folder',
      category: 'File',
      icon: <Icon name="folder-plus" size={16} />,
      action: handleNewFolder,
    },
    // Editor actions
    {
      id: 'editor.ghostText',
      label: 'AI: Inline Completion at Cursor',
      category: 'Editor',
      icon: <Icon name="sparkles" size={16} />,
      shortcut: isMac ? '⌥\\' : 'Alt+\\',
      action: () => {
        const editor = editorRef.current;
        if (!editor) return;
        editor.focus();
        ghostTextRef.current?.trigger(editor);
      },
    },
    {
      id: 'editor.toggleSidebar',
      label: 'Toggle Sidebar',
      category: 'Editor',
      icon: <Icon name="panel-left" size={16} />,
      shortcut: modKey + 'B',
      action: () => setShowSidebar(prev => !prev),
    },
    {
      id: 'editor.toggleAIChat',
      label: 'Toggle AI Chat',
      category: 'Editor',
      icon: <Icon name="message-square" size={16} />,
      shortcut: modKey + 'J',
      action: () => setShowAIChat(prev => !prev),
    },
    {
      id: 'editor.search',
      label: 'Search in Files',
      category: 'Editor',
      icon: <Icon name="search" size={16} />,
      action: () => setActiveSidebarTab('search'),
    },
    // View
    {
      id: 'view.toggleTerminal',
      label: 'Toggle Terminal',
      category: 'View',
      icon: <Icon name="terminal" size={16} />,
      shortcut: modKey + '`',
      action: () => setShowTerminal(prev => !prev),
    },
    {
      id: 'view.explorer',
      label: 'Show Explorer',
      category: 'View',
      icon: <Icon name="folder-open" size={16} />,
      shortcut: modKey + shiftMod + 'E',
      action: () => {
        setShowSidebar(true);
        setActiveSidebarTab('explorer');
      },
    },
    {
      id: 'view.git',
      label: 'Show Source Control',
      category: 'View',
      icon: <Icon name="git-graph" size={16} />,
      shortcut: modKey + shiftMod + 'G',
      action: () => {
        setShowSidebar(true);
        setActiveSidebarTab('git');
      },
    },
    // Debug
    {
      id: 'debug.loadTestExtension',
      label: 'Load Test Extension',
      category: 'Debug',
      icon: <Icon name="puzzle" size={16} />,
      action: () => {
        const code = `
          console.log('Hello from extension!');
          vscode.commands.registerCommand('extension.helloWorld', () => {
            vscode.window.showInformationMessage('Hello World from VibeCoder Extension!');
          });
        `;
        extensionManagerRef.current?.loadExtension(code);
        window.lastExtensionMessage = "Test extension loaded";
      }
    },
    {
      id: 'extension.helloWorld',
      label: 'Hello World (Extension)',
      category: 'Extension',
      icon: <Icon name="hand" size={16} />,
      action: () => {
        extensionManagerRef.current?.executeCommand('extension.helloWorld');
      }
    }
  ];

  // Top menu bar definitions
  const appMenus: MenuGroup[] = [
    {
      label: "File",
      items: [
        { label: "Open Folder...", shortcut: modKey + "O", action: openFolder },
        { label: "New File", action: handleNewFile },
        { label: "New Folder", action: handleNewFolder },
        { separator: true, label: "" },
        { label: "Save", shortcut: modKey + "S", action: saveFile, disabled: !currentFile },
        { separator: true, label: "" },
        { label: "Close File", action: () => { if (activeFilePath) closeFile(activeFilePath); }, disabled: !activeFilePath },
        { label: "Close All Files", action: () => setOpenFiles([]), disabled: openFiles.length === 0 },
      ],
    },
    {
      label: "Edit",
      items: [
        { label: "Undo", shortcut: modKey + "Z", action: () => editorRef.current?.trigger("menu", "undo", null) },
        { label: "Redo", shortcut: modKey + shiftMod + "Z", action: () => editorRef.current?.trigger("menu", "redo", null) },
        { separator: true, label: "" },
        { label: "Cut", shortcut: modKey + "X", action: () => editorRef.current?.trigger("menu", "editor.action.clipboardCutAction", null) },
        { label: "Copy", shortcut: modKey + "C", action: () => editorRef.current?.trigger("menu", "editor.action.clipboardCopyAction", null) },
        { label: "Paste", shortcut: modKey + "V", action: () => editorRef.current?.trigger("menu", "editor.action.clipboardPasteAction", null) },
        { separator: true, label: "" },
        { label: "Find", shortcut: modKey + "F", action: () => editorRef.current?.trigger("menu", "actions.find", null) },
        { label: "Replace", shortcut: modKey + "H", action: () => editorRef.current?.trigger("menu", "editor.action.startFindReplaceAction", null) },
        { separator: true, label: "" },
        { label: "Search in Files", action: () => { setShowSidebar(true); setActiveSidebarTab("search"); } },
      ],
    },
    {
      label: "View",
      items: [
        { label: "Explorer", shortcut: modKey + shiftMod + "E", action: () => { setShowSidebar(true); setActiveSidebarTab("explorer"); } },
        { label: "Source Control", shortcut: modKey + shiftMod + "G", action: () => { setShowSidebar(true); setActiveSidebarTab("git"); } },
        { label: "Search", action: () => { setShowSidebar(true); setActiveSidebarTab("search"); } },
        { separator: true, label: "" },
        { label: showSidebar ? "Hide Sidebar" : "Show Sidebar", shortcut: modKey + "B", action: () => setShowSidebar(prev => !prev) },
        { label: showTerminal ? "Hide Terminal" : "Show Terminal", shortcut: modKey + "`", action: () => setShowTerminal(prev => !prev) },
        { label: showAIChat ? "Hide AI Toolkit" : "Show AI Toolkit", shortcut: modKey + "J", action: () => setShowAIChat(prev => !prev) },
        { label: panelsMaximized ? "Restore Panels" : "Maximize Panels", shortcut: modKey + "⇧M", action: () => { if (!showAIChat) setShowAIChat(true); setPanelsMaximized(prev => !prev); } },
        { separator: true, label: "" },
        { label: "Command Palette...", shortcut: modKey + shiftMod + "P", action: () => setShowCommandPalette(true) },
      ],
    },
    {
      label: "Tools",
      items: [
        { label: "AI Chat", action: () => { setShowAIChat(true); setAiPanelTab("chat"); } },
        { label: "Agent", action: () => { setShowAIChat(true); setAiPanelTab("agent"); } },
        { label: "AI Teams", action: () => { setShowAIChat(true); setAiPanelTab("ai-teams"); } },
        { separator: true, label: "" },
        { label: "Containers", action: () => { setShowAIChat(true); setAiPanelTab("containers"); } },
        { label: "CI/CD", action: () => { setShowAIChat(true); setAiPanelTab("ci-cd"); } },
        { separator: true, label: "" },
        { label: "API Tools", action: () => { setShowAIChat(true); setAiPanelTab("api-tools"); } },
        { label: "Terminal", shortcut: modKey + "`", action: () => setShowTerminal(true) },
        { separator: true, label: "" },
        { label: "Settings", action: () => setShowSettingsModal(true) },
      ],
    },
    {
      label: "Help",
      items: [
        { label: "Welcome Tour", action: () => { localStorage.removeItem("vibecoder-onboarding-complete"); setShowTour(true); } },
        { label: "Command Palette...", shortcut: modKey + shiftMod + "P", action: () => setShowCommandPalette(true) },
        { separator: true, label: "" },
        { label: "Documentation", action: () => window.open("https://github.com/TuringWorks/vibecody", "_blank") },
        { label: "Report Issue", action: () => window.open("https://github.com/TuringWorks/vibecody/issues", "_blank") },
      ],
    },
  ];

  const handleRename = async () => {
    if (!contextMenu) return;
    const file = contextMenu.file;
    setContextMenu(null);

    setModalConfig({
      title: `Rename ${file.name}`,
      placeholder: file.name,
      onConfirm: async (newName) => {
        if (!newName || newName === file.name) return;
        try {
          await invoke('rename_item', { path: file.path, newName });
          // Refresh just the parent folder so the rest of the tree stays
          // expanded as the user left it.
          const sep = file.path.includes("\\") ? "\\" : "/";
          const parent = file.path.substring(0, file.path.lastIndexOf(sep)) || sep;
          await refreshDir(parent);
          // If active file was renamed, we might want to close it or update its path
          // For now, let's just close it to avoid confusion
          if (openFiles.some(f => f.path === file.path)) {
            closeFile(file.path);
          }
        } catch (e) {
          console.error("Failed to rename:", e);
          toast.error(`Failed to rename: ${e}`);
        }
        setModalOpen(false);
      }
    });
    setModalOpen(true);
  };

  const handleDelete = () => {
    if (!contextMenu) return;
    const file = contextMenu.file;
    setContextMenu(null);
    setPendingDeleteFile({ name: file.name, path: file.path });
  };

  const confirmDelete = async () => {
    if (!pendingDeleteFile) return;
    const { path, name } = pendingDeleteFile;
    setPendingDeleteFile(null);
    try {
      await invoke('delete_item', { path });
      const sep = path.includes("\\") ? "\\" : "/";
      const parent = path.substring(0, path.lastIndexOf(sep)) || sep;
      await refreshDir(parent);
      // Drop the deleted entry from caches/expansion so the tree doesn't
      // hold a phantom reference if the user re-expands its parent later.
      setDirContents(prev => {
        const next = new Map(prev);
        next.delete(path);
        return next;
      });
      setExpandedDirs(prev => {
        if (!prev.has(path)) return prev;
        const next = new Set(prev);
        next.delete(path);
        return next;
      });
      if (openFiles.some(f => f.path === path)) {
        closeFile(path);
      }
    } catch (e) {
      toast.error(`Failed to delete ${name}: ${e}`);
    }
  };

  // Close context menu on click elsewhere
  useEffect(() => {
    const handleClick = () => setContextMenu(null);
    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, []);

  // Git Compare Handler
  const handleCompareFile = async (file: string, diff: string) => {
    // Parse diff to get original and modified content
    // For now, we'll need to read the file and reconstruct
    if (!workspaceFolders[0]) return;

    try {
      // Read current file content (modified)
      const modified = await invoke<string>('read_file', { path: `${workspaceFolders[0]}/${file}` });

      // Parse diff to reconstruct original, removing git metadata
      const lines = diff.split('\n');
      const originalLines: string[] = [];

      for (const line of lines) {
        // Skip git metadata lines
        if (line.startsWith('diff --git') ||
          line.startsWith('index ') ||
          line.startsWith('---') ||
          line.startsWith('+++') ||
          line.startsWith('@@')) {
          continue;
        }

        // Process actual diff content
        if (line.startsWith('-')) {
          originalLines.push(line.substring(1));
        } else if (line.startsWith('+')) {
          // Skip added lines in original
          continue;
        } else {
          // Context lines (no prefix or space prefix)
          originalLines.push(line.startsWith(' ') ? line.substring(1) : line);
        }
      }

      const original = originalLines.join('\n');
      setGitDiffView({ file, original, modified });
    } catch (e) {
      console.error('Failed to prepare diff:', e);
      // Fallback: show empty original
      setGitDiffView({ file, original: '', modified: diff });
    }
  };

  // ── Panel docking ─────────────────────────────────────────────────────────
  //
  // Centre-docking positions the panel over the editor's box rather than
  // re-parenting it into the editor subtree. Moving it in the React tree would
  // unmount Terminal and BrowserPanel — dropping the shell session and
  // reloading whatever page was open — so the panel stays exactly where it is
  // and only its geometry changes. Nothing about the move is observable to
  // either child.
  const editorRegionRef = useRef<HTMLElement | null>(null);
  const [centerRect, setCenterRect] = useState<{
    top: number; left: number; width: number; height: number;
  } | null>(null);

  const dockPanel = useCallback((dock: "bottom" | "center") => {
    setPanelDock(dock);
    localStorage.setItem("vibecoder-panel-dock", dock);
  }, []);

  useEffect(() => {
    if (panelDock !== "center" || !showTerminal) {
      setCenterRect(null);
      return;
    }
    const el = editorRegionRef.current;
    const app = el?.closest(".app") as HTMLElement | null;
    if (!el || !app) return;

    const measure = () => {
      const e = el.getBoundingClientRect();
      const a = app.getBoundingClientRect();
      // Relative to `.app`, which is the positioned ancestor.
      setCenterRect({
        top: e.top - a.top,
        left: e.left - a.left,
        width: e.width,
        height: e.height,
      });
    };
    measure();

    // The editor's box moves whenever the sidebar or AI panel is resized or
    // toggled, not just on window resize — observe the element itself.
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    ro.observe(app);
    window.addEventListener("resize", measure);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, [panelDock, showTerminal, sidebarWidth, aiPanelWidth, showEditorArea]);

  // Resize Handlers
  const startResizing = (type: 'sidebar' | 'terminal' | 'aipanel') => {
    setIsResizing(type);
  };

  const stopResizing = () => {
    setIsResizing(null);
  };

  const resize = useCallback((e: MouseEvent) => {
    if (isResizing === 'sidebar') {
      const newWidth = e.clientX - 48; // Subtract activity bar width
      if (newWidth > 150 && newWidth < 600) {
        setSidebarWidth(newWidth);
      }
    } else if (isResizing === 'terminal') {
      const newHeight = window.innerHeight - e.clientY;
      if (newHeight > 100 && newHeight < 600) {
        setTerminalHeight(newHeight);
      }
    } else if (isResizing === 'aipanel') {
      const newWidth = window.innerWidth - e.clientX;
      if (newWidth > 350 && newWidth < 900) {
        setAiPanelWidth(newWidth);
      }
    }
  }, [isResizing]);

  useEffect(() => {
    if (isResizing) {
      window.addEventListener('mousemove', resize);
      window.addEventListener('mouseup', stopResizing);
    } else {
      window.removeEventListener('mousemove', resize);
      window.removeEventListener('mouseup', stopResizing);
    }
    return () => {
      window.removeEventListener('mousemove', resize);
      window.removeEventListener('mouseup', stopResizing);
    };
  }, [isResizing, resize]);

  // Recursive renderer for the VS Code-style tree explorer. Depth controls
  // the indent (workspace root = 0, its children = 1, etc.). Folder click
  // toggles expansion; file click opens it.
  const renderTreeNode = (entry: FileEntry, depth: number): React.ReactNode => {
    const isExpanded = entry.is_directory && expandedDirs.has(entry.path);
    const isActive = !entry.is_directory && activeFilePath === entry.path;
    const children = isExpanded ? dirContents.get(entry.path) : undefined;
    const gitChar = gitStatus
      ? Object.entries(gitStatus.file_statuses).find(([p]) => entry.path.endsWith(p))?.[1].charAt(0)
      : undefined;
    return (
      <React.Fragment key={entry.path}>
        <div
          className={`file-item ${entry.is_directory ? "directory" : "file"}${isActive ? " active" : ""}`}
          role="button"
          tabIndex={0}
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
          title={entry.path}
          onClick={() => entry.is_directory ? toggleDir(entry.path) : openFile(entry.path)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              if (entry.is_directory) toggleDir(entry.path);
              else openFile(entry.path);
            }
          }}
          onContextMenu={(e) => {
            e.preventDefault();
            setContextMenu({ x: e.clientX, y: e.clientY, file: entry });
          }}
        >
          {entry.is_directory ? (
            <Icon name={isExpanded ? "chevron-down" : "chevron-right"} size={12} />
          ) : (
            <span style={{ display: "inline-block", width: 12, flexShrink: 0 }} />
          )}
          <span className="file-icon">{getFileIcon(entry.name, entry.is_directory)}</span>
          <span className="file-name" style={{ color: getFileColor(entry.path) }}>{entry.name}</span>
          {gitChar && (
            <span style={{ marginLeft: "auto", fontSize: "10px", color: getFileColor(entry.path) }}>
              {gitChar}
            </span>
          )}
        </div>
        {children && children.map(child => renderTreeNode(child, depth + 1))}
      </React.Fragment>
    );
  };

  return (
    <div className="app" onMouseUp={stopResizing}>
      <a href="#main-editor" className="skip-to-content">Skip to editor</a>
      <Toaster toasts={toasts} onDismiss={dismiss} />
      {/* Header */}
      <header className="header">
        <div className="header-left">
          <button className="icon-button" onClick={() => setShowSidebar(!showSidebar)} aria-label="Toggle sidebar">
            <Icon name="menu" size={18} />
          </button>
          <h1 className="app-title">VibeCoder</h1>
          <MenuBar menus={appMenus} />
        </div>
        <div className="header-center" />
        <div className="header-right">
          <select
            className="ai-selector"
            value={selectedProvider}
            onChange={(e) => setSelectedProvider(e.target.value)}
          >
            <option value="">Select AI Provider</option>
            {aiProviders.map((provider) => (
              <option key={provider} value={provider}>
                {provider}
              </option>
            ))}
          </select>
          <select
            className="ai-selector"
            value={selectedEffort}
            title={`Reasoning effort: ${effortLabel(selectedEffort)}`}
            onChange={(e) => {
              const lvl = e.target.value as EffortLevel;
              setSelectedEffortState(lvl);
              setSelectedEffort(lvl);
            }}
          >
            {EFFORT_LEVELS.map((lvl) => (
              <option key={lvl} value={lvl} title={effortLabel(lvl)}>
                {lvl === "xhigh" ? "x-high" : lvl}
              </option>
            ))}
          </select>
          <button
            className="btn-secondary"
            onClick={() => { setShowAIChat(!showAIChat); if (!showAIChat) setShowFilterBar(false); }}
            title="Toggle Vibe Toolkit"
          >
            <Icon name="layout-grid" size={14} /> Vibe Toolkit
          </button>
          <button className="btn-primary" onClick={saveFile} disabled={!currentFile}>
            <Icon name="save" size={14} /> Save
          </button>
          {currentFile && currentFile.endsWith('.md') && (
            <button
              className="btn-secondary"
              onClick={() => setShowMarkdownPreview(!showMarkdownPreview)}
            >
              {showMarkdownPreview ? <><Icon name="file-text" size={14} /> Edit</> : <><Icon name="eye" size={14} /> Preview</>}
            </button>
          )}
          {currentFile && (currentFile.endsWith('.html') || currentFile.endsWith('.htm')) && (
            <button
              className="btn-secondary"
              onClick={() => setShowHtmlPreview(!showHtmlPreview)}
            >
              {showHtmlPreview ? <><Icon name="file-code" size={14} /> Edit</> : <><Icon name="globe" size={14} /> Preview</>}
            </button>
          )}
          {currentFile && currentFile.endsWith('.svg') && (
            <button
              className="btn-secondary"
              onClick={() => setShowSvgPreview(!showSvgPreview)}
            >
              {showSvgPreview ? <><Icon name="file-code" size={14} /> Edit</> : <><Icon name="image" size={14} /> Preview</>}
            </button>
          )}
          {currentFile && (currentFile.endsWith('.drawio') || currentFile.endsWith('.dio')) && (
            <button
              className="btn-secondary"
              onClick={() => setShowDrawioPreview(!showDrawioPreview)}
            >
              {showDrawioPreview ? <><Icon name="file-code" size={14} /> Edit</> : <><Icon name="monitor-play" size={14} /> Preview</>}
            </button>
          )}
          <NotificationCenter
            notifications={notifications}
            unreadCount={unreadCount}
            onMarkRead={markRead}
            onMarkAllRead={markAllRead}
            onDismiss={dismissNotification}
          />
        </div>
      </header>

      <div className="main-container">
        {/* Activity Bar */}
        <div className="activity-bar">
          {([
            { id: "explorer" as const, icon: <Icon name="files" size={20} />, title: "Explorer", shortcut: `${modKey}${shiftMod}E` },
            { id: "search" as const, icon: <Icon name="search" size={20} />, title: "Search", shortcut: undefined },
            { id: "git" as const, icon: <Icon name="git-graph" size={20} />, title: "Source Control", shortcut: `${modKey}${shiftMod}G` },
            { id: "testing" as const, icon: <Icon name="test-tube" size={20} />, title: "Testing & Debug", shortcut: undefined },
            { id: "project" as const, icon: <Icon name="clipboard-list" size={20} />, title: "Project", shortcut: undefined },
            { id: "infra" as const, icon: <Icon name="hammer" size={20} />, title: "Build & Infra", shortcut: undefined },
            { id: "ai" as const, icon: <Icon name="bot" size={20} />, title: "AI Toolkit", shortcut: `${modKey}J` },
            { id: "security" as const, icon: <Icon name="shield" size={20} />, title: "Security", shortcut: undefined },
          ]).map(({ id, icon, title, shortcut }) => (
            <button
              key={id}
              className={`activity-bar-item ${activeSidebarTab === id && showSidebar ? 'active' : ''}`}
              onClick={() => {
                if (id === 'ai') {
                  // AI button toggles the right-side AI panel directly
                  setShowAIChat(prev => !prev);
                } else if (activeSidebarTab === id && showSidebar) {
                  setShowSidebar(false);
                } else {
                  setActiveSidebarTab(id);
                  setShowSidebar(true);
                }
              }}
              title={shortcut ? `${title} (${shortcut})` : title}
              aria-label={shortcut ? `${title} (${shortcut})` : title}
            >
              {icon}
            </button>
          ))}
          <div className="activity-bar-spacer" />
          <button className="activity-bar-item" title="Terminal" aria-label={`Terminal (${modKey}\`)`} onClick={() => setShowTerminal(prev => !prev)}>
            <Icon name="terminal" size={20} />
          </button>
          <button className="activity-bar-item" title="Settings" aria-label="Settings" onClick={() => setShowSettingsModal(true)}>
            <Icon name="settings" size={20} />
          </button>
        </div>

        {/* Sidebar */}
        {showSidebar && (
          <aside className="sidebar" style={{ width: `${sidebarWidth}px` }}>
            {/* Removed old tabs */}

            {activeSidebarTab === 'explorer' && (
              <>
                <div className="sidebar-header sidebar-header--compact">
                  <div className="sidebar-actions">
                    <button className="btn-icon" onClick={() => handleNewFile()} title="New File" disabled={!currentDirectory}>
                      <Icon name="file-plus" size={16} />
                    </button>
                    <button className="btn-icon" onClick={() => handleNewFolder()} title="New Folder" disabled={!currentDirectory}>
                      <Icon name="folder-plus" size={16} />
                    </button>
                    <button className="btn-icon" onClick={openFolder} title="Open Folder">
                      <Icon name="folder-open" size={16} />
                    </button>
                    <button className="btn-icon" onClick={() => { if (currentDirectory) refreshDir(currentDirectory); }} title="Refresh" disabled={!currentDirectory}>
                      <Icon name="refresh-cw" size={16} />
                    </button>
                    <button className="btn-icon" onClick={collapseAll} title="Collapse All" disabled={!currentDirectory}>
                      <Icon name="chevron-up" size={16} />
                    </button>
                  </div>
                </div>
                <div className="file-tree">
                  {workspaceFolders.length === 0 ? (
                    <div className="empty-state">
                      <p>No folder opened</p>
                      <button className="btn-secondary" onClick={openFolder} aria-label="Open folder via system picker">
                        Open Folder
                      </button>
                      {recentWorkspaces.length > 0 && (
                        <div role="region" aria-label="Recent workspaces" style={{ marginTop: 12, width: "100%" }}>
                          <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600 }}>
                            Recent
                          </div>
                          {recentWorkspaces.map((p) => {
                            const label = p.split("/").filter(Boolean).pop() || p;
                            return (
                              <div
                                key={p}
                                style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 6px", borderRadius: 4, fontSize: "var(--font-size-sm)" }}
                              >
                                <button
                                  onClick={() => openWorkspacePath(p)}
                                  onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openWorkspacePath(p); } }}
                                  aria-label={`Open recent workspace: ${p}`}
                                  title={p}
                                  style={{ flex: 1, minWidth: 0, textAlign: "left", background: "none", border: "none", color: "var(--text-primary)", cursor: "pointer", padding: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                                >
                                  <span style={{ fontWeight: 600 }}>{label}</span>
                                  <span style={{ color: "var(--text-secondary)", marginLeft: 6, fontSize: "var(--font-size-xs)" }}>{p}</span>
                                </button>
                                <button
                                  onClick={(e) => { e.stopPropagation(); removeRecentWorkspace(p); }}
                                  aria-label={`Remove ${p} from recents`}
                                  title="Remove from recents"
                                  style={{ flexShrink: 0, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", padding: "2px 4px", opacity: 0.6 }}
                                >
                                  ×
                                </button>
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  ) : currentDirectory ? (
                    <div>
                      {/* Workspace root header — click to toggle the whole tree. */}
                      {(() => {
                        const rootName = currentDirectory.split(/[/\\]/).filter(Boolean).pop() || currentDirectory;
                        const isRootExpanded = expandedDirs.has(currentDirectory);
                        return (
                          <div
                            className="file-item workspace-root-header"
                            role="button"
                            tabIndex={0}
                            title={currentDirectory}
                            onClick={() => toggleDir(currentDirectory)}
                            onKeyDown={(e) => { if (e.key === "Enter") toggleDir(currentDirectory); }}
                            onContextMenu={(e) => {
                              e.preventDefault();
                              setContextMenu({
                                x: e.clientX,
                                y: e.clientY,
                                file: { name: rootName, path: currentDirectory, is_directory: true },
                              });
                            }}
                          >
                            <Icon name={isRootExpanded ? "chevron-down" : "chevron-right"} size={12} />
                            <span className="file-name workspace-root-label">{rootName}</span>
                          </div>
                        );
                      })()}
                      {expandedDirs.has(currentDirectory) && files.map(f => renderTreeNode(f, 1))}
                    </div>
                  ) : null}
                </div>
              </>
            )}
            {activeSidebarTab === 'search' && (
              <div className="search-panel" style={{ padding: '8px', display: 'flex', flexDirection: 'column', height: '100%' }}>
                <div className="search-input-container" style={{ display: 'flex', gap: '4px', marginBottom: '8px' }}>
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                    placeholder="Search..."
                    style={{ flex: 1, minWidth: 0, padding: '3px 6px', fontSize: 12 }}
                  />
                  <button onClick={handleSearch} className="btn-primary" disabled={isSearching} style={{ padding: '3px 10px', fontSize: 11, flexShrink: 0 }}>
                    {isSearching ? '...' : 'Go'}
                  </button>
                </div>
                <div className="search-results" style={{ flex: 1, overflowY: 'auto' }}>
                  {searchResults.map((result) => (
                    <div
                      key={`${result.path}:${result.line_number}`}
                      className="search-result-item"
                      role="button"
                      tabIndex={0}
                      onClick={() => handleSearchResultClick(result)}
                      onKeyDown={e => e.key === "Enter" && handleSearchResultClick(result)}
                      style={{ padding: '5px', borderBottom: '1px solid var(--border-color)', cursor: 'pointer' }}
                    >
                      <div style={{ fontSize: '12px', color: 'var(--accent-blue)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {result.path.split('/').pop()} <span style={{ color: 'var(--text-secondary)' }}>:{result.line_number}</span>
                      </div>
                      <div style={{ fontSize: '13px', whiteSpace: 'pre-wrap', fontFamily: 'var(--font-mono)' }}>
                        {result.line_content.trim()}
                      </div>
                    </div>
                  ))}
                  {searchResults.length === 0 && searchQuery && !isSearching && (
                    <div style={{ textAlign: 'center', color: 'var(--text-secondary)', marginTop: '20px' }}>No results found</div>
                  )}
                </div>
              </div>
            )}
            {activeSidebarTab === 'git' && (
              <GitPanel workspacePath={workspaceFolders[0] || null} onCompareFile={handleCompareFile} selectedProvider={selectedProvider} />
            )}

            {activeSidebarTab === 'testing' && (
              <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
                <div className="sidebar-section-title">Testing</div>
                <button className="btn-secondary" style={{ width: "100%", justifyContent: "center", gap: 6, display: "flex", alignItems: "center" }}
                  onClick={() => { setShowAIChat(true); setAiPanelTab("testing"); }}>
                  <Icon name="play" size={14} /> Run Tests
                </button>
                <div style={{ fontSize: 11, color: "var(--text-muted)", lineHeight: 1.5 }}>
                  Run tests, view coverage, and use AI to auto-fix failures.
                </div>
                {([
                  { label: "Test Runner", panel: "testing" },
                  { label: "Coverage Report", panel: "testing" },
                  { label: "BugBot", panel: "testing" },
                  { label: "Autofix", panel: "testing" },
                ] as const).map(({ label, panel }) => (
                  <button key={label} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
                <div className="sidebar-section-title" style={{ marginTop: 8 }}>Debug</div>
                {([
                  { label: "Debug Mode", panel: "system-monitor" },
                  { label: "Profiler", panel: "system-monitor" },
                  { label: "Diagnostics", panel: "diagnostics" },
                  { label: "Git Bisect", panel: "version-control" },
                ] as const).map(({ label, panel }) => (
                  <button key={label} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
              </div>
            )}

            {activeSidebarTab === 'project' && (
              <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
                <div className="sidebar-section-title">Project</div>
                {([
                  { label: "Project Hub", panel: "project-hub" },
                  { label: "Planning & Specs", panel: "planning" },
                  { label: "Code Analysis", panel: "code-analysis" },
                  { label: "Observability", panel: "observability" },
                  { label: "Design", panel: "design" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
                <div className="sidebar-section-title" style={{ marginTop: 8 }}>Extensions</div>
                {([
                  { label: "Marketplace", panel: "marketplace" },
                  { label: "MCP Servers", panel: "integrations" },
                  { label: "Configuration", panel: "config" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
              </div>
            )}

            {activeSidebarTab === 'infra' && (
              <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
                <div className="sidebar-section-title">Build & Deploy</div>
                {([
                  { label: "Build & Deploy", panel: "build-deploy" },
                  { label: "CI/CD Pipelines", panel: "ci-cd" },
                  { label: "GitHub Actions", panel: "github" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
                <div className="sidebar-section-title" style={{ marginTop: 8 }}>Infrastructure</div>
                {([
                  { label: "Containers", panel: "containers" },
                  { label: "Cloud & Platform", panel: "cloud-platform" },
                  { label: "Database", panel: "database" },
                  { label: "API Tools", panel: "api-tools" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
                <div className="sidebar-section-title" style={{ marginTop: 8 }}>Monitor</div>
                {([
                  { label: "System Monitor", panel: "system-monitor" },
                  { label: "Terminal", panel: "terminal" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
                <div className="sidebar-section-title" style={{ marginTop: 8 }}>Devices</div>
                <button className="sidebar-action-item"
                  onClick={() => { setShowAIChat(true); setAiPanelTab("watch"); }}>
                  Watch Devices
                </button>
              </div>
            )}

            {activeSidebarTab === 'security' && (
              <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
                <div className="sidebar-section-title">Security</div>
                {([
                  { label: "Security Scan", panel: "security" },
                  { label: "Code Analysis", panel: "code-analysis" },
                ] as const).map(({ label, panel }) => (
                  <button key={label} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
                <div className="sidebar-section-title" style={{ marginTop: 8 }}>Governance</div>
                {([
                  { label: "Administration", panel: "administration" },
                  { label: "Collaboration", panel: "collaboration" },
                  { label: "Sandbox Chat", panel: "sandbox-chat" },
                  { label: "Watch Devices", panel: "watch" },
                  { label: "Billing & Usage", panel: "billing" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}>
                    {label}
                  </button>
                ))}
              </div>
            )}
          </aside>
        )}

        {/* Vertical Resizer — sidebar ↔ editor */}
        {showSidebar && showEditorArea && (
          <div
            className="resizer-vertical"
            onMouseDown={(e) => {
              e.preventDefault();
              startResizing('sidebar');
            }}
          />
        )}

        {/* Editor Area */}
        <main
          id="main-editor"
          ref={editorRegionRef}
          className={`editor-container${
            showTerminal && panelDock === "center" ? " editor-container--covered" : ""
          }`}
          style={{ display: showEditorArea ? undefined : "none" }}
        >
          {/* Tab Bar */}
          {openFiles.length > 0 && (
            <div className="tab-bar">
              {openFiles.map((file) => (
                <div
                  key={file.path}
                  className={`tab ${activeFilePath === file.path ? "active" : ""}`}
                  onClick={() => setActiveFilePath(file.path)}
                  title={file.path}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    setContextMenu({
                      x: e.clientX,
                      y: e.clientY,
                      file: {
                        path: file.path,
                        name: file.path.split('/').pop() || file.path.split('\\').pop() || file.path,
                        is_directory: false,
                        // Add dummy values for other fields if needed, or update type
                        // FileEntry interface has optional size/modified, so this is fine
                      } as FileEntry
                    });
                  }}
                >
                  <span className="tab-name">
                    {file.path.split('/').pop() || file.path.split('\\').pop()}
                  </span>
                  {file.isDirty && <span className="tab-dirty" style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--accent-color)", flexShrink: 0 }} />}
                  <button
                    className="tab-close"
                    onClick={(e) => closeFile(file.path, e)}
                    style={{ display: "flex", alignItems: "center", justifyContent: "center" }}
                  >
                    <Icon name="x" size={12} />
                  </button>
                </div>
              ))}
            </div>
          )}

          {gitDiffView ? (
            <div className="diff-container" style={{ height: 'calc(100% - 35px)', display: 'flex', flexDirection: 'column' }}>
              <div className="diff-header" style={{ padding: '10px', background: 'var(--bg-secondary)', display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid var(--border-color)' }}>
                <span style={{ fontSize: '13px' }}>Comparing: {gitDiffView.file}</span>
                <button className="btn-secondary" onClick={() => setGitDiffView(null)} style={{ fontSize: '12px', padding: '4px 8px' }}>Close</button>
              </div>
              <DiffEditor
                height="100%"
                language={detectLanguage(gitDiffView.file)}
                theme={editorTheme}
                original={gitDiffView.original}
                modified={gitDiffView.modified}
                options={{
                  readOnly: true,
                  renderSideBySide: false,
                  minimap: { enabled: false },
                  fontSize: 13,
                }}
              />
            </div>
          ) : (
            <>
              {/* Undo strip — shown after Apply for 30 s */}
              {lastApply && (
                <div style={{
                  display: "flex", alignItems: "center", gap: 8,
                  padding: "3px 10px", flexShrink: 0,
                  background: "var(--bg-secondary)",
                  borderBottom: "1px solid var(--border-color)",
                  fontSize: 12,
                }}>
                  <Icon name="check" size={14} style={{ color: "var(--success-color, #4ade80)", flexShrink: 0 }} />
                  <span style={{ color: "var(--text-secondary)" }}>
                    Applied <strong style={{ color: "var(--text-primary)" }}>{lastApply.filename}</strong>
                  </span>
                  <button
                    onClick={() => {
                      const { path, original } = lastApply;
                      if (lastApplyTimerRef.current) clearTimeout(lastApplyTimerRef.current);
                      setLastApply(null);
                      invoke("write_file", { path, content: original })
                        .then(() => { const d = currentDirectoryRef.current; if (d) loadDirectory(d); })
                        .catch((e) => console.error("Undo write failed:", e));
                      setTimeout(() => {
                        try {
                          setOpenFiles((prev) => prev.map((f) => f.path === path ? { ...f, content: original, isDirty: false } : f));
                          setActiveFilePath(path);
                          // The file changed underneath the server; resync or
                          // IntelliSense answers against the undone text.
                          lspBridgeRef.current?.openDocument(path, detectLanguage(path), original);
                        } catch (e) { console.error("Undo state sync failed:", e); }
                      }, 50);
                    }}
                    style={{
                      marginLeft: "auto", padding: "2px 8px", fontSize: 11,
                      border: "1px solid var(--warning-color, #f59e0b)",
                      color: "var(--warning-color, #f59e0b)",
                      background: "transparent", borderRadius: 3, cursor: "pointer",
                    }}
                  >
                    Undo
                  </button>
                  <button
                    onClick={() => { if (lastApplyTimerRef.current) clearTimeout(lastApplyTimerRef.current); setLastApply(null); }}
                    style={{
                      padding: "2px 6px", fontSize: 11,
                      border: "none", color: "var(--text-secondary)",
                      background: "transparent", cursor: "pointer",
                    }}
                    title="Dismiss"
                  >
                    <Icon name="x" size={12} />
                  </button>
                </div>
              )}
              {/* Editor area. AI writes go straight to disk and are reviewed in
                  Source Control, so nothing overlays Monaco here any more. */}
              <div style={{ height: 'calc(100% - 35px)', position: 'relative' }}>
                {/* Editor — always mounted, never hidden */}
                <div style={{ height: '100%' }}>
                {activeFile ? (
                  activeFile.isImage ? (
                    <ImageViewer
                      filePath={activeFile.path}
                      base64Data={activeFile.base64Data || ''}
                      rawContent={activeFile.content}
                    />
                  ) : activeFile.isDocument ? (
                    <DocumentViewer
                      filePath={activeFile.path}
                      base64Data={activeFile.base64Data || ''}
                    />
                  ) : showMarkdownPreview && currentFile?.endsWith('.md') ? (
                    <MarkdownPreview content={editorContent} />
                  ) : showHtmlPreview && (currentFile?.endsWith('.html') || currentFile?.endsWith('.htm')) ? (
                    <HtmlPreview content={editorContent} filePath={currentFile} />
                  ) : showSvgPreview && currentFile?.endsWith('.svg') ? (
                    <HtmlPreview content={editorContent} filePath={currentFile} />
                  ) : showDrawioPreview && (currentFile?.endsWith('.drawio') || currentFile?.endsWith('.dio')) ? (
                    <DrawioPreview content={editorContent} filePath={currentFile} />
                  ) : (
                    <Editor
                      height="100%"
                      /* `path` gives each file its own model with a real
                         `file://` URI. Without it every file shares one model
                         at `inmemory://model/1` — a URI no language server has
                         heard of, so every LSP request returns nothing, and
                         Monaco's own TypeScript service cannot resolve across
                         files either. It must be the same URI the bridge sends
                         with didOpen, so both come from `fileUri`. */
                      path={fileUri(activeFile.path)}
                      language={editorLanguage}
                      theme={editorTheme}
                      value={editorContent}
                      onChange={handleEditorChange}
                      onMount={handleEditorDidMount}
                      options={{
                        minimap: { enabled: true },
                        fontSize: 14,
                        lineNumbers: "on",
                        roundedSelection: false,
                        scrollBeyondLastLine: false,
                        automaticLayout: true,
                        // Parameter hints and quick suggestions are what make
                        // the LSP providers visible while typing. Matches
                        // VS Code's defaults — suggestions inside strings are
                        // noise everywhere except import paths.
                        quickSuggestions: { other: true, comments: false, strings: false },
                        suggestOnTriggerCharacters: true,
                        parameterHints: { enabled: true },
                        tabCompletion: "on",
                        // Renders ghost text and binds Tab to accept it. The
                        // widget being enabled does NOT mean suggestions are
                        // requested while typing — the provider answers only
                        // the explicit trigger kind (see lib/ghostText.ts).
                        inlineSuggest: { enabled: true },
                      }}
                    />
                  )
                ) : (
                  <div className="welcome-screen">
                    <h2>Welcome to VibeCoder</h2>
                    <p>AI-Powered Code Editor built with Rust + Tauri</p>
                    <div className="welcome-actions">
                      <button className="btn-primary" onClick={openFolder}>
                        <Icon name="folder-open" size={14} /> Open Folder
                      </button>
                      <button className="btn-secondary" onClick={() => setShowTour(true)}>
                        <Icon name="graduation-cap" size={14} /> Take a Tour
                      </button>
                    </div>
                    <div className="features">
                      <h3>Keyboard Shortcuts</h3>
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px 24px', textAlign: 'left', marginBottom: '16px' }}>
                        {globalShortcuts().map(s => (
                          <div key={renderShortcut(s, isMacPlatform) + s.description} style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
                            <kbd>{renderShortcut(s, isMacPlatform)}</kbd> {s.description}
                          </div>
                        ))}
                      </div>
                      {/* Monaco is not mounted on this screen — it is the else-branch
                          of "is a file open" — so these cannot fire here. Listing them
                          flat alongside the working ones is what made them look broken. */}
                      <h3 style={{ fontSize: '13px', opacity: 0.75 }}>With a file open, in the editor</h3>
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px 24px', textAlign: 'left', marginBottom: '24px' }}>
                        {editorShortcuts().map(s => (
                          <div key={renderShortcut(s, isMacPlatform) + s.description} style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
                            <kbd>{renderShortcut(s, isMacPlatform)}</kbd> {s.description}
                          </div>
                        ))}
                      </div>
                      <h3>Features</h3>
                      <ul>
                        <li><Icon name="sparkles" size={14} style={{ verticalAlign: -2 }} /> AI-powered code completion (Ollama ready)</li>
                        <li><Icon name="bot" size={14} style={{ verticalAlign: -2 }} /> Multiple AI providers: Ollama, Claude, ChatGPT, Gemini, Grok</li>
                        <li><Icon name="rocket" size={14} style={{ verticalAlign: -2 }} /> Fast text editing with Rust backend</li>
                        <li><Icon name="plug" size={14} style={{ verticalAlign: -2 }} /> VSCode + JetBrains + Neovim plugin support</li>
                      </ul>
                    </div>
                  </div>
                )}
              </div>{/* end editor */}
              </div>{/* end editor area container */}
            </>
          )}
        </main>

        {/* AI Panel — grouped sidebar + lazy-loaded panels */}
        {showAIChat && (
          <>
            {!panelsMaximized && showEditorArea && (
              <div
                className="resizer-vertical"
                onMouseDown={(e) => { e.preventDefault(); startResizing('aipanel'); }}
              />
            )}
            <aside
              className={`ai-chat-panel${panelsMaximized ? " ai-chat-panel--maximized" : ""}`}
              style={panelsMaximized
                ? undefined
                : !showEditorArea
                  ? { display: "flex", flexDirection: "row", flex: 1, maxWidth: "none", minWidth: 0 }
                  : { display: "flex", flexDirection: "row", width: `${aiPanelWidth}px` }
              }
            >
              {showFilterBar && (
                <GroupedTabBar activeTab={aiPanelTab} onTabChange={setAiPanelTab} onCollapse={() => setShowFilterBar(false)} />
              )}
              <div role="tabpanel" aria-labelledby={`ai-tab-${aiPanelTab}`} style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }}>
                {/* Panel header with maximize/restore button */}
                <div style={{ display: "flex", alignItems: "center", gap: 2, padding: "4px 6px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: 12, flexShrink: 0 }}>
                  {!showFilterBar && (
                    <>
                      <button
                        onClick={() => setShowFilterBar(true)}
                        style={{ display: "flex", alignItems: "center", gap: 5, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", padding: "4px 8px", borderRadius: 4 }}
                        title="Show filter panel"
                      >
                        <Icon name="menu" size={14} /> Panels
                      </button>
                      <span style={{ color: "var(--text-secondary)", opacity: 0.4 }}>|</span>
                    </>
                  )}
                  <span style={{ color: "var(--text-primary)", fontWeight: 500, flex: 1, paddingLeft: 4 }}>{(TAB_META[aiPanelTab] || DEFAULT_TAB_META).label}</span>
                  {/* Hide/show editor — show only sidebar + panels */}
                  {!panelsMaximized && (
                    <button
                      onClick={() => setShowEditorArea(prev => !prev)}
                      title={showEditorArea ? "Hide editor — explorer + panels only" : "Show editor"}
                      style={{
                        display: "flex", alignItems: "center", justifyContent: "center",
                        background: "none", border: "none", cursor: "pointer", padding: "4px 8px", borderRadius: 4,
                        color: !showEditorArea ? "var(--accent-color)" : "var(--text-secondary)",
                      }}
                    >
                      <Icon name="panel-right" size={14} />
                    </button>
                  )}
                  <button
                    onClick={() => setPanelsMaximized(prev => !prev)}
                    title={panelsMaximized ? "Restore panel (Ctrl+Shift+M)" : "Maximize panel (Ctrl+Shift+M)"}
                    style={{
                      display: "flex", alignItems: "center", justifyContent: "center",
                      background: "none", border: "none", cursor: "pointer", padding: "4px 8px", borderRadius: 4,
                      color: panelsMaximized ? "var(--accent-color)" : "var(--text-secondary)",
                    }}
                  >
                    <Icon name={panelsMaximized ? "minimize" : "maximize"} size={14} />
                  </button>
                  {panelsMaximized && (
                    <button
                      onClick={() => setPanelsMaximized(false)}
                      title="Close maximized view (Escape)"
                      style={{ display: "flex", alignItems: "center", justifyContent: "center", background: "none", border: "none", cursor: "pointer", padding: "4px 8px", borderRadius: 4, color: "var(--text-secondary)" }}
                    >
                      <Icon name="x" size={14} />
                    </button>
                  )}
                </div>
                <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
                  <PanelHost
                    tab={aiPanelTab}
                    selectedProvider={selectedProvider}
                    availableProviders={aiProviders}
                    editorContent={editorContent}
                    fileTree={files.map(f => f.path)}
                    currentFile={currentFile}
                    workspacePath={workspaceFolders[0] || null}
                    onPendingWrite={handlePendingWrite}
                    onOpenFile={handlePanelOpenFile}
                    onInjectContext={(text: string) => {
                      setAiPanelTab("chat");
                      window.dispatchEvent(new CustomEvent("vibecoder:inject-context", { detail: text }));
                    }}
                    onSwitchToGoals={(seed?: string) => {
                      setShowAIChat(true);
                      setAiPanelTab("goals");
                      if (seed && seed.length > 0) {
                        setNewGoalSeed(seed);
                      }
                    }}
                    newGoalSeed={newGoalSeed}
                    onNewGoalSeedConsumed={() => setNewGoalSeed(null)}
                    collab={collab}
                  />
                </div>
              </div>
            </aside>
          </>
        )}
      </div>

      {/* Terminal / Browser panel — bottom strip or centre, same DOM node. */}
      {showTerminal && (
        <>
          {/* The drag handle only means something for the bottom strip; the
              centre dock takes its size from the editor's box. */}
          {panelDock === 'bottom' && (
            <div
              className="resizer-horizontal"
              onMouseDown={(e) => {
                e.preventDefault();
                startResizing('terminal');
              }}
            />
          )}
          <div
            className={`terminal-panel${panelDock === 'center' ? ' terminal-panel--center' : ''}`}
            style={
              panelDock === 'center'
                ? {
                    // Until the first measurement lands there is no box to
                    // occupy; staying hidden avoids a full-window flash of the
                    // panel on the frame before the rect arrives.
                    display: centerRect ? 'flex' : 'none',
                    flexDirection: 'column',
                    top: centerRect?.top,
                    left: centerRect?.left,
                    width: centerRect?.width,
                    height: centerRect?.height,
                  }
                : { height: `${terminalHeight}px`, borderTop: 'none', display: 'flex', flexDirection: 'column' }
            }
          >
            {/* Tab bar */}
            <div role="tablist" aria-label="Bottom panel tabs" style={{ display: 'flex', alignItems: 'center', borderBottom: '1px solid var(--border-color)', background: 'var(--bg-secondary)', flexShrink: 0 }}>
              {(['terminal', 'browser'] as const).map((tab) => (
                <button
                  key={tab}
                  role="tab"
                  aria-selected={bottomTab === tab}
                  tabIndex={bottomTab === tab ? 0 : -1}
                  onClick={() => setBottomTab(tab)}
                  style={{
                    padding: '4px 14px', fontSize: '12px', border: 'none', cursor: 'pointer',
                    background: bottomTab === tab ? 'var(--bg-primary)' : 'transparent',
                    color: bottomTab === tab ? 'var(--text-primary)' : 'var(--text-secondary)',
                    borderBottom: bottomTab === tab ? '2px solid var(--accent-blue)' : '2px solid transparent',
                  }}
                >
                  {tab === 'terminal' ? 'Terminal' : 'Browser'}
                </button>
              ))}
              <div style={{ flex: 1 }} />
              <button
                onClick={() => dockPanel(panelDock === 'center' ? 'bottom' : 'center')}
                style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', padding: '4px 8px', display: 'inline-flex', alignItems: 'center' }}
                title={
                  panelDock === 'center'
                    ? 'Move panel to the bottom'
                    : 'Move panel to the editor area'
                }
                aria-label={
                  panelDock === 'center'
                    ? 'Move panel to the bottom'
                    : 'Move panel to the editor area'
                }
                aria-pressed={panelDock === 'center'}
              >
                <Icon name={panelDock === 'center' ? 'minimize' : 'maximize'} size={14} />
              </button>
              <button
                onClick={() => setShowTerminal(false)}
                style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', padding: '4px 10px', fontSize: '16px' }}
                title="Close panel"
                aria-label="Close panel"
              >×</button>
            </div>
            {/* Panel content — keep both mounted to preserve state across tab switches */}
            <div style={{ flex: 1, overflow: 'hidden', display: bottomTab === 'terminal' ? 'block' : 'none' }}>
              <Terminal onClose={() => setShowTerminal(false)} />
            </div>
            <div style={{ flex: 1, overflow: 'hidden', display: bottomTab === 'browser' ? 'block' : 'none' }}>
              <BrowserPanel />
            </div>
          </div>
        </>
      )}

      {/* Status Bar */}
      <footer className="status-bar">
        <div className="status-left">
          <span>VibeCoder v{appVersion}</span>
          {workspaceFolders.length > 0 && <span>• {workspaceFolders.length} folder(s)</span>}
          {currentFile && (
            <span
              className="status-file-path"
              title={currentFile}
              onClick={() => {
                const el = document.querySelector('.status-file-path');
                if (el) el.classList.toggle('status-file-path--expanded');
              }}
            >
              {currentFile.split('/').pop()} <span className="status-file-dir">— {currentFile}</span>
            </span>
          )}
          {currentFile && <span>• {activeFile?.isImage ? 'Image' : editorLanguage}</span>}
          {currentFile && !activeFile?.isImage && !activeFile?.isDocument && (
            <LspStatus
              filePath={currentFile}
              workspaceRoot={workspaceFolders[0] ?? ""}
              invoke={invoke}
            />
          )}
          {gitStatus && (
            <span style={{ marginLeft: '10px', display: 'flex', alignItems: 'center', gap: '4px' }}>
              <span style={{ fontSize: '10px' }}>Branch:</span>
              <strong>{gitStatus.branch}</strong>
            </span>
          )}
        </div>
        <div className="status-right">
          <button className="status-item" onClick={() => { setBottomTab('terminal'); setShowTerminal(true); }}>
            Terminal
          </button>
          <button className="status-item" onClick={() => { setBottomTab('browser'); setShowTerminal(true); }}>
            Browser
          </button>
          <button className="status-item" onClick={() => setShowCommandPalette(true)}>
            {modKey}K Command Palette
          </button>
          <ThemeToggle />
          {currentFile && !activeFile?.isImage && (
            <>
              <span>Lines: {editorContent.split("\n").length}</span>
              <span>•</span>
              <span>Chars: {editorContent.length}</span>
            </>
          )}
        </div>
      </footer>

      {showCommandPalette && (
        <CommandPalette
          isOpen={showCommandPalette}
          onClose={() => setShowCommandPalette(false)}
          commands={commands}
        />
      )}

      <Modal
        isOpen={modalOpen}
        title={modalConfig.title}
        placeholder={modalConfig.placeholder}
        onConfirm={modalConfig.onConfirm}
        onCancel={() => setModalOpen(false)}
      />
      {/* DREAD #1 Slice G part 2 — tainted-argument confirmation bridge.
          Enabled only when the daemon was started with --tainted-http-prompt;
          the daemon will simply never emit pending events otherwise, so the
          modal stays invisible.

          Token plumbing is done: the modal reads the live daemon token itself
          (see daemonFetch.ts). It used to take a `VITE_DAEMON_TOKEN` env var
          that nothing set, so both the SSE subscription and the response POST
          401'd — the user was never shown the prompt, and the daemon fell back
          to its deny-on-timeout path. */}
      <TaintedConfirmationModal
        daemonUrl={(import.meta.env.VITE_DAEMON_URL as string | undefined) ?? "http://localhost:7878"}
        enabled={(import.meta.env.VITE_TAINTED_HTTP_PROMPT as string | undefined) === "1"}
      />
      {/* DiffComplete Modal (⌘.) — diff-mode AI edit */}
      {diffComplete && (
        <DiffCompleteModal
          open={!!diffComplete}
          onClose={() => setDiffComplete(null)}
          filePath={diffComplete.filePath}
          language={diffComplete.language}
          originalContent={diffComplete.originalContent}
          selectionText={diffComplete.selectionText}
          selectionStartLine={diffComplete.selectionStartLine}
          selectionEndLine={diffComplete.selectionEndLine}
          provider={selectedProvider}
          model={selectedModel}
          onApply={(modified) => {
            if (modified === null) return;
            const editor = editorRef.current;
            if (!editor) return;
            const model = editor.getModel();
            if (!model) return;
            const fullRange = model.getFullModelRange();
            editor.executeEdits("diffcomplete-apply", [{
              range: fullRange,
              text: modified,
              forceMoveMarkers: true,
            }]);
            flowContext.add({
              kind: "diffcomplete",
              summary: `DiffComplete applied to ${diffComplete.filePath.split("/").pop() ?? "file"}`,
              detail: diffComplete.selectionText
                ? `Edited selection lines ${diffComplete.selectionStartLine}-${diffComplete.selectionEndLine}`
                : "Whole-file edit",
              filePath: diffComplete.filePath,
            });
          }}
        />
      )}

      {/* Delete Confirmation Modal */}
      {pendingDeleteFile && (
        <div
          role="alertdialog"
          aria-modal="true"
          aria-label="Confirm delete"
          style={{
            position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.5)',
            display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 2000,
          }}
          onClick={() => setPendingDeleteFile(null)}
        >
          <div
            style={{
              background: 'var(--bg-secondary)', border: '1px solid var(--border-color)',
              borderRadius: '8px', padding: '20px 24px', minWidth: '300px', maxWidth: '400px',
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ fontWeight: 600, marginBottom: '8px', fontSize: '14px' }}>Delete file?</div>
            <div style={{ fontSize: '13px', color: 'var(--text-secondary)', marginBottom: '16px', wordBreak: 'break-all' }}>
              {pendingDeleteFile.name}
            </div>
            <div style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end' }}>
              <button
                autoFocus
                onClick={() => setPendingDeleteFile(null)}
                style={{ padding: '6px 14px', borderRadius: '4px', border: '1px solid var(--border-color)', background: 'transparent', color: 'var(--text-primary)', cursor: 'pointer', fontSize: '13px' }}
              >
                Cancel
              </button>
              <button
                onClick={confirmDelete}
                style={{ padding: '6px 14px', borderRadius: '4px', border: 'none', background: 'var(--error-color)', color: 'var(--btn-primary-fg)', cursor: 'pointer', fontSize: '13px', fontWeight: 600 }}
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Onboarding Tour */}
      {showTour && (
        <OnboardingTour onComplete={completeTour} />
      )}

      {/* Settings Modal */}
      {showSettingsModal && (
        <div role="dialog" aria-modal="true" aria-label="Settings" style={{
          position: 'fixed', inset: 0, zIndex: 9999,
          background: 'rgba(0,0,0,0.6)', backdropFilter: 'blur(4px)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }} onClick={() => setShowSettingsModal(false)}>
          <div style={{ width: 760, height: '80vh', maxHeight: 700 }} onClick={e => e.stopPropagation()}>
            <SettingsPanel onClose={() => setShowSettingsModal(false)} workspacePath={workspaceFolders[0] || null} />
          </div>
        </div>
      )}

      {/* Context Menu */}
      {contextMenu && (
        <div
          className="context-menu"
          style={{
            position: 'fixed',
            top: contextMenu.y,
            left: contextMenu.x,
            background: 'var(--bg-secondary)',
            border: '1px solid var(--border-color)',
            borderRadius: '4px',
            padding: '4px 0',
            zIndex: 1000,
            boxShadow: '0 2px 5px rgba(0,0,0,0.2)',
            minWidth: '160px',
          }}
        >
          {contextMenu.file.is_directory && (
            <>
              <div
                className="context-menu-item"
                onClick={(e) => { e.stopPropagation(); const dir = contextMenu.file.path; setContextMenu(null); handleNewFile(dir); }}
                style={{ padding: '8px 12px', cursor: 'pointer', fontSize: '13px', color: 'var(--text-primary)' }}
                onMouseEnter={(e) => e.currentTarget.style.background = 'var(--bg-tertiary)'}
                onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
              >
                New File
              </div>
              <div
                className="context-menu-item"
                onClick={(e) => { e.stopPropagation(); const dir = contextMenu.file.path; setContextMenu(null); handleNewFolder(dir); }}
                style={{ padding: '8px 12px', cursor: 'pointer', fontSize: '13px', color: 'var(--text-primary)' }}
                onMouseEnter={(e) => e.currentTarget.style.background = 'var(--bg-tertiary)'}
                onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
              >
                New Folder
              </div>
              <div style={{ height: 1, background: 'var(--border-color)', margin: '4px 0' }} />
            </>
          )}
          <div
            className="context-menu-item"
            onClick={(e) => {
              e.stopPropagation();
              navigator.clipboard.writeText(contextMenu.file.path).then(
                () => toast.info('Path copied to clipboard'),
                () => toast.error('Failed to copy path'),
              );
              setContextMenu(null);
            }}
            style={{ padding: '8px 12px', cursor: 'pointer', fontSize: '13px', color: 'var(--text-primary)' }}
            onMouseEnter={(e) => e.currentTarget.style.background = 'var(--bg-tertiary)'}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
          >
            Copy Path
          </div>
          {/* Workspace root can't be renamed or deleted from the explorer.
              Anything else gets the standard mutation actions. */}
          {contextMenu.file.path !== currentDirectory && (
            <>
              <div
                className="context-menu-item"
                onClick={(e) => { e.stopPropagation(); handleRename(); }}
                style={{ padding: '8px 12px', cursor: 'pointer', fontSize: '13px', color: 'var(--text-primary)' }}
                onMouseEnter={(e) => e.currentTarget.style.background = 'var(--bg-tertiary)'}
                onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
              >
                Rename
              </div>
              <div
                className="context-menu-item"
                onClick={(e) => { e.stopPropagation(); handleDelete(); }}
                style={{ padding: '8px 12px', cursor: 'pointer', fontSize: '13px', color: 'var(--text-danger, #ff4d4f)' }}
                onMouseEnter={(e) => e.currentTarget.style.background = 'var(--bg-tertiary)'}
                onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
              >
                Delete
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

export default App;
