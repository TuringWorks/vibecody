import { useState, useEffect, useRef, useCallback } from "react";
import { useToast } from "./hooks/useToast";
import { Toaster } from "./components/Toaster";
import Editor, { DiffEditor, OnMount } from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { InlineChat } from "./components/InlineChat";
import type { InlineChatSelection } from "./components/InlineChat";
import { Terminal } from "./components/Terminal";
import { BrowserPanel } from "./components/BrowserPanel";
import { detectLanguage, getFileIcon } from "./utils/fileUtils";
import "./App.css";
import { ThemeToggle } from "./components/ThemeToggle";
import { CommandPalette, Command } from "./components/CommandPalette";
import Modal from "./components/Modal";
import { GitPanel } from "./components/GitPanel";
import { MarkdownPreview } from "./components/MarkdownPreview";
import { FilePlus, FolderPlus, FolderOpen, Files, Search, GitGraph, Settings, Menu, MessageSquare, Save, Terminal as TerminalIcon, PanelLeft, Puzzle, Hand, Sparkles, Bot, Rocket, Plug, Eye, FileText, GraduationCap, LayoutGrid, Bug, Box, Play, Shield, Globe } from "lucide-react";
import "./ActivityBar.css";
import { ExtensionManager } from "./extensions/ExtensionManager";
// Import worker using Vite's syntax
import ExtensionHostWorker from "./extensions/ExtensionHost?worker";
import { DiffReviewPanel } from "./components/DiffReviewPanel";
import { useCollab } from "./hooks/useCollab";
import { flowContext } from "./utils/FlowContext";
import { supercompleteEngine } from "./utils/SupercompleteEngine";
import { OnboardingTour } from "./components/OnboardingTour";
import { GroupedTabBar } from "./components/GroupedTabBar";
import { MenuBar, MenuGroup } from "./components/MenuBar";
import "./components/GroupedTabBar.css";
import { PanelHost } from "./components/LazyPanels";
import { useEditorTheme } from "./hooks/useEditorTheme";
import { SettingsPanel } from "./components/SettingsPanel";
import { ALL_TABS } from "./constants/tabGroups";
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

interface OpenFile {
  path: string;
  content: string;
  language: string;
  isDirty: boolean;
}

function App() {
  const { toasts, toast, dismiss } = useToast();
  const { themeName: editorTheme, defineTheme: defineEditorTheme } = useEditorTheme();
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const [workspaceFolders, setWorkspaceFolders] = useState<string[]>([]);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [aiProviders, setAiProviders] = useState<string[]>([]);
  const [selectedProvider, setSelectedProvider] = useState<string>("");
  const [showSidebar, setShowSidebar] = useState(true);
  const [activeSidebarTab, setActiveSidebarTab] = useState<"explorer" | "search" | "git" | "debug" | "extensions" | "docker" | "ai" | "security">("explorer");
  const [showAIChat, setShowAIChat] = useState(false);
  const [aiPanelTab, setAiPanelTab] = useState("chat");
  const [showFilterBar, setShowFilterBar] = useState(true);
  const [showTerminal, setShowTerminal] = useState(false);
  const [bottomTab, setBottomTab] = useState<"terminal" | "browser">("terminal");
  const [showCommandPalette, setShowCommandPalette] = useState(false);
  const [showTour, setShowTour] = useState(() => !localStorage.getItem('vibeui-onboarding-complete'));
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [appVersion, setAppVersion] = useState("0.0.0");

  const completeTour = useCallback(() => {
    localStorage.setItem('vibeui-onboarding-complete', 'true');
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
  const [pendingDiff, setPendingDiff] = useState<{ path: string; original: string; modified: string } | null>(null);

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

  // Markdown Preview State
  const [showMarkdownPreview, setShowMarkdownPreview] = useState(false);

  // Git Diff View State
  const [gitDiffView, setGitDiffView] = useState<{ file: string; original: string; modified: string } | null>(null);

  // Extension Manager
  const extensionManagerRef = useRef<ExtensionManager | null>(null);

  // Ref so editor-mount callbacks always see the current provider
  const selectedProviderRef = useRef<string>(selectedProvider);
  useEffect(() => {
    selectedProviderRef.current = selectedProvider;
  }, [selectedProvider]);

  // Derived state for active file
  const activeFile = openFiles.find(f => f.path === activeFilePath);
  const editorContent = activeFile?.content || "";
  const editorLanguage = activeFile?.language || "typescript";
  const currentFile = activeFilePath; // Alias for backward compatibility in some checks

  useEffect(() => {
    // Load available AI providers
    const refreshProviders = (providers: string[]) => {
      setAiProviders(providers);
      if (providers.length > 0 && !selectedProvider) {
        const defaultProvider = providers.find(p => p.startsWith("Ollama")) || providers[0];
        setSelectedProvider(defaultProvider);
      }
    };
    invoke<string[]>("get_available_ai_providers")
      .then(refreshProviders)
      .catch(console.error);

    // Listen for provider updates from Settings panel (API key changes)
    const onProvidersUpdated = (e: Event) => {
      const providers = (e as CustomEvent<string[]>).detail;
      setAiProviders(providers);
      // If current selection is no longer valid, pick the first available
      if (providers.length > 0 && !providers.includes(selectedProvider)) {
        setSelectedProvider(providers[0]);
      }
    };
    window.addEventListener("vibeui:providers-updated", onProvidersUpdated);

    // Load workspace folders
    invoke<string[]>("get_workspace_folders")
      .then(setWorkspaceFolders)
      .catch(console.error);

    // Load app version from Tauri
    getVersion().then(setAppVersion).catch(() => {});

    // Initialize Extension Manager
    const manager = new ExtensionManager({
      showInformationMessage: (message) => {
        (window as any).lastExtensionMessage = message;
      },
      showErrorMessage: (message) => {
        console.error(`[Extension Error] ${message}`);
        (window as any).lastExtensionMessage = `Error: ${message}`;
      },
    });

    try {
      const worker = new ExtensionHostWorker();
      manager.setWorker(worker);
      extensionManagerRef.current = manager;
      (window as any).extensionManager = manager;
      // Extension Manager initialized
    } catch (e) {
      toast.error(`Failed to initialize extension worker: ${e}`);
    }

    return () => {
      window.removeEventListener("vibeui:providers-updated", onProvidersUpdated);
    };
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
  }, []);

  const openFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Open Folder",
      });

      if (selected && typeof selected === 'string') {
        await invoke("add_workspace_folder", { path: selected });
        setWorkspaceFolders((prev) => [...prev, selected]);
        loadDirectory(selected);
      }
    } catch (error) {
      console.error("Failed to open folder:", error);
      toast.error(`Failed to open folder: ${error}`);
    }
  };

  const loadDirectory = async (path: string) => {
    try {
      const entries = await invoke<FileEntry[]>("list_directory", { path });
      setFiles(entries);
      setCurrentDirectory(path);
      // Fetch git status when directory loads
      fetchGitStatus();
    } catch (error) {
      console.error("Failed to load directory:", error);
    }
  };

  const fetchGitStatus = async () => {
    try {
      const status = await invoke<GitStatus>("get_git_status");
      setGitStatus(status);
    } catch (error) {
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

  const handleGoUp = () => {
    if (!currentDirectory) return;
    const separator = currentDirectory.includes('\\') ? '\\' : '/';
    const parts = currentDirectory.split(separator);
    // Handle trailing slash if present
    if (parts[parts.length - 1] === '') parts.pop();
    parts.pop();
    const parentPath = parts.join(separator) || separator;
    loadDirectory(parentPath);
  };

  const openFile = async (path: string) => {
    // Check if already open
    if (openFiles.some(f => f.path === path)) {
      setActiveFilePath(path);
      return;
    }

    try {
      const content = await invoke<string>("read_file", { path });
      const filename = path.split('/').pop() || path.split('\\').pop() || '';
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

      // Phase 3: LSP lifecycle — notify server that document was opened
      const rootPath = workspaceFolders[0] || "";
      if (rootPath) {
        const uri = `file://${path}`;
        invoke("lsp_did_open", { language, rootPath, uri, text: content }).catch(() => {});
      }
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
  };

  const saveFile = async () => {
    if (!activeFilePath || !activeFile) return;
    try {
      await invoke("write_file", { path: activeFilePath, content: activeFile.content });

      // Update dirty state
      setOpenFiles(prev => prev.map(f =>
        f.path === activeFilePath ? { ...f, isDirty: false } : f
      ));
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
      // Phase 3: Flow tracking (fire-and-forget)
      invoke("track_flow_event", { kind: "file_edit", data: activeFilePath }).catch(() => {});
    }
  };

  const cursorUpdateTimeoutRef = useRef<number | null>(null);

  // Inline Chat (Cmd+K) state
  const [inlineChat, setInlineChat] = useState<{
    selection: InlineChatSelection;
    position: { top: number; left: number };
  } | null>(null);
  const editorRef = useRef<any>(null);

  // Recent edits buffer for next-edit prediction
  const recentEditsRef = useRef<Array<{
    line: number; col: number; old_text: string; new_text: string; elapsed_ms: number;
  }>>([]);
  const nextEditDebounceRef = useRef<number | null>(null);

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    // Store editor reference for Inline Chat
    editorRef.current = editor;

    // Register VibeUI theme with Monaco so the editor matches the app theme
    defineEditorTheme(monaco);

    const getRootPath = () => workspaceFolders[0] || ""; // Simple assumption for MVP

    // ── Cmd+K: Inline Chat (edit selection) or Generate Code (no selection) ──
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK,
      () => {
        const selection = editor.getSelection();
        const model = editor.getModel();
        if (!model) return;

        const pos = editor.getPosition();
        const lineNum = pos?.lineNumber ?? 1;
        const lineTop = editor.getTopForLineNumber(lineNum);
        const scrollTop = editor.getScrollTop();
        const layoutInfo = editor.getLayoutInfo();
        const editorDom = editor.getDomNode();
        const rect = editorDom?.getBoundingClientRect() ?? { top: 0, left: 0 };

        const hasSelection = selection && !selection.isEmpty();
        const selectedText = hasSelection ? model.getValueInRange(selection) : "";

        setInlineChat({
          selection: {
            text: selectedText,
            startLine: hasSelection ? selection.startLineNumber - 1 : lineNum - 1,
            endLine: hasSelection ? selection.endLineNumber - 1 : lineNum - 1,
            filePath: activeFilePath ?? "",
            language: model.getLanguageId(),
          },
          position: {
            top: rect.top + lineTop - scrollTop + 20,
            left: rect.left + layoutInfo.contentLeft + 20,
          },
        });
      }
    );

    monaco.languages.registerCompletionItemProvider('*', {
      provideCompletionItems: async (model: any, position: any) => {
        const language = model.getLanguageId();
        const rootPath = getRootPath();
        if (!rootPath) return { suggestions: [] };

        try {
          const response = await invoke<any>("lsp_completion", {
            language,
            rootPath,
            params: {
              text_document_position: {
                text_document: { uri: model.uri.toString() },
                position: { line: position.lineNumber - 1, character: position.column - 1 }
              },
              context: { trigger_kind: 1 } // Invoked
            }
          });

          if (!response) return { suggestions: [] };

          // Transform LSP items to Monaco items
          const suggestions = (Array.isArray(response) ? response : response.items).map((item: any) => ({
            label: item.label,
            kind: item.kind, // Map LSP kind to Monaco kind if needed
            insertText: item.insertText || item.label,
            detail: item.detail,
            documentation: item.documentation,
            range: item.textEdit ? {
              startLineNumber: item.textEdit.range.start.line + 1,
              startColumn: item.textEdit.range.start.character + 1,
              endLineNumber: item.textEdit.range.end.line + 1,
              endColumn: item.textEdit.range.end.character + 1
            } : undefined
          }));

          return { suggestions };
        } catch (e) {
          console.error("LSP Completion failed:", e);
          return { suggestions: [] };
        }
      }
    });

    monaco.languages.registerHoverProvider('*', {
      provideHover: async (model: any, position: any) => {
        const language = model.getLanguageId();
        const rootPath = getRootPath();
        if (!rootPath) return null;

        try {
          const response = await invoke<any>("lsp_hover", {
            language,
            rootPath,
            params: {
              text_document_position_params: {
                text_document: { uri: model.uri.toString() },
                position: { line: position.lineNumber - 1, character: position.column - 1 }
              }
            }
          });

          if (!response || !response.contents) return null;

          // Handle different content formats (MarkupContent, MarkedString, etc.)
          let contents: any[] = [];
          if (typeof response.contents === 'string') {
            contents = [{ value: response.contents }];
          } else if ('kind' in response.contents) {
            contents = [{ value: response.contents.value }];
          } else if (Array.isArray(response.contents)) {
            contents = response.contents.map((c: any) => typeof c === 'string' ? { value: c } : { value: c.value });
          }

          return {
            contents
          };
        } catch (e) {
          console.error("LSP Hover failed:", e);
          return null;
        }
      }
    });

    monaco.languages.registerDefinitionProvider('*', {
      provideDefinition: async (model: any, position: any) => {
        const language = model.getLanguageId();
        const rootPath = getRootPath();
        if (!rootPath) return null;

        try {
          const response = await invoke<any>("lsp_goto_definition", {
            language,
            rootPath,
            params: {
              text_document_position_params: {
                text_document: { uri: model.uri.toString() },
                position: { line: position.lineNumber - 1, character: position.column - 1 }
              }
            }
          });

          if (!response) return null;

          // Handle Location or Location[] or LocationLink[]
          const locations = Array.isArray(response) ? response : [response];

          return locations.map((loc: any) => ({
            uri: monaco.Uri.parse(loc.uri),
            range: {
              startLineNumber: loc.range.start.line + 1,
              startColumn: loc.range.start.character + 1,
              endLineNumber: loc.range.end.line + 1,
              endColumn: loc.range.end.character + 1
            }
          }));
        } catch (e) {
          console.error("LSP Definition failed:", e);
          return null;
        }
      }
    });

    // Phase 3: Inline AI completions (ghost text / FIM)
    if (typeof (monaco.languages as any).registerInlineCompletionsProvider === 'function') {
      (monaco.languages as any).registerInlineCompletionsProvider('*', {
        provideInlineCompletions: async (model: any, position: any) => {
          const provider = selectedProviderRef.current;
          if (!provider) return { items: [] };

          const text = model.getValue() as string;
          const offset = model.getOffsetAt(position) as number;
          const prefix = text.slice(0, offset);
          const suffix = text.slice(offset);
          const language = model.getLanguageId() as string;
          const filePath = model.uri.path as string;

          // Try next-edit prediction first (debounced)
          if (nextEditDebounceRef.current) {
            window.clearTimeout(nextEditDebounceRef.current);
          }
          const nextEditPromise = new Promise<string | null>((resolve) => {
            nextEditDebounceRef.current = window.setTimeout(async () => {
              try {
                const pred = await invoke<{
                  target_line: number; target_col: number; suggested_text: string; confidence: number;
                } | null>("predict_next_edit", {
                  currentFile: filePath,
                  content: text,
                  cursorLine: position.lineNumber - 1,
                  cursorCol: position.column - 1,
                  recentEdits: recentEditsRef.current.slice(-5).map(e => ({ ...e, elapsed_ms: Date.now() - e.elapsed_ms })),
                  provider,
                });
                resolve(pred && pred.confidence >= 0.5 ? pred.suggested_text : null);
              } catch {
                resolve(null);
              }
            }, 500);
          });

          // Also request FIM completion in parallel
          const fimPromise = invoke<string>("request_inline_completion", {
            prefix, suffix, language, provider,
          }).catch(() => null);

          // Supercomplete: cross-file context via embedding search (races alongside FIM+next-edit)
          const supercompletePromise = supercompleteEngine.predict({
            filePath,
            prefix,
            suffix,
            language,
            cursorLine: position.lineNumber - 1,
            cursorCol: position.column - 1,
            recentEdits: recentEditsRef.current.slice(-10).map(e => ({ ...e, elapsed_ms: Date.now() - e.elapsed_ms })),
            provider,
          }).catch(() => null);

          const [nextEdit, fim, superResult] = await Promise.all([nextEditPromise, fimPromise, supercompletePromise]);
          // Prefer supercomplete if it has high confidence, else next-edit, else FIM
          const suggestion = (superResult && superResult.confidence >= 0.65)
            ? superResult.insertText
            : nextEdit ?? fim ?? null;
          if (!suggestion) return { items: [] };
          return { items: [{ insertText: suggestion }] };
        },
        freeInlineCompletions: () => {},
      });
    }

    // Track content changes for next-edit prediction
    editor.onDidChangeModelContent((event: any) => {
      const model = editor.getModel();
      if (!model) return;
      const now = Date.now();
      for (const change of event.changes) {
        recentEditsRef.current.push({
          line: change.range.startLineNumber - 1,
          col: change.range.startColumn - 1,
          old_text: change.rangeLength > 0
            ? model.getValueInRange(change.range).slice(0, 50)
            : "",
          new_text: change.text.slice(0, 50),
          elapsed_ms: now, // store creation timestamp; converted to relative age at read time
        });
        if (recentEditsRef.current.length > 20) {
          recentEditsRef.current.shift();
        }
      }
    });

    editor.onDidChangeCursorSelection((_) => {
      if (!activeFilePath) return;

      if (cursorUpdateTimeoutRef.current) {
        window.clearTimeout(cursorUpdateTimeoutRef.current);
      }

      cursorUpdateTimeoutRef.current = window.setTimeout(() => {
        const selections = editor.getSelections();
        if (!selections) return;

        const cursors = selections.map(sel => ({
          position: { line: sel.positionLineNumber - 1, column: sel.positionColumn - 1 },
          selection: {
            start: { line: sel.selectionStartLineNumber - 1, column: sel.selectionStartColumn - 1 },
            end: { line: sel.positionLineNumber - 1, column: sel.positionColumn - 1 }
          }
        }));

        invoke("update_cursors", { path: activeFilePath, cursors })
          .catch(() => { /* best-effort: cursor sync failures are non-critical */ });
      }, 100); // Debounce 100ms
    });
  };

  const handlePendingWrite = async (path: string, content: string) => {
    try {
      // Read current file content for diff
      let original = "";
      try {
        original = await invoke<string>("read_file", { path });
      } catch (_e) {
        // File might not exist yet — treat as new file
      }

      setPendingDiff({
        path,
        original,
        modified: content
      });
    } catch (error) {
      console.error("Failed to prepare diff:", error);
    }
  };

  // acceptDiff was removed — DiffReviewPanel handles accept/reject inline

  const rejectDiff = () => {
    setPendingDiff(null);
  };

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
  }, [currentFile, editorContent]);

  const handleNewFile = () => {
    if (!currentDirectory) {
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
        const separator = currentDirectory.includes('\\') ? '\\' : '/';
        const cleanDir = currentDirectory.endsWith(separator) ? currentDirectory : currentDirectory + separator;
        const path = cleanDir + name;

        try {
          await invoke("write_file", { path, content: "" });
          loadDirectory(currentDirectory);
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

  const handleNewFolder = () => {
    if (!currentDirectory) {
      toast.warn("Please open a folder first.");
      return;
    }

    setModalConfig({
      title: 'Create New Folder',
      placeholder: 'Enter folder name',
      onConfirm: async (name) => {
        setModalOpen(false);
        if (!name) return;

        const separator = currentDirectory.includes('\\') ? '\\' : '/';
        const cleanDir = currentDirectory.endsWith(separator) ? currentDirectory : currentDirectory + separator;
        const path = cleanDir + name;

        try {
          await invoke("create_directory", { path });
          loadDirectory(currentDirectory);
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
      icon: <FolderOpen size={16} strokeWidth={1.5} />,
      shortcut: modKey + 'O',
      action: openFolder,
    },
    {
      id: 'file.save',
      label: 'Save File',
      category: 'File',
      icon: <Save size={16} strokeWidth={1.5} />,
      shortcut: modKey + 'S',
      action: saveFile,
    },
    {
      id: 'file.createFile',
      label: 'Create New File',
      category: 'File',
      icon: <FilePlus size={16} strokeWidth={1.5} />,
      action: handleNewFile,
    },
    {
      id: 'file.createFolder',
      label: 'Create New Folder',
      category: 'File',
      icon: <FolderPlus size={16} strokeWidth={1.5} />,
      action: handleNewFolder,
    },
    // Editor actions
    {
      id: 'editor.toggleSidebar',
      label: 'Toggle Sidebar',
      category: 'Editor',
      icon: <PanelLeft size={16} strokeWidth={1.5} />,
      shortcut: modKey + 'B',
      action: () => setShowSidebar(prev => !prev),
    },
    {
      id: 'editor.toggleAIChat',
      label: 'Toggle AI Chat',
      category: 'Editor',
      icon: <MessageSquare size={16} strokeWidth={1.5} />,
      shortcut: modKey + 'J',
      action: () => setShowAIChat(prev => !prev),
    },
    {
      id: 'editor.search',
      label: 'Search in Files',
      category: 'Editor',
      icon: <Search size={16} strokeWidth={1.5} />,
      action: () => setActiveSidebarTab('search'),
    },
    // View
    {
      id: 'view.toggleTerminal',
      label: 'Toggle Terminal',
      category: 'View',
      icon: <TerminalIcon size={16} strokeWidth={1.5} />,
      shortcut: modKey + '`',
      action: () => setShowTerminal(prev => !prev),
    },
    {
      id: 'view.explorer',
      label: 'Show Explorer',
      category: 'View',
      icon: <FolderOpen size={16} strokeWidth={1.5} />,
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
      icon: <GitGraph size={16} strokeWidth={1.5} />,
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
      icon: <Puzzle size={16} strokeWidth={1.5} />,
      action: () => {
        const code = `
          console.log('Hello from extension!');
          vscode.commands.registerCommand('extension.helloWorld', () => {
            vscode.window.showInformationMessage('Hello World from VibeUI Extension!');
          });
        `;
        extensionManagerRef.current?.loadExtension(code);
        (window as any).lastExtensionMessage = "Test extension loaded";
      }
    },
    {
      id: 'extension.helloWorld',
      label: 'Hello World (Extension)',
      category: 'Extension',
      icon: <Hand size={16} strokeWidth={1.5} />,
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
        { separator: true, label: "" },
        { label: "Command Palette...", shortcut: modKey + shiftMod + "P", action: () => setShowCommandPalette(true) },
      ],
    },
    {
      label: "Tools",
      items: [
        { label: "AI Chat", action: () => { setShowAIChat(true); setAiPanelTab("chat"); } },
        { label: "Agent", action: () => { setShowAIChat(true); setAiPanelTab("agent"); } },
        { label: "Agent Teams", action: () => { setShowAIChat(true); setAiPanelTab("agentteams"); } },
        { separator: true, label: "" },
        { label: "Docker", action: () => { setShowAIChat(true); setAiPanelTab("docker"); } },
        { label: "CI/CD", action: () => { setShowAIChat(true); setAiPanelTab("cicd"); } },
        { label: "Kubernetes", action: () => { setShowAIChat(true); setAiPanelTab("k8s"); } },
        { separator: true, label: "" },
        { label: "HTTP Playground", action: () => { setShowAIChat(true); setAiPanelTab("http"); } },
        { label: "GraphQL", action: () => { setShowAIChat(true); setAiPanelTab("graphql"); } },
        { label: "Terminal", shortcut: modKey + "`", action: () => setShowTerminal(true) },
        { separator: true, label: "" },
        { label: "Settings", action: () => setShowSettingsModal(true) },
      ],
    },
    {
      label: "Help",
      items: [
        { label: "Welcome Tour", action: () => { localStorage.removeItem("vibeui-onboarding-complete"); setShowTour(true); } },
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
          if (currentDirectory) loadDirectory(currentDirectory);
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
      if (currentDirectory) loadDirectory(currentDirectory);
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

  return (
    <div className="app" onMouseUp={stopResizing}>
      <a href="#main-editor" className="skip-to-content">Skip to editor</a>
      <Toaster toasts={toasts} onDismiss={dismiss} />
      {/* Header */}
      <header className="header">
        <div className="header-left">
          <button className="icon-button" onClick={() => setShowSidebar(!showSidebar)} aria-label="Toggle sidebar">
            <Menu size={18} strokeWidth={1.5} />
          </button>
          <h1 className="app-title">VibeUI</h1>
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
          <button
            className="btn-secondary"
            onClick={() => setShowAIChat(!showAIChat)}
            title="Toggle Vibe Toolkit"
          >
            <LayoutGrid size={14} strokeWidth={1.5} /> Vibe Toolkit
          </button>
          <button className="btn-primary" onClick={saveFile} disabled={!currentFile}>
            <Save size={14} strokeWidth={1.5} /> Save
          </button>
          {currentFile && currentFile.endsWith('.md') && (
            <button
              className="btn-secondary"
              onClick={() => setShowMarkdownPreview(!showMarkdownPreview)}
            >
              {showMarkdownPreview ? <><FileText size={14} strokeWidth={1.5} /> Edit</> : <><Eye size={14} strokeWidth={1.5} /> Preview</>}
            </button>
          )}
        </div>
      </header>

      <div className="main-container">
        {/* Activity Bar */}
        <div className="activity-bar">
          {([
            { id: "explorer" as const, icon: <Files size={20} strokeWidth={1.5} />, title: "Explorer", shortcut: `${modKey}${shiftMod}E` },
            { id: "search" as const, icon: <Search size={20} strokeWidth={1.5} />, title: "Search", shortcut: undefined },
            { id: "git" as const, icon: <GitGraph size={20} strokeWidth={1.5} />, title: "Source Control", shortcut: `${modKey}${shiftMod}G` },
            { id: "debug" as const, icon: <Bug size={20} strokeWidth={1.5} />, title: "Debug", shortcut: undefined },
            { id: "extensions" as const, icon: <Puzzle size={20} strokeWidth={1.5} />, title: "Extensions", shortcut: undefined },
            { id: "docker" as const, icon: <Box size={20} strokeWidth={1.5} />, title: "Containers", shortcut: undefined },
            { id: "ai" as const, icon: <Bot size={20} strokeWidth={1.5} />, title: "AI Toolkit", shortcut: `${modKey}J` },
            { id: "security" as const, icon: <Shield size={20} strokeWidth={1.5} />, title: "Security", shortcut: undefined },
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
            <TerminalIcon size={20} strokeWidth={1.5} />
          </button>
          <button className="activity-bar-item" title="Settings" aria-label="Settings" onClick={() => setShowSettingsModal(true)}>
            <Settings size={20} strokeWidth={1.5} />
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
                    <button className="btn-icon" onClick={handleNewFile} title="New File" disabled={!currentDirectory}>
                      <FilePlus size={16} />
                    </button>
                    <button className="btn-icon" onClick={handleNewFolder} title="New Folder" disabled={!currentDirectory}>
                      <FolderPlus size={16} />
                    </button>
                    <button className="btn-icon" onClick={openFolder} title="Open Folder">
                      <FolderOpen size={16} />
                    </button>
                  </div>
                </div>
                <div className="file-tree">
                  {currentDirectory && (
                    <div className="file-item directory" onClick={handleGoUp} onKeyDown={e => e.key === "Enter" && handleGoUp()} role="button" tabIndex={0} title="Go to Parent">
                      <span className="file-icon"><FolderOpen size={14} strokeWidth={1.5} /></span>
                      <span className="file-name">..</span>
                    </div>
                  )}
                  {workspaceFolders.length === 0 ? (
                    <div className="empty-state">
                      <p>No folder opened</p>
                      <button className="btn-secondary" onClick={openFolder}>
                        Open Folder
                      </button>
                    </div>
                  ) : (
                    <div>
                      {files.map((file) => <div
                        key={file.path}
                        className={`file-item ${file.is_directory ? "directory" : "file"}`}
                        role="button"
                        tabIndex={0}
                        onClick={() => {
                          if (file.is_directory) {
                            loadDirectory(file.path);
                          } else {
                            openFile(file.path);
                          }
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            if (file.is_directory) loadDirectory(file.path);
                            else openFile(file.path);
                          }
                        }}
                        onContextMenu={(e) => {
                          e.preventDefault();
                          setContextMenu({ x: e.clientX, y: e.clientY, file });
                        }}
                      >
                        <span className="file-icon">
                          {getFileIcon(file.name, file.is_directory)}
                        </span>
                        <span className="file-name" style={{ color: getFileColor(file.path) }}>{file.name}</span>
                        {gitStatus && Object.entries(gitStatus.file_statuses).some(([p]) => file.path.endsWith(p)) && (
                          <span style={{ marginLeft: 'auto', fontSize: '10px', color: getFileColor(file.path) }}>
                            {Object.entries(gitStatus.file_statuses).find(([p]) => file.path.endsWith(p))?.[1].charAt(0)}
                          </span>
                        )}
                      </div>
                      )}
                    </div>
                  )}
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
                      <div style={{ fontSize: '13px', whiteSpace: 'pre-wrap', fontFamily: 'monospace' }}>
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
              <GitPanel workspacePath={workspaceFolders[0] || null} onCompareFile={handleCompareFile} />
            )}

            {activeSidebarTab === 'debug' && (
              <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
                <div className="sidebar-section-title">Run & Debug</div>
                <button className="btn-secondary" style={{ width: "100%", justifyContent: "center", gap: 6, display: "flex", alignItems: "center" }}
                  onClick={() => { setShowAIChat(true); setAiPanelTab("debugmode"); }}>
                  <Play size={14} strokeWidth={1.5} /> Start Debug Session
                </button>
                <div style={{ fontSize: 11, color: "var(--text-muted)", lineHeight: 1.5 }}>
                  Launch AI-assisted debugging. The debug agent can set breakpoints, inspect variables, and step through code.
                </div>
                <div className="sidebar-section-title" style={{ marginTop: 8 }}>Quick Actions</div>
                {([
                  { label: "Debug Mode", panel: "debugmode" },
                  { label: "Profiler", panel: "profiler" },
                  { label: "Coverage", panel: "coverage" },
                  { label: "Test Runner", panel: "tests" },
                  { label: "Bisect", panel: "bisect" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}
>
                    {label}
                  </button>
                ))}
              </div>
            )}

            {activeSidebarTab === 'extensions' && (
              <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
                <div className="sidebar-section-title">Extensions</div>
                <button className="btn-secondary" style={{ width: "100%", justifyContent: "center", gap: 6, display: "flex", alignItems: "center" }}
                  onClick={() => { setShowAIChat(true); setAiPanelTab("marketplace"); }}>
                  <Globe size={14} strokeWidth={1.5} /> Browse Marketplace
                </button>
                <div className="sidebar-section-title" style={{ marginTop: 8 }}>Manage</div>
                {([
                  { label: "Installed Plugins", panel: "marketplace" },
                  { label: "MCP Servers", panel: "mcp" },
                  { label: "MCP Directory", panel: "mcpdirectory" },
                  { label: "Skills Library", panel: "manager" },
                  { label: "Hooks", panel: "hooks" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}
>
                    {label}
                  </button>
                ))}
              </div>
            )}

            {activeSidebarTab === 'docker' && (
              <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
                <div className="sidebar-section-title">Containers & Infra</div>
                {([
                  { label: "Docker", panel: "docker" },
                  { label: "Kubernetes", panel: "k8s" },
                  { label: "CI/CD Pipelines", panel: "cicd" },
                  { label: "GitHub Actions", panel: "ghactions" },
                  { label: "Sandbox", panel: "sandbox" },
                  { label: "Environment", panel: "env" },
                  { label: "Cloud Provider", panel: "cloudproviders" },
                  { label: "SSH", panel: "ssh" },
                  { label: "Service Health", panel: "health" },
                  { label: "Processes", panel: "processes" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}
>
                    {label}
                  </button>
                ))}
              </div>
            )}

            {activeSidebarTab === 'security' && (
              <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, height: "100%", overflow: "auto" }}>
                <div className="sidebar-section-title">Security & Compliance</div>
                {([
                  { label: "Security Scan", panel: "securityscan" },
                  { label: "Red Team", panel: "redteam" },
                  { label: "Blue Team", panel: "blueteam" },
                  { label: "Purple Team", panel: "purpleteam" },
                  { label: "Compliance", panel: "compliance" },
                  { label: "Audit Admin", panel: "admin" },
                ] as const).map(({ label, panel }) => (
                  <button key={panel} className="sidebar-action-item"
                    onClick={() => { setShowAIChat(true); setAiPanelTab(panel); }}
>
                    {label}
                  </button>
                ))}
              </div>
            )}
          </aside>
        )}

        {/* Vertical Resizer */}
        {showSidebar && (
          <div
            className="resizer-vertical"
            onMouseDown={(e) => {
              e.preventDefault();
              startResizing('sidebar');
            }}
          />
        )}

        {/* Editor Area */}
        <main id="main-editor" className="editor-container">
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
                  {file.isDirty && <span className="tab-dirty">●</span>}
                  <button
                    className="tab-close"
                    onClick={(e) => closeFile(file.path, e)}
                  >
                    ×
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
          ) : pendingDiff ? (
            <div style={{ height: 'calc(100% - 35px)' }}>
              <DiffReviewPanel
                original={pendingDiff.original}
                modified={pendingDiff.modified}
                filePath={pendingDiff.path}
                onApply={(result) => {
                  if (result === null) {
                    rejectDiff();
                  } else {
                    // Write the assembled (partial-accept) result
                    invoke("write_file", { path: pendingDiff!.path, content: result })
                      .then(() => {
                        const language = detectLanguage(pendingDiff!.path);
                        setOpenFiles((prev) => {
                          const exists = prev.some((f) => f.path === pendingDiff!.path);
                          if (exists) return prev.map((f) => f.path === pendingDiff!.path ? { ...f, content: result, isDirty: false } : f);
                          return [...prev, { path: pendingDiff!.path, content: result, language, isDirty: false }];
                        });
                        setActiveFilePath(pendingDiff!.path);
                        setPendingDiff(null);
                      })
                      .catch(console.error);
                  }
                }}
              />
            </div>
          ) : activeFile ? (
            showMarkdownPreview && currentFile?.endsWith('.md') ? (
              <MarkdownPreview content={editorContent} />
            ) : (
              <Editor
                height="calc(100% - 35px)" // Subtract tab bar height
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
                }}
              />
            )
          ) : (
            <div className="welcome-screen">
              <h2>Welcome to VibeUI</h2>
              <p>AI-Powered Code Editor built with Rust + Tauri</p>
              <div className="welcome-actions">
                <button className="btn-primary" onClick={openFolder}>
                  <FolderOpen size={14} strokeWidth={1.5} /> Open Folder
                </button>
                <button className="btn-secondary" onClick={() => setShowTour(true)}>
                  <GraduationCap size={14} strokeWidth={1.5} /> Take a Tour
                </button>
              </div>
              <div className="features">
                <h3>Keyboard Shortcuts</h3>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px 24px', textAlign: 'left', marginBottom: '24px' }}>
                  {(() => {
                    const isMac = /Mac/.test(navigator.userAgent);
                    const mod = isMac ? '⌘' : 'Ctrl+';
                    const shift = isMac ? '⇧' : 'Shift+';
                    return [
                      [`${mod}K`, 'Command Palette / Inline Chat'],
                      [`${mod}${shift}P`, 'Command Palette'],
                      [`${mod}J`, 'Toggle AI Panel'],
                      [`${mod}B`, 'Toggle Sidebar'],
                      [`${mod}\``, 'Toggle Terminal'],
                      [`${mod}${shift}E`, 'Explorer'],
                      [`${mod}${shift}G`, 'Source Control'],
                      [`${mod}S`, 'Save File'],
                      [`${mod}O`, 'Open Folder'],
                      [`${mod}1-9`, 'Switch AI Tab'],
                    ];
                  })().map(([key, desc]) => (
                    <div key={key} style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
                      <kbd>{key}</kbd> {desc}
                    </div>
                  ))}
                </div>
                <h3>Features</h3>
                <ul>
                  <li><Sparkles size={14} strokeWidth={1.5} style={{ verticalAlign: -2 }} /> AI-powered code completion (Ollama ready)</li>
                  <li><Bot size={14} strokeWidth={1.5} style={{ verticalAlign: -2 }} /> Multiple AI providers: Ollama, Claude, ChatGPT, Gemini, Grok</li>
                  <li><Rocket size={14} strokeWidth={1.5} style={{ verticalAlign: -2 }} /> Fast text editing with Rust backend</li>
                  <li><Plug size={14} strokeWidth={1.5} style={{ verticalAlign: -2 }} /> VSCode + JetBrains + Neovim plugin support</li>
                </ul>
              </div>
            </div>
          )}
        </main>

        {/* AI Panel — grouped sidebar + lazy-loaded panels */}
        {showAIChat && (
          <>
            <div
              className="resizer-vertical"
              onMouseDown={(e) => { e.preventDefault(); startResizing('aipanel'); }}
            />
            <aside className="ai-chat-panel" style={{ display: "flex", flexDirection: "row", width: `${aiPanelWidth}px` }}>
              {showFilterBar && (
                <GroupedTabBar activeTab={aiPanelTab} onTabChange={setAiPanelTab} onCollapse={() => setShowFilterBar(false)} />
              )}
              <div role="tabpanel" aria-labelledby={`ai-tab-${aiPanelTab}`} style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }}>
                {!showFilterBar && (
                  <div style={{ display: "flex", alignItems: "center", gap: 4, padding: "4px 8px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: 12 }}>
                    <button
                      onClick={() => setShowFilterBar(true)}
                      style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: 12, padding: "2px 6px" }}
                      title="Show filter panel"
                    >
                      ☰ Panels
                    </button>
                    <span style={{ color: "var(--text-secondary)", opacity: 0.5 }}>|</span>
                    <span style={{ color: "var(--text-primary)", fontWeight: 500 }}>{(TAB_META[aiPanelTab] || DEFAULT_TAB_META).label}</span>
                  </div>
                )}
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
                      window.dispatchEvent(new CustomEvent("vibeui:inject-context", { detail: text }));
                    }}
                    collab={collab}
                  />
                </div>
              </div>
            </aside>
          </>
        )}
      </div>

      {/* Bottom Panel (Terminal / Browser) */}
      {showTerminal && (
        <>
          <div
            className="resizer-horizontal"
            onMouseDown={(e) => {
              e.preventDefault();
              startResizing('terminal');
            }}
          />
          <div className="terminal-panel" style={{ height: `${terminalHeight}px`, borderTop: 'none', display: 'flex', flexDirection: 'column' }}>
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
                onClick={() => setShowTerminal(false)}
                style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', padding: '4px 10px', fontSize: '16px' }}
                title="Close panel"
                aria-label="Close panel"
              >×</button>
            </div>
            {/* Panel content */}
            <div style={{ flex: 1, overflow: 'hidden' }}>
              {bottomTab === 'terminal' && <Terminal onClose={() => setShowTerminal(false)} />}
              {bottomTab === 'browser' && <BrowserPanel />}
            </div>
          </div>
        </>
      )}

      {/* Status Bar */}
      <footer className="status-bar">
        <div className="status-left">
          <span>VibeUI v{appVersion}</span>
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
          {currentFile && <span>• {editorLanguage}</span>}
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
          {currentFile && (
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
      {/* Inline Chat Overlay (Cmd+K) */}
      {inlineChat && (
        <InlineChat
          selection={inlineChat.selection}
          position={inlineChat.position}
          provider={selectedProvider}
          fileContent={editorContent}
          onAccept={(newText) => {
            const editor = editorRef.current;
            const isGenerate = !inlineChat.selection.text.trim();
            if (editor) {
              const model = editor.getModel();
              if (model) {
                const sel = inlineChat.selection;
                if (isGenerate) {
                  // Generate mode: insert at cursor line
                  const insertLine = sel.startLine + 1;
                  const col = model.getLineMaxColumn(insertLine);
                  editor.executeEdits("inline-generate", [{
                    range: { startLineNumber: insertLine, startColumn: col, endLineNumber: insertLine, endColumn: col },
                    text: "\n" + newText,
                    forceMoveMarkers: true,
                  }]);
                } else {
                  // Edit mode: replace selection
                  const range = {
                    startLineNumber: sel.startLine + 1,
                    startColumn: 1,
                    endLineNumber: sel.endLine + 1,
                    endColumn: model.getLineMaxColumn(sel.endLine + 1),
                  };
                  editor.executeEdits("inline-chat", [{
                    range,
                    text: newText,
                    forceMoveMarkers: true,
                  }]);
                }
              }
            }
            flowContext.add({
              kind: isGenerate ? "inline_generate" : "inline_edit",
              summary: isGenerate
                ? `Generated code at line ${inlineChat.selection.startLine + 1} in ${inlineChat.selection.filePath.split("/").pop() ?? "file"}`
                : `Inline edit in ${inlineChat.selection.filePath.split("/").pop() ?? "file"} (lines ${inlineChat.selection.startLine + 1}–${inlineChat.selection.endLine + 1})`,
              detail: isGenerate
                ? `Generated:\n${newText.slice(0, 400)}`
                : `Original:\n${inlineChat.selection.text.slice(0, 400)}\n\nReplaced with:\n${newText.slice(0, 400)}`,
              filePath: inlineChat.selection.filePath,
            });
            supercompleteEngine.invalidate();
            setInlineChat(null);
          }}
          onReject={() => setInlineChat(null)}
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
      {showTour && workspaceFolders.length > 0 && (
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
            <SettingsPanel onClose={() => setShowSettingsModal(false)} />
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
            minWidth: '120px',
          }}
        >
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
        </div>
      )}
    </div>
  );
}

export default App;
