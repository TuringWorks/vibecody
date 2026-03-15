/**
 * KnowledgeGraphPanel — Cross-repository knowledge graph explorer.
 *
 * Interactive symbol graph with repo registration, cross-repo queries,
 * callers/callees/implementors, shortest path finder, and DOT export.
 */
import { useState } from "react";

interface GraphNode {
 id: number;
 name: string;
 kind: string;
 repo: string;
 file: string;
 line: number;
 signature: string;
}

interface GraphEdge {
 from: number;
 to: number;
 kind: string;
}

interface GraphStats {
 total_nodes: number;
 total_edges: number;
 nodes_per_repo: Record<string, number>;
 cross_repo_edges: number;
 most_connected: [string, number][];
 orphan_count: number;
}

type QueryMode = "callers" | "callees" | "implementors" | "dependencies" | "dependents" | "path" | "subgraph";

// ── Sample data for demo ─────────────────────────────────────────────────────

const SAMPLE_NODES: GraphNode[] = [
 { id: 1, name: "AgentLoop", kind: "struct", repo: "vibe-ai", file: "src/agent.rs", line: 42, signature: "pub struct AgentLoop { ... }" },
 { id: 2, name: "run", kind: "function", repo: "vibe-ai", file: "src/agent.rs", line: 100, signature: "pub async fn run(&mut self) -> Result<()>" },
 { id: 3, name: "AIProvider", kind: "trait", repo: "vibe-ai", file: "src/provider.rs", line: 15, signature: "pub trait AIProvider: Send + Sync" },
 { id: 4, name: "OllamaProvider", kind: "struct", repo: "vibe-ai", file: "src/providers/ollama.rs", line: 8, signature: "pub struct OllamaProvider { ... }" },
 { id: 5, name: "ToolExecutor", kind: "struct", repo: "vibecli", file: "src/tool_executor.rs", line: 20, signature: "pub struct ToolExecutor { ... }" },
 { id: 6, name: "execute_tool", kind: "function", repo: "vibecli", file: "src/tool_executor.rs", line: 55, signature: "pub async fn execute_tool(&self, name: &str, args: &Value)" },
 { id: 7, name: "EmbeddingIndex", kind: "struct", repo: "vibe-core", file: "src/index/embeddings.rs", line: 98, signature: "pub struct EmbeddingIndex { ... }" },
 { id: 8, name: "search", kind: "function", repo: "vibe-core", file: "src/index/embeddings.rs", line: 150, signature: "pub async fn search(&self, query: &str, k: usize)" },
];

const SAMPLE_EDGES: GraphEdge[] = [
 { from: 1, to: 2, kind: "contains" },
 { from: 2, to: 3, kind: "calls" },
 { from: 4, to: 3, kind: "implements" },
 { from: 2, to: 6, kind: "calls" },
 { from: 5, to: 6, kind: "contains" },
 { from: 2, to: 8, kind: "calls" },
 { from: 7, to: 8, kind: "contains" },
];

const SAMPLE_STATS: GraphStats = {
 total_nodes: 8,
 total_edges: 7,
 nodes_per_repo: { "vibe-ai": 4, "vibecli": 2, "vibe-core": 2 },
 cross_repo_edges: 3,
 most_connected: [["AgentLoop", 5], ["AIProvider", 3], ["ToolExecutor", 2]],
 orphan_count: 0,
};

const KIND_COLORS: Record<string, string> = {
 function: "var(--accent-color)",
 struct: "var(--success-color)",
 trait: "var(--warning-color)",
 interface: "var(--warning-color)",
 class: "#cba6f7",
 module: "#89dceb",
 file: "#9399b2",
 repo: "#f38ba8",
 enum: "#a6e3a1",
};

const REPO_COLORS: Record<string, string> = {
 "vibe-ai": "#89b4fa",
 "vibecli": "#a6e3a1",
 "vibe-core": "#fab387",
};

export default function KnowledgeGraphPanel() {
 const [repos, setRepos] = useState<{ name: string; path: string }[]>([
   { name: "vibe-ai", path: "vibeui/crates/vibe-ai" },
   { name: "vibecli", path: "vibecli/vibecli-cli" },
   { name: "vibe-core", path: "vibeui/crates/vibe-core" },
 ]);
 const [newRepoName, setNewRepoName] = useState("");
 const [newRepoPath, setNewRepoPath] = useState("");
 const [queryMode, setQueryMode] = useState<QueryMode>("callers");
 const [querySymbol, setQuerySymbol] = useState("AIProvider");
 const [targetSymbol, setTargetSymbol] = useState("");
 const [depth, setDepth] = useState(2);
 const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
 const [tab, setTab] = useState<"graph" | "stats" | "export">("graph");

 const queryResults = (): GraphNode[] => {
   switch (queryMode) {
     case "callers":
       return SAMPLE_NODES.filter(n => ["AgentLoop", "OllamaProvider"].includes(n.name));
     case "callees":
       return SAMPLE_NODES.filter(n => ["AIProvider", "ToolExecutor", "EmbeddingIndex"].includes(n.name));
     case "implementors":
       return SAMPLE_NODES.filter(n => n.name === "OllamaProvider");
     default:
       return SAMPLE_NODES;
   }
 };

 const results = queryResults();

 const addRepo = () => {
   if (newRepoName && newRepoPath) {
     setRepos([...repos, { name: newRepoName, path: newRepoPath }]);
     setNewRepoName("");
     setNewRepoPath("");
   }
 };

 const graphWidth = 700;
 const graphHeight = 400;
 const nodePositions = SAMPLE_NODES.map((_, i) => ({
   x: 80 + (i % 4) * 170,
   y: 60 + Math.floor(i / 4) * 160,
 }));

 return (
   <div style={{ padding: 16, color: "var(--text-primary)", background: "var(--bg-primary)", minHeight: "100%" }}>
     <h2 style={{ margin: "0 0 12px", fontSize: 18 }}>Knowledge Graph</h2>

     {/* Repo Registration */}
     <div style={{ marginBottom: 12, padding: 10, border: "1px solid var(--border-color)", borderRadius: 6 }}>
       <strong>Registered Repos</strong>
       <div style={{ display: "flex", flexWrap: "wrap", gap: 6, margin: "6px 0" }}>
         {repos.map(r => (
           <span key={r.name} style={{
             padding: "2px 8px", borderRadius: 4, fontSize: 12,
             background: REPO_COLORS[r.name] || "var(--border-color)", color: "#1e1e2e",
           }}>
             {r.name}: {r.path}
             <button onClick={() => setRepos(repos.filter(x => x.name !== r.name))}
               style={{ marginLeft: 4, background: "none", border: "none", cursor: "pointer", color: "#1e1e2e" }}>×</button>
           </span>
         ))}
       </div>
       <div style={{ display: "flex", gap: 4 }}>
         <input value={newRepoName} onChange={e => setNewRepoName(e.target.value)} placeholder="Repo name"
           style={{ flex: 1, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4 }} />
         <input value={newRepoPath} onChange={e => setNewRepoPath(e.target.value)} placeholder="Path"
           style={{ flex: 2, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4 }} />
         <button onClick={addRepo} style={{ padding: "4px 10px", background: "var(--accent-color)", color: "white", border: "none", borderRadius: 4, cursor: "pointer" }}>Add</button>
       </div>
     </div>

     {/* Tabs */}
     <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
       {(["graph", "stats", "export"] as const).map(t => (
         <button key={t} onClick={() => setTab(t)} style={{
           padding: "4px 12px", border: "1px solid var(--border-color)", borderRadius: 4, cursor: "pointer",
           background: tab === t ? "var(--accent-color)" : "transparent", color: tab === t ? "white" : "var(--text-primary)",
         }}>{t.charAt(0).toUpperCase() + t.slice(1)}</button>
       ))}
     </div>

     {tab === "graph" && (
       <>
         {/* Query Controls */}
         <div style={{ display: "flex", gap: 6, marginBottom: 12, flexWrap: "wrap", alignItems: "center" }}>
           <select value={queryMode} onChange={e => setQueryMode(e.target.value as QueryMode)}
             style={{ padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4 }}>
             <option value="callers">Callers</option>
             <option value="callees">Callees</option>
             <option value="implementors">Implementors</option>
             <option value="dependencies">Dependencies</option>
             <option value="dependents">Dependents</option>
             <option value="path">Shortest Path</option>
             <option value="subgraph">Subgraph</option>
           </select>
           <input value={querySymbol} onChange={e => setQuerySymbol(e.target.value)} placeholder="Symbol name"
             style={{ flex: 1, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4 }} />
           {queryMode === "path" && (
             <input value={targetSymbol} onChange={e => setTargetSymbol(e.target.value)} placeholder="Target symbol"
               style={{ flex: 1, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4 }} />
           )}
           {queryMode === "subgraph" && (
             <label style={{ fontSize: 12 }}>
               Depth: <input type="number" value={depth} onChange={e => setDepth(+e.target.value)} min={0} max={5}
                 style={{ width: 40, padding: 2, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4 }} />
             </label>
           )}
         </div>

         {/* Graph SVG */}
         <svg width={graphWidth} height={graphHeight} style={{ border: "1px solid var(--border-color)", borderRadius: 6, background: "var(--bg-primary)", marginBottom: 12 }}>
           <defs>
             <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
               <polygon points="0 0, 8 3, 0 6" fill="var(--border-color)" />
             </marker>
           </defs>
           {SAMPLE_EDGES.map((e, i) => {
             const fromIdx = SAMPLE_NODES.findIndex(n => n.id === e.from);
             const toIdx = SAMPLE_NODES.findIndex(n => n.id === e.to);
             if (fromIdx < 0 || toIdx < 0) return null;
             const fp = nodePositions[fromIdx];
             const tp = nodePositions[toIdx];
             const isCrossRepo = SAMPLE_NODES[fromIdx].repo !== SAMPLE_NODES[toIdx].repo;
             return (
               <g key={i}>
                 <line x1={fp.x} y1={fp.y} x2={tp.x} y2={tp.y}
                   stroke={isCrossRepo ? "var(--warning-color)" : "var(--border-color)"} strokeWidth={isCrossRepo ? 2 : 1}
                   strokeDasharray={isCrossRepo ? "4,2" : "none"} markerEnd="url(#arrowhead)" opacity={0.7} />
                 <text x={(fp.x + tp.x) / 2} y={(fp.y + tp.y) / 2 - 4} fontSize={9} fill="var(--border-color)" textAnchor="middle">{e.kind}</text>
               </g>
             );
           })}
           {SAMPLE_NODES.map((node, i) => {
             const pos = nodePositions[i];
             const isResult = results.some(r => r.id === node.id);
             return (
               <g key={node.id} onClick={() => setSelectedNode(node)} style={{ cursor: "pointer" }}>
                 <circle cx={pos.x} cy={pos.y} r={isResult ? 22 : 18}
                   fill={isResult ? KIND_COLORS[node.kind] || "var(--accent-color)" : "var(--bg-secondary)"}
                   stroke={REPO_COLORS[node.repo] || "var(--border-color)"} strokeWidth={2} />
                 <text x={pos.x} y={pos.y + 4} textAnchor="middle" fontSize={10}
                   fill={isResult ? "#1e1e2e" : "var(--text-primary)"}>{node.name.slice(0, 10)}</text>
                 <text x={pos.x} y={pos.y + 34} textAnchor="middle" fontSize={8} fill="var(--border-color)">{node.kind}</text>
               </g>
             );
           })}
         </svg>

         {/* Legend */}
         <div style={{ display: "flex", gap: 12, flexWrap: "wrap", fontSize: 11, marginBottom: 12 }}>
           {Object.entries(KIND_COLORS).map(([kind, color]) => (
             <span key={kind}><span style={{ display: "inline-block", width: 10, height: 10, borderRadius: "50%", background: color, marginRight: 3 }} />{kind}</span>
           ))}
           <span style={{ marginLeft: 12 }}>--- cross-repo edge</span>
         </div>

         {/* Selected Node Detail */}
         {selectedNode && (
           <div style={{ padding: 8, border: "1px solid var(--border-color)", borderRadius: 6, fontSize: 12, marginBottom: 12 }}>
             <strong>{selectedNode.name}</strong> ({selectedNode.kind})
             <div style={{ marginTop: 4 }}>Repo: {selectedNode.repo} | File: {selectedNode.file}:{selectedNode.line}</div>
             <div style={{ marginTop: 2, fontFamily: "monospace", fontSize: 11 }}>{selectedNode.signature}</div>
           </div>
         )}

         {/* Query Results */}
         <div style={{ fontSize: 12 }}>
           <strong>Results ({results.length})</strong>
           <table style={{ width: "100%", borderCollapse: "collapse", marginTop: 4 }}>
             <thead>
               <tr style={{ borderBottom: "1px solid var(--border-color)" }}>
                 <th style={{ textAlign: "left", padding: 4 }}>Name</th>
                 <th style={{ textAlign: "left", padding: 4 }}>Kind</th>
                 <th style={{ textAlign: "left", padding: 4 }}>Repo</th>
                 <th style={{ textAlign: "left", padding: 4 }}>File</th>
               </tr>
             </thead>
             <tbody>
               {results.map(n => (
                 <tr key={n.id} onClick={() => setSelectedNode(n)} style={{ cursor: "pointer", borderBottom: "1px solid var(--border-color)" }}>
                   <td style={{ padding: 4, color: KIND_COLORS[n.kind] }}>{n.name}</td>
                   <td style={{ padding: 4 }}>{n.kind}</td>
                   <td style={{ padding: 4, color: REPO_COLORS[n.repo] }}>{n.repo}</td>
                   <td style={{ padding: 4, fontFamily: "monospace", fontSize: 11 }}>{n.file}:{n.line}</td>
                 </tr>
               ))}
             </tbody>
           </table>
         </div>
       </>
     )}

     {tab === "stats" && (
       <div style={{ fontSize: 13 }}>
         <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 8, marginBottom: 12 }}>
           <div style={{ padding: 10, border: "1px solid var(--border-color)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 24, fontWeight: 700, color: "var(--accent-color)" }}>{SAMPLE_STATS.total_nodes}</div>
             <div>Nodes</div>
           </div>
           <div style={{ padding: 10, border: "1px solid var(--border-color)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 24, fontWeight: 700, color: "var(--success-color)" }}>{SAMPLE_STATS.total_edges}</div>
             <div>Edges</div>
           </div>
           <div style={{ padding: 10, border: "1px solid var(--border-color)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 24, fontWeight: 700, color: "var(--warning-color)" }}>{SAMPLE_STATS.cross_repo_edges}</div>
             <div>Cross-Repo</div>
           </div>
         </div>

         <strong>Nodes per Repo</strong>
         <div style={{ marginTop: 4, marginBottom: 12 }}>
           {Object.entries(SAMPLE_STATS.nodes_per_repo).map(([repo, count]) => (
             <div key={repo} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
               <span style={{ width: 80, color: REPO_COLORS[repo] }}>{repo}</span>
               <div style={{ flex: 1, height: 16, background: "var(--border-color)", borderRadius: 4, overflow: "hidden" }}>
                 <div style={{ height: "100%", width: `${(count / SAMPLE_STATS.total_nodes) * 100}%`, background: REPO_COLORS[repo] || "var(--accent-color)", borderRadius: 4 }} />
               </div>
               <span style={{ width: 30, textAlign: "right" }}>{count}</span>
             </div>
           ))}
         </div>

         <strong>Most Connected Symbols</strong>
         <table style={{ width: "100%", borderCollapse: "collapse", marginTop: 4 }}>
           <thead><tr style={{ borderBottom: "1px solid var(--border-color)" }}>
             <th style={{ textAlign: "left", padding: 4 }}>Symbol</th>
             <th style={{ textAlign: "right", padding: 4 }}>Connections</th>
           </tr></thead>
           <tbody>
             {SAMPLE_STATS.most_connected.map(([name, count]) => (
               <tr key={name} style={{ borderBottom: "1px solid var(--border-color)" }}>
                 <td style={{ padding: 4, color: "var(--accent-color)" }}>{name}</td>
                 <td style={{ padding: 4, textAlign: "right" }}>{count}</td>
               </tr>
             ))}
           </tbody>
         </table>

         <div style={{ marginTop: 12 }}>Orphan symbols: <strong>{SAMPLE_STATS.orphan_count}</strong></div>
       </div>
     )}

     {tab === "export" && (
       <div>
         <strong>DOT Export</strong>
         <pre style={{
           marginTop: 8, padding: 10, background: "var(--bg-primary)", border: "1px solid var(--border-color)",
           borderRadius: 6, fontSize: 11, overflow: "auto", maxHeight: 300,
         }}>
{`digraph KnowledgeGraph {
 rankdir=LR;
 node [shape=box];

 n1 [label="AgentLoop\\n(struct)" shape=box];
 n2 [label="run\\n(function)" shape=ellipse];
 n3 [label="AIProvider\\n(trait)" shape=diamond];
 n4 [label="OllamaProvider\\n(struct)" shape=box];
 n5 [label="ToolExecutor\\n(struct)" shape=box];
 n6 [label="execute_tool\\n(function)" shape=ellipse];
 n7 [label="EmbeddingIndex\\n(struct)" shape=box];
 n8 [label="search\\n(function)" shape=ellipse];

 n1 -> n2 [label="contains"];
 n2 -> n3 [label="calls"];
 n4 -> n3 [label="implements"];
 n2 -> n6 [label="calls"];
 n5 -> n6 [label="contains"];
 n2 -> n8 [label="calls"];
 n7 -> n8 [label="contains"];
}`}
         </pre>
         <button style={{ marginTop: 8, padding: "6px 12px", background: "var(--accent-color)", color: "white", border: "none", borderRadius: 4, cursor: "pointer" }}>
           Copy to Clipboard
         </button>
       </div>
     )}
   </div>
 );
}
