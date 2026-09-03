/**
 * DesignImportPanel — bring a design in and turn it into code.
 *
 * Two real sources, both backed by generators that already exist:
 *  - an image (drop, browse, or paste) → `generate_app_from_image`
 *  - a Figma file URL → `import_figma`, using the token the Design Hub stores
 *
 * The previous version of this panel recorded an import without performing
 * one: dropping a file called `create_design_import` with the file's *name*,
 * never its bytes, and the history it wrote reported `0 components` for every
 * entry and was discarded when the window closed. Nothing below reports a
 * result it did not get from a generator.
 */
import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../hooks/useToast";
import { Toaster } from "./Toaster";
import { GeneratedFileList, type GeneratedFile } from "./design/GeneratedFileList";
import { loadFigmaToken } from "../lib/figmaToken";

type Tab = "import" | "result" | "history";

const TABS: { id: Tab; label: string }[] = [
  { id: "import", label: "Import" },
  { id: "result", label: "Result" },
  { id: "history", label: "History" },
];

/** Frameworks `generate_app_from_image` has instructions for. */
const FRAMEWORKS = [
  { value: "react", label: "React (TSX)" },
  { value: "vue", label: "Vue (SFC)" },
  { value: "svelte", label: "Svelte" },
  { value: "nextjs", label: "Next.js" },
  { value: "html", label: "HTML / CSS / JS" },
];

const ACCEPTED_IMAGE_TYPES = ["image/png", "image/jpeg", "image/jpg", "image/webp"];

interface DesignImportRecord {
  id: string;
  name: string;
  framework: string;
  source: string;
  created_at: string;
  files: string[];
  written: string[];
}

interface Props {
  workspacePath?: string | null;
  provider?: string;
  onOpenFile?: (path: string, line?: number) => void;
}

const dropZoneStyle = (active: boolean): React.CSSProperties => ({
  border: `2px dashed ${active ? "var(--accent-blue)" : "var(--border-color)"}`,
  background: active ? "var(--bg-elevated, var(--bg-secondary))" : "var(--bg-secondary)",
  borderRadius: "var(--radius-sm-alt)",
  padding: "var(--space-6)",
  textAlign: "center",
  color: "var(--text-secondary)",
  cursor: "pointer",
  marginBottom: "var(--space-3)",
  transition: "background 0.12s, border-color 0.12s",
});

/** Render an ISO timestamp in the viewer's locale; show it raw if unparseable. */
function formatWhen(iso: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? iso : at.toLocaleString();
}

const DesignImportPanel: React.FC<Props> = ({ workspacePath = null, provider = "", onOpenFile }) => {
  const { toasts, toast, dismiss } = useToast();
  const [tab, setTab] = useState<Tab>("import");
  const [framework, setFramework] = useState("react");
  const [figmaUrl, setFigmaUrl] = useState("");
  const [history, setHistory] = useState<DesignImportRecord[]>([]);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [dragActive, setDragActive] = useState(false);

  const [imageBase64, setImageBase64] = useState<string | null>(null);
  const [imageMime, setImageMime] = useState("image/png");
  const [imagePreview, setImagePreview] = useState<string | null>(null);
  const [imageName, setImageName] = useState("");

  const [generating, setGenerating] = useState(false);
  const [files, setFiles] = useState<GeneratedFile[]>([]);
  const [lastRecordId, setLastRecordId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshHistory = useCallback(async () => {
    try {
      const rows = await invoke<DesignImportRecord[]>("design_import_history");
      setHistory(Array.isArray(rows) ? rows : []);
      setHistoryError(null);
    } catch (e) {
      // An unreadable history is not an empty history — say which it is.
      setHistoryError(String(e));
    }
  }, []);

  useEffect(() => { void refreshHistory(); }, [refreshHistory]);

  const loadImage = useCallback((file: File) => {
    if (!ACCEPTED_IMAGE_TYPES.includes(file.type)) {
      setError(`${file.name || "That file"} is ${file.type || "an unknown type"} — use PNG, JPG, or WEBP.`);
      return;
    }
    setError(null);
    setFiles([]);
    setImageName(file.name || "Dropped image");
    setImageMime(file.type === "image/jpg" ? "image/jpeg" : file.type);
    const reader = new FileReader();
    reader.onerror = () => setError(`Could not read ${file.name}.`);
    reader.onload = () => {
      const dataUrl = String(reader.result ?? "");
      setImagePreview(dataUrl);
      setImageBase64(dataUrl.replace(/^data:image\/\w+;base64,/, ""));
    };
    reader.readAsDataURL(file);
  }, []);

  const recordImport = useCallback(
    async (name: string, source: string, generated: GeneratedFile[], usedFramework: string) => {
      try {
        const record = await invoke<DesignImportRecord>("design_import_record", {
          name,
          framework: usedFramework,
          source,
          files: generated.map((f) => f.path),
          written: [],
        });
        setLastRecordId(record.id);
        await refreshHistory();
      } catch (e) {
        // The import itself succeeded; only the bookkeeping failed. Saying so
        // beats a toast that implies the generated files are gone.
        toast.warn(`Import succeeded but was not added to history: ${e}`);
      }
    },
    [refreshHistory, toast],
  );

  const generateFromImage = async () => {
    if (!imageBase64) return;
    if (!provider) {
      setError("No provider selected — pick one in the toolbar dropdown.");
      return;
    }
    setGenerating(true);
    setError(null);
    setFiles([]);
    setLastRecordId(null);
    try {
      const result = await invoke<GeneratedFile[]>("generate_app_from_image", {
        imageBase64,
        mediaType: imageMime,
        framework,
        provider,
      });
      setFiles(result);
      setTab("result");
      toast.success(`${result.length} file(s) generated from ${imageName}`);
      await recordImport(imageName || "Image import", "image", result, framework);
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const generateFromFigma = async () => {
    const url = figmaUrl.trim();
    if (!url) return;
    if (!provider) {
      setError("No provider selected — pick one in the toolbar dropdown.");
      return;
    }
    setGenerating(true);
    setError(null);
    setFiles([]);
    setLastRecordId(null);
    try {
      const token = await loadFigmaToken();
      if (!token) {
        setError("No Figma token saved. Add one on the Hub tab under Figma, then import here.");
        return;
      }
      const result = await invoke<GeneratedFile[]>("import_figma", {
        url,
        token,
        workspacePath: workspacePath ?? "",
        workspace_path: workspacePath ?? "",
        provider,
      });
      setFiles(result);
      setTab("result");
      toast.success(`${result.length} component(s) generated from Figma`);
      // `import_figma` always generates React components — recording the
      // framework picker's value would claim an output nobody produced.
      await recordImport(url, "figma", result, "react");
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const noteWritten = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      toast.success(`Wrote ${paths.length} file(s) into the workspace`);
      if (!lastRecordId) return;
      try {
        // Persisted, not just reflected in local state: a "written" count that
        // vanishes on the next launch is a claim the history cannot back up.
        await invoke("design_import_mark_written", { id: lastRecordId, paths });
        await refreshHistory();
      } catch (e) {
        toast.warn(`Files were written, but history was not updated: ${e}`);
      }
    },
    [lastRecordId, refreshHistory, toast],
  );

  const forget = async (id?: string) => {
    try {
      const rows = await invoke<DesignImportRecord[]>("design_import_forget", { id: id ?? null });
      setHistory(rows);
      setHistoryError(null);
    } catch (e) {
      toast.error(`Could not update history: ${e}`);
    }
  };

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    const file = Array.from(e.dataTransfer?.files ?? [])[0];
    if (!file) {
      setError("That drop carried no file.");
      return;
    }
    loadImage(file);
  };

  const openFilePicker = () => document.getElementById("design-import-file")?.click();

  return (
    <div className="panel-container" role="region" aria-label="Design Import Panel">
      <div className="panel-tab-bar" role="tablist" aria-label="Design Import tabs">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            aria-selected={tab === t.id}
            className={`panel-tab ${tab === t.id ? "active" : ""}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
            {t.id === "result" && files.length > 0 && ` (${files.length})`}
          </button>
        ))}
      </div>

      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {error && (
          <div role="alert" style={{ background: "color-mix(in srgb, var(--error-color) 12%, transparent)", color: "var(--error-color)", padding: "var(--space-2) var(--space-3)", borderRadius: "var(--radius-xs-plus)", marginBottom: "var(--space-3)", whiteSpace: "pre-wrap", fontSize: "var(--font-size-base)" }}>
            {error}
          </div>
        )}

        {tab === "import" && (
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)", flexWrap: "wrap" }}>
              <label className="panel-label" htmlFor="design-import-framework" style={{ marginBottom: 0 }}>Framework:</label>
              <select
                id="design-import-framework"
                className="panel-input"
                style={{ minWidth: 160 }}
                value={framework}
                onChange={(e) => setFramework(e.target.value)}
                aria-label="Select framework"
              >
                {FRAMEWORKS.map((f) => <option key={f.value} value={f.value}>{f.label}</option>)}
              </select>
              <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                (image imports only — Figma import always generates React)
              </span>
              <span style={{ fontSize: "var(--font-size-sm)", color: provider ? "var(--text-secondary)" : "var(--warning-color)" }}>
                {provider ? `Model: ${provider}` : "⚠ Pick a provider in the toolbar dropdown."}
              </span>
            </div>

            <div
              role="button"
              tabIndex={0}
              aria-label="Drop zone for design files"
              style={dropZoneStyle(dragActive)}
              onDragOver={(e) => { e.preventDefault(); e.stopPropagation(); if (!dragActive) setDragActive(true); }}
              onDragEnter={(e) => { e.preventDefault(); e.stopPropagation(); setDragActive(true); }}
              onDragLeave={(e) => { e.preventDefault(); e.stopPropagation(); setDragActive(false); }}
              onDrop={handleDrop}
              onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openFilePicker(); } }}
              onClick={openFilePicker}
            >
              {imagePreview ? (
                <div>
                  <img src={imagePreview} alt={`Preview of ${imageName}`} style={{ maxWidth: "100%", maxHeight: 220, borderRadius: "var(--radius-xs-plus)" }} />
                  <div style={{ fontSize: "var(--font-size-sm)", marginTop: "var(--space-2)" }}>{imageName}</div>
                </div>
              ) : (
                <>
                  <div style={{ fontSize: "var(--font-size-xl)", marginBottom: "var(--space-2)" }}>
                    {dragActive ? "Release to load" : "Drop an image here"}
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)" }}>PNG, JPG, or WEBP — or click to browse</div>
                </>
              )}
              <input
                id="design-import-file"
                type="file"
                accept="image/png,image/jpeg,image/webp"
                style={{ display: "none" }}
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  if (file) loadImage(file);
                  e.target.value = "";
                }}
              />
            </div>

            <div style={{ display: "flex", gap: "var(--space-2)", marginBottom: "var(--space-4)", flexWrap: "wrap" }}>
              <button
                type="button"
                className="panel-btn panel-btn-primary"
                onClick={generateFromImage}
                disabled={!imageBase64 || generating || !provider}
                aria-label="Generate from image"
              >
                {generating ? "Generating…" : "Generate from Image"}
              </button>
              {imagePreview && (
                <button
                  type="button"
                  className="panel-btn panel-btn-secondary"
                  onClick={() => { setImageBase64(null); setImagePreview(null); setImageName(""); setError(null); }}
                >
                  Clear image
                </button>
              )}
            </div>

            <div style={{ borderTop: "1px solid var(--border-color)", paddingTop: "var(--space-4)" }}>
              <label className="panel-label" htmlFor="design-import-figma-url">Or import a Figma file</label>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
                <input
                  id="design-import-figma-url"
                  className="panel-input"
                  style={{ flex: 1 }}
                  placeholder="https://www.figma.com/file/…"
                  value={figmaUrl}
                  onChange={(e) => setFigmaUrl(e.target.value)}
                  aria-label="Figma URL input"
                />
                <button
                  type="button"
                  className="panel-btn panel-btn-primary"
                  aria-label="Import design"
                  onClick={generateFromFigma}
                  disabled={!figmaUrl.trim() || generating || !provider}
                >
                  {generating ? "Importing…" : "Import"}
                </button>
              </div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: "var(--space-2)" }}>
                Uses the personal access token saved on the Hub tab. Nothing is written to the
                workspace until you choose the files on the Result tab.
              </div>
            </div>
          </div>
        )}

        {tab === "result" && (
          files.length === 0 ? (
            <div className="panel-empty">
              Nothing generated yet. Run an import on the Import tab.
            </div>
          ) : (
            <GeneratedFileList
              files={files}
              workspacePath={workspacePath}
              onWritten={noteWritten}
              onError={setError}
              onOpenFile={onOpenFile}
            />
          )
        )}

        {tab === "history" && (
          <>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "var(--space-3)", gap: "var(--space-2)" }}>
              <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
                {history.length} import{history.length === 1 ? "" : "s"} recorded
              </span>
              <button
                type="button"
                className="panel-btn panel-btn-secondary panel-btn-sm"
                onClick={() => forget()}
                disabled={history.length === 0}
              >
                Clear history
              </button>
            </div>
            {historyError && (
              <div role="alert" style={{ color: "var(--error-color)", fontSize: "var(--font-size-sm)", marginBottom: "var(--space-3)" }}>
                Could not read import history: {historyError}
              </div>
            )}
            {!historyError && history.length === 0 && <div className="panel-empty">No imports yet.</div>}
            {history.map((h) => (
              <div key={h.id} className="panel-card" style={{ marginBottom: "var(--space-2)", padding: "var(--space-3)" }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
                  <strong style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{h.name}</strong>
                  <span className="panel-tag">{h.source}</span>
                </div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
                  {h.framework} &middot; {h.files.length} file{h.files.length === 1 ? "" : "s"} generated
                  {h.written.length > 0 && ` · ${h.written.length} written`}
                  {" · "}{formatWhen(h.created_at)}
                </div>
                <button
                  type="button"
                  className="panel-btn panel-btn-secondary panel-btn-sm"
                  style={{ marginTop: "var(--space-2)" }}
                  onClick={() => forget(h.id)}
                >
                  Forget
                </button>
              </div>
            ))}
          </>
        )}
      </div>
      <Toaster toasts={toasts} onDismiss={dismiss} />
    </div>
  );
};

export default DesignImportPanel;
