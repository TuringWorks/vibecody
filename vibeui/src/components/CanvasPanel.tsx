/* eslint-disable @typescript-eslint/no-explicit-any */
import React, { useState, useCallback, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronLeft } from "lucide-react";

// ── Types ──────────────────────────────────────────────────────────────────

interface CanvasNode {
 id: string;
 type: "provider" | "skill" | "tool" | "gateway" | "transform";
 label: string;
 x: number;
 y: number;
 config: Record<string, string>;
}

interface CanvasEdge {
 from: string;
 to: string;
 label?: string;
}

interface CanvasWorkflow {
 name: string;
 nodes: CanvasNode[];
 edges: CanvasEdge[];
}

const NODE_COLORS: Record<CanvasNode["type"], string> = {
 provider: "var(--accent-color)",
 skill: "var(--success-color)",
 tool: "var(--warning-color)",
 gateway: "var(--accent-purple)",
 transform: "var(--error-color)",
};

const NODE_WIDTH = 140;
const NODE_HEIGHT = 50;

// ── Component ──────────────────────────────────────────────────────────────

export default function CanvasPanel() {
 const [workflows, setWorkflows] = useState<CanvasWorkflow[]>([]);
 const [currentWorkflow, setCurrentWorkflow] = useState<CanvasWorkflow>({
 name: "Untitled",
 nodes: [],
 edges: [],
 });
 const [selectedNode, setSelectedNode] = useState<string | null>(null);
 const [dragging, setDragging] = useState<{ id: string; offsetX: number; offsetY: number } | null>(null);
 const [connecting, setConnecting] = useState<string | null>(null);
 const [newName, setNewName] = useState("");
 const [showPalette, setShowPalette] = useState(true);
 const svgRef = useRef<SVGSVGElement>(null);

 useEffect(() => {
 loadWorkflows();
 }, []);

 const loadWorkflows = async () => {
 try {
 const list: CanvasWorkflow[] = await invoke("list_canvas_workflows");
 setWorkflows(list);
 } catch {
 // Commands may not exist yet
 }
 };

 const addNode = useCallback((type: CanvasNode["type"]) => {
 const id = `${type}_${Date.now()}`;
 const newNode: CanvasNode = {
 id,
 type,
 label: `${type.charAt(0).toUpperCase() + type.slice(1)} ${currentWorkflow.nodes.filter(n => n.type === type).length + 1}`,
 x: 100 + Math.random() * 300,
 y: 100 + Math.random() * 200,
 config: {},
 };
 setCurrentWorkflow(w => ({ ...w, nodes: [...w.nodes, newNode] }));
 setSelectedNode(id);
 }, [currentWorkflow.nodes]);

 const deleteNode = useCallback((id: string) => {
 setCurrentWorkflow(w => ({
 ...w,
 nodes: w.nodes.filter(n => n.id !== id),
 edges: w.edges.filter(e => e.from !== id && e.to !== id),
 }));
 if (selectedNode === id) setSelectedNode(null);
 }, [selectedNode]);

 const handleMouseDown = (nodeId: string, e: React.MouseEvent) => {
 e.preventDefault();
 if (connecting) {
 if (connecting !== nodeId) {
 setCurrentWorkflow(w => ({
 ...w,
 edges: [...w.edges, { from: connecting, to: nodeId }],
 }));
 }
 setConnecting(null);
 return;
 }
 const node = currentWorkflow.nodes.find(n => n.id === nodeId);
 if (!node) return;
 const svgRect = svgRef.current?.getBoundingClientRect();
 if (!svgRect) return;
 setDragging({ id: nodeId, offsetX: e.clientX - svgRect.left - node.x, offsetY: e.clientY - svgRect.top - node.y });
 setSelectedNode(nodeId);
 };

 const handleMouseMove = (e: React.MouseEvent) => {
 if (!dragging) return;
 const svgRect = svgRef.current?.getBoundingClientRect();
 if (!svgRect) return;
 const x = Math.max(0, e.clientX - svgRect.left - dragging.offsetX);
 const y = Math.max(0, e.clientY - svgRect.top - dragging.offsetY);
 setCurrentWorkflow(w => ({
 ...w,
 nodes: w.nodes.map(n => n.id === dragging.id ? { ...n, x, y } : n),
 }));
 };

 const handleMouseUp = () => {
 setDragging(null);
 };

 const handleSave = async () => {
 const name = newName.trim() || currentWorkflow.name;
 const wf = { ...currentWorkflow, name };
 try {
 await invoke("save_canvas_workflow", { workflow: wf });
 setCurrentWorkflow(wf);
 loadWorkflows();
 } catch (err: any) {
 console.error("save canvas:", err);
 }
 };

 const handleRun = async () => {
 try {
 await invoke("run_canvas_workflow", { workflow: currentWorkflow });
 } catch (err: any) {
 console.error("run canvas:", err);
 }
 };

 const nodeCenter = (id: string) => {
 const n = currentWorkflow.nodes.find(n => n.id === id);
 if (!n) return { x: 0, y: 0 };
 return { x: n.x + NODE_WIDTH / 2, y: n.y + NODE_HEIGHT / 2 };
 };

 return (
 <div className="panel-container" style={{ flexDirection: "row" }}>
 {/* Palette sidebar */}
 {showPalette && (
 <div style={{ width: 180, flexShrink: 0, borderRight: "1px solid var(--border-color)", padding: 12, display: "flex", flexDirection: "column", gap: 8 }}>
 <div style={{ fontWeight: 600, marginBottom: 8 }}>Node Palette</div>
 {(["provider", "skill", "tool", "gateway", "transform"] as CanvasNode["type"][]).map(type => (
 <button
 key={type}
 onClick={() => addNode(type)}
 style={{
 background: NODE_COLORS[type] + "22",
 border: `1px solid ${NODE_COLORS[type]}`,
 color: NODE_COLORS[type],
 padding: "8px 12px",
 borderRadius: "var(--radius-xs-plus)",
 cursor: "pointer",
 textAlign: "left",
 }}
 >
 + {type.charAt(0).toUpperCase() + type.slice(1)}
 </button>
 ))}
 <hr style={{ borderColor: "var(--border-color)", margin: "8px 0" }} />
 <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.7 }}>
 Click a node type to add it to the canvas. Drag nodes to position. Right-click to connect.
 </div>

 <hr style={{ borderColor: "var(--border-color)", margin: "8px 0" }} />
 <div style={{ fontWeight: 600 }}>Workflows</div>
 {workflows.map((w, i) => (
 <button
 key={i}
 onClick={() => setCurrentWorkflow(w)}
 style={{
 background: "var(--bg-secondary)",
 border: "1px solid var(--border-color)",
 color: "var(--text-secondary)",
 padding: "4px 8px",
 borderRadius: "var(--radius-xs-plus)",
 cursor: "pointer",
 textAlign: "left",
 fontSize: "var(--font-size-base)",
 }}
 >
 {w.name} ({w.nodes.length} nodes)
 </button>
 ))}
 </div>
 )}

 {/* Canvas */}
 <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
 {/* Toolbar */}
 <div style={{ display: "flex", gap: 8, padding: "8px 12px", borderBottom: "1px solid var(--border-color)", alignItems: "center" }}>
 <button onClick={() => setShowPalette(!showPalette)} style={{ background: "var(--bg-secondary)", border: "none", color: "var(--text-secondary)", padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", cursor: "pointer" }}>
 <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>{showPalette && <ChevronLeft size={12} />} Palette</span>
 </button>
 <input
 id="canvas-workflow-name"
 value={newName || currentWorkflow.name}
 onChange={e => setNewName(e.target.value)}
 placeholder="Workflow name"
 aria-label="Workflow name"
 className="panel-input"
 style={{ flex: 1, maxWidth: 200 }}
 />
 <button onClick={handleSave} className="panel-btn panel-btn-primary panel-btn-xs">
 Save
 </button>
 <button onClick={handleRun} className="panel-btn panel-btn-primary panel-btn-xs">
 Run
 </button>
 {connecting && (
 <span style={{ color: "var(--warning-color)", fontSize: "var(--font-size-base)" }}>Click a target node to connect...</span>
 )}
 <span style={{ marginLeft: "auto", fontSize: "var(--font-size-base)", opacity: 0.5 }}>
 {currentWorkflow.nodes.length} nodes, {currentWorkflow.edges.length} edges
 </span>
 </div>

 {/* SVG canvas */}
 <svg
 ref={svgRef}
 style={{ flex: 1, cursor: dragging ? "grabbing" : "default" }}
 onMouseMove={handleMouseMove}
 onMouseUp={handleMouseUp}
 onMouseLeave={handleMouseUp}
 >
 {/* Grid pattern */}
 <defs>
 <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
 <path d="M 20 0 L 0 0 0 20" fill="none" stroke="var(--border-color)" strokeWidth="0.5" />
 </pattern>
 </defs>
 <rect width="100%" height="100%" fill="url(#grid)" />

 {/* Edges */}
 {currentWorkflow.edges.map((edge, i) => {
 const from = nodeCenter(edge.from);
 const to = nodeCenter(edge.to);
 const midX = (from.x + to.x) / 2;
 const midY = (from.y + to.y) / 2;
 return (
 <g key={i}>
 <path
 d={`M ${from.x} ${from.y} Q ${midX} ${from.y} ${to.x} ${to.y}`}
 fill="none"
 stroke="var(--text-secondary)"
 strokeWidth={2}
 markerEnd="url(#arrowhead)"
 />
 {edge.label && (
 <text x={midX} y={midY - 5} fill="var(--text-secondary)" fontSize={10} textAnchor="middle">
 {edge.label}
 </text>
 )}
 </g>
 );
 })}

 {/* Arrow marker */}
 <defs>
 <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
 <polygon points="0 0, 10 3.5, 0 7" fill="var(--text-secondary)" />
 </marker>
 </defs>

 {/* Nodes */}
 {currentWorkflow.nodes.map(node => (
 <g
 key={node.id}
 onMouseDown={e => handleMouseDown(node.id, e)}
 onContextMenu={e => { e.preventDefault(); setConnecting(node.id); }}
 style={{ cursor: dragging?.id === node.id ? "grabbing" : "grab" }}
 >
 <rect
 x={node.x}
 y={node.y}
 width={NODE_WIDTH}
 height={NODE_HEIGHT}
 rx={6}
 fill={NODE_COLORS[node.type] + "22"}
 stroke={selectedNode === node.id ? "white" : NODE_COLORS[node.type]}
 strokeWidth={selectedNode === node.id ? 2 : 1}
 />
 <circle
 cx={node.x + 12}
 cy={node.y + NODE_HEIGHT / 2}
 r={5}
 fill={NODE_COLORS[node.type]}
 />
 <text
 x={node.x + 24}
 y={node.y + NODE_HEIGHT / 2 + 4}
 fill="var(--text-secondary)"
 fontSize={12}
 fontFamily="monospace"
 >
 {node.label.length > 14 ? node.label.slice(0, 14) + "…" : node.label}
 </text>
 {/* Delete button */}
 <g
 transform={`translate(${node.x + NODE_WIDTH - 14}, ${node.y + 10})`}
 style={{ cursor: "pointer" }}
 onClick={(e) => { e.stopPropagation(); deleteNode(node.id); }}
 role="button"
 aria-label={`Delete element ${node.label}`}
 >
 <line x1="-4" y1="-4" x2="4" y2="4" stroke="var(--text-secondary)" strokeWidth="1.5" strokeLinecap="round" />
 <line x1="4" y1="-4" x2="-4" y2="4" stroke="var(--text-secondary)" strokeWidth="1.5" strokeLinecap="round" />
 </g>
 </g>
 ))}
 </svg>
 </div>

 {/* Properties sidebar — shown when a node is selected */}
 {selectedNode && (() => {
  const node = currentWorkflow.nodes.find(n => n.id === selectedNode);
  if (!node) return null;

  const updateConfig = (key: string, value: string) => {
   setCurrentWorkflow(w => ({
    ...w,
    nodes: w.nodes.map(n => n.id === selectedNode ? { ...n, config: { ...n.config, [key]: value } } : n),
   }));
  };

  const updateLabel = (label: string) => {
   setCurrentWorkflow(w => ({
    ...w,
    nodes: w.nodes.map(n => n.id === selectedNode ? { ...n, label } : n),
   }));
  };

  const pInput: React.CSSProperties = { width: "100%", padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", boxSizing: "border-box" as const };
  const pLabel: React.CSSProperties = { fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginBottom: 2, display: "block", marginTop: 8 };

  const toNodes = currentWorkflow.edges.filter(e => e.from === selectedNode).map(e => currentWorkflow.nodes.find(n => n.id === e.to)?.label).filter(Boolean);
  const fromNodes = currentWorkflow.edges.filter(e => e.to === selectedNode).map(e => currentWorkflow.nodes.find(n => n.id === e.from)?.label).filter(Boolean);

  return (
   <div style={{ width: 220, flexShrink: 0, borderLeft: "1px solid var(--border-color)", padding: 12, overflowY: "auto", fontSize: "var(--font-size-base)" }}>
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
     <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Properties</span>
     <button onClick={() => setSelectedNode(null)} aria-label="Close properties panel" style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: "var(--font-size-lg)" }}>x</button>
    </div>

    <div style={{ padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", background: NODE_COLORS[node.type] + "22", color: NODE_COLORS[node.type], fontSize: "var(--font-size-sm)", fontWeight: 600, marginBottom: 8 }}>
     {node.type.toUpperCase()}
    </div>

    <label style={pLabel}>Label</label>
    <input style={pInput} value={node.label} onChange={e => updateLabel(e.target.value)} />

    <label style={pLabel}>ID</label>
    <input style={{ ...pInput, opacity: 0.5 }} value={node.id} readOnly />

    {node.type === "provider" && (<>
     <label style={pLabel}>Provider</label>
     <select style={pInput} value={node.config.provider || ""} onChange={e => updateConfig("provider", e.target.value)}>
      <option value="">Select...</option>
      {["ollama","claude","openai","gemini","grok","groq","deepseek","perplexity","together","fireworks","sambanova","mistral"].map(p => <option key={p} value={p}>{p}</option>)}
     </select>
     <label style={pLabel}>Model</label>
     <input style={pInput} placeholder="e.g. llama3.2:8b" value={node.config.model || ""} onChange={e => updateConfig("model", e.target.value)} />
     <label style={pLabel}>Temperature</label>
     <input style={pInput} type="number" step="0.1" min="0" max="2" placeholder="0.7" value={node.config.temperature || ""} onChange={e => updateConfig("temperature", e.target.value)} />
    </>)}

    {node.type === "skill" && (<>
     <label style={pLabel}>Skill File</label>
     <input style={pInput} placeholder="e.g. code-review" value={node.config.skill || ""} onChange={e => updateConfig("skill", e.target.value)} />
     <label style={pLabel}>Trigger Keywords</label>
     <input style={pInput} placeholder="e.g. review, lint" value={node.config.triggers || ""} onChange={e => updateConfig("triggers", e.target.value)} />
    </>)}

    {node.type === "tool" && (<>
     <label style={pLabel}>Tool Type</label>
     <select style={pInput} value={node.config.tool_type || ""} onChange={e => updateConfig("tool_type", e.target.value)}>
      <option value="">Select...</option>
      {["bash","read_file","write_file","search_files","web_search","fetch_url","mcp"].map(t => <option key={t} value={t}>{t}</option>)}
     </select>
     {node.config.tool_type === "bash" && (<>
      <label style={pLabel}>Command</label>
      <input style={pInput} placeholder="e.g. cargo test" value={node.config.command || ""} onChange={e => updateConfig("command", e.target.value)} />
     </>)}
     {node.config.tool_type === "mcp" && (<>
      <label style={pLabel}>MCP Server</label>
      <input style={pInput} placeholder="e.g. terraform" value={node.config.mcp_server || ""} onChange={e => updateConfig("mcp_server", e.target.value)} />
      <label style={pLabel}>Tool Name</label>
      <input style={pInput} placeholder="e.g. tf_plan" value={node.config.mcp_tool || ""} onChange={e => updateConfig("mcp_tool", e.target.value)} />
     </>)}
    </>)}

    {node.type === "gateway" && (<>
     <label style={pLabel}>Platform</label>
     <select style={pInput} value={node.config.platform || ""} onChange={e => updateConfig("platform", e.target.value)}>
      <option value="">Select...</option>
      {["slack","discord","telegram","github","linear","teams","webhook"].map(p => <option key={p} value={p}>{p}</option>)}
     </select>
     <label style={pLabel}>Channel</label>
     <input style={pInput} placeholder="e.g. #general" value={node.config.channel || ""} onChange={e => updateConfig("channel", e.target.value)} />
    </>)}

    {node.type === "transform" && (<>
     <label style={pLabel}>Transform Type</label>
     <select style={pInput} value={node.config.transform || ""} onChange={e => updateConfig("transform", e.target.value)}>
      <option value="">Select...</option>
      {["filter","map","split","merge","delay"].map(t => <option key={t} value={t}>{t}</option>)}
     </select>
     <label style={pLabel}>Expression</label>
     <input style={pInput} placeholder="e.g. status == 'error'" value={node.config.expression || ""} onChange={e => updateConfig("expression", e.target.value)} />
    </>)}

    {(fromNodes.length > 0 || toNodes.length > 0) && (
     <div style={{ marginTop: 12, padding: 8, background: "var(--bg-secondary)", borderRadius: "var(--radius-xs-plus)" }}>
      <div style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 4 }}>CONNECTIONS</div>
      {fromNodes.length > 0 && <div style={{ fontSize: "var(--font-size-sm)" }}>From: {fromNodes.join(", ")}</div>}
      {toNodes.length > 0 && <div style={{ fontSize: "var(--font-size-sm)" }}>To: {toNodes.join(", ")}</div>}
     </div>
    )}

    <div style={{ marginTop: 12, display: "flex", flexDirection: "column", gap: 4 }}>
     <button onClick={() => setConnecting(selectedNode)} style={{ padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-secondary)", cursor: "pointer", fontSize: "var(--font-size-sm)" }}>
      Connect to another node
     </button>
     <button onClick={() => deleteNode(selectedNode)} style={{ padding: "4px 8px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--error-color)", background: "transparent", color: "var(--error-color)", cursor: "pointer", fontSize: "var(--font-size-sm)" }}>
      Delete node
     </button>
    </div>
   </div>
  );
 })()}
 </div>
 );
}
