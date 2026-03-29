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

const containerStyle: React.CSSProperties = {
  display: "flex", flexDirection: "column", height: "100%",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  fontFamily: "inherit", overflow: "hidden",
};
const tabBarStyle: React.CSSProperties = {
  display: "flex", gap: 2, padding: "8px 12px 0",
  borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)",
  overflowX: "auto", flexShrink: 0,
};
const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px", cursor: "pointer",
  background: active ? "var(--bg-primary)" : "transparent",
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  border: "none", borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  fontSize: 13, fontFamily: "inherit", whiteSpace: "nowrap",
});
const contentStyle: React.CSSProperties = { flex: 1, overflow: "auto", padding: 16 };
const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 8,
  border: "1px solid var(--border-color)",
};
const btnStyle: React.CSSProperties = {
  padding: "6px 14px", background: "var(--accent-color)", color: "var(--bg-primary)",
  border: "none", borderRadius: 4, cursor: "pointer", fontSize: 12, fontFamily: "inherit",
};
const inputStyle: React.CSSProperties = {
  width: "100%", padding: "8px 12px", background: "var(--bg-tertiary)", color: "var(--text-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 13, fontFamily: "inherit",
  boxSizing: "border-box",
};
const dropZoneStyle: React.CSSProperties = {
  border: "2px dashed var(--border-color)", borderRadius: 8, padding: 40, textAlign: "center",
  color: "var(--text-secondary)", cursor: "pointer", marginBottom: 12,
};
const selectStyle: React.CSSProperties = {
  ...inputStyle, width: "auto", minWidth: 140,
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
    <div style={containerStyle} role="region" aria-label="Design Import Panel">
      <div style={tabBarStyle} role="tablist" aria-label="Design Import tabs">
        {TABS.map(t => (
          <button key={t} role="tab" aria-selected={tab === t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle} role="tabpanel" aria-label={tab}>
        {tab === "Import" && (
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16 }}>
              <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Framework:</label>
              <select style={selectStyle} value={framework} onChange={e => setFramework(e.target.value)} aria-label="Select framework">
                {FRAMEWORKS.map(f => <option key={f} value={f}>{f}</option>)}
              </select>
            </div>
            <div style={dropZoneStyle} role="button" aria-label="Drop zone for design files">
              <div style={{ fontSize: 24, marginBottom: 8 }}>Drop image here</div>
              <div style={{ fontSize: 12 }}>PNG, JPG, SVG, or screenshot</div>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <input style={{ ...inputStyle, flex: 1 }} placeholder="Or paste Figma URL..." value={figmaUrl} onChange={e => setFigmaUrl(e.target.value)} aria-label="Figma URL input" />
              <button style={btnStyle} aria-label="Import design" onClick={handleImport}>Import</button>
            </div>
          </div>
        )}
        {tab === "Preview" && (
          <div style={{ textAlign: "center", padding: 40, color: "var(--text-secondary)" }}>
            <div style={{ fontSize: 16, marginBottom: 8 }}>No active preview</div>
            <div style={{ fontSize: 12 }}>Import a design to see the generated component preview here</div>
          </div>
        )}
        {tab === "History" && history.map((h, i) => (
          <div key={i} style={cardStyle}>
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
