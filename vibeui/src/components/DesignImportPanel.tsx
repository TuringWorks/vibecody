/**
 * DesignImportPanel — import designs from Figma URLs or dropped images and
 * generate framework components.
 *
 * Tabs: Import, Preview, History
 */
import React, { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../hooks/useToast";
import { Toaster } from "./Toaster";

type Tab = "Import" | "Preview" | "History";
const TABS: Tab[] = ["Import", "Preview", "History"];

const FRAMEWORKS = ["React", "Vue", "Svelte", "Angular", "HTML/CSS", "React Native"];

const dropZoneStyle = (active: boolean): React.CSSProperties => ({
  border: `2px dashed ${active ? "var(--accent-blue)" : "var(--border-color)"}`,
  background: active ? "var(--bg-elevated, var(--bg-secondary))" : "transparent",
  borderRadius: "var(--radius-sm-alt)",
  padding: "var(--space-8)",
  textAlign: "center",
  color: "var(--text-secondary)",
  cursor: "pointer",
  marginBottom: "var(--space-3)",
  transition: "background 0.12s, border-color 0.12s",
});

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)",
  background: color,
  color: "var(--bg-primary)",
  fontWeight: 600,
});

interface DesignImport {
  id: number;
  name: string;
  framework: string;
  source: string;
  date: string;
  components: number;
}

const DesignImportPanel: React.FC = () => {
  const { toasts, toast, dismiss } = useToast();
  const [tab, setTab] = useState<Tab>("Import");
  const [framework, setFramework] = useState("React");
  const [figmaUrl, setFigmaUrl] = useState("");
  const [history, setHistory] = useState<DesignImport[]>([]);
  const [dragActive, setDragActive] = useState(false);

  useEffect(() => {
    invoke<DesignImport[]>("list_design_imports")
      .then(setHistory)
      .catch((e) => toast.error(`Failed to load history: ${e}`));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const submitImport = useCallback(async (source: "Figma" | "Image", name: string) => {
    try {
      const result = await invoke<DesignImport>("create_design_import", {
        name, framework, source,
      });
      setHistory((prev) => [result, ...prev]);
      toast.success(`${source} import created — ${result.components} component(s)`);
      setTab("History");
    } catch (e) {
      toast.error(`Import failed: ${e}`);
    }
  }, [framework, toast]);

  const handleImport = async () => {
    if (!figmaUrl) return;
    const isFigma = figmaUrl.includes("figma.com");
    await submitImport(isFigma ? "Figma" : "Image", isFigma ? "Figma Import" : "URL Import");
    if (!toast) return; // appease ts
    setFigmaUrl("");
  };

  const handleDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    if (!dragActive) setDragActive(true);
  };

  const handleDragLeave = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
  };

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (files.length === 0) {
      toast.warn("No files in drop");
      return;
    }
    const file = files[0];
    void submitImport("Image", file.name || "Dropped Image");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    // Allow keyboard activation: Enter/Space on the focused drop zone clicks
    // the underlying file input via the labeled control.
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      const fileInput = document.getElementById("design-import-file") as HTMLInputElement | null;
      fileInput?.click();
    }
  };

  const handleFileInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    void submitImport("Image", file.name);
    e.target.value = "";
  };

  return (
    <div className="panel-container" role="region" aria-label="Design Import Panel">
      <div className="panel-tab-bar" role="tablist" aria-label="Design Import tabs">
        {TABS.map((t) => (
          <button
            key={t}
            type="button"
            role="tab"
            aria-selected={tab === t}
            className={`panel-tab ${tab === t ? "active" : ""}`}
            onClick={() => setTab(t)}
          >
            {t}
          </button>
        ))}
      </div>
      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {tab === "Import" && (
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-4)" }}>
              <label className="panel-label" htmlFor="design-import-framework" style={{ marginBottom: 0 }}>Framework:</label>
              <select
                id="design-import-framework"
                className="panel-input"
                style={{ minWidth: 140 }}
                value={framework}
                onChange={(e) => setFramework(e.target.value)}
                aria-label="Select framework"
              >
                {FRAMEWORKS.map((f) => <option key={f} value={f}>{f}</option>)}
              </select>
            </div>

            <div
              role="button"
              tabIndex={0}
              aria-label="Drop zone for design files"
              style={dropZoneStyle(dragActive)}
              onDragOver={handleDragOver}
              onDragEnter={handleDragOver}
              onDragLeave={handleDragLeave}
              onDrop={handleDrop}
              onKeyDown={handleKeyDown}
              onClick={() => document.getElementById("design-import-file")?.click()}
            >
              <div style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-2)" }}>
                {dragActive ? "Release to import" : "Drop image here"}
              </div>
              <div style={{ fontSize: "var(--font-size-base)" }}>PNG, JPG, SVG, or screenshot — or click to browse</div>
              <input
                id="design-import-file"
                type="file"
                accept="image/png,image/jpeg,image/svg+xml,image/webp"
                style={{ display: "none" }}
                onChange={handleFileInput}
              />
            </div>

            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)" }}>
              <input
                className="panel-input"
                style={{ flex: 1 }}
                placeholder="Or paste Figma URL..."
                value={figmaUrl}
                onChange={(e) => setFigmaUrl(e.target.value)}
                aria-label="Figma URL input"
              />
              <button
                type="button"
                className="panel-btn panel-btn-primary"
                aria-label="Import design"
                onClick={handleImport}
                disabled={!figmaUrl.trim()}
              >
                Import
              </button>
            </div>
          </div>
        )}

        {tab === "Preview" && (
          <div className="panel-empty">
            <div style={{ fontSize: "var(--font-size-2xl)", marginBottom: "var(--space-2)" }}>No active preview</div>
            <div style={{ fontSize: "var(--font-size-base)" }}>Import a design to see the generated component preview here</div>
          </div>
        )}

        {tab === "History" && history.length === 0 && (
          <div className="panel-empty">No imports yet.</div>
        )}
        {tab === "History" && history.map((h) => (
          <div key={h.id} className="panel-card" style={{ marginBottom: "var(--space-2)" }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "var(--space-1)" }}>
              <strong>{h.name}</strong>
              <span style={badgeStyle("var(--info-color)")}>{h.source}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
              {h.framework} &middot; {h.components} components &middot; {h.date}
            </div>
          </div>
        ))}
      </div>
      <Toaster toasts={toasts} onDismiss={dismiss} />
    </div>
  );
};

export default DesignImportPanel;
