import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RuntimeInfo {
  docker: string | null;
  podman: string | null;
  opensandbox: string | null;
  active: string;
}

interface SandboxInstance {
  id: string;
  name: string;
  image: string;
  status: string;
  created_at: string;
  runtime: string;
}

interface ExecResult {
  exit_code: number;
  stdout: string;
  stderr: string;
}

export function SandboxPanel() {
  const [runtimes, setRuntimes] = useState<RuntimeInfo | null>(null);
  const [instances, setInstances] = useState<SandboxInstance[]>([]);
  const [loading, setLoading] = useState(false);
  const [execCmd, setExecCmd] = useState("");
  const [execOutput, setExecOutput] = useState("");
  const [execContainerId, setExecContainerId] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Config form
  const [newImage, setNewImage] = useState("ubuntu:22.04");
  const [newCpus, setNewCpus] = useState("2");
  const [newMemory, setNewMemory] = useState("4g");
  const [newNetwork, setNewNetwork] = useState("full");

  const detectRuntimes = useCallback(async () => {
    try {
      const info = await invoke<RuntimeInfo>("detect_sandbox_runtime");
      setRuntimes(info);
    } catch (e) {
      console.error("detect_sandbox_runtime:", e);
    }
  }, []);

  const refreshInstances = useCallback(async () => {
    try {
      const list = await invoke<SandboxInstance[]>("list_sandboxes");
      setInstances(list);
      if (list.length > 0 && !execContainerId) {
        setExecContainerId(list[0].id);
      }
    } catch (e) {
      console.error("list_sandboxes:", e);
    }
  }, [execContainerId]);

  useEffect(() => {
    detectRuntimes();
    refreshInstances();
  }, [detectRuntimes, refreshInstances]);

  const handleCreate = async () => {
    setLoading(true);
    try {
      await invoke("create_sandbox", {
        image: newImage,
        cpus: parseFloat(newCpus) || undefined,
        memory: newMemory || undefined,
        networkMode: newNetwork === "none" ? "none" : undefined,
      });
      await refreshInstances();
    } catch (e) {
      setError(`Failed to create sandbox: ${e}`);
    }
    setLoading(false);
  };

  const handleStop = async (id: string) => {
    try {
      await invoke("stop_sandbox", { containerId: id });
      await refreshInstances();
    } catch (e) {
      setError(`Stop failed: ${e}`);
    }
  };

  const handlePause = async (id: string) => {
    try {
      await invoke("pause_sandbox", { containerId: id });
      await refreshInstances();
    } catch (e) {
      setError(`Pause failed: ${e}`);
    }
  };

  const handleResume = async (id: string) => {
    try {
      await invoke("resume_sandbox", { containerId: id });
      await refreshInstances();
    } catch (e) {
      setError(`Resume failed: ${e}`);
    }
  };

  const handleExec = async () => {
    if (!execContainerId || !execCmd.trim()) return;
    try {
      const result = await invoke<ExecResult>("sandbox_exec", {
        containerId: execContainerId,
        command: execCmd,
      });
      let output = result.stdout;
      if (result.stderr) output += "\nSTDERR:\n" + result.stderr;
      if (result.exit_code !== 0) output += `\n[exit code: ${result.exit_code}]`;
      setExecOutput(output);
    } catch (e) {
      setExecOutput(`Error: ${e}`);
    }
  };

  const statusColor = (status: string) => {
    if (status.toLowerCase().includes("up") || status === "running") return "var(--success-color)";
    if (status.toLowerCase().includes("paused")) return "var(--warning-color)";
    if (status.toLowerCase().includes("exited")) return "var(--error-color)";
    return "var(--text-secondary)";
  };

  return (
    <div className="panel-container">
      <div className="panel-header">Container Sandbox</div>
      <div className="panel-body" style={{ color: "var(--text-secondary)", fontSize: 13 }}>

      {error && <div className="panel-error" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 12px", marginBottom: 12, background: "var(--error-color)", borderRadius: 4, color: "var(--btn-primary-fg)", fontSize: 12 }}><span>{error}</span><button onClick={() => setError(null)} style={{ background: "none", border: "none", color: "var(--btn-primary-fg)", cursor: "pointer", fontSize: 14 }}>&#x2715;</button></div>}

      {/* Runtime Detection */}
      <div style={{ marginBottom: 16, padding: "8px 12px", background: "var(--bg-primary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
        <div style={{ fontWeight: 600, marginBottom: 6, color: "var(--text-primary)" }}>Available Runtimes</div>
        {runtimes ? (
          <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
            <RuntimeBadge name="Docker" version={runtimes.docker} active={runtimes.active === "docker"} />
            <RuntimeBadge name="Podman" version={runtimes.podman} active={runtimes.active === "podman"} />
            <RuntimeBadge name="OpenSandbox" version={runtimes.opensandbox} active={runtimes.active === "opensandbox"} />
          </div>
        ) : (
          <span style={{ color: "var(--text-secondary)" }}>Detecting...</span>
        )}
        <button onClick={detectRuntimes} className="panel-btn panel-btn-secondary" title="Refresh">Refresh</button>
      </div>

      {/* Create Sandbox Form */}
      <div style={{ marginBottom: 16, padding: "8px 12px", background: "var(--bg-primary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
        <div style={{ fontWeight: 600, marginBottom: 8, color: "var(--text-primary)" }}>Create Sandbox</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <label style={labelStyle}>
            Image
            <input value={newImage} onChange={(e) => setNewImage(e.target.value)} style={inputStyle} placeholder="ubuntu:22.04" />
          </label>
          <label style={labelStyle}>
            CPUs
            <input value={newCpus} onChange={(e) => setNewCpus(e.target.value)} style={{ ...inputStyle, width: 60 }} />
          </label>
          <label style={labelStyle}>
            Memory
            <input value={newMemory} onChange={(e) => setNewMemory(e.target.value)} style={{ ...inputStyle, width: 70 }} />
          </label>
          <label style={labelStyle}>
            Network
            <select value={newNetwork} onChange={(e) => setNewNetwork(e.target.value)} style={inputStyle}>
              <option value="full">Full</option>
              <option value="restricted">Restricted</option>
              <option value="none">None</option>
            </select>
          </label>
          <button onClick={handleCreate} disabled={loading} className="panel-btn panel-btn-primary">
            {loading ? "Starting..." : "Start"}
          </button>
        </div>
      </div>

      {/* Instances Table */}
      <div style={{ marginBottom: 16 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <span style={{ fontWeight: 600, color: "var(--text-primary)" }}>Running Instances ({instances.length})</span>
          <button onClick={refreshInstances} className="panel-btn panel-btn-secondary">Refresh</button>
        </div>
        {instances.length === 0 ? (
          <div style={{ color: "var(--text-secondary)", fontStyle: "italic" }}>No sandbox containers running.</div>
        ) : (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--border-color)", textAlign: "left" }}>
                <th style={thStyle}>ID</th>
                <th style={thStyle}>Image</th>
                <th style={thStyle}>Status</th>
                <th style={thStyle}>Runtime</th>
                <th style={thStyle}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {instances.map((c) => (
                <tr key={c.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={tdStyle}><code>{c.id.substring(0, 12)}</code></td>
                  <td style={tdStyle}>{c.image}</td>
                  <td style={tdStyle}>
                    <span style={{ color: statusColor(c.status) }}>{c.status}</span>
                  </td>
                  <td style={tdStyle}>{c.runtime}</td>
                  <td style={tdStyle}>
                    <button onClick={() => handlePause(c.id)} style={smallBtn} title="Pause">Pause</button>
                    <button onClick={() => handleResume(c.id)} style={smallBtn} title="Resume">Resume</button>
                    <button onClick={() => handleStop(c.id)} style={{ ...smallBtn, color: "var(--error-color)" }} title="Stop & Remove">Stop</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Exec Console */}
      <div style={{ padding: "8px 12px", background: "var(--bg-primary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
        <div style={{ fontWeight: 600, marginBottom: 8, color: "var(--text-primary)" }}>Execute Command</div>
        {instances.length > 0 ? (
          <>
            <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
              <select
                value={execContainerId}
                onChange={(e) => setExecContainerId(e.target.value)}
                style={{ ...inputStyle, width: 160 }}
              >
                {instances.map((c) => (
                  <option key={c.id} value={c.id}>{c.id.substring(0, 12)} ({c.image})</option>
                ))}
              </select>
              <input
                value={execCmd}
                onChange={(e) => setExecCmd(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleExec()}
                style={{ ...inputStyle, flex: 1 }}
                placeholder="ls -la /workspace"
              />
              <button onClick={handleExec} className="panel-btn panel-btn-primary">Run</button>
            </div>
            {execOutput && (
              <pre style={{
                background: "var(--bg-primary)",
                padding: 8,
                borderRadius: 4,
                maxHeight: 200,
                overflow: "auto",
                fontSize: 11,
                color: "var(--text-secondary)",
                margin: 0,
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
              }}>
                {execOutput}
              </pre>
            )}
          </>
        ) : (
          <div className="panel-empty">Start a sandbox to use the exec console.</div>
        )}
      </div>
      </div>
    </div>
  );
}

function RuntimeBadge({ name, version, active }: { name: string; version: string | null; active: boolean }) {
  return (
    <span style={{
      display: "inline-flex",
      alignItems: "center",
      gap: 4,
      padding: "2px 8px",
      borderRadius: 12,
      fontSize: 12,
      background: version ? (active ? "rgba(14,99,156,0.2)" : "var(--bg-primary)") : "var(--bg-secondary)",
      border: `1px solid ${version ? (active ? "var(--accent-color)" : "var(--border-color)") : "var(--border-color)"}`,
      color: version ? (active ? "var(--success-color)" : "var(--text-secondary)") : "var(--text-secondary)",
    }}>
      <span style={{ width: 6, height: 6, borderRadius: "50%", background: version ? "var(--success-color)" : "var(--text-secondary)" }} />
      {name}
      {version && <span style={{ color: "var(--text-secondary)", marginLeft: 4 }}>v{version}</span>}
      {active && <span style={{ color: "var(--success-color)", fontWeight: 600 }}>(active)</span>}
    </span>
  );
}


const smallBtn: React.CSSProperties = {
  background: "transparent",
  color: "var(--text-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: 3,
  padding: "2px 6px",
  cursor: "pointer",
  fontSize: 11,
  marginRight: 4,
};

const inputStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  color: "var(--text-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: 4,
  padding: "4px 8px",
  fontSize: 12,
};

const labelStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 2,
  fontSize: 11,
  color: "var(--text-secondary)",
};

const thStyle: React.CSSProperties = {
  padding: "4px 8px",
  color: "var(--text-secondary)",
  fontWeight: 500,
  fontSize: 11,
};

const tdStyle: React.CSSProperties = {
  padding: "4px 8px",
};
