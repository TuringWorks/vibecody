/**
 * DockerPanel — Docker & Container Management.
 *
 * Sub-tabs: Containers | Images | Compose
 * - Containers: list all (running+stopped), start/stop/restart/remove/logs
 * - Images: list local images, pull new image
 * - Compose: up/down/ps/logs/build/restart per service
 */
import React, { useState, useEffect, useRef } from "react";
import { Circle } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

type SubTab = "containers" | "images" | "compose";

interface DockerContainer {
 id: string;
 name: string;
 image: string;
 status: string;
 ports: string;
 created: string;
}

interface DockerImage {
 id: string;
 repository: string;
 tag: string;
 size: string;
 created: string;
}

interface DockerPanelProps {
 workspacePath: string | null;
}

const statusColor = (status: string) => {
 const s = status.toLowerCase();
 if (s.startsWith("up")) return "#a6e3a1";
 if (s.startsWith("exited")) return "#f38ba8";
 if (s.startsWith("paused")) return "#f9e2af";
 return "var(--text-primary)";
};

const statusIcon = (status: string): React.ReactNode => {
 const s = status.toLowerCase();
 if (s.startsWith("up")) return <Circle size={10} strokeWidth={0} fill="#a6e3a1" />;
 if (s.startsWith("exited")) return <Circle size={10} strokeWidth={0} fill="#f38ba8" />;
 if (s.startsWith("paused")) return <Circle size={10} strokeWidth={0} fill="var(--text-primary)" />;
 return <Circle size={10} strokeWidth={0} fill="var(--text-primary)" />;
};

export function DockerPanel({ workspacePath }: DockerPanelProps) {
 const [subTab, setSubTab] = useState<SubTab>("containers");

 // ── Containers ──
 const [containers, setContainers] = useState<DockerContainer[]>([]);
 const [containersLoading, setContainersLoading] = useState(false);
 const [selectedContainer, setSelectedContainer] = useState<string | null>(null);
 const [actionOutput, setActionOutput] = useState("");
 const [actionLoading, setActionLoading] = useState(false);
 const outputRef = useRef<HTMLPreElement>(null);

 // ── Images ──
 const [images, setImages] = useState<DockerImage[]>([]);
 const [imagesLoading, setImagesLoading] = useState(false);
 const [pullImage, setPullImage] = useState("");
 const [pullOutput, setPullOutput] = useState("");
 const [pulling, setPulling] = useState(false);

 // ── Compose ──
 const [composeOutput, setComposeOutput] = useState("");
 const [composeLoading, setComposeLoading] = useState(false);
 const [composeService, setComposeService] = useState("");

 const [error, setError] = useState<string | null>(null);

 // Auto-scroll output
 useEffect(() => {
 if (outputRef.current) {
 outputRef.current.scrollTop = outputRef.current.scrollHeight;
 }
 }, [actionOutput, pullOutput, composeOutput]);

 // ── Load containers ──
 const loadContainers = async () => {
 setContainersLoading(true);
 setError(null);
 try {
 const result = await invoke<DockerContainer[]>("list_docker_containers");
 setContainers(result);
 } catch (e) {
 setError(String(e));
 } finally {
 setContainersLoading(false);
 }
 };

 // ── Load images ──
 const loadImages = async () => {
 setImagesLoading(true);
 setError(null);
 try {
 const result = await invoke<DockerImage[]>("list_docker_images");
 setImages(result);
 } catch (e) {
 setError(String(e));
 } finally {
 setImagesLoading(false);
 }
 };

 useEffect(() => {
 if (subTab === "containers") loadContainers();
 if (subTab === "images") loadImages();
 }, [subTab]);

 // ── Container action ──
 const runAction = async (id: string, action: string) => {
 setActionLoading(true);
 setActionOutput("");
 setError(null);
 try {
 const out = await invoke<string>("docker_container_action", {
 containerId: id,
 action,
 tailLines: action === "logs" ? 200 : undefined,
 });
 setActionOutput(out);
 if (action !== "logs") await loadContainers();
 } catch (e) {
 setActionOutput(`Error: ${e}`);
 } finally {
 setActionLoading(false);
 }
 };

 // ── Pull image ──
 const handlePull = async () => {
 if (!pullImage.trim()) return;
 setPulling(true);
 setPullOutput("");
 setError(null);
 try {
 const out = await invoke<string>("docker_pull_image", { image: pullImage.trim() });
 setPullOutput(out);
 await loadImages();
 } catch (e) {
 setPullOutput(`Error: ${e}`);
 } finally {
 setPulling(false);
 }
 };

 // ── Compose action ──
 const runCompose = async (action: string) => {
 if (!workspacePath) { setError("No workspace open."); return; }
 setComposeLoading(true);
 setComposeOutput("");
 setError(null);
 try {
 const out = await invoke<string>("docker_compose_action", {
 workspace: workspacePath,
 action,
 service: composeService.trim() || null,
 });
 setComposeOutput(out);
 } catch (e) {
 setComposeOutput(`Error: ${e}`);
 } finally {
 setComposeLoading(false);
 }
 };

 const inputStyle: React.CSSProperties = {
 padding: "5px 8px", fontSize: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 };

 const btnStyle = (variant: "primary" | "secondary" | "danger" = "secondary"): React.CSSProperties => ({
 padding: "4px 10px", fontSize: 11, borderRadius: 4, border: "none", cursor: "pointer",
 background: variant === "primary" ? "var(--accent-color)"
 : variant === "danger" ? "#3a1a1a"
 : "var(--bg-secondary)",
 color: variant === "primary" ? "#fff"
 : variant === "danger" ? "#f38ba8"
 : "var(--text-secondary)",
 borderWidth: variant === "danger" ? 1 : 0,
 borderStyle: "solid",
 borderColor: variant === "danger" ? "#f38ba8" : "transparent",
 });

 const terminal: React.CSSProperties = {
 background: "#0d1117", color: "#e6edf3",
 border: "1px solid var(--border-color)", borderRadius: 6,
 padding: 10, fontSize: 11, lineHeight: 1.5,
 minHeight: 60, maxHeight: 260,
 overflow: "auto", whiteSpace: "pre-wrap", wordBreak: "break-all",
 fontFamily: "monospace",
 };

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Sub-tab bar */}
 <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
 {(["containers", "images", "compose"] as SubTab[]).map((t) => (
 <button
 key={t}
 onClick={() => setSubTab(t)}
 style={{
 padding: "6px 14px", fontSize: 12, background: "transparent",
 color: subTab === t ? "var(--text-primary)" : "var(--text-muted)",
 border: "none",
 borderBottom: subTab === t ? "2px solid var(--accent-color)" : "2px solid transparent",
 cursor: "pointer", fontWeight: subTab === t ? 600 : 400,
 }}
 >
 {t === "containers" ? "Containers"
 : t === "images" ? "Images"
 : "Compose"}
 </button>
 ))}
 </div>

 <div style={{ flex: 1, overflow: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
 {error && (
 <div style={{ padding: "7px 10px", background: "var(--error-bg, #2a1a1a)", color: "var(--text-danger, #f38ba8)", borderRadius: 4, fontSize: 12 }}>
 {error}
 </div>
 )}

 {/* ── Containers ── */}
 {subTab === "containers" && (
 <>
 <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
 {containers.length} container{containers.length !== 1 ? "s" : ""}
 </span>
 <button onClick={loadContainers} disabled={containersLoading} style={btnStyle()}>
 {containersLoading ? "" : "↻ Refresh"}
 </button>
 </div>

 {containers.length === 0 && !containersLoading ? (
 <div style={{ textAlign: "center", padding: "20px 0", color: "var(--text-muted)", fontSize: 12 }}>
 <div style={{ fontSize: 28, marginBottom: 8 }}></div>
 No containers found. Start Docker Desktop or run <code>docker run ...</code>
 </div>
 ) : (
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 {containers.map((c) => (
 <div
 key={c.id}
 onClick={() => setSelectedContainer(selectedContainer === c.id ? null : c.id)}
 style={{
 padding: "8px 10px",
 background: selectedContainer === c.id ? "var(--bg-selected, #1a2a3a)" : "var(--bg-secondary)",
 border: `1px solid ${selectedContainer === c.id ? "var(--accent-color)" : "var(--border-color)"}`,
 borderRadius: 6, cursor: "pointer",
 }}
 >
 <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
 <span>{statusIcon(c.status)}</span>
 <span style={{ fontWeight: 600, fontSize: 12, flex: 1, fontFamily: "monospace" }}>{c.name}</span>
 <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{c.image}</span>
 <span style={{ fontSize: 11, color: statusColor(c.status) }}>{c.status.split(" ")[0]}</span>
 </div>
 {c.ports && (
 <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 3, fontFamily: "monospace" }}>
 {c.ports}
 </div>
 )}

 {/* Action buttons when selected */}
 {selectedContainer === c.id && (
 <div style={{ display: "flex", gap: 5, marginTop: 8, flexWrap: "wrap" }}
 onClick={(e) => e.stopPropagation()}>
 {c.status.toLowerCase().startsWith("exited") ? (
 <button onClick={() => runAction(c.id, "start")} disabled={actionLoading} style={btnStyle("primary")}>Start</button>
 ) : (
 <button onClick={() => runAction(c.id, "stop")} disabled={actionLoading} style={btnStyle()}>Stop</button>
 )}
 <button onClick={() => runAction(c.id, "restart")} disabled={actionLoading} style={btnStyle()}>↻ Restart</button>
 <button onClick={() => runAction(c.id, "logs")} disabled={actionLoading} style={btnStyle()}>Logs</button>
 <button
 onClick={() => { if (confirm(`Remove container ${c.name}?`)) runAction(c.id, "remove"); }}
 disabled={actionLoading}
 style={btnStyle("danger")}
 >
 ✕ Remove
 </button>
 </div>
 )}
 </div>
 ))}
 </div>
 )}

 {(actionOutput || actionLoading) && (
 <pre ref={outputRef} style={terminal}>
 {actionLoading ? "Running…" : actionOutput}
 </pre>
 )}
 </>
 )}

 {/* ── Images ── */}
 {subTab === "images" && (
 <>
 <div style={{ display: "flex", gap: 8 }}>
 <input
 style={{ ...inputStyle, flex: 1 }}
 value={pullImage}
 onChange={(e) => setPullImage(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && handlePull()}
 placeholder="Pull image: nginx:latest, node:20-alpine, ..."
 />
 <button onClick={handlePull} disabled={pulling || !pullImage.trim()} style={btnStyle("primary")}>
 {pulling ? "" : "Pull"}
 </button>
 <button onClick={loadImages} disabled={imagesLoading} style={btnStyle()}>
 {imagesLoading ? "" : "↻"}
 </button>
 </div>

 {(pullOutput || pulling) && (
 <pre ref={outputRef} style={terminal}>
 {pulling ? "Pulling…" : pullOutput}
 </pre>
 )}

 {images.length === 0 && !imagesLoading ? (
 <div style={{ textAlign: "center", padding: "20px 0", color: "var(--text-muted)", fontSize: 12 }}>
 No local images. Pull one above.
 </div>
 ) : (
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 {images.map((img) => (
 <div key={img.id} style={{
 display: "flex", alignItems: "center", gap: 10,
 padding: "7px 10px",
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 6, fontSize: 12,
 }}>
 <span style={{ fontFamily: "monospace", flex: 1 }}>
 {img.repository}:{img.tag}
 </span>
 <span style={{ color: "var(--text-muted)", fontSize: 11 }}>{img.size}</span>
 <span style={{ color: "var(--text-muted)", fontSize: 10, fontFamily: "monospace" }}>{img.id.slice(0, 12)}</span>
 </div>
 ))}
 </div>
 )}
 </>
 )}

 {/* ── Compose ── */}
 {subTab === "compose" && (
 <>
 <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.5 }}>
 Runs <code>docker compose</code> from your workspace root. Detects{" "}
 <code>docker-compose.yml</code>, <code>compose.yml</code>, etc.
 </div>

 <div style={{ display: "flex", gap: 8 }}>
 <input
 style={{ ...inputStyle, flex: 1 }}
 value={composeService}
 onChange={(e) => setComposeService(e.target.value)}
 placeholder="Service name (optional, leave blank for all)"
 />
 </div>

 {/* Action buttons */}
 <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
 {[
 { action: "up", label: "Up", variant: "primary" },
 { action: "down", label: "Down", variant: "secondary" },
 { action: "ps", label: " ps", variant: "secondary" },
 { action: "logs", label: "Logs", variant: "secondary" },
 { action: "build", label: "Build", variant: "secondary" },
 { action: "restart", label: "↻ Restart", variant: "secondary" },
 { action: "pull", label: "Pull", variant: "secondary" },
 ].map(({ action, label, variant }) => (
 <button
 key={action}
 onClick={() => runCompose(action)}
 disabled={composeLoading || !workspacePath}
 style={btnStyle(variant as "primary" | "secondary")}
 >
 {composeLoading ? "" : label}
 </button>
 ))}
 </div>

 {!workspacePath && (
 <div style={{ fontSize: 12, color: "var(--text-muted)" }}>
 Open a workspace folder to use Docker Compose.
 </div>
 )}

 {(composeOutput || composeLoading) && (
 <pre ref={outputRef} style={terminal}>
 {composeLoading ? "Running…" : composeOutput}
 </pre>
 )}
 </>
 )}
 </div>
 </div>
 );
}
