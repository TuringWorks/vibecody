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

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)", background: color, color: "var(--bg-primary)", fontWeight: 600,
});

const loadingStyle: React.CSSProperties = {
  display: "flex", justifyContent: "center", alignItems: "center",
  padding: 32, color: "var(--text-secondary)", fontSize: "var(--font-size-md)",
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
    error ? <div className="panel-error">{error}</div> : null;

  const renderLoading = () => (
    <div className="panel-loading" style={loadingStyle}>Loading...</div>
  );

  const renderPipelines = () => (
    <div>
      {renderError()}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>Pipelines ({pipelines.length})</span>
        <button className="panel-btn panel-btn-primary" onClick={handleCreatePipeline} disabled={creating}>
          {creating ? "Creating..." : "+ Create Pipeline"}
        </button>
      </div>
      {loading && pipelines.length === 0 ? renderLoading() : pipelines.length === 0 ? (
        <div style={loadingStyle}>No pipelines yet. Create one to get started.</div>
      ) : (
        pipelines.map((p) => (
          <div role="button" tabIndex={0}
            key={p.id}
            className="panel-card"
            style={{
              cursor: "pointer",
              borderLeft: selectedPipeline === p.id ? "3px solid var(--accent-blue)" : "3px solid transparent",
            }}
            onClick={() => setSelectedPipeline(p.id)}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{p.name}</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 }}>
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
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 12 }}>
        Graph: {selectedPipelineName || "No pipeline selected"}
      </div>
      {!selectedPipeline ? (
        <div style={loadingStyle}>Select a pipeline to view its graph.</div>
      ) : loading ? renderLoading() : graphNodes.length === 0 ? (
        <div style={loadingStyle}>No graph data available for this pipeline.</div>
      ) : (
        <>
          <div style={{ marginBottom: 16 }}>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>Nodes</div>
            {graphNodes.map((n) => (
              <div key={n.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={badgeStyle(NODE_TYPE_COLORS[n.type] || "var(--text-secondary)")}>{n.type}</span>
                  <span style={{ fontWeight: 500, fontSize: "var(--font-size-md)" }}>{n.name}</span>
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>({n.id})</span>
                </div>
                {n.isEntry && (
                  <span style={{
                    fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: "var(--radius-xs-plus)",
                    background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", fontWeight: 600,
                  }}>
                    ENTRY
                  </span>
                )}
              </div>
            ))}
          </div>
          <div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>Edges</div>
            {graphEdges.length === 0 ? (
              <div style={loadingStyle}>No edges found.</div>
            ) : (
              graphEdges.map((e, i) => (
                <div key={i} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontSize: "var(--font-size-md)" }}>
                    <strong>{e.from}</strong> &rarr; <strong>{e.to}</strong>
                  </span>
                  {e.condition ? (
                    <span style={{
                      fontSize: "var(--font-size-sm)", padding: "2px 8px", borderRadius: "var(--radius-xs-plus)",
                      background: "var(--bg-tertiary)", color: "var(--warning-color)",
                      fontFamily: "monospace",
                    }}>
                      {e.condition}
                    </span>
                  ) : (
                    <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>unconditional</span>
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
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 12 }}>
        Checkpoints ({checkpoints.length})
      </div>
      {!selectedPipeline ? (
        <div style={loadingStyle}>Select a pipeline to view checkpoints.</div>
      ) : loading && checkpoints.length === 0 ? renderLoading() : checkpoints.length === 0 ? (
        <div style={loadingStyle}>No checkpoints for this pipeline.</div>
      ) : (
        checkpoints.map((cp) => (
          <div key={cp.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{cp.id}</div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 }}>
                {cp.pipeline} &middot; step {cp.step} &middot; {cp.state}
              </div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{cp.timestamp}</div>
            </div>
            <button className="panel-btn panel-btn-secondary">Restore</button>
          </div>
        ))
      )}
    </div>
  );

  const renderEvents = () => (
    <div>
      {renderError()}
      <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)", marginBottom: 12 }}>
        Event Log ({events.length} events)
      </div>
      {!selectedPipeline ? (
        <div style={loadingStyle}>Select a pipeline to view events.</div>
      ) : loading && events.length === 0 ? renderLoading() : events.length === 0 ? (
        <div className="panel-empty" style={loadingStyle}>No events for this pipeline.</div>
      ) : (
        events.map((ev, i) => (
          <div key={i} className="panel-card" style={{ display: "flex", gap: 10, alignItems: "flex-start" }}>
            <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", whiteSpace: "nowrap", minWidth: 60 }}>
              {ev.time}
            </span>
            <span style={badgeStyle(EVENT_COLORS[ev.type] || "var(--text-secondary)")}>
              {ev.type}
            </span>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: "var(--font-size-base)" }}>{ev.data}</div>
              {ev.node && (
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 2 }}>
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
    <div className="panel-container">
      <div style={{ padding: "8px 16px", background: "var(--bg-tertiary)", borderBottom: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "var(--font-size-base)", flexShrink: 0 }}>
        <span>{pipelines.length} pipelines &middot; {active} running &middot; {totalSteps} total steps</span>
        <span>{checkpoints.length} checkpoints &middot; {events.length} events</span>
      </div>
      <div className="panel-tab-bar">
        {TABS.map((t) => (
          <button key={t} className={`panel-tab${tab === t ? " active" : ""}`} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body">
        {tab === "Pipelines" && renderPipelines()}
        {tab === "Graph" && renderGraph()}
        {tab === "Checkpoints" && renderCheckpoints()}
        {tab === "Events" && renderEvents()}
      </div>
    </div>
  );
};

export default LangGraphPanel;
