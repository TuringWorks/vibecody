import { useState } from "react";

interface SymbolEntry {
  name: string;
  kind: "function" | "struct" | "trait" | "enum" | "type" | "const" | "module";
  file: string;
  line: number;
}

interface CallEdge {
  caller: string;
  callee: string;
  file: string;
  line: number;
}

interface TypeNode {
  name: string;
  kind: string;
  children: TypeNode[];
}

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};


const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: color,
  color: "#fff",
  marginRight: 4,
});

const kindColors: Record<string, string> = {
  function: "#3b82f6", struct: "#8b5cf6", trait: "#ec4899", enum: "#f59e0b", type: "#22c55e", const: "#6b7280", module: "#14b8a6",
};

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: 8,
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontSize: 13,
};

export function SemanticIndexPanel() {
  const [tab, setTab] = useState("overview");
  const [searchQuery, setSearchQuery] = useState("");
  const [kindFilter, setKindFilter] = useState("all");
  const [callQuery, setCallQuery] = useState("");

  const symbols: SymbolEntry[] = [
    { name: "AIProvider", kind: "trait", file: "vibe-ai/src/provider.rs", line: 15 },
    { name: "WorktreeManager", kind: "trait", file: "vibe-ai/src/worktree.rs", line: 8 },
    { name: "execute_tool", kind: "function", file: "vibecli/src/tool_executor.rs", line: 42 },
    { name: "Config", kind: "struct", file: "vibecli/src/config.rs", line: 20 },
    { name: "ProviderKind", kind: "enum", file: "vibe-ai/src/provider.rs", line: 5 },
    { name: "MAX_RETRIES", kind: "const", file: "vibecli/src/agent.rs", line: 3 },
  ];

  const callEdges: CallEdge[] = [
    { caller: "agent_loop", callee: "execute_tool", file: "agent.rs", line: 120 },
    { caller: "execute_tool", callee: "run_command", file: "tool_executor.rs", line: 55 },
    { caller: "agent_loop", callee: "stream_response", file: "agent.rs", line: 85 },
  ];

  const typeTree: TypeNode[] = [
    { name: "AIProvider", kind: "trait", children: [
      { name: "OllamaProvider", kind: "struct", children: [] },
      { name: "ClaudeProvider", kind: "struct", children: [] },
      { name: "OpenAIProvider", kind: "struct", children: [] },
    ]},
    { name: "ContainerRuntime", kind: "trait", children: [
      { name: "DockerRuntime", kind: "struct", children: [] },
      { name: "PodmanRuntime", kind: "struct", children: [] },
    ]},
  ];

  const filtered = symbols.filter((s) => {
    const matchesQuery = !searchQuery || s.name.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesKind = kindFilter === "all" || s.kind === kindFilter;
    return matchesQuery && matchesKind;
  });

  const matchedEdges = callEdges.filter((e) =>
    !callQuery || e.caller.toLowerCase().includes(callQuery.toLowerCase()) || e.callee.toLowerCase().includes(callQuery.toLowerCase())
  );

  const renderTypeTree = (nodes: TypeNode[], depth: number): React.ReactElement[] =>
    nodes.map((n) => (
      <div key={n.name}>
        <div style={{ paddingLeft: depth * 20, padding: "4px 0 4px " + depth * 20 + "px", fontSize: 13 }}>
          <span style={badgeStyle(kindColors[n.kind] || "#6b7280")}>{n.kind}</span>
          <strong>{n.name}</strong>
        </div>
        {n.children.length > 0 && renderTypeTree(n.children, depth + 1)}
      </div>
    ));

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Deep Semantic Index</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "overview")} onClick={() => setTab("overview")}>Overview</button>
        <button style={tabStyle(tab === "search")} onClick={() => setTab("search")}>Search</button>
        <button style={tabStyle(tab === "callgraph")} onClick={() => setTab("callgraph")}>Call Graph</button>
        <button style={tabStyle(tab === "types")} onClick={() => setTab("types")}>Types</button>
      </div>

      {tab === "overview" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Symbols</div><div style={{ fontSize: 24, fontWeight: 700 }}>{symbols.length}</div></div>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Call Edges</div><div style={{ fontSize: 24, fontWeight: 700 }}>{callEdges.length}</div></div>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Files Indexed</div><div style={{ fontSize: 24, fontWeight: 700 }}>4</div></div>
        </div>
      )}

      {tab === "search" && (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <input style={{ ...inputStyle, flex: 1 }} placeholder="Search symbols..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} />
            <select value={kindFilter} onChange={(e) => setKindFilter(e.target.value)} style={{ ...inputStyle, width: "auto" }}>
              <option value="all">All kinds</option>
              {["function", "struct", "trait", "enum", "type", "const", "module"].map((k) => <option key={k} value={k}>{k}</option>)}
            </select>
          </div>
          {filtered.map((s, i) => (
            <div key={i} style={cardStyle}>
              <span style={badgeStyle(kindColors[s.kind])}>{s.kind}</span>
              <strong>{s.name}</strong>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>{s.file}:{s.line}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "callgraph" && (
        <div>
          <input style={{ ...inputStyle, marginBottom: 12 }} placeholder="Lookup function name..." value={callQuery} onChange={(e) => setCallQuery(e.target.value)} />
          {matchedEdges.map((e, i) => (
            <div key={i} style={cardStyle}>
              <div style={{ fontSize: 13 }}>
                <strong style={{ color: "var(--accent-color)" }}>{e.caller}</strong>
                <span style={{ margin: "0 8px", color: "var(--text-secondary)" }}>&rarr;</span>
                <strong>{e.callee}</strong>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>{e.file}:{e.line}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "types" && (
        <div>{renderTypeTree(typeTree, 0)}</div>
      )}
    </div>
  );
}
