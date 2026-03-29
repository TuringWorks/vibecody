/**
 * LangGraphPanel — Visualizes and manages LangGraph-style agent pipelines,
 * checkpoints, and execution events.
 *
 * Tabs: Pipelines, Graph, Checkpoints, Events
 */
import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "Pipelines" | "Graph" | "Checkpoints" | "Events";
const TABS: Tab[] = ["Pipelines", "Graph", "Checkpoints", "Events"];

interface Pipeline {
  id: string;
  name: string;
  status: string;
  created_at?: number;
  nodes?: number;
  steps?: number;
  checkpoints?: unknown[];
  events?: unknown[];
}

interface Checkpoint {
  id: string;
  pipeline: string;
  step: number;
  timestamp: string;
  state: string;
}

interface GraphNode {
  id: string;
  type: string;
  name: string;
  isEntry: boolean;
}

interface GraphEdge {
  from: string;
  to: string;
  condition: string | null;
}

interface PipelineEvent {
  time: string;
  type: string;
  node: string;
  data: string;
}

const STATUS_COLORS: Record<string, string> = {
  Idle: "var(--text-secondary)",
  idle: "var(--text-secondary)",
  Running: "var(--accent-blue)",
  running: "var(--accent-blue)",
  Paused: "var(--warning-color)",
  paused: "var(--warning-color)",
  Completed: "var(--success-color)",
  completed: "var(--success-color)",
  Failed: "var(--error-color)",
  failed: "var(--error-color)",
};

const EVENT_COLORS: Record<string, string> = {
  NodeEnter: "var(--accent-blue)",
  NodeExit: "var(--success-color)",
  EdgeTraversal: "var(--warning-color)",
  CheckpointSaved: "#9b59b6",
  StateUpdated: "var(--text-secondary)",
  Error: "var(--error-color)",
};

const NODE_TYPE_COLORS: Record<string, string> = {
  Tool: "var(--accent-blue)",
  Agent: "var(--success-color)",
  Router: "var(--warning-color)",
  Checkpoint: "#9b59b6",
  End: "var(--error-color)",
};

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
const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: 10,
  fontSize: 11, background: color, color: "var(--bg-primary)", fontWeight: 600,
});
const statusBarStyle: React.CSSProperties = {
  padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)",
  display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12, flexShrink: 0,
};
const btnStyle: React.CSSProperties = {
  padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)",
  background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", cursor: "pointer",
  fontSize: 12, fontFamily: "inherit",
};
const btnSecondaryStyle: React.CSSProperties = {
  ...btnStyle, background: "var(--bg-secondary)", color: "var(--text-primary)",
};
const loadingStyle: React.CSSProperties = {
  display: "flex", justifyContent: "center", alignItems: "center",
  padding: 32, color: "var(--text-secondary)", fontSize: 13,
};
const errorStyle: React.CSSProperties = {
  padding: 12, borderRadius: 6, marginBottom: 8,
  background: "rgba(255,0,0,0.08)", border: "1px solid var(--error-color)",
  color: "var(--error-color)", fontSize: 12,
};

// -- Component --

const LangGraphPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Pipelines");
  const [selectedPipeline, setSelectedPipeline] = useState<string>("");

  const [pipelines, setPipelines] = useState<Pipeline[]>([]);
  const [checkpoints, setCheckpoints] = useState<Checkpoint[]>([]);
  const [events, setEvents] = useState<PipelineEvent[]>([]);
  const [graphNodes, setGraphNodes] = useState<GraphNode[]>([]);
  const [graphEdges, setGraphEdges] = useState<GraphEdge[]>([]);

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const fetchPipelines = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<Pipeline[] | unknown>("langgraph_list_pipelines");
      const list = Array.isArray(result) ? result as Pipeline[] : [];
      setPipelines(list);
      if (list.length > 0 && !selectedPipeline) {
        setSelectedPipeline(list[0].id);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [selectedPipeline]);

  const fetchCheckpoints = useCallback(async (pipelineId: string) => {
    if (!pipelineId) return;
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<{ pipeline_id: string; checkpoints: Checkpoint[] }>(
        "langgraph_get_checkpoints",
        { pipelineId }
      );
      setCheckpoints(Array.isArray(result?.checkpoints) ? result.checkpoints : []);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const fetchEvents = useCallback(async (pipelineId: string) => {
    if (!pipelineId) return;
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<{ pipeline_id: string; events: PipelineEvent[] }>(
        "langgraph_get_events",
        { pipelineId }
      );
      const evts = Array.isArray(result?.events) ? result.events : [];
      setEvents(evts);
      // Derive graph nodes/edges from events if available
      const nodeSet = new Map<string, GraphNode>();
      const edgeSet: GraphEdge[] = [];
      let prevNode: string | null = null;
      for (const ev of evts) {
        if (ev.node && !nodeSet.has(ev.node)) {
          nodeSet.set(ev.node, {
            id: ev.node,
            type: ev.type === "Error" ? "End" : "Agent",
            name: ev.node,
            isEntry: nodeSet.size === 0,
          });
        }
        if (ev.type === "EdgeTraversal" && ev.data) {
          const match = ev.data.match(/(\S+)\s*->\s*(\S+)/);
          if (match) {
            edgeSet.push({ from: match[1], to: match[2], condition: null });
          }
        }
        if (ev.type === "NodeEnter" && prevNode && ev.node && prevNode !== ev.node) {
          const exists = edgeSet.some(e => e.from === prevNode && e.to === ev.node);
          if (!exists) {
            edgeSet.push({ from: prevNode, to: ev.node, condition: null });
          }
        }
        if (ev.type === "NodeEnter" && ev.node) {
          prevNode = ev.node;
        }
      }
      setGraphNodes(Array.from(nodeSet.values()));
      setGraphEdges(edgeSet);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  // Fetch pipelines on mount
  useEffect(() => {
    fetchPipelines();
  }, [fetchPipelines]);

  // Fetch checkpoints and events when selected pipeline changes
  useEffect(() => {
    if (selectedPipeline) {
      fetchCheckpoints(selectedPipeline);
      fetchEvents(selectedPipeline);
    }
  }, [selectedPipeline, fetchCheckpoints, fetchEvents]);

  const handleCreatePipeline = async () => {
    const name = prompt("Pipeline name:");
    if (!name) return;
    setCreating(true);
    setError(null);
    try {
      await invoke("langgraph_create_pipeline", { name });
      await fetchPipelines();
    } catch (err) {
      setError(String(err));
    } finally {
      setCreating(false);
    }
  };

  const renderError = () =>
    error ? <div style={errorStyle}>{error}</div> : null;

  const renderLoading = () => (
    <div style={loadingStyle}>Loading...</div>
  );

  const renderPipelines = () => (
    <div>
      {renderError()}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Pipelines ({pipelines.length})</span>
        <button style={btnStyle} onClick={handleCreatePipeline} disabled={creating}>
          {creating ? "Creating..." : "+ Create Pipeline"}
        </button>
      </div>
      {loading && pipelines.length === 0 ? renderLoading() : pipelines.length === 0 ? (
        <div style={loadingStyle}>No pipelines yet. Create one to get started.</div>
      ) : (
        pipelines.map((p) => (
          <div
            key={p.id}
            style={{
              ...cardStyle,
              cursor: "pointer",
              borderLeft: selectedPipeline === p.id ? "3px solid var(--accent-blue)" : "3px solid transparent",
            }}
            onClick={() => setSelectedPipeline(p.id)}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: 13 }}>{p.name}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>
                  {p.id}
                  {p.nodes != null && <> &middot; {p.nodes} nodes</>}
                  {p.steps != null && <> &middot; {p.steps} steps</>}
                </div>
              </div>
              <span style={badgeStyle(STATUS_COLORS[p.status] || "var(--text-secondary)")}>
                {p.status}
              </span>
            </div>
          </div>
        ))
      )}
    </div>
  );

  const selectedPipelineName = pipelines.find((p) => p.id === selectedPipeline)?.name || selectedPipeline;

  const renderGraph = () => (
    <div>
      {renderError()}
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>
        Graph: {selectedPipelineName || "No pipeline selected"}
      </div>
      {!selectedPipeline ? (
        <div style={loadingStyle}>Select a pipeline to view its graph.</div>
      ) : loading ? renderLoading() : graphNodes.length === 0 ? (
        <div style={loadingStyle}>No graph data available for this pipeline.</div>
      ) : (
        <>
          <div style={{ marginBottom: 16 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Nodes</div>
            {graphNodes.map((n) => (
              <div key={n.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={badgeStyle(NODE_TYPE_COLORS[n.type] || "var(--text-secondary)")}>{n.type}</span>
                  <span style={{ fontWeight: 500, fontSize: 13 }}>{n.name}</span>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>({n.id})</span>
                </div>
                {n.isEntry && (
                  <span style={{
                    fontSize: 10, padding: "2px 6px", borderRadius: 4,
                    background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", fontWeight: 600,
                  }}>
                    ENTRY
                  </span>
                )}
              </div>
            ))}
          </div>
          <div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Edges</div>
            {graphEdges.length === 0 ? (
              <div style={loadingStyle}>No edges found.</div>
            ) : (
              graphEdges.map((e, i) => (
                <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontSize: 13 }}>
                    <strong>{e.from}</strong> &rarr; <strong>{e.to}</strong>
                  </span>
                  {e.condition ? (
                    <span style={{
                      fontSize: 11, padding: "2px 8px", borderRadius: 4,
                      background: "var(--bg-tertiary)", color: "var(--warning-color)",
                      fontFamily: "monospace",
                    }}>
                      {e.condition}
                    </span>
                  ) : (
                    <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>unconditional</span>
                  )}
                </div>
              ))
            )}
          </div>
        </>
      )}
    </div>
  );

  const renderCheckpoints = () => (
    <div>
      {renderError()}
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>
        Checkpoints ({checkpoints.length})
      </div>
      {!selectedPipeline ? (
        <div style={loadingStyle}>Select a pipeline to view checkpoints.</div>
      ) : loading && checkpoints.length === 0 ? renderLoading() : checkpoints.length === 0 ? (
        <div style={loadingStyle}>No checkpoints for this pipeline.</div>
      ) : (
        checkpoints.map((cp) => (
          <div key={cp.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontWeight: 600, fontSize: 13 }}>{cp.id}</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>
                {cp.pipeline} &middot; step {cp.step} &middot; {cp.state}
              </div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{cp.timestamp}</div>
            </div>
            <button style={btnSecondaryStyle}>Restore</button>
          </div>
        ))
      )}
    </div>
  );

  const renderEvents = () => (
    <div>
      {renderError()}
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>
        Event Log ({events.length} events)
      </div>
      {!selectedPipeline ? (
        <div style={loadingStyle}>Select a pipeline to view events.</div>
      ) : loading && events.length === 0 ? renderLoading() : events.length === 0 ? (
        <div style={loadingStyle}>No events for this pipeline.</div>
      ) : (
        events.map((ev, i) => (
          <div key={i} style={{ ...cardStyle, display: "flex", gap: 10, alignItems: "flex-start" }}>
            <span style={{ fontSize: 11, color: "var(--text-secondary)", whiteSpace: "nowrap", minWidth: 60 }}>
              {ev.time}
            </span>
            <span style={badgeStyle(EVENT_COLORS[ev.type] || "var(--text-secondary)")}>
              {ev.type}
            </span>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 12 }}>{ev.data}</div>
              {ev.node && (
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>
                  node: {ev.node}
                </div>
              )}
            </div>
          </div>
        ))
      )}
    </div>
  );

  const active = pipelines.filter((p) => ["Running", "running"].includes(p.status)).length;
  const totalSteps = pipelines.reduce((s, p) => s + (p.steps || 0), 0);

  return (
    <div style={containerStyle}>
      <div style={statusBarStyle}>
        <span>{pipelines.length} pipelines &middot; {active} running &middot; {totalSteps} total steps</span>
        <span>{checkpoints.length} checkpoints &middot; {events.length} events</span>
      </div>
      <div style={tabBarStyle}>
        {TABS.map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div style={contentStyle}>
        {tab === "Pipelines" && renderPipelines()}
        {tab === "Graph" && renderGraph()}
        {tab === "Checkpoints" && renderCheckpoints()}
        {tab === "Events" && renderEvents()}
      </div>
    </div>
  );
};

export default LangGraphPanel;
