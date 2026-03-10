import React, { useState } from "react";

interface DirtyRegion {
  id: string;
  startLine: number;
  endLine: number;
  reason: string;
}

const RenderOptimizePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("stats");
  const [stats, setStats] = useState({ cacheHits: 847, cacheMisses: 153, totalFrames: 1000, avgReduction: 64 });
  const [frameWidth] = useState(1920);
  const [frameHeight] = useState(1080);
  const [dirtyRegions, setDirtyRegions] = useState<DirtyRegion[]>([
    { id: "r1", startLine: 12, endLine: 18, reason: "Text edit" },
    { id: "r2", startLine: 45, endLine: 45, reason: "Cursor blink" },
    { id: "r3", startLine: 102, endLine: 110, reason: "Scroll reveal" },
    { id: "r4", startLine: 200, endLine: 205, reason: "Diagnostic update" },
  ]);

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--vscode-foreground)",
    backgroundColor: "var(--vscode-editor-background)",
    fontFamily: "var(--vscode-font-family)", fontSize: "var(--vscode-font-size)",
    height: "100%", overflow: "auto",
  };
  const tabBar: React.CSSProperties = { display: "flex", gap: "4px", marginBottom: "16px", borderBottom: "1px solid var(--vscode-panel-border)" };
  const tab = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--vscode-tab-activeBackground)" : "transparent",
    color: active ? "var(--vscode-tab-activeForeground)" : "var(--vscode-tab-inactiveForeground)",
    borderBottom: active ? "2px solid var(--vscode-focusBorder)" : "2px solid transparent",
  });
  const btn: React.CSSProperties = {
    padding: "6px 14px", border: "none", borderRadius: "4px", cursor: "pointer",
    backgroundColor: "var(--vscode-button-background)", color: "var(--vscode-button-foreground)",
  };
  const card: React.CSSProperties = {
    padding: "12px", marginBottom: "8px", borderRadius: "6px",
    backgroundColor: "var(--vscode-editorWidget-background)", border: "1px solid var(--vscode-panel-border)",
  };

  const hitRate = stats.totalFrames > 0 ? Math.round((stats.cacheHits / stats.totalFrames) * 100) : 0;

  const handleForceRerender = () => {
    setDirtyRegions([]);
    setStats(prev => ({ ...prev, cacheMisses: prev.cacheMisses + 1, totalFrames: prev.totalFrames + 1 }));
  };

  const handleClearCache = () => {
    setStats({ cacheHits: 0, cacheMisses: 0, totalFrames: 0, avgReduction: 0 });
    setDirtyRegions([]);
  };

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Render Optimization</h3>
      <div style={tabBar}>
        {["stats", "frames", "config"].map(t => (
          <button key={t} style={tab(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "stats" && (
        <div>
          <div style={card}>
            <h4 style={{ margin: "0 0 8px" }}>Cache Hit Rate</h4>
            <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "4px" }}>
              <div style={{ flex: 1, height: "24px", borderRadius: "12px", backgroundColor: "var(--vscode-panel-border)", overflow: "hidden" }}>
                <div style={{ height: "100%", borderRadius: "12px", width: `${hitRate}%`, backgroundColor: hitRate > 70 ? "#2ea043" : hitRate > 40 ? "#d29922" : "#f85149", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "12px", fontWeight: 700, color: "#fff", transition: "width 0.3s" }}>
                  {hitRate}%
                </div>
              </div>
            </div>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: "8px" }}>
            {[
              { label: "Total Frames", value: stats.totalFrames, color: "var(--vscode-foreground)" },
              { label: "Avg Reduction", value: `${stats.avgReduction}%`, color: "#1f6feb" },
              { label: "Cache Hits", value: stats.cacheHits, color: "#2ea043" },
              { label: "Cache Misses", value: stats.cacheMisses, color: "#f85149" },
            ].map(s => (
              <div key={s.label} style={{ ...card, textAlign: "center" }}>
                <div style={{ fontSize: "22px", fontWeight: 700, color: s.color }}>{s.value}</div>
                <div style={{ opacity: 0.7, fontSize: "12px" }}>{s.label}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {activeTab === "frames" && (
        <div>
          <div style={card}>
            <h4 style={{ margin: "0 0 8px" }}>Current Frame</h4>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "12px" }}>
              <div>
                <div style={{ opacity: 0.6, fontSize: "12px" }}>Width</div>
                <div style={{ fontWeight: 600 }}>{frameWidth}px</div>
              </div>
              <div>
                <div style={{ opacity: 0.6, fontSize: "12px" }}>Height</div>
                <div style={{ fontWeight: 600 }}>{frameHeight}px</div>
              </div>
              <div>
                <div style={{ opacity: 0.6, fontSize: "12px" }}>Dirty Lines</div>
                <div style={{ fontWeight: 600, color: dirtyRegions.length > 0 ? "#d29922" : "#2ea043" }}>{dirtyRegions.reduce((sum, r) => sum + (r.endLine - r.startLine + 1), 0)}</div>
              </div>
            </div>
          </div>
          <h4 style={{ margin: "12px 0 8px" }}>Dirty Regions ({dirtyRegions.length})</h4>
          {dirtyRegions.length === 0 && <p style={{ opacity: 0.6 }}>No dirty regions. Frame is clean.</p>}
          {dirtyRegions.map(r => (
            <div key={r.id} style={{ ...card, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600 }}>Lines {r.startLine}-{r.endLine}</span>
                <span style={{ opacity: 0.6, marginLeft: "8px" }}>({r.endLine - r.startLine + 1} line{r.endLine - r.startLine > 0 ? "s" : ""})</span>
              </div>
              <span style={{ opacity: 0.7, fontSize: "12px", padding: "2px 8px", borderRadius: "10px", backgroundColor: "var(--vscode-badge-background)", color: "var(--vscode-badge-foreground)" }}>
                {r.reason}
              </span>
            </div>
          ))}
        </div>
      )}

      {activeTab === "config" && (
        <div>
          <div style={card}>
            <h4 style={{ margin: "0 0 12px" }}>Render Actions</h4>
            <div style={{ display: "flex", gap: "8px" }}>
              <button style={btn} onClick={handleForceRerender}>Force Full Rerender</button>
              <button style={{ ...btn, backgroundColor: "#f85149" }} onClick={handleClearCache}>Clear Cache</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default RenderOptimizePanel;
