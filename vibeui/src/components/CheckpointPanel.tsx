import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Checkpoint {
  index: number;
  message: string;
  oid: string;
}

interface CheckpointPanelProps {
  workspacePath?: string | null;
}

function extractLabel(message: string): string {
  // Strip "vibeui-checkpoint: " prefix if present
  const prefix = "vibeui-checkpoint: ";
  if (message.startsWith(prefix)) return message.slice(prefix.length);
  // Strip "vibeui-pre-ai: " prefix from stash entries
  const preAi = "vibeui-pre-ai: ";
  if (message.startsWith(preAi)) return message.slice(preAi.length);
  return message;
}

export function CheckpointPanel({ workspacePath }: CheckpointPanelProps) {
  const [checkpoints, setCheckpoints] = useState<Checkpoint[]>([]);
  const [label, setLabel] = useState("");
  const [loading, setLoading] = useState(false);
  const [restoring, setRestoring] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmRestore, setConfirmRestore] = useState<Checkpoint | null>(null);

  useEffect(() => {
    if (workspacePath) loadCheckpoints();
  }, [workspacePath]);

  async function loadCheckpoints() {
    setLoading(true);
    setError(null);
    try {
      const list = await invoke<Checkpoint[]>("list_checkpoints");
      setCheckpoints(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function createCheckpoint() {
    if (!label.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await invoke("create_checkpoint", { label: label.trim() });
      setLabel("");
      await loadCheckpoints();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function restoreCheckpoint(index: number) {
    setRestoring(index);
    setError(null);
    try {
      await invoke("restore_checkpoint", { index });
      setConfirmRestore(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setRestoring(null);
    }
  }

  if (!workspacePath) {
    return (
      <div style={{ padding: "16px", color: "var(--text-secondary)", fontSize: "13px" }}>
        Open a workspace folder to use checkpoints.
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", fontFamily: "var(--font-mono, monospace)" }}>
      {/* Header */}
      <div style={{ padding: "12px", borderBottom: "1px solid var(--border-color)" }}>
        <div style={{ fontSize: "11px", fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--text-secondary)", marginBottom: "8px" }}>
          Checkpoints
        </div>

        {/* Create new checkpoint */}
        <div style={{ display: "flex", gap: "6px" }}>
          <input
            type="text"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") createCheckpoint(); }}
            placeholder="Checkpoint label…"
            style={{
              flex: 1,
              padding: "5px 8px",
              fontSize: "12px",
              background: "var(--bg-input, var(--bg-primary))",
              border: "1px solid var(--border-color)",
              borderRadius: "4px",
              color: "var(--text-primary)",
              outline: "none",
            }}
          />
          <button
            onClick={createCheckpoint}
            disabled={!label.trim() || loading}
            style={{
              padding: "5px 10px",
              fontSize: "12px",
              background: "var(--accent-blue, #007acc)",
              color: "#fff",
              border: "none",
              borderRadius: "4px",
              cursor: label.trim() && !loading ? "pointer" : "not-allowed",
              opacity: label.trim() && !loading ? 1 : 0.5,
              whiteSpace: "nowrap",
            }}
          >
            + Save
          </button>
        </div>

        {error && (
          <div style={{ marginTop: "8px", padding: "6px 8px", background: "rgba(220,50,50,0.15)", borderRadius: "4px", fontSize: "12px", color: "#f44" }}>
            {error}
          </div>
        )}
      </div>

      {/* Checkpoint list */}
      <div style={{ flex: 1, overflowY: "auto", padding: "8px 0" }}>
        {loading && checkpoints.length === 0 && (
          <div style={{ padding: "16px", color: "var(--text-secondary)", fontSize: "12px", textAlign: "center" }}>
            Loading…
          </div>
        )}

        {!loading && checkpoints.length === 0 && (
          <div style={{ padding: "16px", color: "var(--text-secondary)", fontSize: "12px", textAlign: "center" }}>
            No checkpoints yet.{" "}
            <span style={{ opacity: 0.7 }}>Save one above before making risky changes.</span>
          </div>
        )}

        {checkpoints.map((cp) => (
          <div
            key={cp.index}
            style={{
              display: "flex",
              alignItems: "center",
              padding: "8px 12px",
              borderBottom: "1px solid var(--border-color)",
              gap: "8px",
            }}
          >
            {/* Timeline dot */}
            <div style={{
              width: "8px",
              height: "8px",
              borderRadius: "50%",
              background: "var(--accent-blue, #007acc)",
              flexShrink: 0,
            }} />

            {/* Label + meta */}
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{
                fontSize: "12px",
                color: "var(--text-primary)",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}>
                {extractLabel(cp.message)}
              </div>
              <div style={{ fontSize: "10px", color: "var(--text-secondary)", marginTop: "2px" }}>
                stash@{"{" + cp.index + "}"} · {cp.oid.slice(0, 8)}
              </div>
            </div>

            {/* Restore button */}
            <button
              onClick={() => setConfirmRestore(cp)}
              disabled={restoring === cp.index}
              style={{
                padding: "3px 8px",
                fontSize: "11px",
                background: "var(--bg-secondary)",
                border: "1px solid var(--border-color)",
                borderRadius: "3px",
                color: "var(--text-primary)",
                cursor: "pointer",
                whiteSpace: "nowrap",
                flexShrink: 0,
              }}
            >
              {restoring === cp.index ? "…" : "Restore"}
            </button>
          </div>
        ))}
      </div>

      {/* Confirm restore modal */}
      {confirmRestore && (
        <div style={{
          position: "absolute",
          inset: 0,
          background: "rgba(0,0,0,0.5)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          zIndex: 100,
        }}>
          <div style={{
            background: "var(--bg-secondary)",
            border: "1px solid var(--border-color)",
            borderRadius: "8px",
            padding: "20px",
            maxWidth: "320px",
            width: "90%",
          }}>
            <div style={{ fontSize: "13px", fontWeight: 600, marginBottom: "10px", color: "var(--text-primary)" }}>
              Restore Checkpoint?
            </div>
            <div style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: "16px", lineHeight: 1.5 }}>
              This will apply stash <strong style={{ color: "var(--text-primary)" }}>
                "{extractLabel(confirmRestore.message)}"
              </strong> to your current working tree. Uncommitted changes may conflict.
            </div>
            <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
              <button
                onClick={() => setConfirmRestore(null)}
                style={{
                  padding: "6px 14px", fontSize: "12px",
                  background: "var(--bg-primary)", border: "1px solid var(--border-color)",
                  borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer",
                }}
              >
                Cancel
              </button>
              <button
                onClick={() => restoreCheckpoint(confirmRestore.index)}
                style={{
                  padding: "6px 14px", fontSize: "12px",
                  background: "var(--accent-blue, #007acc)", border: "none",
                  borderRadius: "4px", color: "#fff", cursor: "pointer",
                }}
              >
                Restore
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
