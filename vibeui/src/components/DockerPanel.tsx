/**
 * DockerPanel — Docker & Container Management.
 *
 * Sub-tabs: Containers | Images | Compose
 * - Containers: list all (running+stopped), start/stop/restart/remove/logs
 * - Images: list local images, pull new image
 * - Compose: up/down/ps/logs/build/restart per service
 */
import React, { useState, useEffect, useRef } from "react";
import { CheckCircle2, XCircle, PauseCircle, MinusCircle, Package } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { EmptyState } from "./EmptyState";
import { StatusMessage } from "./StatusMessage";

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
 if (s.startsWith("up")) return "var(--success-color)";
 if (s.startsWith("exited")) return "var(--error-color)";
 if (s.startsWith("paused")) return "var(--warning-color)";
 return "var(--text-primary)";
};

const statusIcon = (status: string): React.ReactNode => {
 const s = status.toLowerCase();
 if (s.startsWith("up")) return <CheckCircle2 size={12} strokeWidth={1.5} style={{ color: "var(--accent-green)" }} />;
 if (s.startsWith("exited")) return <XCircle size={12} strokeWidth={1.5} style={{ color: "var(--accent-rose)" }} />;
 if (s.startsWith("paused")) return <PauseCircle size={12} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />;
 return <MinusCircle size={12} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />;
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


 const terminal: React.CSSProperties = {
 background: "var(--bg-primary)", color: "var(--text-primary)",
 border: "1px solid var(--border-color)", borderRadius: 6,
 padding: 10, fontSize: 11, lineHeight: 1.5,
 minHeight: 60, maxHeight: 260,
 overflow: "auto", whiteSpace: "pre-wrap", wordBreak: "break-all",
 fontFamily: "var(--font-mono)",
 };

 return (
 <div className="panel-container">
 {/* Sub-tab bar */}
 <div className="panel-tab-bar">
 {(["containers", "images", "compose"] as SubTab[]).map((t) => (
 <button
 key={t}
 onClick={() => setSubTab(t)}
 className={`panel-tab ${subTab === t ? "active" : ""}`}
 >
 {t === "containers" ? "Containers"
 : t === "images" ? "Images"
 : "Compose"}
 </button>
 ))}
 </div>

 <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 {error && (
 <StatusMessage variant="error" message={error} inline />
 )}

 {/* ── Containers ── */}
 {subTab === "containers" && (
 <>
 <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>
 {containers.length} container{containers.length !== 1 ? "s" : ""}
 </span>
 <button onClick={loadContainers} disabled={containersLoading} className="panel-btn panel-btn-secondary">
 {containersLoading ? "" : "↻ Refresh"}
 </button>
 </div>

 {containers.length === 0 && !containersLoading ? (
 <EmptyState
   icon={<Package size={32} strokeWidth={1.5} />}
   title="No containers found"
   description="Start Docker Desktop or run docker run ..."
 />
 ) : (
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 {containers.map((c) => (
 <div
 key={c.id}
 onClick={() => setSelectedContainer(selectedContainer === c.id ? null : c.id)}
 style={{
 padding: "8px 10px",
 background: selectedContainer === c.id ? "var(--bg-selected)" : "var(--bg-secondary)",
 border: `1px solid ${selectedContainer === c.id ? "var(--accent-color)" : "var(--border-color)"}`,
 borderRadius: 6, cursor: "pointer",
 }}
 >
 <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
 <span>{statusIcon(c.status)}</span>
 <span style={{ fontWeight: 600, fontSize: 12, flex: 1, fontFamily: "var(--font-mono)" }}>{c.name}</span>
 <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{c.image}</span>
 <span style={{ fontSize: 11, color: statusColor(c.status) }}>{c.status.split(" ")[0]}</span>
 </div>
 {c.ports && (
 <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 3, fontFamily: "var(--font-mono)" }}>
 {c.ports}
 </div>
 )}

 {/* Action buttons when selected */}
 {selectedContainer === c.id && (
 <div style={{ display: "flex", gap: 5, marginTop: 8, flexWrap: "wrap" }}
 onClick={(e) => e.stopPropagation()}>
 {c.status.toLowerCase().startsWith("exited") ? (
 <button onClick={() => runAction(c.id, "start")} disabled={actionLoading} className="panel-btn panel-btn-primary">Start</button>
 ) : (
 <button onClick={() => runAction(c.id, "stop")} disabled={actionLoading} className="panel-btn panel-btn-secondary">Stop</button>
 )}
 <button onClick={() => runAction(c.id, "restart")} disabled={actionLoading} className="panel-btn panel-btn-secondary">↻ Restart</button>
 <button onClick={() => runAction(c.id, "logs")} disabled={actionLoading} className="panel-btn panel-btn-secondary">Logs</button>
 <button
 onClick={() => { if (confirm(`Remove container ${c.name}?`)) runAction(c.id, "remove"); }}
 disabled={actionLoading}
 className="panel-btn panel-btn-danger"
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
 <button onClick={handlePull} disabled={pulling || !pullImage.trim()} className="panel-btn panel-btn-primary">
 {pulling ? "" : "Pull"}
 </button>
 <button onClick={loadImages} disabled={imagesLoading} className="panel-btn panel-btn-secondary">
 {imagesLoading ? "" : "↻"}
 </button>
 </div>

 {(pullOutput || pulling) && (
 <pre ref={outputRef} style={terminal}>
 {pulling ? "Pulling…" : pullOutput}
 </pre>
 )}

 {images.length === 0 && !imagesLoading ? (
 <EmptyState title="No local images" description="Pull one above." />
 ) : (
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 {images.map((img) => (
 <div key={img.id} style={{
 display: "flex", alignItems: "center", gap: 10,
 padding: "7px 10px",
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 6, fontSize: 12,
 }}>
 <span style={{ fontFamily: "var(--font-mono)", flex: 1 }}>
 {img.repository}:{img.tag}
 </span>
 <span style={{ color: "var(--text-secondary)", fontSize: 11 }}>{img.size}</span>
 <span style={{ color: "var(--text-secondary)", fontSize: 10, fontFamily: "var(--font-mono)" }}>{img.id.slice(0, 12)}</span>
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
 className={`panel-btn panel-btn-${variant === "primary" ? "primary" : "secondary"}`}
 >
 {composeLoading ? "" : label}
 </button>
 ))}
 </div>

 {!workspacePath && (
 <StatusMessage variant="empty" message="Open a workspace folder to use Docker Compose." inline />
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
