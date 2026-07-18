/**
 * RLModelLineage — Visual DAG of policy ancestry.
 *
 * Nodes for training/distillation/deployment events, edge labels,
 * click-to-inspect node details, zoom/pan controls, and environment
 * version annotations at each node.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface LineageGraph {
  nodes: LineageNode[];
  edges: LineageEdge[];
}

interface LineageNode {
  id: string;
  label: string;
  nodeType: string;
  envVersion: string;
  timestamp: number;
  metrics: Record<string, number>;
}

interface LineageEdge {
  from: string;
  to: string;
  label: string;
}

const badgeStyle: React.CSSProperties = { fontSize: "var(--font-size-xs)", padding: "2px 6px", borderRadius: 3, color: "var(--btn-primary-fg, #fff)", marginLeft: 4 };

const typeColor = (t: string) => t === "training" ? "var(--info-color)" : t === "distillation" ? "#9c27b0" : t === "deployment" ? "var(--success-color)" : "var(--warning-color)";

export function RLModelLineage() {
  const [policyId, setPolicyId] = useState("");
  const [graph, setGraph] = useState<LineageGraph | null>(null);
  const [selected, setSelected] = useState<LineageNode | null>(null);
  const [zoom, setZoom] = useState(1);
  const [loading, setLoading] = useState(false);

  const fetchLineage = useCallback(async () => {
    if (!policyId) return;
    setLoading(true);
    try {
      const res = await invoke<LineageGraph>("rl_get_model_lineage", { policyId });
      setGraph(res);
      setSelected(null);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [policyId]);

  const parentMap = graph ? Object.fromEntries(graph.edges.map(e => [e.to, e])) : {};

  const depth = (nodeId: string, visited = new Set<string>()): number => {
    if (visited.has(nodeId)) return 0;
    visited.add(nodeId);
    const parent = parentMap[nodeId];
    return parent ? 1 + depth(parent.from, visited) : 0;
  };

  return (
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: "var(--font-size-xl)", fontWeight: 600, color: "var(--text-primary)" }}>Model Lineage</h2>

      <div className="panel-card" style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <label className="panel-label">Policy ID:</label>
        <input value={policyId} onChange={e => setPolicyId(e.target.value)} style={{ flex: 1, padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: "var(--font-size-base)" }} />
        <button className="panel-btn panel-btn-primary" onClick={fetchLineage} disabled={loading}>{loading ? "..." : "Load"}</button>
      </div>

      {graph && (
        <>
          <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
            <button className="panel-btn panel-btn-secondary" onClick={() => setZoom(z => Math.min(z + 0.2, 2))}>Zoom +</button>
            <button className="panel-btn panel-btn-secondary" onClick={() => setZoom(z => Math.max(z - 0.2, 0.4))}>Zoom -</button>
            <span className="panel-label">{(zoom * 100).toFixed(0)}%</span>
          </div>

          <div className="panel-card" style={{ transform: `scale(${zoom})`, transformOrigin: "top left" }}>
            <div className="panel-label">DAG ({graph.nodes.length} nodes, {graph.edges.length} edges)</div>
            {graph.nodes.map(n => {
              const d = depth(n.id);
              const edge = parentMap[n.id];
              return (
                <div key={n.id} style={{ paddingLeft: d * 24, padding: "6px 0", paddingRight: 0, borderBottom: "1px solid var(--border-color)", cursor: "pointer", background: selected?.id === n.id ? "var(--bg-tertiary)" : undefined }} onClick={() => setSelected(n)}>
                  {d > 0 && <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", marginRight: 4 }}>{edge?.label ?? "---"} &rarr;</span>}
                  <span style={{ fontWeight: 600 }}>{n.label}</span>
                  <span style={{ ...badgeStyle, background: typeColor(n.nodeType) }}>{n.nodeType}</span>
                  <span style={{ ...badgeStyle, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>env v{n.envVersion}</span>
                </div>
              );
            })}
          </div>

          {selected && (
            <div className="panel-card">
              <div style={{ fontWeight: 600, marginBottom: 6 }}>{selected.label}</div>
              <div className="panel-label">Type: {selected.nodeType} | Env: v{selected.envVersion} | Time: {new Date(selected.timestamp * 1000).toLocaleString()}</div>
              <div className="panel-label" style={{ marginTop: 6 }}>Metrics</div>
              {Object.entries(selected.metrics).map(([k, v]) => (
                <div key={k} style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-base)", padding: "2px 0" }}>
                  <span>{k}</span><span style={{ fontWeight: 600 }}>{v.toFixed(4)}</span>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {!graph && !loading && <div className="panel-empty">Enter a Policy ID and click Load to view lineage.</div>}
    </div>
  );
}
