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
      <div className="panel-header"><h3>Container Sandbox</h3></div>
      <div className="panel-body" style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>

      {error && <div className="panel-error" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}><span>{error}</span><button onClick={() => setError(null)} style={{ background: "none", border: "none", cursor: "pointer", fontSize: "var(--font-size-lg)" }}>&#x2715;</button></div>}

      {/* Runtime Detection */}
      <div style={{ marginBottom: 16, padding: "8px 12px", background: "var(--bg-primary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
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
      <div style={{ marginBottom: 16, padding: "8px 12px", background: "var(--bg-primary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
        <div style={{ fontWeight: 600, marginBottom: 8, color: "var(--text-primary)" }}>Create Sandbox</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
            Image
            <input value={newImage} onChange={(e) => setNewImage(e.target.value)} className="panel-input" placeholder="ubuntu:22.04" />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
            CPUs
            <input value={newCpus} onChange={(e) => setNewCpus(e.target.value)} className="panel-input" style={{ width: 60 }} />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
            Memory
            <input value={newMemory} onChange={(e) => setNewMemory(e.target.value)} className="panel-input" style={{ width: 70 }} />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
            Network
            <select value={newNetwork} onChange={(e) => setNewNetwork(e.target.value)} className="panel-select">
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
          <table className="panel-table">
            <thead>
              <tr>
                <th>ID</th>
                <th>Image</th>
                <th>Status</th>
                <th>Runtime</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {instances.map((c) => (
                <tr key={c.id}>
                  <td><code>{c.id.substring(0, 12)}</code></td>
                  <td>{c.image}</td>
                  <td>
                    <span style={{ color: statusColor(c.status) }}>{c.status}</span>
                  </td>
                  <td>{c.runtime}</td>
                  <td>
                    <button onClick={() => handlePause(c.id)} className="panel-btn panel-btn-secondary panel-btn-xs" title="Pause">Pause</button>
                    <button onClick={() => handleResume(c.id)} className="panel-btn panel-btn-secondary panel-btn-xs" title="Resume">Resume</button>
                    <button onClick={() => handleStop(c.id)} className="panel-btn panel-btn-danger panel-btn-xs" title="Stop & Remove">Stop</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Exec Console */}
      <div style={{ padding: "8px 12px", background: "var(--bg-primary)", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)" }}>
        <div style={{ fontWeight: 600, marginBottom: 8, color: "var(--text-primary)" }}>Execute Command</div>
        {instances.length > 0 ? (
          <>
            <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
              <select
                value={execContainerId}
                onChange={(e) => setExecContainerId(e.target.value)}
                className="panel-select"
                style={{ width: 160 }}
              >
                {instances.map((c) => (
                  <option key={c.id} value={c.id}>{c.id.substring(0, 12)} ({c.image})</option>
                ))}
              </select>
              <input
                value={execCmd}
                onChange={(e) => setExecCmd(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleExec()}
                className="panel-input"
                style={{ flex: 1 }}
                placeholder="ls -la /workspace"
              />
              <button onClick={handleExec} className="panel-btn panel-btn-primary">Run</button>
            </div>
            {execOutput && (
              <pre style={{
                background: "var(--bg-primary)",
                padding: 8,
                borderRadius: "var(--radius-xs-plus)",
                maxHeight: 200,
                overflow: "auto",
                fontSize: "var(--font-size-sm)",
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
      fontSize: "var(--font-size-base)",
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


