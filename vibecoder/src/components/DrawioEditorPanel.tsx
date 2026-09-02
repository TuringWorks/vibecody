/**
 * DrawioEditorPanel — draw.io diagrams, as a document you open, edit and save.
 *
 * # What this used to be, and why it changed
 *
 * The panel had every part of a diagram editor except the file. It opened blank
 * every time with no way to reach a diagram that already existed; its Save wrote
 * to a hard-coded `<workspace>/diagrams/diagram.drawio`, so the second diagram
 * you made silently replaced the first, and the toolbar said "Saved to
 * workspace" without ever naming the file. It could not export at all. Its eight
 * templates all returned the same single labelled rectangle. And the embedded
 * editor showed a **Save & Exit** button that saved but never exited, because
 * the `exit` event had no handler on this side — a button that does half of what
 * it says.
 *
 * The model here is now a document: `currentPath` is the file you are editing,
 * shown in the toolbar at all times, and every action names where its bytes went.
 *
 * # The three coordinate systems of "what is the current XML"
 *
 * 1. What the user sees in the iframe.
 * 2. What this component holds in `diagramXml`.
 * 3. What is on disk.
 *
 * (1) and (2) drift unless the editor pushes changes, which is what `autosave=1`
 * in the editor URL is for — the previous URL omitted it while the message
 * handler already had an `autosave` branch, so that branch never ran and Save
 * could write a diagram several edits stale. `dirty` tracks (2) against (3).
 */
import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";
import { extractFencedBlock, prepareDrawioXml } from "../utils/llmOutput";

interface DrawioEditorPanelProps {
  workspacePath: string | null;
  provider: string;
}

type DioTab = "files" | "editor" | "preview" | "generate" | "templates" | "mcp";

const TAB_DEFS: { id: DioTab; label: string }[] = [
  { id: "files", label: "Diagrams" },
  { id: "editor", label: "Editor" },
  { id: "preview", label: "Preview" },
  { id: "generate", label: "AI Generate" },
  { id: "templates", label: "Templates" },
  { id: "mcp", label: "MCP Bridge" },
];

const DIAGRAM_KINDS = [
  "flowchart", "sequence", "class_diagram", "entity_relationship",
  "component_diagram", "deployment_diagram", "c4_context", "c4_container",
  "c4_component", "architecture", "state_machine", "network_topology",
];

/** An empty but valid document, so a new diagram opens on a real canvas. */
const BLANK_DIAGRAM =
  '<mxfile host="VibeCody" type="device"><diagram name="Page-1" id="page-1">' +
  '<mxGraphModel dx="1100" dy="800" grid="1" gridSize="10" guides="1" tooltips="1" ' +
  'connect="1" arrows="1" fold="1" page="1" pageScale="1" pageWidth="1169" pageHeight="826">' +
  '<root><mxCell id="0"/><mxCell id="1" parent="0"/></root></mxGraphModel></diagram></mxfile>';

/** Where a new diagram lands unless the user names somewhere else. */
const DEFAULT_DIR = "diagrams";

/** One diagram in the workspace, as `list_drawio_files` reports it. */
interface DrawioFile {
  path: string;
  name: string;
  size_bytes: number;
  /** Absent where the filesystem does not report one — never defaulted to now. */
  modified_unix: number | null;
  /** Absent when the file was too large to count during the listing. */
  pages: number | null;
  vertices: number | null;
  edges: number | null;
  is_embedded_export: boolean;
}

/** Where a save or export actually went. */
interface DrawioSaved {
  path: string;
  absolute_path: string;
  size_bytes: number;
  created: boolean;
}

interface DrawioTemplate {
  id: string;
  label: string;
  kind: string;
  summary: string;
}

/** Exportable formats, with the extension each one writes. */
const EXPORT_FORMATS = [
  { format: "png", ext: "png", label: "PNG" },
  { format: "svg", ext: "svg", label: "SVG" },
  { format: "xmlsvg", ext: "drawio.svg", label: "SVG (editable)" },
  { format: "pdf", ext: "pdf", label: "PDF" },
] as const;

type ExportFormat = (typeof EXPORT_FORMATS)[number]["format"];

/** A status line the panel shows: what happened, and whether it worked. */
type Status =
  | { kind: "idle" }
  | { kind: "busy"; text: string }
  | { kind: "ok"; text: string }
  | { kind: "error"; text: string };

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

/** "3 minutes ago" — or nothing at all when the filesystem gave no mtime. */
function formatWhen(unix: number | null): string | null {
  if (unix === null || unix === undefined) return null;
  const secs = Math.max(0, Math.floor(Date.now() / 1000) - unix);
  if (secs < 60) return "just now";
  if (secs < 3600) return `${Math.floor(secs / 60)} min ago`;
  if (secs < 86400) return `${Math.floor(secs / 3600)} h ago`;
  return `${Math.floor(secs / 86400)} d ago`;
}

/**
 * Structure counts as a phrase, or an explicit "not counted".
 *
 * `pages === null` means the listing did not read the file, which is different
 * from a diagram with no pages. Rendering the absent case as `0 pages` would
 * describe a large diagram as an empty one.
 */
function describeStructure(f: DrawioFile): string {
  if (f.pages === null) return "not counted (large file)";
  const shapes = (f.vertices ?? 0) + (f.edges ?? 0);
  return `${f.pages} page${f.pages === 1 ? "" : "s"} · ${shapes} shape${shapes === 1 ? "" : "s"}`;
}

/** Swap a path's extension, keeping its directory and stem. */
function withExtension(path: string, ext: string): string {
  const stem = path.replace(/\.(drawio\.svg|drawio\.xml|drawio|dio)$/i, "");
  return `${stem}.${ext}`;
}

export function DrawioEditorPanel({ workspacePath, provider }: DrawioEditorPanelProps) {
  const [activeTab, setActiveTab] = useState<DioTab>("files");

  // ── The document ──────────────────────────────────────────────────────
  const [diagramXml, setDiagramXml] = useState("");
  /** Workspace-relative path being edited. Null until the file has a name. */
  const [currentPath, setCurrentPath] = useState<string | null>(null);
  /** `diagramXml` differs from what is on disk. */
  const [dirty, setDirty] = useState(false);

  const [files, setFiles] = useState<DrawioFile[]>([]);
  const [filesError, setFilesError] = useState<string | null>(null);
  const [loadingFiles, setLoadingFiles] = useState(false);
  const [filter, setFilter] = useState("");

  const [templates, setTemplates] = useState<DrawioTemplate[]>([]);
  const [templatesError, setTemplatesError] = useState<string | null>(null);

  const [genDescription, setGenDescription] = useState("");
  const [genKind, setGenKind] = useState("flowchart");
  const [isGenerating, setIsGenerating] = useState(false);
  const [generatedXml, setGeneratedXml] = useState("");
  const [generateWarning, setGenerateWarning] = useState<string | null>(null);

  const [mcpFilePath, setMcpFilePath] = useState("");
  const [mcpCommand, setMcpCommand] = useState("read_file");
  const [mcpResult, setMcpResult] = useState("");

  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [saveAsOpen, setSaveAsOpen] = useState(false);
  const [saveAsName, setSaveAsName] = useState("");

  const editorRef = useRef<HTMLIFrameElement>(null);
  const previewRef = useRef<HTMLIFrameElement>(null);

  // The XML to inject when the embedded editor sends `init`. A ref, not a
  // closure over `diagramXml`: the listener is attached once and the iframe
  // re-mounts on every tab switch, so a closure would capture a stale value and
  // a per-change listener would leak handlers — the cause of the old "editor
  // loads partial or wrong XML" bug.
  const xmlToLoadRef = useRef<string>("");
  useEffect(() => { xmlToLoadRef.current = diagramXml; }, [diagramXml]);

  // What an in-flight export is for. The editor's `export` reply does not carry
  // the destination, so the request has to remember it.
  const pendingExportRef = useRef<{ path: string; label: string } | null>(null);

  // Refs so the message handler — attached once — always sees current values
  // without being torn down and rebuilt on every keystroke.
  const currentPathRef = useRef<string | null>(null);
  useEffect(() => { currentPathRef.current = currentPath; }, [currentPath]);
  const dirtyRef = useRef(false);
  useEffect(() => { dirtyRef.current = dirty; }, [dirty]);

  const showOk = useCallback((text: string) => {
    setStatus({ kind: "ok", text });
    setTimeout(() => setStatus((s) => (s.kind === "ok" ? { kind: "idle" } : s)), 6000);
  }, []);
  const showError = useCallback((text: string) => setStatus({ kind: "error", text }), []);

  // ── Editor embed ──────────────────────────────────────────────────────
  //
  // `autosave=1` — the editor pushes every change back, so Save writes what is
  //   on screen rather than the last time the user pressed the editor's own
  //   Save. The old URL omitted it while this file already had an `autosave`
  //   branch that therefore never ran.
  // `noExitBtn=1&saveAndExit=0` — no "Save & Exit". It saved and did not exit,
  //   because nothing here handled the `exit` event, and there is nothing to
  //   exit *to*: this is a tab, not a modal opened over a document. The `exit`
  //   handler below still exists in case a future embed build shows the button
  //   anyway; it closes the document rather than doing nothing.
  const editorSrc =
    "https://embed.diagrams.net/?embed=1&ui=dark&spin=1&proto=json&configure=1" +
    "&autosave=1&noExitBtn=1&saveAndExit=0&modified=unsavedChanges";

  const closeDocument = useCallback(() => {
    setDiagramXml("");
    setCurrentPath(null);
    setDirty(false);
    setActiveTab("files");
  }, []);

  // ── Loading a document ────────────────────────────────────────────────

  const refreshFiles = useCallback(async () => {
    if (!workspacePath) return;
    setLoadingFiles(true);
    setFilesError(null);
    try {
      setFiles(await invoke<DrawioFile[]>("list_drawio_files", { workspacePath }));
    } catch (e) {
      // Named, not swallowed. A silent empty list is indistinguishable from a
      // workspace with no diagrams, and sends the user looking for the wrong
      // problem.
      setFilesError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoadingFiles(false);
    }
  }, [workspacePath]);

  useEffect(() => { void refreshFiles(); }, [refreshFiles]);

  useEffect(() => {
    invoke<DrawioTemplate[]>("list_drawio_templates")
      .then(setTemplates)
      .catch((e: unknown) =>
        // The list comes from the backend precisely so the tab cannot advertise
        // a template this build does not have. If it cannot be read, show
        // nothing and say why rather than falling back to a hard-coded list.
        setTemplatesError(e instanceof Error ? e.message : String(e)),
      );
  }, []);

  const openFile = useCallback(
    async (file: DrawioFile) => {
      if (!workspacePath) return;
      if (dirtyRef.current && !window.confirm(
        `${currentPathRef.current ?? "This diagram"} has unsaved changes. Open ${file.name} and lose them?`,
      )) return;
      setStatus({ kind: "busy", text: `Opening ${file.name}…` });
      try {
        const xml = await invoke<string>("read_drawio_file", {
          workspacePath,
          relativePath: file.path,
        });
        setDiagramXml(xml);
        // An editable export opens read-only-ish: saving back into it is
        // refused by the backend, so it needs a new name. Leaving `currentPath`
        // set would make Save look available and then fail.
        setCurrentPath(file.is_embedded_export ? null : file.path);
        setDirty(false);
        setActiveTab("editor");
        showOk(
          file.is_embedded_export
            ? `Opened the diagram inside ${file.name}. Save it as a .drawio file to keep edits.`
            : `Opened ${file.path}`,
        );
      } catch (e) {
        showError(e instanceof Error ? e.message : String(e));
      }
    },
    [workspacePath, showOk, showError],
  );

  const newDiagram = useCallback(() => {
    if (dirtyRef.current && !window.confirm(
      `${currentPathRef.current ?? "This diagram"} has unsaved changes. Start a new one and lose them?`,
    )) return;
    setDiagramXml(BLANK_DIAGRAM);
    setCurrentPath(null);
    setDirty(true);
    setActiveTab("editor");
  }, []);

  // ── Saving ────────────────────────────────────────────────────────────

  const writeTo = useCallback(
    async (relativePath: string, xml: string) => {
      if (!workspacePath) return;
      setStatus({ kind: "busy", text: `Saving ${relativePath}…` });
      try {
        const saved = await invoke<DrawioSaved>("save_drawio_file", {
          workspacePath,
          relativePath,
          xml,
        });
        setCurrentPath(saved.path);
        setDirty(false);
        // Name the file. "Saved to workspace" was true and useless — it is the
        // question this panel most often left the user holding.
        showOk(
          `${saved.created ? "Created" : "Saved"} ${saved.path} · ${formatBytes(saved.size_bytes)}`,
        );
        void refreshFiles();
      } catch (e) {
        // The old call was `.catch(() => {})` followed by an unconditional
        // "Saved to workspace" — a failed write reported as a success.
        showError(e instanceof Error ? e.message : String(e));
      }
    },
    [workspacePath, showOk, showError, refreshFiles],
  );

  const save = useCallback(() => {
    if (!diagramXml) return;
    if (!currentPath) { setSaveAsOpen(true); return; }
    void writeTo(currentPath, diagramXml);
  }, [currentPath, diagramXml, writeTo]);

  const saveAs = useCallback(() => {
    const raw = saveAsName.trim();
    if (!raw) return;
    // Default the directory and the extension so a bare "auth-flow" works, but
    // never rewrite a path the user spelled out.
    const withDir = raw.includes("/") ? raw : `${DEFAULT_DIR}/${raw}`;
    const named = /\.(drawio|dio|drawio\.xml)$/i.test(withDir) ? withDir : `${withDir}.drawio`;
    setSaveAsOpen(false);
    setSaveAsName("");
    void writeTo(named, diagramXml);
  }, [saveAsName, diagramXml, writeTo]);

  // ── Exporting ─────────────────────────────────────────────────────────

  const requestExport = useCallback(
    (format: ExportFormat, ext: string, label: string) => {
      if (!diagramXml) return;
      const base = currentPath ?? `${DEFAULT_DIR}/untitled.drawio`;
      const target = withExtension(base, ext);
      pendingExportRef.current = { path: target, label };
      setStatus({ kind: "busy", text: `Exporting ${label}…` });
      editorRef.current?.contentWindow?.postMessage(
        JSON.stringify({ action: "export", format, xml: diagramXml, spin: "Exporting" }),
        "*",
      );
    },
    [diagramXml, currentPath],
  );

  // ── The embed's message protocol ──────────────────────────────────────

  useEffect(() => {
    async function handleExport(dataUrl: string) {
      const pending = pendingExportRef.current;
      pendingExportRef.current = null;
      if (!pending || !workspacePath) return;
      try {
        const saved = await invoke<DrawioSaved>("export_drawio_file", {
          workspacePath,
          relativePath: pending.path,
          dataUrl,
        });
        // The whole point of the feature: the file is in the workspace and the
        // user is told where. draw.io's own export menu downloads through the
        // browser, which the Tauri webview does not surface at all — an export
        // that goes nowhere is an export that did not happen.
        showOk(`Exported ${saved.path} · ${formatBytes(saved.size_bytes)}`);
        void refreshFiles();
      } catch (e) {
        showError(e instanceof Error ? e.message : String(e));
      }
    }

    function onMessage(event: MessageEvent) {
      const fromEditor = event.source === editorRef.current?.contentWindow;
      const fromPreview = event.source === previewRef.current?.contentWindow;
      if (!fromEditor && !fromPreview) return;
      if (!event.data || typeof event.data !== "string") return;
      let msg: Record<string, unknown>;
      try {
        msg = JSON.parse(event.data);
      } catch {
        return; // not the embed protocol
      }
      const target = fromEditor ? editorRef.current : previewRef.current;

      if (msg.event === "configure") {
        // `configure=1` makes drawio block until the host replies; without this
        // branch `init` never arrives and the editor stays blank forever.
        target?.contentWindow?.postMessage(
          JSON.stringify({ action: "configure", config: {} }),
          "*",
        );
        return;
      }

      if (msg.event === "init") {
        const raw = xmlToLoadRef.current || BLANK_DIAGRAM;
        const prep = prepareDrawioXml(raw);
        if (!prep.ok && prep.warning) {
          // Say it on screen. A rejected load leaves drawio showing an empty
          // canvas with no error of its own, so silence here reads as "the
          // editor is broken".
          setStatus({ kind: "error", text: `Could not load the diagram: ${prep.warning}` });
        }
        target?.contentWindow?.postMessage(
          JSON.stringify({ action: "load", xml: prep.prepared || raw, autosave: 1 }),
          "*",
        );
        return;
      }

      if (!fromEditor) return; // the preview is read-only; nothing else applies

      if (msg.event === "export" && typeof msg.data === "string") {
        void handleExport(msg.data);
        return;
      }

      if (msg.event === "autosave" && typeof msg.xml === "string") {
        setDiagramXml(msg.xml);
        setDirty(true);
        return;
      }

      if (msg.event === "save" && typeof msg.xml === "string") {
        // The editor's own Save (and Ctrl+S, and File → Save) land here. Write
        // to disk — anything less makes the button a lie.
        setDiagramXml(msg.xml);
        const path = currentPathRef.current;
        if (path) void writeTo(path, msg.xml);
        else setSaveAsOpen(true);
        return;
      }

      if (msg.event === "exit") {
        // Reachable only if a future embed build shows an exit control despite
        // `noExitBtn=1`. Closing the document is the honest meaning of "exit"
        // for a panel that is a tab; doing nothing is what made "Save & Exit"
        // a button that half-worked.
        if (dirtyRef.current && !window.confirm("Close this diagram and lose unsaved changes?")) return;
        closeDocument();
      }
    }

    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, [workspacePath, writeTo, showOk, showError, refreshFiles, closeDocument]);

  // Ctrl/Cmd+S anywhere in the panel, not only inside the iframe.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        save();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [save]);

  // ── AI generation ─────────────────────────────────────────────────────

  const handleGenerate = async () => {
    if (!genDescription.trim() || !workspacePath) return;
    setIsGenerating(true);
    setGeneratedXml("");
    setGenerateWarning(null);
    setStatus({ kind: "busy", text: "Generating…" });
    try {
      const raw = await invoke<string>("generate_drawio_xml", {
        description: genDescription,
        kind: genKind,
        workspacePath,
        provider,
      });
      // LLMs habitually wrap the XML in ```xml … ``` and bracket it with prose.
      // Both break the preview, which expects raw mxGraphModel XML.
      const result = extractFencedBlock(raw);
      if (!result) {
        showError("The model returned no diagram XML. Try a shorter description.");
        return;
      }
      setGeneratedXml(result);
      setStatus({ kind: "idle" });
    } catch (e) {
      showError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsGenerating(false);
    }
  };

  const loadGeneratedInEditor = () => {
    // Validate before navigating — most "editor stays blank" reports trace back
    // to truncated LLM output (a max-token limit hit mid-XML). Say so instead
    // of letting drawio silently reject the load.
    const prep = prepareDrawioXml(generatedXml);
    if (!prep.ok && prep.warning) { setGenerateWarning(prep.warning); return; }
    setGenerateWarning(null);
    setDiagramXml(generatedXml);
    setCurrentPath(null);
    setDirty(true);
    setActiveTab("editor");
  };

  // ── Templates ─────────────────────────────────────────────────────────

  const loadTemplate = async (templateId: string) => {
    try {
      const xml = await invoke<string>("get_drawio_template", { templateId });
      setDiagramXml(xml);
      setCurrentPath(null);
      setDirty(true);
      setActiveTab("editor");
      showOk("Template opened — Save to name it");
    } catch (e) {
      // Previously this fell back to building a placeholder box, so a missing
      // template looked like a working one.
      showError(e instanceof Error ? e.message : String(e));
    }
  };

  // ── MCP bridge ────────────────────────────────────────────────────────

  const executeMcpCommand = async () => {
    if (!mcpFilePath.trim() || !workspacePath) { showError("Enter a workspace-relative file path."); return; }
    try {
      setMcpResult(
        await invoke<string>("execute_drawio_mcp", {
          command: mcpCommand,
          filePath: mcpFilePath,
          workspacePath,
          content: mcpCommand === "write_file" ? diagramXml : undefined,
        }),
      );
    } catch (e) {
      setMcpResult(e instanceof Error ? e.message : String(e));
    }
  };

  // ── Render ────────────────────────────────────────────────────────────

  const visibleFiles = useMemo(() => {
    const q = filter.trim().toLowerCase();
    return q ? files.filter((f) => f.path.toLowerCase().includes(q)) : files;
  }, [files, filter]);

  const statusBar = status.kind === "idle" ? null : (
    <div
      role={status.kind === "error" ? "alert" : "status"}
      style={{
        padding: "6px 12px",
        fontSize: "var(--font-size-sm)",
        borderTop: "1px solid var(--border-color)",
        background: status.kind === "error" ? "var(--bg-secondary)" : "transparent",
        color:
          status.kind === "error"
            ? "var(--error-color, #e53e3e)"
            : status.kind === "ok"
              ? "var(--text-success)"
              : "var(--text-secondary)",
        flexShrink: 0,
        display: "flex",
        gap: 8,
        alignItems: "center",
      }}
    >
      <span>{status.kind === "ok" ? "✓" : status.kind === "error" ? "⚠" : "…"}</span>
      <span style={{ flex: 1 }}>{status.text}</span>
      {status.kind === "error" && (
        <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => setStatus({ kind: "idle" })}>
          Dismiss
        </button>
      )}
    </div>
  );

  const renderFiles = () => (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
      <div style={{ padding: "8px 12px", display: "flex", gap: 8, alignItems: "center", borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
        <input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter diagrams…"
          style={{ flex: 1, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "6px 10px", fontSize: "var(--font-size-base)" }}
        />
        <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => void refreshFiles()} disabled={loadingFiles}>
          {loadingFiles ? "Scanning…" : "Refresh"}
        </button>
        <button className="panel-btn panel-btn-primary panel-btn-sm" onClick={newDiagram}>New diagram</button>
      </div>
      <div style={{ flex: 1, overflow: "auto", padding: 12 }}>
        {filesError && (
          <div role="alert" style={{ color: "var(--error-color, #e53e3e)", fontSize: "var(--font-size-base)", marginBottom: 12 }}>
            ⚠ Could not list diagrams: {filesError}
          </div>
        )}
        {!filesError && visibleFiles.length === 0 && !loadingFiles && (
          <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", lineHeight: 1.7, padding: 12 }}>
            {files.length === 0
              ? "No .drawio, .dio or .drawio.svg files in this workspace yet. Start one with New diagram, a template, or AI Generate — saving puts it in "
              : "No diagram matches that filter."}
            {files.length === 0 && <code>{DEFAULT_DIR}/</code>}
            {files.length === 0 && " unless you name somewhere else."}
          </div>
        )}
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 10 }}>
          {visibleFiles.map((f) => {
            const when = formatWhen(f.modified_unix);
            return (
              <button
                key={f.path}
                onClick={() => void openFile(f)}
                title={`Open ${f.path}`}
                style={{
                  background: currentPath === f.path ? "var(--accent-blue)" : "var(--bg-secondary)",
                  border: `1px solid ${currentPath === f.path ? "var(--accent-blue)" : "var(--border-color)"}`,
                  borderRadius: "var(--radius-sm-alt)",
                  padding: 12,
                  cursor: "pointer",
                  color: currentPath === f.path ? "var(--btn-primary-fg)" : "inherit",
                  textAlign: "left",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                  <Icon name="chart-bar" size={14} />
                  <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {f.name}
                  </span>
                  {f.is_embedded_export && (
                    <span style={{ fontSize: "var(--font-size-xs, 10px)", opacity: 0.7, border: "1px solid currentColor", borderRadius: 3, padding: "0 4px" }}>
                      export
                    </span>
                  )}
                </div>
                <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.75, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {f.path}
                </div>
                <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.6, marginTop: 4 }}>
                  {describeStructure(f)} · {formatBytes(f.size_bytes)}
                  {/* No timestamp at all where the filesystem reported none. */}
                  {when ? ` · ${when}` : ""}
                </div>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );

  const renderEditor = () => (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      <div style={{ padding: "8px 12px", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", display: "flex", gap: 8, alignItems: "center", flexShrink: 0, flexWrap: "wrap" }}>
        {/* The question this panel could never answer: which file is this? */}
        <span
          title={currentPath ?? "Not saved yet — Save will ask for a name"}
          style={{ fontSize: "var(--font-size-base)", fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace", color: currentPath ? "var(--text-primary)" : "var(--text-secondary)", maxWidth: 380, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
        >
          {currentPath ?? "untitled — not saved"}
        </span>
        {dirty && <span title="Unsaved changes" style={{ color: "var(--accent-blue)", fontSize: 18, lineHeight: "18px" }}>•</span>}

        <span style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
          <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => setActiveTab("preview")} disabled={!diagramXml}>
            Preview
          </button>
          {EXPORT_FORMATS.map((f) => (
            <button
              key={f.format}
              className="panel-btn panel-btn-secondary panel-btn-sm"
              onClick={() => requestExport(f.format, f.ext, f.label)}
              disabled={!diagramXml}
              title={`Export ${f.label} into ${withExtension(currentPath ?? `${DEFAULT_DIR}/untitled.drawio`, f.ext)}`}
            >
              {f.label}
            </button>
          ))}
          <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => { setSaveAsName(""); setSaveAsOpen(true); }} disabled={!diagramXml}>
            Save as…
          </button>
          <button className="panel-btn panel-btn-primary panel-btn-sm" onClick={save} disabled={!diagramXml || (!dirty && !!currentPath)}>
            {currentPath ? "Save" : "Save…"}
          </button>
        </span>
      </div>

      {saveAsOpen && (
        <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", display: "flex", gap: 8, alignItems: "center", flexShrink: 0 }}>
          <label htmlFor="drawio-save-as" style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
            Save as
          </label>
          <input
            id="drawio-save-as"
            autoFocus
            value={saveAsName}
            onChange={(e) => setSaveAsName(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") saveAs(); if (e.key === "Escape") setSaveAsOpen(false); }}
            placeholder={`e.g. auth-flow  →  ${DEFAULT_DIR}/auth-flow.drawio`}
            style={{ flex: 1, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "6px 10px", fontSize: "var(--font-size-base)", fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace" }}
          />
          <button className="panel-btn panel-btn-primary panel-btn-sm" onClick={saveAs} disabled={!saveAsName.trim()}>Save</button>
          <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => setSaveAsOpen(false)}>Cancel</button>
        </div>
      )}

      <iframe
        ref={editorRef}
        src={editorSrc}
        title="Draw.io Editor"
        sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
        style={{ flex: 1, border: "none" }}
      />
    </div>
  );

  const renderPreview = () => (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
      {diagramXml ? (
        <>
          <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", flexShrink: 0, display: "flex", gap: 8 }}>
            <span>{currentPath ?? "untitled — not saved"}</span>
            <button className="panel-btn panel-btn-secondary panel-btn-sm" style={{ marginLeft: "auto" }} onClick={() => setActiveTab("editor")}>
              Edit
            </button>
          </div>
          {/* The diagram is delivered by postMessage, not in the URL. The old
              preview encoded the whole XML into a `#R…` fragment, which stops
              working somewhere past a few thousand shapes — and fails by
              rendering a blank frame rather than saying anything. */}
          <iframe
            ref={previewRef}
            src="https://viewer.diagrams.net/?embed=1&proto=json&configure=1&nav=1&layers=1&spin=1&highlight=0000ff"
            title="Diagram Preview"
            sandbox="allow-scripts allow-same-origin allow-popups"
            style={{ flex: 1, border: "none", background: "var(--btn-primary-fg)" }}
          />
        </>
      ) : (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 12, color: "var(--text-secondary)" }}>
          <Icon name="chart-bar" size={48} style={{ opacity: 0.3 }} />
          <div style={{ fontSize: "var(--font-size-lg)" }}>No diagram open</div>
          <div style={{ fontSize: "var(--font-size-base)", maxWidth: 320, textAlign: "center", lineHeight: 1.6 }}>
            Open one from Diagrams, start a template, or generate one with AI.
          </div>
        </div>
      )}
    </div>
  );

  const renderGenerate = () => (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", padding: 20, gap: 12 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)" }}>AI Diagram Generation</div>
      <label htmlFor="dio-kind" style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Diagram Type</label>
      <select
        id="dio-kind"
        value={genKind}
        onChange={(e) => setGenKind(e.target.value)}
        style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "8px 12px", fontSize: "var(--font-size-md)", marginBottom: 14 }}
      >
        {DIAGRAM_KINDS.map((k) => (
          <option key={k} value={k}>{k.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}</option>
        ))}
      </select>
      <label htmlFor="dio-desc" style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>Description</label>
      <textarea
        id="dio-desc"
        value={genDescription}
        onChange={(e) => setGenDescription(e.target.value)}
        placeholder="Describe the diagram you want to generate..."
        rows={6}
        style={{ width: "100%", resize: "vertical", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: 10, fontSize: "var(--font-size-md)", boxSizing: "border-box" }}
      />
      <button
        className="panel-btn"
        onClick={handleGenerate}
        disabled={isGenerating || !genDescription.trim()}
        style={{ width: "100%", marginTop: 8, background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isGenerating || !genDescription.trim() ? 0.5 : 1 }}
      >
        {isGenerating ? "Generating…" : "Generate Diagram"}
      </button>
      {generatedXml && (
        <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, gap: 8 }}>
          <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
            <button className="panel-btn panel-btn-secondary" onClick={() => { setDiagramXml(generatedXml); setCurrentPath(null); setDirty(true); setActiveTab("preview"); }} style={{ flex: 1 }}>
              View Preview
            </button>
            <button className="panel-btn panel-btn-primary" onClick={loadGeneratedInEditor} style={{ flex: 1 }}>Open in Editor</button>
          </div>
          {generateWarning && (
            <div role="alert" style={{ fontSize: "var(--font-size-sm)", color: "var(--error-color, #e53e3e)", background: "var(--bg-secondary)", border: "1px solid var(--error-color, #e53e3e)", borderRadius: "var(--radius-sm)", padding: "8px 10px", flexShrink: 0 }}>
              ⚠ {generateWarning}
            </div>
          )}
          <textarea
            value={generatedXml}
            onChange={(e) => setGeneratedXml(e.target.value)}
            spellCheck={false}
            aria-label="Generated diagram XML"
            style={{ width: "100%", flex: 1, minHeight: 200, fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace", fontSize: "var(--font-size-sm)", color: "var(--text-primary)", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 10, border: "1px solid var(--border-color)", resize: "none", boxSizing: "border-box", whiteSpace: "pre", tabSize: 2 }}
          />
        </div>
      )}
    </div>
  );

  const renderTemplates = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 4 }}>Diagram Templates</div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 12 }}>
        Each opens a real starter diagram in the editor. Save gives it a name.
      </div>
      {templatesError && (
        <div role="alert" style={{ color: "var(--error-color, #e53e3e)", fontSize: "var(--font-size-base)" }}>
          ⚠ Could not read the template list: {templatesError}
        </div>
      )}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))", gap: 10 }}>
        {templates.map((t) => (
          <button
            key={t.id}
            onClick={() => void loadTemplate(t.id)}
            style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: 16, cursor: "pointer", color: "inherit", textAlign: "left" }}
          >
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 4 }}>{t.label}</div>
            <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.75, lineHeight: 1.5 }}>{t.summary}</div>
            <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.5, marginTop: 6 }}>{t.kind.replace(/_/g, " ")}</div>
          </button>
        ))}
      </div>
    </div>
  );

  const renderMcp = () => (
    <div style={{ flex: 1, overflow: "auto", padding: 20, maxWidth: 600, margin: "0 auto" }}>
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-xl)", marginBottom: 4 }}>drawio-mcp Bridge</div>
      <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16, lineHeight: 1.6 }}>
        Inspect and write diagram files by path. Paths are workspace-relative and resolved inside the open workspace.
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
        {["read_file", "write_file", "list_pages", "export_svg"].map((cmd) => (
          <button
            key={cmd}
            onClick={() => setMcpCommand(cmd)}
            style={{ background: mcpCommand === cmd ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "4px 12px", cursor: "pointer", color: mcpCommand === cmd ? "var(--btn-primary-fg)" : "inherit", fontSize: "var(--font-size-sm)", fontWeight: mcpCommand === cmd ? 600 : 400 }}
          >
            {cmd}
          </button>
        ))}
      </div>
      <label htmlFor="dio-mcp-path" style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>File path (workspace-relative)</label>
      <input
        id="dio-mcp-path"
        value={mcpFilePath}
        onChange={(e) => setMcpFilePath(e.target.value)}
        placeholder="diagrams/architecture.drawio"
        style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: "8px 12px", fontSize: "var(--font-size-md)", marginBottom: 12, boxSizing: "border-box" }}
      />
      <button
        className="panel-btn"
        onClick={executeMcpCommand}
        style={{ width: "100%", background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "12px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)" }}
      >
        Execute {mcpCommand}
      </button>
      {mcpResult && (
        <pre style={{ marginTop: 16, fontSize: "var(--font-size-base)", overflow: "auto", maxHeight: 400, background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: 12, border: "1px solid var(--border-color)", color: "var(--text-primary)", whiteSpace: "pre-wrap" }}>
          {mcpResult}
        </pre>
      )}
    </div>
  );

  if (!workspacePath) {
    return <div className="empty-state"><p>Open a workspace to use the Draw.io editor.</p></div>;
  }

  return (
    <div className="panel-container">
      <div className="panel-tab-bar" style={{ overflow: "auto" }}>
        {TAB_DEFS.map(({ id, label }) => (
          <button className={`panel-tab${activeTab === id ? " active" : ""}`} key={id} onClick={() => setActiveTab(id)}>
            {label}
            {id === "editor" && dirty && <span title="Unsaved changes" style={{ marginLeft: 4, color: "var(--accent-blue)" }}>•</span>}
          </button>
        ))}
      </div>
      <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        {activeTab === "files" && renderFiles()}
        {activeTab === "editor" && renderEditor()}
        {activeTab === "preview" && renderPreview()}
        {activeTab === "generate" && renderGenerate()}
        {activeTab === "templates" && renderTemplates()}
        {activeTab === "mcp" && renderMcp()}
      </div>
      {statusBar}
    </div>
  );
}
