import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CloudAgentResult {
  container_id: string;
  status: string;
  image: string;
  task: string;
}

interface CloudAgentStatusResult {
  container_id: string;
  status: string;
}

export function CloudAgentPanel() {
  const [image, setImage] = useState("ubuntu:22.04");
  const [task, setTask] = useState("");
  const [workspace, setWorkspace] = useState("");
  const [status, setStatus] = useState<CloudAgentResult | null>(null);
  const [pollStatus, setPollStatus] = useState<CloudAgentStatusResult | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [launching, setLaunching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const PRESET_IMAGES = [
    { label: "Ubuntu 22.04", value: "ubuntu:22.04" },
    { label: "Node 20", value: "node:20-slim" },
    { label: "Python 3.12", value: "python:3.12-slim" },
    { label: "Rust 1.77", value: "rust:1.77-slim" },
    { label: "Go 1.22", value: "golang:1.22-alpine" },
    { label: "Alpine", value: "alpine:3.19" },
  ];

  const handleLaunch = async () => {
    if (!task.trim()) return;
    setLaunching(true);
    setError(null);
    setStatus(null);
    setPollStatus(null);
    setLogs([]);
    try {
      const result = await invoke<CloudAgentResult>("start_cloud_agent", {
        image,
        task: task.trim(),
        workspace: workspace.trim() || null,
      });
      setStatus(result);
      setLogs([
        `Container: ${result.container_id}`,
        `Image: ${result.image}`,
        `Status: ${result.status}`,
        `Task: ${result.task}`,
      ]);
    } catch (e) {
      setError(String(e));
    } finally {
      setLaunching(false);
    }
  };

  const handleCheckStatus = async () => {
    if (!status?.container_id) return;
    try {
      const result = await invoke<CloudAgentStatusResult>("get_cloud_agent_status", {
        containerId: status.container_id,
      });
      setPollStatus(result);
      setLogs((prev) => [...prev, `[poll] ${result.container_id}: ${result.status}`]);
    } catch (e) {
      setLogs((prev) => [...prev, `[error] ${String(e)}`]);
    }
  };

  const statusColor = (s: string) => {
    if (s === "complete" || s === "running") return "var(--success-color)";
    if (s === "failed") return "var(--error-color)";
    if (s === "queued") return "var(--warning-color)";
    return "var(--text-secondary)";
  };

  return (
    <div style={{ padding: 16, height: "100%", overflowY: "auto", color: "var(--text-primary)" }}>
      <h3 style={{ margin: "0 0 12px 0", fontSize: 15 }}>Cloud Agent (Docker)</h3>
      <p style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 16 }}>
        Run agent tasks inside isolated Docker containers. Requires Docker to be installed and running.
      </p>

      {/* Image selector */}
      <div style={{ marginBottom: 12 }}>
        <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Docker Image</label>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 6 }}>
          {PRESET_IMAGES.map((preset) => (
            <button
              key={preset.value}
              onClick={() => setImage(preset.value)}
              style={{
                padding: "3px 8px",
                fontSize: 11,
                border: "1px solid var(--border-color)",
                borderRadius: 4,
                cursor: "pointer",
                background: image === preset.value ? "var(--accent-color)" : "var(--bg-secondary)",
                color: image === preset.value ? "var(--text-primary)" : "var(--text-secondary)",
              }}
            >
              {preset.label}
            </button>
          ))}
        </div>
        <input
          type="text"
          value={image}
          onChange={(e) => setImage(e.target.value)}
          placeholder="docker image (e.g. ubuntu:22.04)"
          style={{
            width: "100%",
            padding: "6px 8px",
            fontSize: 12,
            background: "var(--bg-primary)",
            color: "var(--text-primary)",
            border: "1px solid var(--border-color)",
            borderRadius: 4,
            boxSizing: "border-box",
          }}
        />
      </div>

      {/* Workspace path */}
      <div style={{ marginBottom: 12 }}>
        <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
          Workspace Path (optional, mounted as /workspace)
        </label>
        <input
          type="text"
          value={workspace}
          onChange={(e) => setWorkspace(e.target.value)}
          placeholder="/path/to/your/project"
          style={{
            width: "100%",
            padding: "6px 8px",
            fontSize: 12,
            background: "var(--bg-primary)",
            color: "var(--text-primary)",
            border: "1px solid var(--border-color)",
            borderRadius: 4,
            boxSizing: "border-box",
          }}
        />
      </div>

      {/* Task input */}
      <div style={{ marginBottom: 12 }}>
        <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Agent Task</label>
        <textarea
          value={task}
          onChange={(e) => setTask(e.target.value)}
          placeholder="Describe the task for the agent (e.g. 'Fix all clippy warnings and open a PR')"
          rows={3}
          style={{
            width: "100%",
            padding: "6px 8px",
            fontSize: 12,
            background: "var(--bg-primary)",
            color: "var(--text-primary)",
            border: "1px solid var(--border-color)",
            borderRadius: 4,
            resize: "vertical",
            boxSizing: "border-box",
            fontFamily: "inherit",
          }}
        />
      </div>

      {/* Launch button */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <button
          onClick={handleLaunch}
          disabled={launching || !task.trim()}
          style={{
            padding: "6px 16px",
            fontSize: 12,
            background: launching ? "var(--bg-secondary)" : "var(--accent-color)",
            color: "var(--text-primary)",
            border: "none",
            borderRadius: 4,
            cursor: launching || !task.trim() ? "not-allowed" : "pointer",
            opacity: launching || !task.trim() ? 0.6 : 1,
          }}
        >
          {launching ? "Launching..." : "Launch Container"}
        </button>
        {status && (
          <button
            onClick={handleCheckStatus}
            style={{
              padding: "6px 12px",
              fontSize: 12,
              background: "var(--bg-secondary)",
              color: "var(--text-primary)",
              border: "1px solid var(--border-color)",
              borderRadius: 4,
              cursor: "pointer",
            }}
          >
            Check Status
          </button>
        )}
      </div>

      {/* Error */}
      {error && (
        <div
          style={{
            padding: "8px 12px",
            marginBottom: 12,
            background: "rgba(244, 67, 54, 0.15)",
            border: "1px solid var(--error-color)",
            borderRadius: 4,
            fontSize: 12,
            color: "var(--error-color)",
          }}
        >
          {error}
        </div>
      )}

      {/* Status badge */}
      {status && (
        <div
          style={{
            padding: "8px 12px",
            marginBottom: 12,
            background: "var(--bg-secondary)",
            border: "1px solid var(--border-color)",
            borderRadius: 4,
            fontSize: 12,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
            <span
              style={{
                display: "inline-block",
                width: 8,
                height: 8,
                borderRadius: "50%",
                background: statusColor(pollStatus?.status || status.status),
              }}
            />
            <strong>{status.container_id}</strong>
          </div>
          <div style={{ color: "var(--text-secondary)" }}>
            Status: <span style={{ color: statusColor(pollStatus?.status || status.status) }}>
              {pollStatus?.status || status.status}
            </span>
          </div>
        </div>
      )}

      {/* Log output */}
      {logs.length > 0 && (
        <div>
          <label style={{ fontSize: 12, display: "block", marginBottom: 4, color: "var(--text-secondary)" }}>
            Container Logs
          </label>
          <pre
            style={{
              padding: "8px 10px",
              background: "var(--bg-primary)",
              color: "var(--text-primary)",
              border: "1px solid var(--border-color)",
              borderRadius: 4,
              fontSize: 11,
              fontFamily: "'Fira Code', 'Cascadia Code', 'Consolas', monospace",
              maxHeight: 200,
              overflowY: "auto",
              margin: 0,
              whiteSpace: "pre-wrap",
              wordBreak: "break-all",
            }}
          >
            {logs.join("\n")}
          </pre>
        </div>
      )}
    </div>
  );
}
