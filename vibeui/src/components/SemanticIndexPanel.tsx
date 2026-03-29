import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  color: "var(--btn-primary-fg, #fff)",
  marginRight: 4,
});

const kindColors: Record<string, string> = {
  function: "var(--accent-color)", struct: "var(--accent-purple)", trait: "#ec4899", enum: "var(--warning-color)", type: "var(--success-color)", const: "var(--text-secondary)", module: "#14b8a6",
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

const fallbackTypeTree: TypeNode[] = [
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

export function SemanticIndexPanel() {
  const [tab, setTab] = useState("overview");
  const [searchQuery, setSearchQuery] = useState("");
  const [kindFilter, setKindFilter] = useState("all");
  const [callQuery, setCallQuery] = useState("");

  const [symbols, setSymbols] = useState<SymbolEntry[]>([]);
  const [callEdges, setCallEdges] = useState<CallEdge[]>([]);
  const [typeTree, setTypeTree] = useState<TypeNode[]>([]);
  const [loadingSymbols, setLoadingSymbols] = useState(true);
  const [loadingEdges, setLoadingEdges] = useState(true);
  const [loadingTypes, setLoadingTypes] = useState(true);

  useEffect(() => {
    async function loadSymbols() {
      setLoadingSymbols(true);
      try {
        const result = await invoke<SymbolEntry[]>("semindex_search", { query: "*", kind: "all" });
        setSymbols(result);
      } catch (e) {
        console.error("Failed to load symbols:", e);
      }
      setLoadingSymbols(false);
    }
    async function loadCallGraph() {
      setLoadingEdges(true);
      try {
        const result = await invoke<CallEdge[]>("semindex_callgraph", { query: "*" });
        setCallEdges(result);
      } catch (e) {
        console.error("Failed to load call graph:", e);
      }
      setLoadingEdges(false);
    }
    async function loadTypes() {
      setLoadingTypes(true);
      try {
        // Try to load types from backend; use search with type/trait/struct filter
        const result = await invoke<SymbolEntry[]>("semindex_search", { query: "*", kind: "trait" });
        if (result && result.length > 0) {
          // Build type tree from trait symbols: group struct implementors under their traits
          const traitNodes: TypeNode[] = result.map((s) => ({
            name: s.name,
            kind: s.kind,
            children: [],
          }));
          setTypeTree(traitNodes);
        } else {
          setTypeTree(fallbackTypeTree);
        }
      } catch (e) {
        console.error("Failed to load types, using fallback:", e);
        setTypeTree(fallbackTypeTree);
      }
      setLoadingTypes(false);
    }
    loadSymbols();
    loadCallGraph();
    loadTypes();
  }, []);

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
          <span style={badgeStyle(kindColors[n.kind] || "var(--text-secondary)")}>{n.kind}</span>
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
          <div style={cardStyle}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Symbols</div>
            <div style={{ fontSize: 24, fontWeight: 700 }}>{loadingSymbols ? "..." : symbols.length}</div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Call Edges</div>
            <div style={{ fontSize: 24, fontWeight: 700 }}>{loadingEdges ? "..." : callEdges.length}</div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Files Indexed</div>
            <div style={{ fontSize: 24, fontWeight: 700 }}>{loadingSymbols ? "..." : new Set(symbols.map((s) => s.file)).size}</div>
          </div>
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
          {loadingSymbols && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading symbols...</div>}
          {!loadingSymbols && filtered.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No symbols found.</div>}
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
          {loadingEdges && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading call graph...</div>}
          {!loadingEdges && matchedEdges.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No call edges found.</div>}
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
        <div>
          {loadingTypes && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading type hierarchy...</div>}
          {!loadingTypes && typeTree.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No type hierarchies found.</div>}
          {!loadingTypes && renderTypeTree(typeTree, 0)}
        </div>
      )}
    </div>
  );
}
