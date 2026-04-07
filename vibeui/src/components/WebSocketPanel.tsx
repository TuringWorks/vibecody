/**
 * WebSocketPanel — Interactive WebSocket Tester.
 *
 * Connect to any ws:// or wss:// endpoint, send text/JSON messages,
 * view a live message log with direction indicators, and measure
 * round-trip latency via ping/pong. Saved connections persist to disk.
 */
import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface WsConfig {
 id: string;
 label: string;
 url: string;
 protocols: string[];
}

type MsgDirection = "sent" | "received" | "system" | "error";

interface WsMessage {
 id: number;
 direction: MsgDirection;
 data: string;
 ts: number; // ms timestamp
 latency?: number; // for pong responses
}

const DIR_COLORS: Record<MsgDirection, string> = {
 sent: "var(--accent-color)",
 received: "var(--success-color)",
 system: "var(--text-primary)",
 error: "var(--error-color)",
};
const DIR_ICONS: Record<MsgDirection, string> = {
 sent: "↑",
 received: "↓",
 system: "•",
 error: "✕",
};

/** Maximum number of messages retained in the log. Older messages are dropped. */
const MAX_MESSAGES = 500;

let msgCounter = 0;

function makeMsg(direction: MsgDirection, data: string, latency?: number): WsMessage {
 return { id: ++msgCounter, direction, data, ts: Date.now(), latency };
}

function fmt(ts: number) {
 const d = new Date(ts);
 return d.toLocaleTimeString([], { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" }) +
 "." + String(d.getMilliseconds()).padStart(3, "0");
}

const PRESETS: WsConfig[] = [
 { id: "echo", label: "Echo (echo.websocket.org)", url: "wss://echo.websocket.org", protocols: [] },
 { id: "hn", label: "HN Algolia stream", url: "wss://hn.algolia.com/api/v1/changes", protocols: [] },
];

export function WebSocketPanel() {
 const [saved, setSaved] = useState<WsConfig[]>([]);
 const [url, setUrl] = useState("wss://echo.websocket.org");
 const [protocols, setProtocols] = useState("");
 const [label, setLabel] = useState("");
 const [status, setStatus] = useState<"idle" | "connecting" | "open" | "closed" | "error">("idle");
 const [messages, setMessages] = useState<WsMessage[]>([]);
 const [input, setInput] = useState("");
 const [prettyJson, setPrettyJson] = useState(true);
 const [autoScroll, setAutoScroll] = useState(true);
 const [filterDir, setFilterDir] = useState<MsgDirection | "all">("all");
 const wsRef = useRef<WebSocket | null>(null);
 const pingTsRef = useRef<number>(0);
 const logRef = useRef<HTMLDivElement>(null);

 useEffect(() => {
 invoke<WsConfig[]>("get_ws_configs").then(c => setSaved(c.length > 0 ? c : PRESETS)).catch(() => setSaved(PRESETS));
 }, []);

 useEffect(() => {
 if (autoScroll && logRef.current) {
 logRef.current.scrollTop = logRef.current.scrollHeight;
 }
 }, [messages, autoScroll]);

 // Cleanup on unmount
 useEffect(() => () => { wsRef.current?.close(); }, []);

 const push = useCallback((m: WsMessage) => setMessages(prev => [...prev.slice(-MAX_MESSAGES), m]), []);

 const connect = () => {
 if (wsRef.current && wsRef.current.readyState < 2) wsRef.current.close();
 setMessages([]);
 setStatus("connecting");
 const protos = protocols.trim() ? protocols.split(",").map(s => s.trim()).filter(Boolean) : [];
 try {
 const ws = protos.length > 0 ? new WebSocket(url, protos) : new WebSocket(url);
 wsRef.current = ws;

 ws.onopen = () => {
 setStatus("open");
 push(makeMsg("system", `Connected to ${url}`));
 };
 ws.onclose = (e) => {
 setStatus("closed");
 push(makeMsg("system", `Disconnected (code ${e.code}${e.reason ? ": " + e.reason : ""})`));
 };
 ws.onerror = () => {
 setStatus("error");
 push(makeMsg("error", "WebSocket error — check URL and server CORS/TLS settings"));
 };
 ws.onmessage = (e) => {
 const data = typeof e.data === "string" ? e.data : "[binary]";
 const latency = pingTsRef.current > 0 ? Date.now() - pingTsRef.current : undefined;
 if (pingTsRef.current > 0) pingTsRef.current = 0;
 push(makeMsg("received", data, latency));
 };
 } catch (err) {
 setStatus("error");
 push(makeMsg("error", String(err)));
 }
 };

 const disconnect = () => { wsRef.current?.close(1000, "User disconnected"); };

 const send = () => {
 if (!input.trim() || !wsRef.current || wsRef.current.readyState !== 1) return;
 wsRef.current.send(input);
 push(makeMsg("sent", input));
 setInput("");
 };

 const sendPing = () => {
 if (!wsRef.current || wsRef.current.readyState !== 1) return;
 pingTsRef.current = Date.now();
 const ping = JSON.stringify({ type: "ping", ts: pingTsRef.current });
 wsRef.current.send(ping);
 push(makeMsg("sent", ping));
 };

 const saveConfig = async () => {
 if (!url || !label) return;
 const cfg: WsConfig = { id: `ws-${Date.now()}`, label, url, protocols: protocols.split(",").map(s => s.trim()).filter(Boolean) };
 const next = [...saved.filter(s => s.url !== url), cfg];
 setSaved(next);
 await invoke("save_ws_configs", { configs: next }).catch(() => {});
 setLabel("");
 };

 const removeConfig = async (id: string) => {
 const next = saved.filter(s => s.id !== id);
 setSaved(next);
 await invoke("save_ws_configs", { configs: next }).catch(() => {});
 };

 const loadConfig = (c: WsConfig) => {
 setUrl(c.url);
 setProtocols(c.protocols.join(", "));
 };

 const tryPretty = (raw: string) => {
 if (!prettyJson) return raw;
 try { return JSON.stringify(JSON.parse(raw), null, 2); } catch { return raw; }
 };

 const statusColor = status === "open" ? "var(--success-color)" : status === "connecting" ? "var(--warning-color)" : status === "error" ? "var(--error-color)" : "var(--text-secondary)";

 const filtered = filterDir === "all" ? messages : messages.filter(m => m.direction === filterDir);

 return (
 <div style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
 {/* Sidebar — saved configs */}
 <div style={{ width: 200, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", flexShrink: 0 }}>
 <div style={{ padding: "10px 10px 6px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: 11, fontWeight: 600 }}>
 Saved
 </div>
 <div style={{ flex: 1, overflowY: "auto" }}>
 {saved.map(c => (
 <div
 key={c.id}
 style={{ padding: "7px 10px", borderBottom: "1px solid var(--border-color)", cursor: "pointer", display: "flex", alignItems: "center", gap: 6 }}
 onClick={() => loadConfig(c)}
 >
 <div style={{ flex: 1, minWidth: 0 }}>
 <div style={{ fontSize: 11, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.label}</div>
 <div style={{ fontSize: 9, color: "var(--text-secondary)", fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.url}</div>
 </div>
 <button
 onClick={e => { e.stopPropagation(); removeConfig(c.id); }}
 style={{ fontSize: 10, background: "none", border: "none", color: "var(--text-danger)", cursor: "pointer", padding: "0 2px" }}
 >✕</button>
 </div>
 ))}
 </div>
 {/* Save form */}
 <div style={{ padding: "8px 10px", borderTop: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: 4 }}>
 <input
 value={label}
 onChange={e => setLabel(e.target.value)}
 placeholder="Name to save…"
 style={{ padding: "3px 7px", fontSize: 10, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }}
 />
 <button
 onClick={saveConfig}
 disabled={!label || !url}
 style={{ padding: "3px 0", fontSize: 10, fontWeight: 600, background: "var(--accent-color)", border: "none", borderRadius: 4, color: "var(--text-primary)", cursor: "pointer" }}
 >
 Save current
 </button>
 </div>
 </div>

 {/* Main panel */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
 {/* Connection bar */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 <div style={{ width: 8, height: 8, borderRadius: "50%", background: statusColor, boxShadow: status === "open" ? `0 0 6px ${statusColor}` : "none", flexShrink: 0 }} />
 <input
 value={url}
 onChange={e => setUrl(e.target.value)}
 onKeyDown={e => e.key === "Enter" && status !== "open" && connect()}
 placeholder="wss://echo.websocket.org"
 style={{ flex: 1, padding: "5px 10px", fontSize: 12, fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", minWidth: 200 }}
 />
 <input
 value={protocols}
 onChange={e => setProtocols(e.target.value)}
 placeholder="subprotocols (comma-sep)"
 style={{ width: 160, padding: "5px 8px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-secondary)", outline: "none" }}
 />
 {status !== "open" ? (
 <button
 onClick={connect}
 style={{ padding: "5px 16px", fontSize: 11, fontWeight: 700, background: "var(--accent-color)", border: "none", borderRadius: 4, color: "var(--text-primary)", cursor: "pointer" }}
 >
 {status === "connecting" ? "Connecting…" : "Connect"}
 </button>
 ) : (
 <button
 onClick={disconnect}
 style={{ padding: "5px 16px", fontSize: 11, fontWeight: 700, background: "var(--error-color)", border: "none", borderRadius: 4, color: "var(--text-primary)", cursor: "pointer" }}
 >
 ■ Disconnect
 </button>
 )}
 </div>

 {/* Filter bar */}
 <div style={{ padding: "5px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 6, alignItems: "center" }}>
 {(["all", "sent", "received", "system", "error"] as const).map(d => (
 <button key={d} onClick={() => setFilterDir(d)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: filterDir === d ? "color-mix(in srgb, var(--accent-blue) 25%, transparent)" : "var(--bg-primary)", border: `1px solid ${filterDir === d ? "var(--accent-color)" : "var(--border-color)"}`, color: d === "all" ? "var(--text-primary)" : DIR_COLORS[d as MsgDirection], cursor: "pointer", fontWeight: filterDir === d ? 700 : 400 }}>
 {d === "all" ? "All" : `${DIR_ICONS[d as MsgDirection]} ${d}`}
 </button>
 ))}
 <div style={{ flex: 1 }} />
 <label style={{ fontSize: 10, color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: 4 }}>
 <input type="checkbox" checked={prettyJson} onChange={e => setPrettyJson(e.target.checked)} />Pretty JSON
 </label>
 <label style={{ fontSize: 10, color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: 4 }}>
 <input type="checkbox" checked={autoScroll} onChange={e => setAutoScroll(e.target.checked)} />Auto-scroll
 </label>
 <button onClick={() => setMessages([])} style={{ fontSize: 10, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-secondary)", cursor: "pointer" }}>
 Clear
 </button>
 </div>

 {/* Message log */}
 <div ref={logRef} style={{ flex: 1, overflowY: "auto", padding: "6px 0", fontFamily: "var(--font-mono)" }}>
 {filtered.length === 0 && (
 <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 12 }}>
 {status === "idle" ? "Enter a WebSocket URL and click Connect" : "No messages yet"}
 </div>
 )}
 {filtered.map(m => (
 <div key={m.id} style={{ padding: "4px 12px", display: "flex", gap: 10, alignItems: "flex-start", borderBottom: "1px solid var(--border-subtle)" }}>
 <span style={{ fontSize: 9, color: "var(--text-secondary)", flexShrink: 0, paddingTop: 2, minWidth: 86 }}>{fmt(m.ts)}</span>
 <span style={{ fontSize: 11, fontWeight: 700, color: DIR_COLORS[m.direction], flexShrink: 0, width: 12 }}>{DIR_ICONS[m.direction]}</span>
 <pre style={{ margin: 0, fontSize: 11, color: "var(--text-primary)", whiteSpace: "pre-wrap", wordBreak: "break-word", flex: 1, lineHeight: 1.5 }}>
 {tryPretty(m.data)}
 </pre>
 {m.latency !== undefined && (
 <span style={{ fontSize: 9, color: "var(--text-warning)", flexShrink: 0, paddingTop: 2 }}>{m.latency}ms</span>
 )}
 </div>
 ))}
 </div>

 {/* Send bar */}
 <div style={{ padding: "8px 12px", borderTop: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8 }}>
 <textarea
 value={input}
 onChange={e => setInput(e.target.value)}
 onKeyDown={e => { if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); send(); } }}
 placeholder='{"type":"hello"} — Ctrl+Enter to send'
 rows={2}
 style={{ flex: 1, padding: "6px 10px", fontSize: 11, fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", resize: "none" }}
 />
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 <button
 onClick={send}
 disabled={status !== "open" || !input.trim()}
 style={{ padding: "5px 16px", fontSize: 11, fontWeight: 700, background: status === "open" ? "var(--accent-color)" : "var(--bg-secondary)", border: "none", borderRadius: 4, color: status === "open" ? "var(--text-primary)" : "var(--text-secondary)", cursor: status === "open" ? "pointer" : "not-allowed" }}
 >↑ Send</button>
 <button
 onClick={sendPing}
 disabled={status !== "open"}
 style={{ padding: "5px 16px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-secondary)", cursor: status === "open" ? "pointer" : "not-allowed" }}
 >Ping</button>
 </div>
 </div>
 </div>
 </div>
 );
}
