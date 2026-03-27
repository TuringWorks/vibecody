/**
 * LangGraphPanel — Visualizes and manages LangGraph-style agent pipelines,
 * checkpoints, and execution events.
 *
 * Tabs: Pipelines, Graph, Checkpoints, Events
 */
import React, { useState } from "react";

type Tab = "Pipelines" | "Graph" | "Checkpoints" | "Events";
const TABS: Tab[] = ["Pipelines", "Graph", "Checkpoints", "Events"];

const STATUS_COLORS: Record<string, string> = {
  Idle: "var(--text-secondary)",
  Running: "var(--accent-blue)",
  Paused: "var(--warning-color)",
  Completed: "var(--success-color)",
  Failed: "var(--error-color)",
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
  background: "var(--accent-blue)", color: "#fff", cursor: "pointer",
  fontSize: 12, fontFamily: "inherit",
};
const btnSecondaryStyle: React.CSSProperties = {
  ...btnStyle, background: "var(--bg-secondary)", color: "var(--text-primary)",
};

// -- Demo data --

const PIPELINES = [
  { id: "rag-pipeline", name: "RAG Pipeline", status: "Running", nodes: 5, steps: 42 },
  { id: "code-review", name: "Code Review Agent", status: "Completed", nodes: 4, steps: 18 },
  { id: "debug-flow", name: "Debug Flow", status: "Paused", nodes: 6, steps: 7 },
  { id: "deploy-check", name: "Deploy Checker", status: "Idle", nodes: 3, steps: 0 },
  { id: "failed-run", name: "Migration Pipeline", status: "Failed", nodes: 8, steps: 31 },
];

const GRAPH_NODES = [
  { id: "start", type: "Agent", name: "Planner Agent", isEntry: true },
  { id: "search", type: "Tool", name: "Code Search", isEntry: false },
  { id: "router", type: "Router", name: "Quality Router", isEntry: false },
  { id: "fix", type: "Tool", name: "Apply Fix", isEntry: false },
  { id: "end", type: "End", name: "End", isEntry: false },
];

const GRAPH_EDGES = [
  { from: "start", to: "search", condition: null },
  { from: "search", to: "router", condition: null },
  { from: "router", to: "fix", condition: "score > 0.8" },
  { from: "router", to: "end", condition: "score <= 0.8" },
  { from: "fix", to: "end", condition: null },
];

const CHECKPOINTS = [
  { id: "cp-001", pipeline: "rag-pipeline", step: 12, timestamp: "2026-03-26 09:14:32", state: "3 values" },
  { id: "cp-002", pipeline: "rag-pipeline", step: 28, timestamp: "2026-03-26 09:18:45", state: "5 values" },
  { id: "cp-003", pipeline: "code-review", step: 10, timestamp: "2026-03-26 08:52:11", state: "2 values" },
  { id: "cp-004", pipeline: "debug-flow", step: 7, timestamp: "2026-03-26 10:01:03", state: "4 values" },
];

const EVENTS = [
  { time: "10:01:03", type: "CheckpointSaved", node: "router", data: "Checkpoint cp-004 saved" },
  { time: "10:00:58", type: "NodeExit", node: "router", data: "Quality routing completed" },
  { time: "10:00:55", type: "NodeEnter", node: "router", data: "Entering quality router" },
  { time: "10:00:50", type: "EdgeTraversal", node: "search", data: "search -> router" },
  { time: "10:00:42", type: "NodeExit", node: "search", data: "Found 14 results" },
  { time: "10:00:30", type: "NodeEnter", node: "search", data: "Starting code search" },
  { time: "10:00:28", type: "StateUpdated", node: "start", data: "State initialized with 2 values" },
  { time: "10:00:25", type: "NodeEnter", node: "start", data: "Pipeline execution started" },
];

// -- Component --

const LangGraphPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("Pipelines");
  const [selectedPipeline, setSelectedPipeline] = useState<string>("rag-pipeline");

  const renderPipelines = () => (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Pipelines ({PIPELINES.length})</span>
        <button style={btnStyle}>+ Create Pipeline</button>
      </div>
      {PIPELINES.map((p) => (
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
                {p.id} &middot; {p.nodes} nodes &middot; {p.steps} steps
              </div>
            </div>
            <span style={badgeStyle(STATUS_COLORS[p.status] || "var(--text-secondary)")}>
              {p.status}
            </span>
          </div>
        </div>
      ))}
    </div>
  );

  const renderGraph = () => (
    <div>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>
        Graph: {PIPELINES.find((p) => p.id === selectedPipeline)?.name || selectedPipeline}
      </div>
      <div style={{ marginBottom: 16 }}>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Nodes</div>
        {GRAPH_NODES.map((n) => (
          <div key={n.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={badgeStyle(NODE_TYPE_COLORS[n.type] || "var(--text-secondary)")}>{n.type}</span>
              <span style={{ fontWeight: 500, fontSize: 13 }}>{n.name}</span>
              <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>({n.id})</span>
            </div>
            {n.isEntry && (
              <span style={{
                fontSize: 10, padding: "2px 6px", borderRadius: 4,
                background: "var(--accent-blue)", color: "#fff", fontWeight: 600,
              }}>
                ENTRY
              </span>
            )}
          </div>
        ))}
      </div>
      <div>
        <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 8 }}>Edges</div>
        {GRAPH_EDGES.map((e, i) => (
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
        ))}
      </div>
    </div>
  );

  const renderCheckpoints = () => (
    <div>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>
        Checkpoints ({CHECKPOINTS.length})
      </div>
      {CHECKPOINTS.map((cp) => (
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
      ))}
    </div>
  );

  const renderEvents = () => (
    <div>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>
        Event Log ({EVENTS.length} events)
      </div>
      {EVENTS.map((ev, i) => (
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
      ))}
    </div>
  );

  const active = PIPELINES.filter((p) => p.status === "Running").length;
  const totalSteps = PIPELINES.reduce((s, p) => s + p.steps, 0);

  return (
    <div style={containerStyle}>
      <div style={statusBarStyle}>
        <span>{PIPELINES.length} pipelines &middot; {active} running &middot; {totalSteps} total steps</span>
        <span>{CHECKPOINTS.length} checkpoints &middot; {EVENTS.length} events</span>
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
