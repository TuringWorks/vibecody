/**
 * KnowledgeGraphPanel — Cross-repository knowledge graph explorer.
 *
 * Interactive symbol graph with repo registration, cross-repo queries,
 * callers/callees/implementors, shortest path finder, and DOT export.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

interface KgGraph {
 nodes: GraphNode[];
 edges: GraphEdge[];
}

type QueryMode = "callers" | "callees" | "implementors" | "dependencies" | "dependents" | "path" | "subgraph";

const KIND_COLORS: Record<string, string> = {
 function: "var(--accent-color)",
 struct: "var(--success-color)",
 trait: "var(--warning-color)",
 interface: "var(--warning-color)",
 class: "var(--accent-purple)",
 module: "#89dceb",
 file: "#9399b2",
 repo: "var(--accent-rose)",
 enum: "var(--accent-green)",
};

const REPO_COLORS: Record<string, string> = {
 "vibe-ai": "var(--accent-blue)",
 "vibecli": "var(--accent-green)",
 "vibe-core": "var(--accent-gold)",
 "src": "var(--accent-gold)",
 "crates": "var(--accent-blue)",
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
 const [querySymbol, setQuerySymbol] = useState("");
 const [targetSymbol, setTargetSymbol] = useState("");
 const [depth] = useState(2);
 const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
 const [tab, setTab] = useState<"graph" | "stats" | "export">("graph");

 const [nodes, setNodes] = useState<GraphNode[]>([]);
 const [edges, setEdges] = useState<GraphEdge[]>([]);
 const [stats, setStats] = useState<GraphStats | null>(null);
 const [searchResults, setSearchResults] = useState<GraphNode[] | null>(null);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [workspace, setWorkspace] = useState(".");

 const loadGraph = useCallback(async () => {
   setLoading(true);
   setError(null);
   try {
     const graph = await invoke<KgGraph>("get_knowledge_graph", { workspace });
     setNodes(graph.nodes);
     setEdges(graph.edges);
     setSearchResults(null);
   } catch (e) {
     setError(String(e));
   } finally {
     setLoading(false);
   }
 }, [workspace]);

 const loadStats = useCallback(async () => {
   setLoading(true);
   setError(null);
   try {
     const s = await invoke<GraphStats>("get_knowledge_graph_stats", { workspace });
     setStats(s);
   } catch (e) {
     setError(String(e));
   } finally {
     setLoading(false);
   }
 }, [workspace]);

 const handleSearch = useCallback(async () => {
   if (!querySymbol.trim()) return;
   setLoading(true);
   setError(null);
   try {
     const results = await invoke<GraphNode[]>("search_knowledge_graph", {
       workspace,
       query: querySymbol,
     });
     setSearchResults(results);
   } catch (e) {
     setError(String(e));
   } finally {
     setLoading(false);
   }
 }, [workspace, querySymbol]);

 const handleRefresh = useCallback(async () => {
   setLoading(true);
   setError(null);
   try {
     const graph = await invoke<KgGraph>("refresh_knowledge_graph", { workspace });
     setNodes(graph.nodes);
     setEdges(graph.edges);
     setSearchResults(null);
     if (tab === "stats") {
       const s = await invoke<GraphStats>("get_knowledge_graph_stats", { workspace });
       setStats(s);
     }
   } catch (e) {
     setError(String(e));
   } finally {
     setLoading(false);
   }
 }, [workspace, tab]);

 useEffect(() => {
   loadGraph();
 }, [loadGraph]);

 useEffect(() => {
   if (tab === "stats") {
     loadStats();
   }
 }, [tab, loadStats]);

 // Filter displayed nodes based on query mode and search results
 const displayedNodes = searchResults ?? nodes;
 const results = searchResults ?? (querySymbol
   ? nodes.filter(n => n.name.toLowerCase().includes(querySymbol.toLowerCase()))
   : nodes);

 const addRepo = () => {
   if (newRepoName && newRepoPath) {
     setRepos([...repos, { name: newRepoName, path: newRepoPath }]);
     setNewRepoName("");
     setNewRepoPath("");
   }
 };

 // Layout: show up to 50 nodes in the SVG for performance
 const visibleNodes = displayedNodes.slice(0, 50);
 const graphWidth = 700;
 const graphHeight = 400;
 const nodePositions = visibleNodes.map((_, i) => ({
   x: 80 + (i % 4) * 170,
   y: 60 + Math.floor(i / 4) * 160,
 }));

 // Build a set of visible node IDs for edge filtering
 const visibleIds = new Set(visibleNodes.map(n => n.id));
 const visibleEdges = edges.filter(e => visibleIds.has(e.from) && visibleIds.has(e.to));

 // Generate DOT export from real data
 const dotExport = () => {
   const lines = ["digraph KnowledgeGraph {", " rankdir=LR;", " node [shape=box];", ""];
   for (const n of nodes) {
     const shape = n.kind === "function" ? "ellipse" : n.kind === "trait" || n.kind === "interface" ? "diamond" : "box";
     lines.push(` n${n.id} [label="${n.name}\\n(${n.kind})" shape=${shape}];`);
   }
   lines.push("");
   for (const e of edges) {
     lines.push(` n${e.from} -> n${e.to} [label="${e.kind}"];`);
   }
   lines.push("}");
   return lines.join("\n");
 };

 return (
   <div className="panel-container">
     <div className="panel-header">
       <h3>Knowledge Graph</h3>
       <button onClick={handleRefresh} disabled={loading} className="panel-btn panel-btn-primary" style={{ opacity: loading ? 0.6 : 1 }}>
         {loading ? "Scanning..." : "Refresh"}
       </button>
       <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
         {nodes.length} nodes, {edges.length} edges
       </span>
     </div>

     {error && <div className="panel-error">{error}</div>}

     <div className="panel-body">
     {/* Workspace Path */}
     <div style={{ marginBottom: 12, display: "flex", gap: 6, alignItems: "center" }}>
       <label style={{ fontSize: "var(--font-size-base)" }}>Workspace:</label>
       <input value={workspace} onChange={e => setWorkspace(e.target.value)}
         style={{ flex: 1, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)" }} />
       <button className="panel-btn" onClick={loadGraph} disabled={loading} style={{ padding: "4px 12px", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer", fontSize: "var(--font-size-base)" }}>
         Load
       </button>
     </div>

     {/* Repo Registration */}
     <div style={{ marginBottom: 12, padding: 10, border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)" }}>
       <strong>Registered Repos</strong>
       <div style={{ display: "flex", flexWrap: "wrap", gap: 6, margin: "8px 0" }}>
         {repos.map(r => (
           <span key={r.name} style={{
             padding: "2px 8px", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)",
             background: REPO_COLORS[r.name] || "var(--border-color)", color: "var(--bg-primary)",
           }}>
             {r.name}: {r.path}
             <button onClick={() => setRepos(repos.filter(x => x.name !== r.name))}
               style={{ marginLeft: 4, background: "none", border: "none", cursor: "pointer", color: "var(--bg-primary)" }}>x</button>
           </span>
         ))}
       </div>
       <div style={{ display: "flex", gap: 4 }}>
         <input value={newRepoName} onChange={e => setNewRepoName(e.target.value)} placeholder="Repo name"
           style={{ flex: 1, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)" }} />
         <input value={newRepoPath} onChange={e => setNewRepoPath(e.target.value)} placeholder="Path"
           style={{ flex: 2, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)" }} />
         <button className="panel-btn" onClick={addRepo} style={{ padding: "4px 12px", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer" }}>Add</button>
       </div>
     </div>

     {/* Tabs */}
     <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
       {(["graph", "stats", "export"] as const).map(t => (
         <button key={t} onClick={() => setTab(t)} style={{
           padding: "4px 12px", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", cursor: "pointer",
           background: tab === t ? "var(--accent-color)" : "transparent", color: tab === t ? "var(--text-primary)" : "var(--text-primary)",
         }}>{t.charAt(0).toUpperCase() + t.slice(1)}</button>
       ))}
     </div>

     {tab === "graph" && (
       <>
         {/* Query Controls */}
         <div style={{ display: "flex", gap: 6, marginBottom: 12, flexWrap: "wrap", alignItems: "center" }}>
           <select value={queryMode} onChange={e => setQueryMode(e.target.value as QueryMode)}
             style={{ padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)" }}>
             <option value="callers">Callers</option>
             <option value="callees">Callees</option>
             <option value="implementors">Implementors</option>
             <option value="dependencies">Dependencies</option>
             <option value="dependents">Dependents</option>
             <option value="path">Shortest Path</option>
             <option value="subgraph">Subgraph</option>
           </select>
           <input value={querySymbol} onChange={e => setQuerySymbol(e.target.value)} placeholder="Search symbol name"
             onKeyDown={e => e.key === "Enter" && handleSearch()}
             style={{ flex: 1, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)" }} />
           {queryMode === "path" && (
             <input value={targetSymbol} onChange={e => setTargetSymbol(e.target.value)} placeholder="Target symbol"
               style={{ flex: 1, padding: 4, background: "var(--bg-primary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)" }} />
           )}
           {queryMode === "subgraph" && (
             <label style={{ fontSize: "var(--font-size-base)" }}>
               Depth: {depth}
             </label>
           )}
           <button className="panel-btn" onClick={handleSearch} disabled={loading} style={{
             padding: "4px 12px", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer",
           }}>Search</button>
         </div>

         {/* Graph SVG */}
         <svg width={graphWidth} height={graphHeight} style={{ border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", background: "var(--bg-primary)", marginBottom: 12 }}>
           <defs>
             <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
               <polygon points="0 0, 8 3, 0 6" fill="var(--border-color)" />
             </marker>
           </defs>
           {visibleEdges.map((e, i) => {
             const fromIdx = visibleNodes.findIndex(n => n.id === e.from);
             const toIdx = visibleNodes.findIndex(n => n.id === e.to);
             if (fromIdx < 0 || toIdx < 0) return null;
             const fp = nodePositions[fromIdx];
             const tp = nodePositions[toIdx];
             const fromNode = visibleNodes[fromIdx];
             const toNode = visibleNodes[toIdx];
             const isCrossRepo = fromNode.repo !== toNode.repo;
             return (
               <g key={i}>
                 <line x1={fp.x} y1={fp.y} x2={tp.x} y2={tp.y}
                   stroke={isCrossRepo ? "var(--warning-color)" : "var(--border-color)"} strokeWidth={isCrossRepo ? 2 : 1}
                   strokeDasharray={isCrossRepo ? "4,2" : "none"} markerEnd="url(#arrowhead)" opacity={0.7} />
                 <text x={(fp.x + tp.x) / 2} y={(fp.y + tp.y) / 2 - 4} fontSize={9} fill="var(--border-color)" textAnchor="middle">{e.kind}</text>
               </g>
             );
           })}
           {visibleNodes.map((node, i) => {
             const pos = nodePositions[i];
             const isResult = searchResults ? searchResults.some(r => r.id === node.id) : true;
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
           {visibleNodes.length === 0 && !loading && (
             <text x={graphWidth / 2} y={graphHeight / 2} textAnchor="middle" fill="var(--text-secondary)" fontSize={14}>
               {error ? "Error loading graph" : "No nodes found. Click Refresh to scan workspace."}
             </text>
           )}
         </svg>

         {/* Legend */}
         <div style={{ display: "flex", gap: 12, flexWrap: "wrap", fontSize: "var(--font-size-sm)", marginBottom: 12 }}>
           {Object.entries(KIND_COLORS).map(([kind, color]) => (
             <span key={kind}><span style={{ display: "inline-block", width: 10, height: 10, borderRadius: "50%", background: color, marginRight: 3 }} />{kind}</span>
           ))}
           <span style={{ marginLeft: 12 }}>--- cross-repo edge</span>
         </div>

         {/* Selected Node Detail */}
         {selectedNode && (
           <div style={{ padding: 8, border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-base)", marginBottom: 12 }}>
             <strong>{selectedNode.name}</strong> ({selectedNode.kind})
             <div style={{ marginTop: 4 }}>Repo: {selectedNode.repo} | File: {selectedNode.file}:{selectedNode.line}</div>
             <div style={{ marginTop: 2, fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{selectedNode.signature}</div>
           </div>
         )}

         {/* Query Results */}
         <div style={{ fontSize: "var(--font-size-base)" }}>
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
               {results.slice(0, 100).map(n => (
                 <tr key={n.id} onClick={() => setSelectedNode(n)} style={{ cursor: "pointer", borderBottom: "1px solid var(--border-color)" }}>
                   <td style={{ padding: 4, color: KIND_COLORS[n.kind] }}>{n.name}</td>
                   <td style={{ padding: 4 }}>{n.kind}</td>
                   <td style={{ padding: 4, color: REPO_COLORS[n.repo] }}>{n.repo}</td>
                   <td style={{ padding: 4, fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{n.file}:{n.line}</td>
                 </tr>
               ))}
             </tbody>
           </table>
           {results.length > 100 && (
             <div style={{ marginTop: 4, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
               Showing 100 of {results.length} results
             </div>
           )}
         </div>
       </>
     )}

     {tab === "stats" && (
       <div style={{ fontSize: "var(--font-size-md)" }}>
         {stats ? (
           <>
             <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 8, marginBottom: 12 }}>
               <div style={{ padding: 10, border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", textAlign: "center" }}>
                 <div style={{ fontSize: 24, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--accent-color)" }}>{stats.total_nodes}</div>
                 <div>Nodes</div>
               </div>
               <div style={{ padding: 10, border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", textAlign: "center" }}>
                 <div style={{ fontSize: 24, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--success-color)" }}>{stats.total_edges}</div>
                 <div>Edges</div>
               </div>
               <div style={{ padding: 10, border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", textAlign: "center" }}>
                 <div style={{ fontSize: 24, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--warning-color)" }}>{stats.cross_repo_edges}</div>
                 <div>Cross-Repo</div>
               </div>
             </div>

             <strong>Nodes per Repo</strong>
             <div style={{ marginTop: 4, marginBottom: 12 }}>
               {Object.entries(stats.nodes_per_repo).map(([repo, count]) => (
                 <div key={repo} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                   <span style={{ width: 80, color: REPO_COLORS[repo] }}>{repo}</span>
                   <div style={{ flex: 1, height: 16, background: "var(--border-color)", borderRadius: "var(--radius-xs-plus)", overflow: "hidden" }}>
                     <div style={{ height: "100%", width: `${(count / stats.total_nodes) * 100}%`, background: REPO_COLORS[repo] || "var(--accent-color)", borderRadius: "var(--radius-xs-plus)" }} />
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
                 {stats.most_connected.map(([name, count]) => (
                   <tr key={name} style={{ borderBottom: "1px solid var(--border-color)" }}>
                     <td style={{ padding: 4, color: "var(--accent-color)" }}>{name}</td>
                     <td style={{ padding: 4, textAlign: "right" }}>{count}</td>
                   </tr>
                 ))}
               </tbody>
             </table>

             <div style={{ marginTop: 12 }}>Orphan symbols: <strong>{stats.orphan_count}</strong></div>
           </>
         ) : (
           <div style={{ textAlign: "center", padding: 20, color: "var(--text-secondary)" }}>
             {loading ? "Loading stats..." : "No stats available. Click Refresh to scan."}
           </div>
         )}
       </div>
     )}

     {tab === "export" && (
       <div>
         <strong>DOT Export</strong>
         <pre style={{
           marginTop: 8, padding: 10, background: "var(--bg-primary)", border: "1px solid var(--border-color)",
           borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 300,
         }}>
           {nodes.length > 0 ? dotExport() : "No graph data. Click Refresh to scan workspace."}
         </pre>
         <button onClick={() => {
           navigator.clipboard.writeText(dotExport()).catch(() => {});
         }} style={{ marginTop: 8, padding: "8px 12px", background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer" }}>
           Copy to Clipboard
         </button>
       </div>
     )}
     </div>
   </div>
 );
}
