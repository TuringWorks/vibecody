/**
 * DeployPanel — One-Click Deployment for web projects.
 *
 * Supports: Vercel, Netlify, Railway, GitHub Pages
 * Flow: detect project type → show recommended target → Deploy → stream logs → show URL
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface DeployTarget {
  target: string;
  build_cmd: string;
  out_dir: string;
  detected_framework: string;
}

interface DeployRecord {
  id: string;
  target: string;
  url: string | null;
  timestamp: number;
  status: "success" | "failed" | "running";
}

const TARGETS = [
  { id: "vercel", label: "Vercel", icon: "▲", color: "#000" },
  { id: "netlify", label: "Netlify", icon: "◆", color: "#00C7B7" },
  { id: "railway", label: "Railway", icon: "🚂", color: "#0B0D0E" },
  { id: "github-pages", label: "GitHub Pages", icon: "⚙", color: "#24292e" },
];

interface DeployPanelProps {
  workspacePath: string;
}

export function DeployPanel({ workspacePath }: DeployPanelProps) {
  const [detected, setDetected] = useState<DeployTarget | null>(null);
  const [selectedTarget, setSelectedTarget] = useState("vercel");
  const [isDeploying, setIsDeploying] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [deployedUrl, setDeployedUrl] = useState<string | null>(null);
  const [history, setHistory] = useState<DeployRecord[]>([]);
  const logsEndRef = useRef<HTMLDivElement>(null);

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

  const handleDeploy = async () => {
    setIsDeploying(true);
    setLogs([`Starting deployment to ${selectedTarget}...`]);
    setDeployedUrl(null);

    // Listen for streaming log events
    const unlisten = await listen<string>("deploy:log", (e) => {
      setLogs(prev => [...prev, e.payload]);
    });

    try {
      const result = await invoke<{ url: string | null }>("run_deploy", {
        target: selectedTarget,
        workspace: workspacePath,
      });
      if (result.url) {
        setDeployedUrl(result.url);
        setLogs(prev => [...prev, `✅ Deployed to: ${result.url}`]);
      }
      // Refresh history
      const h = await invoke<DeployRecord[]>("get_deploy_history").catch(() => []);
      setHistory(h);
    } catch (e) {
      setLogs(prev => [...prev, `❌ Deployment failed: ${e}`]);
    } finally {
      setIsDeploying(false);
      unlisten();
    }
  };

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 16, height: "100%", overflowY: "auto" }}>
      {/* Detected project */}
      {detected && (
        <div style={{ background: "var(--bg-secondary, #1e1e2e)", borderRadius: 8, padding: 12, border: "1px solid var(--border, #2a2a3e)" }}>
          <div style={{ fontSize: 12, opacity: 0.7, marginBottom: 4 }}>Detected Project</div>
          <div style={{ fontWeight: 600 }}>{detected.detected_framework || "Static Site"}</div>
          <div style={{ fontSize: 11, opacity: 0.6, fontFamily: "monospace" }}>Build: {detected.build_cmd}</div>
        </div>
      )}

      {/* Target selection */}
      <div>
        <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>Deploy Target</div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          {TARGETS.map((t) => (
            <button
              key={t.id}
              onClick={() => setSelectedTarget(t.id)}
              style={{
                background: selectedTarget === t.id ? "var(--accent-blue, #6366f1)" : "var(--bg-secondary, #1e1e2e)",
                border: `1px solid ${selectedTarget === t.id ? "var(--accent-blue, #6366f1)" : "var(--border, #2a2a3e)"}`,
                borderRadius: 6,
                padding: "10px 8px",
                cursor: "pointer",
                color: "var(--text-primary, #cdd6f4)",
                fontSize: 13,
                fontWeight: selectedTarget === t.id ? 600 : 400,
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              <span>{t.icon}</span> {t.label}
            </button>
          ))}
        </div>
      </div>

      {/* Deploy button */}
      <button
        onClick={handleDeploy}
        disabled={isDeploying}
        style={{
          background: isDeploying ? "var(--bg-tertiary, #2a2a3e)" : "#6366f1",
          color: "#fff",
          border: "none",
          borderRadius: 6,
          padding: "10px 0",
          cursor: isDeploying ? "not-allowed" : "pointer",
          fontWeight: 700,
          fontSize: 14,
        }}
      >
        {isDeploying ? "Deploying…" : "🚀 Deploy"}
      </button>

      {/* Deployed URL */}
      {deployedUrl && (
        <div style={{ background: "rgba(166,227,161,0.1)", border: "1px solid #a6e3a1", borderRadius: 6, padding: 10 }}>
          <div style={{ fontSize: 12, color: "#a6e3a1", marginBottom: 4 }}>✅ Live at</div>
          <a href={deployedUrl} target="_blank" rel="noopener noreferrer" style={{ color: "#89b4fa", fontSize: 13, fontFamily: "monospace" }}>
            {deployedUrl}
          </a>
        </div>
      )}

      {/* Log stream */}
      {logs.length > 0 && (
        <div style={{ background: "var(--bg-secondary, #1e1e2e)", borderRadius: 6, padding: 10, maxHeight: 200, overflowY: "auto", fontFamily: "monospace", fontSize: 11 }}>
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
            <div key={rec.id} style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 0", borderBottom: "1px solid var(--border, #2a2a3e)", fontSize: 12 }}>
              <span>{rec.status === "success" ? "✅" : rec.status === "running" ? "🔄" : "❌"}</span>
              <span style={{ opacity: 0.7 }}>{rec.target}</span>
              {rec.url && <a href={rec.url} target="_blank" rel="noopener noreferrer" style={{ color: "#89b4fa", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>{rec.url}</a>}
              <span style={{ opacity: 0.4, flexShrink: 0 }}>{new Date(rec.timestamp).toLocaleDateString()}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
