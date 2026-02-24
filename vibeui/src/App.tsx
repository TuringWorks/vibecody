import { useState, useEffect, useRef, useCallback } from "react";
import Editor, { DiffEditor, OnMount } from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { AIChat } from "./components/AIChat";
import { AgentPanel } from "./components/AgentPanel";
import { MemoryPanel } from "./components/MemoryPanel";
import { Terminal } from "./components/Terminal";
import { detectLanguage, getFileIcon } from "./utils/fileUtils";
import "./App.css";
import { ThemeToggle } from "./components/ThemeToggle";
import { CommandPalette, Command } from "./components/CommandPalette";
import Modal from "./components/Modal";
import { GitPanel } from "./components/GitPanel";
import { MarkdownPreview } from "./components/MarkdownPreview";
import { FilePlus, FolderPlus, FolderOpen, Files, Search, GitGraph, Settings } from "lucide-react";
import "./ActivityBar.css";
import { ExtensionManager } from "./extensions/ExtensionManager";
// Import worker using Vite's syntax
import ExtensionHostWorker from "./extensions/ExtensionHost?worker";

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
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const [workspaceFolders, setWorkspaceFolders] = useState<string[]>([]);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [aiProviders, setAiProviders] = useState<string[]>([]);
  const [selectedProvider, setSelectedProvider] = useState<string>("");
  const [showSidebar, setShowSidebar] = useState(true);
  const [activeSidebarTab, setActiveSidebarTab] = useState<"explorer" | "search" | "git">("explorer");
  const [showAIChat, setShowAIChat] = useState(false);
  const [aiPanelTab, setAiPanelTab] = useState<"chat" | "agent" | "memory">("chat");
  const [showTerminal, setShowTerminal] = useState(false);
  const [showCommandPalette, setShowCommandPalette] = useState(false);

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [modalConfig, setModalConfig] = useState<{
    title: string;
    placeholder: string;
    onConfirm: (value: string) => void;
  }>({ title: '', placeholder: '', onConfirm: () => { } });
  const [currentDirectory, setCurrentDirectory] = useState<string | null>(null);
  const [pendingDiff, setPendingDiff] = useState<{ path: string; original: string; modified: string } | null>(null);

  // Search state
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  // Git state
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);

  // Context Menu
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; file: FileEntry } | null>(null);

  // Resizable Panes State
  const [sidebarWidth, setSidebarWidth] = useState(250);
  const [terminalHeight, setTerminalHeight] = useState(200);
  const [isResizing, setIsResizing] = useState<'sidebar' | 'terminal' | null>(null);

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
    invoke<string[]>("get_available_ai_providers")
      .then((providers) => {
        setAiProviders(providers);
        if (providers.length > 0 && !selectedProvider) {
          // Default to first Ollama model if available, otherwise first provider
          const defaultProvider = providers.find(p => p.startsWith("Ollama")) || providers[0];
          setSelectedProvider(defaultProvider);
        }
      })
      .catch(console.error);

    // Load workspace folders
    invoke<string[]>("get_workspace_folders")
      .then(setWorkspaceFolders)
      .catch(console.error);

    // Initialize Extension Manager
    const manager = new ExtensionManager({
      showInformationMessage: (message) => {
        console.log(`[Extension Info] ${message}`);
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
      console.log("Extension Manager initialized");
    } catch (e) {
      console.error("Failed to initialize extension worker:", e);
    }
  }, []);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd+K (Mac) or Ctrl+K (Windows/Linux) to open command palette
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setShowCommandPalette(true);
      }
      // Cmd+B (Mac) or Ctrl+B (Windows/Linux) to toggle sidebar
      if ((e.metaKey || e.ctrlKey) && e.key === 'b') {
        e.preventDefault();
        setShowSidebar(prev => !prev);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const openFolder = async () => {
    console.log("openFolder called");
    try {
      console.log("Calling dialog.open...");
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Open Folder",
      });

      console.log("Dialog result:", selected);

      if (selected && typeof selected === 'string') {
        console.log("Selected folder:", selected);
        await invoke("add_workspace_folder", { path: selected });
        setWorkspaceFolders([...workspaceFolders, selected]);
        loadDirectory(selected);
      } else if (selected === null) {
        // User cancelled the dialog
        console.log("Folder selection cancelled");
      } else {
        console.log("Unexpected dialog result type:", typeof selected, selected);
      }
    } catch (error) {
      console.error("Failed to open folder:", error);
      alert(`Failed to open folder: ${error}\n\nMake sure the dialog plugin is properly configured.\n\nError details: ${JSON.stringify(error)}`);
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
      console.log("Git status fetch failed (maybe not a git repo):", error);
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
      alert("Failed to open file: " + error);
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
      console.log("File saved successfully");

      // Update dirty state
      setOpenFiles(prev => prev.map(f =>
        f.path === activeFilePath ? { ...f, isDirty: false } : f
      ));
    } catch (error) {
      console.error("Failed to save file:", error);
      alert("Failed to save file: " + error);
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

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    // Register LSP providers
    // Note: We register for all languages for now, or we could be more specific
    // Ideally we should only register once per language, but for simplicity we'll do it on mount
    // and maybe check if already registered? Monaco doesn't easily let us check.
    // A better place might be a useEffect that runs once, but we need the 'monaco' instance.
    // For now, let's just register generic providers that check the language ID.

    const getRootPath = () => workspaceFolders[0] || ""; // Simple assumption for MVP

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

          try {
            const completion = await invoke<string>("request_inline_completion", {
              prefix,
              suffix,
              language,
              provider,
            });
            if (!completion) return { items: [] };
            return { items: [{ insertText: completion }] };
          } catch {
            return { items: [] };
          }
        },
        freeInlineCompletions: () => {},
      });
    }

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
          .catch(err => console.error("Failed to update cursors:", err));
      }, 100); // Debounce 100ms
    });
  };

  const handlePendingWrite = async (path: string, content: string) => {
    console.log("DEBUG: handlePendingWrite called for path:", path);
    try {
      // Read current file content for diff
      let original = "";
      try {
        original = await invoke<string>("read_file", { path });
        console.log("DEBUG: Original content loaded, length:", original.length);
      } catch (e) {
        // File might not exist yet
        console.log("File does not exist, creating new file diff");
      }

      setPendingDiff({
        path,
        original,
        modified: content
      });
      console.log("DEBUG: setPendingDiff called");

      // Ensure file is open (or at least active context)
      // For diff view, we might not need to add it to tabs yet, 
      // but if accepted it should be.
      // For now, let's just make sure we track it if we accept.
    } catch (error) {
      console.error("Failed to prepare diff:", error);
    }
  };

  const acceptDiff = async () => {
    if (!pendingDiff) return;
    try {
      // Phase 3: Stash current working-tree changes before applying AI edits
      // so the user can pop the stash to undo if needed.
      if (workspaceFolders[0]) {
        invoke("git_stash_create", {
          path: workspaceFolders[0],
          name: `pre-ai-${pendingDiff.path.split('/').pop()}-${Date.now()}`,
        }).catch(() => {}); // best-effort — ignore if repo has nothing to stash
      }

      await invoke("write_file", { path: pendingDiff.path, content: pendingDiff.modified });
      console.log("Changes saved to disk for:", pendingDiff.path);

      setPendingDiff(null);
      setPendingDiff(null);

      // Update or add to open files
      const filename = pendingDiff.path.split('/').pop() || pendingDiff.path.split('\\').pop() || '';
      const language = detectLanguage(filename);

      setOpenFiles(prev => {
        const exists = prev.some(f => f.path === pendingDiff.path);
        if (exists) {
          return prev.map(f => f.path === pendingDiff.path ? { ...f, content: pendingDiff.modified, isDirty: false } : f);
        } else {
          return [...prev, { path: pendingDiff.path, content: pendingDiff.modified, language, isDirty: false }];
        }
      });
      setActiveFilePath(pendingDiff.path);

      // Optional: Show a small notification or just rely on the UI update
      // alert("Changes saved to disk!"); 
    } catch (error) {
      console.error("Failed to accept changes:", error);
      alert("Failed to save changes: " + error);
    }
  };

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
      alert("Please open a folder first.");
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

        console.log("DEBUG: Attempting to create file at:", path);

        try {
          await invoke("write_file", { path, content: "" });
          console.log("DEBUG: File created successfully");
          loadDirectory(currentDirectory);
          // Optionally open the new file
          openFile(path);
        } catch (error) {
          console.error("Failed to create file:", error);
          alert("Failed to create file: " + error);
        }
      }
    });
    setModalOpen(true);
  };

  const handleNewFolder = () => {
    if (!currentDirectory) {
      alert("Please open a folder first.");
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

        console.log("DEBUG: Attempting to create folder at:", path);

        try {
          await invoke("create_directory", { path });
          console.log("DEBUG: Folder created successfully");
          loadDirectory(currentDirectory);
        } catch (error) {
          console.error("Failed to create folder:", error);
          alert("Failed to create folder: " + error);
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
      alert("Search failed: " + error);
    } finally {
      setIsSearching(false);
    }
  };

  const handleSearchResultClick = async (result: SearchResult) => {
    await openFile(result.path);
    // TODO: Scroll to line number (requires editor ref)
  };

  // Define commands for command palette
  const commands: Command[] = [
    // File operations
    {
      id: 'file.openFolder',
      label: 'Open Folder',
      category: 'File',
      icon: '📁',
      shortcut: '⌘O',
      action: openFolder,
    },
    {
      id: 'file.save',
      label: 'Save File',
      category: 'File',
      icon: '💾',
      shortcut: '⌘S',
      action: saveFile,
    },
    {
      id: 'file.createFile',
      label: 'Create New File',
      category: 'File',
      icon: '📄',
      action: handleNewFile,
    },
    {
      id: 'file.createFolder',
      label: 'Create New Folder',
      category: 'File',
      icon: '📁',
      action: handleNewFolder,
    },
    // Editor actions
    {
      id: 'editor.toggleSidebar',
      label: 'Toggle Sidebar',
      category: 'Editor',
      icon: '☰',
      shortcut: '⌘B',
      action: () => setShowSidebar(prev => !prev),
    },
    {
      id: 'editor.toggleAIChat',
      label: 'Toggle AI Chat',
      category: 'Editor',
      icon: '💬',
      action: () => setShowAIChat(prev => !prev),
    },
    {
      id: 'editor.search',
      label: 'Search in Files',
      category: 'Editor',
      icon: '🔍',
      action: () => setActiveSidebarTab('search'),
    },
    // View
    {
      id: 'view.toggleTerminal',
      label: 'Toggle Terminal',
      category: 'View',
      icon: '⌨️',
      action: () => setShowTerminal(prev => !prev),
    },
    {
      id: 'view.explorer',
      label: 'Show Explorer',
      category: 'View',
      icon: '📂',
      action: () => {
        setShowSidebar(true);
        setActiveSidebarTab('explorer');
      },
    },
    // Debug
    {
      id: 'debug.loadTestExtension',
      label: 'Load Test Extension',
      category: 'Debug',
      icon: '🧩',
      action: () => {
        const code = `
          console.log('Hello from extension!');
          vscode.commands.registerCommand('extension.helloWorld', () => {
            vscode.window.showInformationMessage('Hello World from VibeUI Extension!');
          });
        `;
        extensionManagerRef.current?.loadExtension(code);
        console.log("Test extension loaded! Try running 'extension.helloWorld' command.");
        (window as any).lastExtensionMessage = "Test extension loaded";
      }
    },
    {
      id: 'extension.helloWorld',
      label: 'Hello World (Extension)',
      category: 'Extension',
      icon: '👋',
      action: () => {
        extensionManagerRef.current?.executeCommand('extension.helloWorld');
      }
    }
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
          alert(`Failed to rename: ${e}`);
        }
        setModalOpen(false);
      }
    });
    setModalOpen(true);
  };

  const handleDelete = async () => {
    if (!contextMenu) return;
    const file = contextMenu.file;
    setContextMenu(null);

    if (confirm(`Are you sure you want to delete ${file.name}?`)) {
      try {
        await invoke('delete_item', { path: file.path });
        if (currentDirectory) loadDirectory(currentDirectory);
        if (openFiles.some(f => f.path === file.path)) {
          closeFile(file.path);
        }
      } catch (e) {
        console.error("Failed to delete:", e);
        alert(`Failed to delete: ${e}`);
      }
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
  const startResizing = (type: 'sidebar' | 'terminal') => {
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
      {/* Header */}
      <header className="header">
        <div className="header-left">
          <button className="icon-button" onClick={() => setShowSidebar(!showSidebar)}>
            ☰
          </button>
          <h1 className="app-title">VibeUI</h1>
        </div>
        <div className="header-center">
          {currentFile && <span className="current-file">{currentFile}</span>}
        </div>
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
            onClick={() => {
              console.log("AI Chat button clicked, current state:", showAIChat);
              setShowAIChat(!showAIChat);
              console.log("AI Chat toggled to:", !showAIChat);
            }}
            title="Toggle AI Chat"
          >
            💬 AI Chat
          </button>
          <button className="btn-primary" onClick={saveFile} disabled={!currentFile}>
            💾 Save {currentFile && "(⌘S)"}
          </button>
          {currentFile && currentFile.endsWith('.md') && (
            <button
              className="btn-secondary"
              onClick={() => setShowMarkdownPreview(!showMarkdownPreview)}
            >
              {showMarkdownPreview ? '📝 Edit' : '👁️ Preview'}
            </button>
          )}
        </div>
      </header>

      <div className="main-container">
        {/* Activity Bar */}
        <div className="activity-bar">
          <div
            className={`activity-bar-item ${activeSidebarTab === 'explorer' && showSidebar ? 'active' : ''}`}
            onClick={() => {
              if (activeSidebarTab === 'explorer' && showSidebar) {
                setShowSidebar(false);
              } else {
                setActiveSidebarTab('explorer');
                setShowSidebar(true);
              }
            }}
            title="Explorer"
          >
            <Files size={24} />
          </div>
          <div
            className={`activity-bar-item ${activeSidebarTab === 'search' && showSidebar ? 'active' : ''}`}
            onClick={() => {
              if (activeSidebarTab === 'search' && showSidebar) {
                setShowSidebar(false);
              } else {
                setActiveSidebarTab('search');
                setShowSidebar(true);
              }
            }}
            title="Search"
          >
            <Search size={24} />
          </div>
          <div
            className={`activity-bar-item ${activeSidebarTab === 'git' && showSidebar ? 'active' : ''}`}
            onClick={() => {
              if (activeSidebarTab === 'git' && showSidebar) {
                setShowSidebar(false);
              } else {
                setActiveSidebarTab('git');
                setShowSidebar(true);
              }
            }}
            title="Source Control"
          >
            <GitGraph size={24} />
          </div>
          <div className="activity-bar-spacer" />
          <div className="activity-bar-item" title="Settings">
            <Settings size={24} />
          </div>
        </div>

        {/* Sidebar */}
        {showSidebar && (
          <aside className="sidebar" style={{ width: `${sidebarWidth}px` }}>
            {/* Removed old tabs */}

            {activeSidebarTab === 'explorer' && (
              <>
                <div className="sidebar-header">
                  <h2>Explorer</h2>
                  <div className="sidebar-actions">
                    <button className="btn-icon" onClick={handleNewFile} title="New File" disabled={!currentDirectory}>
                      <FilePlus size={18} />
                    </button>
                    <button className="btn-icon" onClick={handleNewFolder} title="New Folder" disabled={!currentDirectory}>
                      <FolderPlus size={18} />
                    </button>
                    <button className="btn-icon" onClick={openFolder} title="Open Folder">
                      <FolderOpen size={18} />
                    </button>
                  </div>
                </div>
                <div className="file-tree">
                  {currentDirectory && (
                    <div className="file-item directory" onClick={handleGoUp} title="Go to Parent">
                      <span className="file-icon">📂</span>
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
                        onClick={() => {
                          if (file.is_directory) {
                            loadDirectory(file.path);
                          } else {
                            openFile(file.path);
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
              <div className="search-panel" style={{ padding: '10px', display: 'flex', flexDirection: 'column', height: '100%' }}>
                <div className="search-input-container" style={{ display: 'flex', gap: '5px', marginBottom: '10px' }}>
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                    placeholder="Search..."
                    style={{ flex: 1, padding: '5px', background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)', color: 'var(--text-primary)' }}
                  />
                  <button onClick={handleSearch} className="btn-primary" disabled={isSearching}>
                    {isSearching ? '...' : 'Go'}
                  </button>
                </div>
                <div className="search-results" style={{ flex: 1, overflowY: 'auto' }}>
                  {searchResults.map((result, index) => (
                    <div
                      key={index}
                      className="search-result-item"
                      onClick={() => handleSearchResultClick(result)}
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
        <main className="editor-container">
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
                theme="vs-dark"
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
            <div className="diff-container" style={{ height: 'calc(100% - 35px)', display: 'flex', flexDirection: 'column' }}>
              <div className="diff-header" style={{ padding: '10px', background: '#252526', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span>Reviewing changes for: {pendingDiff.path}</span>
                <div className="diff-actions">
                  <button className="btn-secondary" onClick={rejectDiff} style={{ marginRight: '10px', background: '#d32f2f' }}>Reject</button>
                  <button className="btn-primary" onClick={acceptDiff} style={{ background: '#388e3c' }}>Accept</button>
                </div>
              </div>
              <DiffEditor
                height="100%"
                language={editorLanguage}
                theme="vs-dark"
                original={pendingDiff.original}
                modified={pendingDiff.modified}
                options={{
                  readOnly: true,
                  renderSideBySide: true
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
                theme="vs-dark"
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
                  📁 Open Folder
                </button>
              </div>
              <div className="features">
                <h3>Quick Start</h3>
                <ul>
                  <li>1️⃣ Click "Open Folder" to browse your project</li>
                  <li>2️⃣ Click on any file in the sidebar to open it</li>
                  <li>3️⃣ Edit your code with Monaco Editor</li>
                  <li>4️⃣ Save with ⌘S (Mac) or Ctrl+S (Windows/Linux)</li>
                </ul>
                <h3>Features</h3>
                <ul>
                  <li>✨ AI-powered code completion (Ollama ready)</li>
                  <li>💬 AI chat assistant (coming soon)</li>
                  <li>🚀 Fast text editing with Rust backend</li>
                  <li>🔌 VSCode plugin support (in development)</li>
                  <li>🤖 Multiple AI providers: Ollama, Claude, ChatGPT, Gemini, Grok</li>
                </ul>
                <p style={{ marginTop: '24px', fontSize: '13px', color: 'var(--text-secondary)' }}>
                  💡 Tip: Try opening the vibeUI folder to see this project's code!
                </p>
              </div>
            </div>
          )}
        </main>

        {/* AI Panel (Chat / Agent / Memory tabs) */}
        {showAIChat && (
          <aside className="ai-chat-panel" style={{ display: "flex", flexDirection: "column" }}>
            {/* Tab bar */}
            <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
              {(["chat", "agent", "memory"] as const).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setAiPanelTab(tab)}
                  style={{
                    flex: 1,
                    padding: "8px 4px",
                    fontSize: "12px",
                    background: "none",
                    border: "none",
                    borderBottom: aiPanelTab === tab ? "2px solid var(--accent-blue, #007acc)" : "2px solid transparent",
                    color: aiPanelTab === tab ? "var(--text-primary)" : "var(--text-secondary)",
                    cursor: "pointer",
                    fontWeight: aiPanelTab === tab ? 600 : 400,
                  }}
                >
                  {tab === "chat" ? "💬 Chat" : tab === "agent" ? "🤖 Agent" : "📋 Rules"}
                </button>
              ))}
            </div>

            {/* Tab content */}
            <div style={{ flex: 1, overflow: "hidden" }}>
              {aiPanelTab === "chat" && (
                <AIChat
                  provider={selectedProvider}
                  context={editorContent}
                  fileTree={files.map(f => f.path)}
                  currentFile={currentFile}
                  onPendingWrite={handlePendingWrite}
                />
              )}
              {aiPanelTab === "agent" && (
                <AgentPanel
                  provider={selectedProvider}
                  workspacePath={workspaceFolders[0] || null}
                />
              )}
              {aiPanelTab === "memory" && (
                <MemoryPanel
                  workspacePath={workspaceFolders[0] || null}
                />
              )}
            </div>
          </aside>
        )}
      </div>

      {/* Terminal Panel */}
      {showTerminal && (
        <>
          <div
            className="resizer-horizontal"
            onMouseDown={(e) => {
              e.preventDefault();
              startResizing('terminal');
            }}
          />
          <div className="terminal-panel" style={{ height: `${terminalHeight}px`, borderTop: 'none' }}>
            <Terminal onClose={() => setShowTerminal(false)} />
          </div>
        </>
      )}

      {/* Status Bar */}
      <footer className="status-bar">
        <div className="status-left">
          <span>VibeUI v0.1.0</span>
          {workspaceFolders.length > 0 && <span>• {workspaceFolders.length} folder(s)</span>}
          {currentFile && <span>• {editorLanguage}</span>}
          {gitStatus && (
            <span style={{ marginLeft: '10px', display: 'flex', alignItems: 'center', gap: '4px' }}>
              <span style={{ fontSize: '10px' }}>Branch:</span>
              <strong>{gitStatus.branch}</strong>
            </span>
          )}
        </div>
        <div className="status-right">
          <button
            className="status-item"
            onClick={() => setShowTerminal(!showTerminal)}
            style={{ background: 'none', border: 'none', color: 'inherit', cursor: 'pointer', marginRight: '10px' }}
          >
            {showTerminal ? 'Hide Terminal' : 'Show Terminal'}
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
