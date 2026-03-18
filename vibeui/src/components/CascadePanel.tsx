/**
 * CascadePanel — unified AI interaction timeline (Cascade flows equivalent).
 *
 * Shows all AI events (chat, inline edits, agent steps, terminal commands)
 * in a single chronological feed so the user has a full picture of what
 * happened in their workspace session.
 *
 * Equivalent to Windsurf's "Cascade" unified context view.
 */

import { useState, useEffect, useCallback } from "react";
import { flowContext, FlowEvent, FlowEventKind } from "../utils/FlowContext";

const KIND_ICONS: Record<FlowEventKind, string> = {
 chat: "",
 inline_edit: "",
 inline_generate: "",
 agent_step: "",
 agent_complete: "",
 terminal_cmd: "",
 file_edit: "",
};

const KIND_LABELS: Record<FlowEventKind, string> = {
 chat: "Chat",
 inline_edit: "Inline Edit",
 inline_generate: "Generate",
 agent_step: "Agent Step",
 agent_complete: "Agent Done",
 terminal_cmd: "Terminal",
 file_edit: "File Edit",
};

type FilterKind = "all" | FlowEventKind;

function timeAgo(ms: number): string {
 const secs = Math.floor((Date.now() - ms) / 1000);
 if (secs < 60) return `${secs}s`;
 const mins = Math.floor(secs / 60);
 if (mins < 60) return `${mins}m`;
 return `${Math.floor(mins / 60)}h`;
}

// ── CascadePanel ─────────────────────────────────────────────────────────────

interface CascadePanelProps {
 /** Called when the user clicks "Inject into chat" on an event. */
 onInjectContext?: (text: string) => void;
}

export function CascadePanel({ onInjectContext }: CascadePanelProps) {
 const [events, setEvents] = useState<FlowEvent[]>([]);
 const [filter, setFilter] = useState<FilterKind>("all");
 const [expandedId, setExpandedId] = useState<string | null>(null);
 const [copied, setCopied] = useState(false);

 // Subscribe to flow context updates
 useEffect(() => {
 setEvents(flowContext.getAll());
 return flowContext.subscribe((evs) => setEvents([...evs]));
 }, []);

 const visible = filter === "all"
 ? [...events].reverse()
 : [...events].filter((e) => e.kind === filter).reverse();

 const handleCopyAll = useCallback(async () => {
 const summary = flowContext.getContextSummary(4000);
 try { await navigator.clipboard.writeText(summary); } catch { /* clipboard may be unavailable */ }
 setCopied(true);
 setTimeout(() => setCopied(false), 2000);
 }, []);

 const handleInject = useCallback((ev: FlowEvent) => {
 const text = `[${KIND_LABELS[ev.kind]}] ${ev.summary}\n${ev.detail}`;
 onInjectContext?.(text);
 }, [onInjectContext]);

 const filters: FilterKind[] = ["all", "chat", "inline_edit", "agent_step", "agent_complete", "terminal_cmd", "file_edit"];

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)" }}>

 {/* ── Header ────────────────────────────────────────────────────────── */}
 <div style={{
 display: "flex",
 alignItems: "center",
 justifyContent: "space-between",
 padding: "8px 12px",
 borderBottom: "1px solid var(--border-color)",
 background: "var(--bg-secondary)",
 flexShrink: 0,
 }}>
 <span style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)" }}>
 Cascade Flow
 </span>
 <div style={{ display: "flex", gap: 6 }}>
 {events.length > 0 && (
 <>
 <button
 onClick={handleCopyAll}
 style={btnStyle}
 title="Copy all events as AI context"
 >
 {copied ? "✓ Copied" : "Copy All"}
 </button>
 <button
 onClick={() => flowContext.clear()}
 style={{ ...btnStyle, color: "var(--text-danger)" }}
 title="Clear flow history"
 >
 Clear
 </button>
 </>
 )}
 </div>
 </div>

 {/* ── Filter bar ────────────────────────────────────────────────────── */}
 <div style={{
 display: "flex",
 gap: 4,
 padding: "6px 10px",
 borderBottom: "1px solid var(--border-color)",
 flexWrap: "wrap",
 flexShrink: 0,
 background: "var(--bg-secondary)",
 }}>
 {filters.map((f) => (
 <button
 key={f}
 onClick={() => setFilter(f)}
 style={{
 padding: "2px 8px",
 fontSize: 11,
 borderRadius: 10,
 border: "1px solid var(--border-color)",
 background: filter === f ? "var(--accent-color)" : "transparent",
 color: filter === f ? "var(--text-primary)" : "var(--text-secondary)",
 cursor: "pointer",
 }}
 >
 {f === "all" ? "All" : KIND_LABELS[f as FlowEventKind]}
 </button>
 ))}
 </div>

 {/* ── Timeline ──────────────────────────────────────────────────────── */}
 <div style={{ flex: 1, overflowY: "auto", padding: "8px 0" }}>
 {visible.length === 0 ? (
 <div style={{
 padding: 24,
 textAlign: "center",
 color: "var(--text-secondary)",
 fontSize: 13,
 }}>
 <div style={{ fontSize: 32, marginBottom: 8 }}></div>
 <div style={{ fontWeight: 600, marginBottom: 4 }}>No activity yet</div>
 <div>Chat messages, inline edits, agent steps,<br />and terminal commands will appear here.</div>
 </div>
 ) : (
 visible.map((ev) => (
 <FlowEventRow
 key={ev.id}
 event={ev}
 expanded={expandedId === ev.id}
 onToggle={() => setExpandedId(expandedId === ev.id ? null : ev.id)}
 onInject={() => handleInject(ev)}
 showInject={!!onInjectContext}
 />
 ))
 )}
 </div>

 {/* ── Footer count ──────────────────────────────────────────────────── */}
 {events.length > 0 && (
 <div style={{
 padding: "4px 12px",
 borderTop: "1px solid var(--border-color)",
 fontSize: 11,
 color: "var(--text-secondary)",
 background: "var(--bg-secondary)",
 flexShrink: 0,
 }}>
 {events.length} event{events.length !== 1 ? "s" : ""} in session
 </div>
 )}
 </div>
 );
}

// ── FlowEventRow ──────────────────────────────────────────────────────────────

function FlowEventRow({
 event, expanded, onToggle, onInject, showInject,
}: {
 event: FlowEvent;
 expanded: boolean;
 onToggle: () => void;
 onInject: () => void;
 showInject: boolean;
}) {
 const hasDetail = !!event.detail;

 return (
 <div style={{
 borderBottom: "1px solid var(--border-color)",
 padding: "6px 12px",
 cursor: hasDetail ? "pointer" : "default",
 }}>
 {/* ── Row header ── */}
 <div
 style={{ display: "flex", alignItems: "flex-start", gap: 8 }}
 onClick={hasDetail ? onToggle : undefined}
 >
 {/* Kind icon + connector line */}
 <div style={{ display: "flex", flexDirection: "column", alignItems: "center", paddingTop: 2 }}>
 <span style={{ fontSize: 14 }}>{KIND_ICONS[event.kind]}</span>
 </div>

 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
 <span style={{
 fontSize: 10,
 fontWeight: 600,
 color: "var(--accent-color)",
 textTransform: "uppercase",
 letterSpacing: 0.5,
 }}>
 {KIND_LABELS[event.kind]}
 </span>
 <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>
 {timeAgo(event.timestamp)}
 </span>
 {event.filePath && (
 <span style={{
 fontSize: 10,
 color: "var(--text-secondary)",
 overflow: "hidden",
 textOverflow: "ellipsis",
 whiteSpace: "nowrap",
 maxWidth: 120,
 }}>
 {event.filePath.split("/").pop()}
 </span>
 )}
 </div>

 <div style={{
 fontSize: 12,
 color: "var(--text-primary)",
 overflow: "hidden",
 textOverflow: "ellipsis",
 whiteSpace: expanded ? "normal" : "nowrap",
 }}>
 {event.summary}
 </div>

 {/* Expanded detail */}
 {expanded && event.detail && (
 <pre style={{
 marginTop: 6,
 padding: "6px 8px",
 background: "var(--bg-tertiary)",
 borderRadius: 4,
 fontSize: 11,
 color: "var(--text-secondary)",
 whiteSpace: "pre-wrap",
 wordBreak: "break-word",
 maxHeight: 240,
 overflowY: "auto",
 }}>
 {event.detail}
 </pre>
 )}
 </div>

 <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
 {showInject && (
 <button
 onClick={(e) => { e.stopPropagation(); onInject(); }}
 style={{ ...btnStyle, fontSize: 10 }}
 title="Inject this event's content into chat"
 >
 Inject
 </button>
 )}
 {hasDetail && (
 <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
 {expanded ? "" : "▼"}
 </span>
 )}
 </div>
 </div>
 </div>
 );
}

// ── Shared styles ─────────────────────────────────────────────────────────────

const btnStyle: React.CSSProperties = {
 padding: "2px 8px",
 fontSize: 11,
 borderRadius: 4,
 border: "1px solid var(--border-color)",
 background: "transparent",
 color: "var(--text-secondary)",
 cursor: "pointer",
};
