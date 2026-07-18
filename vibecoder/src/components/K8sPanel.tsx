import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Ship } from "lucide-react";
import { EmptyState } from "./EmptyState";
import { StatusMessage } from "./StatusMessage";

type SubTab = "manifests" | "deploy" | "argocd" | "contexts";

const QUICK_CMDS = [
 "get pods",
 "get services",
 "get ingress",
 "rollout status deployment/myapp",
 "describe pods",
 "get events --sort-by=.lastTimestamp",
 "get nodes",
 "top pods",
];

interface K8sPanelProps {
 workspacePath: string | null;
}

export default function K8sPanel({ workspacePath }: K8sPanelProps) {
 const [subTab, setSubTab] = useState<SubTab>("manifests");

 // ── Manifests state ──
 const [appName, setAppName] = useState("my-app");
 const [image, setImage] = useState("nginx:latest");
 const [port, setPort] = useState(8080);
 const [replicas, setReplicas] = useState(2);
 const [namespace, setNamespace] = useState("default");
 const [ingressHost, setIngressHost] = useState("");
 const [manifestYaml, setManifestYaml] = useState("");
 const [manifestLoading, setManifestLoading] = useState(false);
 const [manifestCopied, setManifestCopied] = useState(false);

 // ── Deploy state ──
 const [contexts, setContexts] = useState<string[]>([]);
 const [selectedContext, setSelectedContext] = useState("");
 const [deployNamespace, setDeployNamespace] = useState("default");
 const [kubectlCmd, setKubectlCmd] = useState("get pods");
 const [cmdOutput, setCmdOutput] = useState("");
 const [cmdLoading, setCmdLoading] = useState(false);
 const outputRef = useRef<HTMLPreElement>(null);

 // ── ArgoCD state ──
 const [argoAppName, setArgoAppName] = useState("my-app");
 const [argoRepoUrl, setArgoRepoUrl] = useState("https://github.com/org/repo");
 const [argoPath, setArgoPath] = useState("./k8s");
 const [argoNamespace, setArgoNamespace] = useState("default");
 const [argoServer, setArgoServer] = useState("https://kubernetes.default.svc");
 const [argoYaml, setArgoYaml] = useState("");
 const [argoLoading, setArgoLoading] = useState(false);
 const [argoApplied, setArgoApplied] = useState<string | null>(null);
 const [argoCopied, setArgoCopied] = useState(false);

 const [error, setError] = useState<string | null>(null);

 // Load contexts on mount
 useEffect(() => {
 invoke<string[]>("list_k8s_contexts")
 .then((ctxs) => {
 setContexts(ctxs);
 if (ctxs.length > 0 && !selectedContext) setSelectedContext(ctxs[0]);
 })
 .catch(() => setContexts([]));
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, []);

 // Auto-scroll kubectl output
 useEffect(() => {
 if (outputRef.current) {
 outputRef.current.scrollTop = outputRef.current.scrollHeight;
 }
 }, [cmdOutput]);

 // ── Manifests ──
 const handleGenerateManifests = async () => {
 setManifestLoading(true);
 setError(null);
 try {
 const yaml = await invoke<string>("generate_k8s_manifests", {
 appName,
 image,
 port,
 replicas,
 namespace,
 ingressHost: ingressHost.trim() || null,
 });
 setManifestYaml(yaml);
 } catch (e) {
 setError(String(e));
 } finally {
 setManifestLoading(false);
 }
 };

 const handleSaveManifests = async () => {
 if (!workspacePath || !manifestYaml) return;
 try {
 await invoke("write_file", {
 path: `${workspacePath}/k8s/manifests.yaml`,
 content: manifestYaml,
 });
 } catch (e) {
 setError(String(e));
 }
 };

 const handleCopyManifest = () => {
 navigator.clipboard.writeText(manifestYaml).then(() => {
 setManifestCopied(true);
 setTimeout(() => setManifestCopied(false), 1500);
 }).catch(() => {});
 };

 // ── Deploy ──
 const handleRunKubectl = async (cmd?: string) => {
 const command = cmd ?? kubectlCmd;
 if (!command.trim()) return;
 setCmdLoading(true);
 setCmdOutput("");
 setError(null);
 try {
 const out = await invoke<string>("run_kubectl_command", {
 context: selectedContext || null,
 namespace: deployNamespace,
 command,
 });
 setCmdOutput(out);
 } catch (e) {
 setCmdOutput(`Error: ${e}`);
 } finally {
 setCmdLoading(false);
 }
 };

 // ── ArgoCD ──
 const handleGenerateArgo = async () => {
 setArgoLoading(true);
 setArgoApplied(null);
 setError(null);
 try {
 const yaml = await invoke<string>("generate_argocd_app", {
 appName: argoAppName,
 repoUrl: argoRepoUrl,
 path: argoPath,
 namespace: argoNamespace,
 server: argoServer,
 });
 setArgoYaml(yaml);
 } catch (e) {
 setError(String(e));
 } finally {
 setArgoLoading(false);
 }
 };

 const handleApplyArgo = async () => {
 if (!argoYaml) return;
 setArgoLoading(true);
 setArgoApplied(null);
 setError(null);
 // Write to temp file then apply
 try {
 // Write YAML to a temp path, then kubectl apply it
 const tmpPath = workspacePath ? `${workspacePath}/.argocd-app-tmp.yaml` : "/tmp/argocd-app.yaml";
 await invoke("write_file", { path: tmpPath, content: argoYaml });
 const out = await invoke<string>("run_kubectl_command", {
 context: selectedContext || null,
 namespace: "argocd",
 command: `apply -f ${tmpPath}`,
 });
 setArgoApplied(out);
 } catch (e) {
 setError(String(e));
 } finally {
 setArgoLoading(false);
 }
 };

 const subTabs: { id: SubTab; label: string }[] = [
 { id: "manifests", label: "Manifests" },
 { id: "deploy", label: "Deploy" },
 { id: "argocd", label: "ArgoCD" },
 { id: "contexts", label: "Contexts" },
 ];


 return (
 <div className="panel-container">
 {/* Sub-tab bar */}
 <div className="panel-tab-bar">
 {subTabs.map((t) => (
 <button
 key={t.id}
 onClick={() => setSubTab(t.id)}
 className={`panel-tab${subTab === t.id ? " active" : ""}`}
 >
 {t.label}
 </button>
 ))}
 </div>

 <div className="panel-body">
 {error && (
 <StatusMessage variant="error" message={error} inline />
 )}

 {/* ── Manifests ── */}
 {subTab === "manifests" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
 <div>
 <label className="panel-label">APP NAME</label>
 <input className="panel-input panel-input-full" value={appName} onChange={(e) => setAppName(e.target.value)} />
 </div>
 <div>
 <label className="panel-label">IMAGE</label>
 <input className="panel-input panel-input-full" value={image} onChange={(e) => setImage(e.target.value)} placeholder="nginx:latest" />
 </div>
 <div>
 <label className="panel-label">PORT</label>
 <input className="panel-input panel-input-full" type="number" value={port} onChange={(e) => setPort(Number(e.target.value))} />
 </div>
 <div>
 <label className="panel-label">REPLICAS</label>
 <input className="panel-input panel-input-full" type="number" min={1} value={replicas} onChange={(e) => setReplicas(Number(e.target.value))} />
 </div>
 <div>
 <label className="panel-label">NAMESPACE</label>
 <input className="panel-input panel-input-full" value={namespace} onChange={(e) => setNamespace(e.target.value)} />
 </div>
 <div>
 <label className="panel-label">INGRESS HOST (optional)</label>
 <input className="panel-input panel-input-full" value={ingressHost} onChange={(e) => setIngressHost(e.target.value)} placeholder="myapp.example.com" />
 </div>
 </div>

 <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
 <button
 onClick={handleGenerateManifests}
 disabled={manifestLoading}
 className="panel-btn panel-btn-primary"
 style={{ cursor: manifestLoading ? "wait" : "pointer", opacity: manifestLoading ? 0.7 : 1 }}
 >
 {manifestLoading ? "Generating..." : "Generate YAML"}
 </button>
 {manifestYaml && (
 <>
 <button
 onClick={handleCopyManifest}
 className="panel-btn panel-btn-secondary"
 >
 {manifestCopied ? "✓ Copied" : "Copy YAML"}
 </button>
 {workspacePath && (
 <button
 onClick={handleSaveManifests}
 className="panel-btn panel-btn-secondary"
 >
 Save to ./k8s/
 </button>
 )}
 </>
 )}
 </div>

 {manifestYaml && (
 <pre style={{
 margin: 0, padding: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-sm)", lineHeight: 1.5,
 overflow: "auto", maxHeight: 400, whiteSpace: "pre-wrap",
 wordBreak: "break-word", color: "var(--text-primary)",
 }}>
 {manifestYaml}
 </pre>
 )}
 </div>
 )}

 {/* ── Deploy ── */}
 {subTab === "deploy" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
 <div>
 <label className="panel-label">CONTEXT</label>
 {contexts.length > 0 ? (
 <select
 className="panel-input panel-input-full"
 value={selectedContext}
 onChange={(e) => setSelectedContext(e.target.value)}
 >
 <option value="">— any —</option>
 {contexts.map((ctx) => (
 <option key={ctx} value={ctx}>{ctx}</option>
 ))}
 </select>
 ) : (
 <input
 className="panel-input panel-input-full"
 value={selectedContext}
 onChange={(e) => setSelectedContext(e.target.value)}
 placeholder="kubectl not found or no contexts"
 />
 )}
 </div>
 <div>
 <label className="panel-label">NAMESPACE</label>
 <input className="panel-input panel-input-full" value={deployNamespace} onChange={(e) => setDeployNamespace(e.target.value)} />
 </div>
 </div>

 {/* Quick-action chips */}
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 6 }}>QUICK ACTIONS</div>
 <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
 {QUICK_CMDS.map((cmd) => (
 <button
 key={cmd}
 onClick={() => { setKubectlCmd(cmd); handleRunKubectl(cmd); }}
 style={{
 padding: "3px 8px", fontSize: "var(--font-size-sm)",
 background: "var(--bg-secondary)", color: "var(--text-secondary)",
 border: "1px solid var(--border-color)", borderRadius: 12,
 cursor: "pointer",
 }}
 >
 {cmd}
 </button>
 ))}
 </div>
 </div>

 {/* Command input */}
 <div style={{ display: "flex", gap: 6 }}>
 <div style={{ flex: 1 }}>
 <label className="panel-label">KUBECTL COMMAND</label>
 <input
 className="panel-input panel-input-full"
 value={kubectlCmd}
 onChange={(e) => setKubectlCmd(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && handleRunKubectl()}
 placeholder="get pods -l app=myapp"
 />
 </div>
 <div style={{ display: "flex", alignItems: "flex-end" }}>
 <button
 onClick={() => handleRunKubectl()}
 disabled={cmdLoading}
 className="panel-btn panel-btn-primary"
 style={{ cursor: cmdLoading ? "wait" : "pointer", opacity: cmdLoading ? 0.7 : 1 }}
 >
 {cmdLoading ? "" : "Run"}
 </button>
 </div>
 </div>

 {/* Output */}
 {(cmdOutput || cmdLoading) && (
 <pre
 ref={outputRef}
 style={{
 margin: 0, padding: 12,
 background: "var(--bg-primary)", color: "var(--text-primary)",
 border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)",
 fontSize: "var(--font-size-sm)", lineHeight: 1.5,
 minHeight: 80, maxHeight: 340,
 overflow: "auto", whiteSpace: "pre-wrap", wordBreak: "break-all",
 }}
 >
 {cmdLoading ? "Running…" : cmdOutput}
 </pre>
 )}

 {contexts.length === 0 && (
 <StatusMessage variant="empty" message="No kubectl contexts found." detail="Install kubectl and configure your kubeconfig, or type a context name manually above." inline />
 )}
 </div>
 )}

 {/* ── ArgoCD ── */}
 {subTab === "argocd" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
 Generate an <strong>ArgoCD Application CR</strong> for GitOps continuous deployment. Apply it to install and auto-sync your app.
 </div>

 <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
 <div>
 <label className="panel-label">APP NAME</label>
 <input className="panel-input panel-input-full" value={argoAppName} onChange={(e) => setArgoAppName(e.target.value)} />
 </div>
 <div>
 <label className="panel-label">TARGET NAMESPACE</label>
 <input className="panel-input panel-input-full" value={argoNamespace} onChange={(e) => setArgoNamespace(e.target.value)} />
 </div>
 <div style={{ gridColumn: "1 / -1" }}>
 <label className="panel-label">REPO URL</label>
 <input className="panel-input panel-input-full" value={argoRepoUrl} onChange={(e) => setArgoRepoUrl(e.target.value)} placeholder="https://github.com/org/repo" />
 </div>
 <div>
 <label className="panel-label">MANIFESTS PATH</label>
 <input className="panel-input panel-input-full" value={argoPath} onChange={(e) => setArgoPath(e.target.value)} placeholder="./k8s" />
 </div>
 <div>
 <label className="panel-label">ARGOCD SERVER</label>
 <input className="panel-input panel-input-full" value={argoServer} onChange={(e) => setArgoServer(e.target.value)} placeholder="https://kubernetes.default.svc" />
 </div>
 </div>

 <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
 <button
 onClick={handleGenerateArgo}
 disabled={argoLoading}
 className="panel-btn panel-btn-primary"
 style={{ cursor: argoLoading ? "wait" : "pointer", opacity: argoLoading ? 0.7 : 1 }}
 >
 {argoLoading ? "" : "Generate CR"}
 </button>
 {argoYaml && (
 <>
 <button
 onClick={() => { navigator.clipboard.writeText(argoYaml).then(() => { setArgoCopied(true); setTimeout(() => setArgoCopied(false), 1500); }).catch(() => {}); }}
 className="panel-btn panel-btn-secondary"
 >
 {argoCopied ? "✓ Copied" : "Copy YAML"}
 </button>
 <button
 onClick={handleApplyArgo}
 disabled={argoLoading}
 className="panel-btn panel-btn-secondary"
 style={{ color: "var(--success-color)", border: "1px solid var(--success-color)" }}
 >
 Apply to Cluster
 </button>
 </>
 )}
 </div>

 {argoApplied && (
 <div style={{ padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-base)", color: "var(--text-success)" }}>
 {argoApplied}
 </div>
 )}

 {argoYaml && (
 <pre style={{
 margin: 0, padding: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-sm)", lineHeight: 1.5,
 overflow: "auto", maxHeight: 360, whiteSpace: "pre-wrap",
 wordBreak: "break-word", color: "var(--text-primary)",
 }}>
 {argoYaml}
 </pre>
 )}
 </div>
 )}

 {/* ── Contexts ── */}
 {subTab === "contexts" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 {contexts.length === 0 ? (
 <EmptyState
   icon={<Ship size={32} strokeWidth={1.5} />}
   title="No kubectl contexts found"
   description="Install kubectl and configure your kubeconfig, or connect to a cluster."
 />
 ) : (
 <>
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
 {contexts.length} context{contexts.length !== 1 ? "s" : ""} found in your kubeconfig.
 </div>
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 {contexts.map((ctx) => (
 <div role="button" tabIndex={0}
 key={ctx}
 onClick={() => { setSelectedContext(ctx); setSubTab("deploy"); }}
 style={{
 display: "flex", alignItems: "center", justifyContent: "space-between",
 padding: "8px 12px",
 background: selectedContext === ctx ? "var(--bg-selected)" : "var(--bg-secondary)",
 border: "1px solid",
 borderColor: selectedContext === ctx ? "var(--accent-color)" : "var(--border-color)",
 borderRadius: "var(--radius-sm)",
 cursor: "pointer",
 }}
 >
 <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
 <span style={{ fontSize: "var(--font-size-lg)" }}></span>
 <code style={{ fontSize: "var(--font-size-base)" }}>{ctx}</code>
 </div>
 {selectedContext === ctx && (
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--accent-color)" }}>active</span>
 )}
 </div>
 ))}
 </div>
 <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
 Click a context to select it and switch to the Deploy tab.
 </div>
 </>
 )}
 </div>
 )}
 </div>
 </div>
 );
}
