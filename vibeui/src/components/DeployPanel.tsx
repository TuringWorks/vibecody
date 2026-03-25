/**
 * DeployPanel — One-Click Deployment for web projects.
 *
 * Supports: Vercel, Netlify, Railway, GitHub Pages
 * Flow: detect project type → show recommended target → Deploy → stream logs → show URL
 */
import { useState, useEffect, useRef } from "react";
import { Circle } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface DeployTarget {
 target: string;
 build_cmd: string;
 out_dir: string;
 detected_framework: string;
 recommended_targets?: string[];
 required_cli?: string;
}

interface DeployRecord {
 id: string;
 target: string;
 url: string | null;
 timestamp: number;
 status: "success" | "failed" | "running";
}

const TARGETS = [
 // PaaS
 { id: "vercel", label: "Vercel", icon: "" },
 { id: "netlify", label: "Netlify", icon: "" },
 { id: "railway", label: "Railway", icon: "" },
 { id: "github-pages", label: "GitHub Pages", icon: "settings" },
 // Google
 { id: "gcp-run", label: "GCP Cloud Run", icon: "cloud" },
 { id: "firebase", label: "Firebase", icon: "" },
 // AWS
 { id: "aws-apprunner", label: "AWS App Runner", icon: "" },
 { id: "aws-s3", label: "AWS S3 + CF", icon: "" },
 { id: "aws-lambda", label: "AWS Lambda", icon: "λ" },
 { id: "aws-ecs", label: "AWS ECS", icon: "" },
 // Azure
 { id: "azure-appservice", label: "Azure App Svc", icon: "" },
 { id: "azure-containerapp", label: "Azure Container", icon: "" },
 { id: "azure-staticweb", label: "Azure Static", icon: "" },
 // Others
 { id: "digitalocean", label: "DigitalOcean", icon: "" },
 { id: "kubernetes", label: "Kubernetes", icon: "ship" },
 { id: "kubernetes-helm", label: "Helm", icon: "" },
 { id: "oci", label: "Oracle Cloud", icon: <Circle size={14} strokeWidth={0} fill="var(--error-color)" /> },
 { id: "ibm-cloud", label: "IBM Code Engine", icon: "" },
];

interface DeployPanelProps {
 workspacePath: string | null;
}

export function DeployPanel({ workspacePath }: DeployPanelProps) {
 const [detected, setDetected] = useState<DeployTarget | null>(null);
 const [selectedTarget, setSelectedTarget] = useState("vercel");
 const [isDeploying, setIsDeploying] = useState(false);
 const [logs, setLogs] = useState<string[]>([]);
 const [deployedUrl, setDeployedUrl] = useState<string | null>(null);
 const [history, setHistory] = useState<DeployRecord[]>([]);
 const [customDomain, setCustomDomain] = useState("");
 const [domainResult, setDomainResult] = useState<{ domain: string; cname_target: string; instructions: string } | null>(null);
 const [domainBusy, setDomainBusy] = useState(false);
 const logsEndRef = useRef<HTMLDivElement>(null);
 const deployUnlistenRef = useRef<(() => void) | null>(null);

 // Clean up deploy listener on unmount
 useEffect(() => {
 return () => { deployUnlistenRef.current?.(); };
 }, []);

 useEffect(() => {
 if (workspacePath) {
 invoke<DeployTarget>("detect_deploy_target", { workspace: workspacePath })
 .then(setDetected)
 .catch(() => null);
 invoke<DeployRecord[]>("get_deploy_history")
 .then(setHistory)
 .catch(() => []);
 }
 }, [workspacePath]);

 useEffect(() => {
 logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
 }, [logs]);

 if (!workspacePath) {
 return <div className="empty-state"><p>Open a workspace folder to use the deploy panel.</p></div>;
 }

 const handleDeploy = async () => {
 setIsDeploying(true);
 setLogs([`Starting deployment to ${selectedTarget}...`]);
 setDeployedUrl(null);

 // Listen for streaming log events
 deployUnlistenRef.current?.();
 const unlisten = await listen<string>("deploy:log", (e) => {
 setLogs(prev => [...prev, e.payload]);
 });
 deployUnlistenRef.current = unlisten;

 try {
 const result = await invoke<{ url: string | null }>("run_deploy", {
 target: selectedTarget,
 workspace: workspacePath,
 });
 if (result.url) {
 setDeployedUrl(result.url);
 setLogs(prev => [...prev, ` Deployed to: ${result.url}`]);
 }
 // Refresh history
 const h = await invoke<DeployRecord[]>("get_deploy_history").catch(() => []);
 setHistory(h);
 } catch (e) {
 setLogs(prev => [...prev, ` Deployment failed: ${e}`]);
 } finally {
 setIsDeploying(false);
 deployUnlistenRef.current?.();
 deployUnlistenRef.current = null;
 }
 };

 const handleSetDomain = async () => {
 if (!customDomain.trim()) return;
 setDomainBusy(true);
 setDomainResult(null);
 try {
 const result = await invoke<{ domain: string; cname_target: string; instructions: string }>(
 "set_custom_domain", { target: selectedTarget, domain: customDomain.trim() }
 );
 setDomainResult(result);
 } catch (e) {
 setDomainResult({ domain: customDomain, cname_target: "", instructions: `Error: ${e}` });
 } finally {
 setDomainBusy(false);
 }
 };

 return (
 <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 16, height: "100%", overflowY: "auto" }}>
 {/* Detected project */}
 {detected && (
 <div style={{ background: "var(--bg-secondary)", borderRadius: 8, padding: 12, border: "1px solid var(--border-color)" }}>
 <div style={{ fontSize: 12, opacity: 0.7, marginBottom: 4 }}>Detected Project</div>
 <div style={{ fontWeight: 600 }}>{detected.detected_framework || "Static Site"}</div>
 <div style={{ fontSize: 11, opacity: 0.6, fontFamily: "var(--font-mono)" }}>Build: {detected.build_cmd}</div>
 </div>
 )}

 {/* Target selection */}
 <div>
 <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Deploy Target</div>
 <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 6, maxHeight: 260, overflowY: "auto" }}>
 {TARGETS.map((t) => {
 const isRec = detected?.recommended_targets?.includes(t.id);
 return (
 <button
 key={t.id}
 onClick={() => setSelectedTarget(t.id)}
 style={{
 background: selectedTarget === t.id ? "var(--accent-color)" : "var(--bg-secondary)",
 border: `1px solid ${selectedTarget === t.id ? "var(--accent-color)" : "var(--border-color)"}`,
 borderRadius: 6,
 padding: "7px 6px",
 cursor: "pointer",
 color: "var(--text-primary)",
 fontSize: 11,
 fontWeight: selectedTarget === t.id ? 600 : 400,
 display: "flex",
 alignItems: "center",
 gap: 4,
 whiteSpace: "nowrap",
 overflow: "hidden",
 }}
 >
 <span>{t.icon}</span> {t.label}
 {isRec && <span style={{ fontSize: 9, color: "var(--text-success)", marginLeft: 2 }}>★</span>}
 </button>
 );
 })}
 </div>
 </div>

 {/* Deploy button */}
 <button
 onClick={handleDeploy}
 disabled={isDeploying}
 style={{
 background: isDeploying ? "var(--bg-tertiary)" : "var(--accent-color)",
 color: "var(--text-primary)",
 border: "none",
 borderRadius: 6,
 padding: "10px 0",
 cursor: isDeploying ? "not-allowed" : "pointer",
 fontWeight: 700,
 fontSize: 14,
 }}
 >
 {isDeploying ? "Deploying…" : "Deploy"}
 </button>

 {/* Deployed URL */}
 {deployedUrl && (
 <div style={{ background: "color-mix(in srgb, var(--accent-green) 10%, transparent)", border: "1px solid var(--success-color)", borderRadius: 6, padding: 10 }}>
 <div style={{ fontSize: 12, color: "var(--text-success)", marginBottom: 4 }}>Live at</div>
 <a href={deployedUrl} target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-info)", fontSize: 13, fontFamily: "var(--font-mono)" }}>
 {deployedUrl}
 </a>
 </div>
 )}

 {/* Custom Domain */}
 <div>
 <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>Custom Domain</div>
 <div style={{ display: "flex", gap: 8 }}>
 <input
 type="text"
 value={customDomain}
 onChange={(e) => setCustomDomain(e.target.value)}
 placeholder="myapp.example.com"
 onKeyDown={(e) => e.key === "Enter" && handleSetDomain()}
 style={{ flex: 1, padding: "6px 10px", fontSize: 12, fontFamily: "var(--font-mono)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }}
 />
 <button
 onClick={handleSetDomain}
 disabled={domainBusy || !customDomain.trim()}
 style={{ padding: "6px 12px", fontSize: 12, background: "var(--accent-color)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer", whiteSpace: "nowrap" }}
 >
 {domainBusy ? "…" : "Add Domain"}
 </button>
 </div>
 {domainResult && (
 <div style={{ marginTop: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: 10 }}>
 <div style={{ fontSize: 11, color: "var(--text-accent)", marginBottom: 4 }}>DNS Instructions</div>
 <pre style={{ fontSize: 11, margin: 0, whiteSpace: "pre-wrap", fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>
 {domainResult.instructions}
 </pre>
 </div>
 )}
 </div>

 {/* Log stream */}
 {logs.length > 0 && (
 <div style={{ background: "var(--bg-secondary)", borderRadius: 6, padding: 10, maxHeight: 200, overflowY: "auto", fontFamily: "var(--font-mono)", fontSize: 11 }}>
 {logs.map((line, i) => (
 <div key={i} style={{ opacity: 0.8 }}>{line}</div>
 ))}
 <div ref={logsEndRef} />
 </div>
 )}

 {/* Deployment history */}
 {history.length > 0 && (
 <div>
 <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>History</div>
 {history.slice(0, 5).map((rec) => (
 <div key={rec.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 0", borderBottom: "1px solid var(--border-color)", fontSize: 12 }}>
 <span>{rec.status === "success" ? "" : rec.status === "running" ? "" : ""}</span>
 <span style={{ opacity: 0.7 }}>{rec.target}</span>
 {rec.url && <a href={rec.url} target="_blank" rel="noopener noreferrer" style={{ color: "var(--text-info)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>{rec.url}</a>}
 <span style={{ opacity: 0.4, flexShrink: 0 }}>{new Date(rec.timestamp).toLocaleDateString()}</span>
 </div>
 ))}
 </div>
 )}
 </div>
 );
}
