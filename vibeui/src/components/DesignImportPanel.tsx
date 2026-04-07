/**
 * DesignImportPanel — Import designs from Figma URLs or images and generate components.
 *
 * Tabs: Import, Preview, History
 */
import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Import" | "Preview" | "History";
const TABS: Tab[] = ["Import", "Preview", "History"];

const FRAMEWORKS = ["React", "Vue", "Svelte", "Angular", "HTML/CSS", "React Native"];

const dropZoneStyle: React.CSSProperties = {
  border: "2px dashed var(--border-color)", borderRadius: 8, padding: 40, textAlign: "center",
  color: "var(--text-secondary)", cursor: "pointer", marginBottom: 12,
};
const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: 10,
  fontSize: 11, background: color, color: "var(--bg-primary)", fontWeight: 600,
});

interface DesignImport { id: number; name: string; framework: string; source: string; date: string; components: number }

const DesignImportPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Import");
  const [framework, setFramework] = useState("React");
  const [figmaUrl, setFigmaUrl] = useState("");
  const [history, setHistory] = useState<DesignImport[]>([]);

  useEffect(() => {
    invoke<DesignImport[]>("list_design_imports").then(setHistory).catch(() => {});
  }, []);

  const handleImport = async () => {
    if (!figmaUrl) return;
    try {
      const name = figmaUrl.includes("figma.com") ? "Figma Import" : "URL Import";
      const source = figmaUrl.includes("figma.com") ? "Figma" : "Image";
      const result = await invoke<DesignImport>("create_design_import", { name, framework, source });
      setHistory(prev => [result, ...prev]);
      setFigmaUrl("");
      setTab("History");
    } catch (_) { /* ignore */ }
  };

  return (
    <div className="panel-container" role="region" aria-label="Design Import Panel">
      <div className="panel-tab-bar" role="tablist" aria-label="Design Import tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body" role="tabpanel" aria-label={tab}>
        {tab === "Import" && (
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16 }}>
              <label className="panel-label" style={{ marginBottom: 0 }}>Framework:</label>
              <select style={{ padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 13, fontFamily: "inherit", minWidth: 140 }} value={framework} onChange={e => setFramework(e.target.value)} aria-label="Select framework">
                {FRAMEWORKS.map(f => <option key={f} value={f}>{f}</option>)}
              </select>
            </div>
            <div style={dropZoneStyle} role="button" aria-label="Drop zone for design files">
              <div style={{ fontSize: 24, marginBottom: 8 }}>Drop image here</div>
              <div style={{ fontSize: 12 }}>PNG, JPG, SVG, or screenshot</div>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <input style={{ flex: 1, width: "100%", padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 13, fontFamily: "inherit", boxSizing: "border-box" }} placeholder="Or paste Figma URL..." value={figmaUrl} onChange={e => setFigmaUrl(e.target.value)} aria-label="Figma URL input" />
              <button className="panel-btn panel-btn-primary" aria-label="Import design" onClick={handleImport}>Import</button>
            </div>
          </div>
        )}
        {tab === "Preview" && (
          <div className="panel-empty">
            <div style={{ fontSize: 16, marginBottom: 8 }}>No active preview</div>
            <div style={{ fontSize: 12 }}>Import a design to see the generated component preview here</div>
          </div>
        )}
        {tab === "History" && history.map((h, i) => (
          <div key={i} className="panel-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
              <strong>{h.name}</strong>
              <span style={badgeStyle("var(--info-color)")}>{h.source}</span>
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              {h.framework} &middot; {h.components} components &middot; {h.date}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default DesignImportPanel;
