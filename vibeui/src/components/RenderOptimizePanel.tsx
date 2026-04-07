import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DirtyRegion {
  id: string;
  startLine: number;
  endLine: number;
  reason: string;
}

interface RenderStatsData {
  cacheHits: number;
  cacheMisses: number;
  totalFrames: number;
  avgReduction: number;
}

interface OptimizationResult {
  regions_cleared: number;
  cache_hits_before: number;
  cache_hits_after: number;
  reduction_pct: number;
}

const RenderOptimizePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("stats");
  const [stats, setStats] = useState<RenderStatsData>({ cacheHits: 0, cacheMisses: 0, totalFrames: 0, avgReduction: 0 });
  const [frameWidth] = useState(1920);
  const [frameHeight] = useState(1080);
  const [dirtyRegions, setDirtyRegions] = useState<DirtyRegion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [optimizeResult, setOptimizeResult] = useState<OptimizationResult | null>(null);

  const loadData = useCallback(async () => {
    try {
      const [statsResult, regionsResult] = await Promise.all([
        invoke<RenderStatsData>("get_render_stats"),
        invoke<DirtyRegion[]>("get_dirty_regions"),
      ]);
      setStats(statsResult);
      setDirtyRegions(regionsResult);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const hitRate = stats.totalFrames > 0 ? Math.round((stats.cacheHits / stats.totalFrames) * 100) : 0;

  const handleRunOptimization = async () => {
    try {
      const result = await invoke<OptimizationResult>("run_render_optimization");
      setOptimizeResult(result);
      // Reload data to get updated stats
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleResetStats = async () => {
    try {
      await invoke("reset_render_stats");
      setOptimizeResult(null);
      await loadData();
    } catch (e) {
      setError(String(e));
    }
  };

  if (loading) {
    return <div className="panel-container"><div className="panel-loading">Loading render stats...</div></div>;
  }

  return (
    <div className="panel-container">
      <h3 style={{ margin: "0 0 12px", padding: "12px 16px 0" }}>Render Optimization</h3>
      {error && <div className="panel-error">{error}</div>}
      <div className="panel-tab-bar">
        {["stats", "frames", "config"].map(t => (
          <button key={t} className={`panel-tab${activeTab === t ? " active" : ""}`} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      <div className="panel-body">
        {activeTab === "stats" && (
          <div>
            <div className="panel-card">
              <h4 style={{ margin: "0 0 8px" }}>Cache Hit Rate</h4>
              <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "4px" }}>
                <div style={{ flex: 1, height: "24px", borderRadius: "12px", backgroundColor: "var(--border-color)", overflow: "hidden" }}>
                  <div style={{ height: "100%", borderRadius: "12px", width: `${hitRate}%`, backgroundColor: hitRate > 70 ? "var(--success-color)" : hitRate > 40 ? "var(--warning-color)" : "var(--error-color)", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "12px", fontWeight: 700, color: "var(--btn-primary-fg)", transition: "width 0.3s" }}>
                    {hitRate}%
                  </div>
                </div>
              </div>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: "8px" }}>
              {[
                { label: "Total Frames", value: stats.totalFrames, color: "var(--text-primary)" },
                { label: "Avg Reduction", value: `${stats.avgReduction}%`, color: "var(--info-color)" },
                { label: "Cache Hits", value: stats.cacheHits, color: "var(--success-color)" },
                { label: "Cache Misses", value: stats.cacheMisses, color: "var(--error-color)" },
              ].map(s => (
                <div key={s.label} className="panel-card" style={{ textAlign: "center" }}>
                  <div style={{ fontSize: "22px", fontWeight: 700, color: s.color }}>{s.value}</div>
                  <div style={{ opacity: 0.7, fontSize: "12px" }}>{s.label}</div>
                </div>
              ))}
            </div>
            <div style={{ marginTop: "8px" }}>
              <button className="panel-btn panel-btn-primary" onClick={loadData}>Refresh Stats</button>
            </div>
          </div>
        )}

        {activeTab === "frames" && (
          <div>
            <div className="panel-card">
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
                  <div style={{ fontWeight: 600, color: dirtyRegions.length > 0 ? "var(--warning-color)" : "var(--success-color)" }}>{dirtyRegions.reduce((sum, r) => sum + (r.endLine - r.startLine + 1), 0)}</div>
                </div>
              </div>
            </div>
            <h4 style={{ margin: "12px 0 8px" }}>Dirty Regions ({dirtyRegions.length})</h4>
            {dirtyRegions.length === 0 && <div className="panel-empty">No dirty regions. Frame is clean.</div>}
            {dirtyRegions.map(r => (
              <div key={r.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <span style={{ fontWeight: 600 }}>Lines {r.startLine}-{r.endLine}</span>
                  <span style={{ opacity: 0.6, marginLeft: "8px" }}>({r.endLine - r.startLine + 1} line{r.endLine - r.startLine > 0 ? "s" : ""})</span>
                </div>
                <span style={{ opacity: 0.7, fontSize: "12px", padding: "2px 8px", borderRadius: "10px", backgroundColor: "var(--bg-tertiary)", color: "var(--btn-primary-fg)" }}>
                  {r.reason}
                </span>
              </div>
            ))}
          </div>
        )}

        {activeTab === "config" && (
          <div>
            <div className="panel-card">
              <h4 style={{ margin: "0 0 12px" }}>Render Actions</h4>
              <div style={{ display: "flex", gap: "8px" }}>
                <button className="panel-btn panel-btn-primary" onClick={handleRunOptimization}>Run Optimization</button>
                <button className="panel-btn panel-btn-danger" onClick={handleResetStats}>Reset Stats</button>
              </div>
            </div>
            {optimizeResult && (
              <div className="panel-card">
                <h4 style={{ margin: "0 0 8px" }}>Last Optimization Result</h4>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: "8px" }}>
                  <div><span style={{ opacity: 0.6 }}>Regions Cleared:</span> {optimizeResult.regions_cleared}</div>
                  <div><span style={{ opacity: 0.6 }}>Reduction:</span> {optimizeResult.reduction_pct}%</div>
                  <div><span style={{ opacity: 0.6 }}>Hits Before:</span> {optimizeResult.cache_hits_before}</div>
                  <div><span style={{ opacity: 0.6 }}>Hits After:</span> {optimizeResult.cache_hits_after}</div>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default RenderOptimizePanel;
