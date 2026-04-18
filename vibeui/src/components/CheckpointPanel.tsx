import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, Loader2 } from "lucide-react";

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
  const [deleting, setDeleting] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmRestore, setConfirmRestore] = useState<Checkpoint | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<Checkpoint | null>(null);

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

  async function deleteCheckpoint(index: number) {
    setDeleting(index);
    setError(null);
    try {
      await invoke("delete_checkpoint", { index });
      setConfirmDelete(null);
      await loadCheckpoints();
    } catch (e) {
      setError(String(e));
    } finally {
      setDeleting(null);
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
      <div style={{ padding: "16px", color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
        Open a workspace folder to use checkpoints.
      </div>
    );
  }

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header" style={{ padding: "12px", flexDirection: "column", alignItems: "stretch" }}>
        <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--text-secondary)", marginBottom: "8px" }}>
          Checkpoints
        </div>

        {/* Create new checkpoint */}
        <div style={{ display: "flex", gap: "8px" }}>
          <input
            type="text"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") createCheckpoint(); }}
            placeholder="Checkpoint label…"
            style={{
              flex: 1,
              padding: "4px 8px",
              fontSize: "var(--font-size-base)",
              background: "var(--bg-input, var(--bg-primary))",
              border: "1px solid var(--border-color)",
              borderRadius: "var(--radius-xs-plus)",
              color: "var(--text-primary)",
              outline: "none",
            }}
          />
          <button
            onClick={createCheckpoint}
            disabled={!label.trim() || loading}
            className="panel-btn panel-btn-primary"
            style={{
              cursor: label.trim() && !loading ? "pointer" : "not-allowed",
              opacity: label.trim() && !loading ? 1 : 0.5,
              whiteSpace: "nowrap",
            }}
          >
            + Save
          </button>
        </div>

        {error && (
          <div style={{ marginTop: "8px", padding: "8px 8px", background: "color-mix(in srgb, var(--accent-rose) 15%, transparent)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)", color: "var(--error-color)" }}>
            {error}
          </div>
        )}
      </div>

      {/* Checkpoint list */}
      <div className="panel-body" style={{ padding: "8px 0" }}>
        {loading && checkpoints.length === 0 && (
          <div className="panel-loading">
            Loading…
          </div>
        )}

        {!loading && checkpoints.length === 0 && (
          <div className="panel-empty">
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
              background: "var(--accent-color)",
              flexShrink: 0,
            }} />

            {/* Label + meta */}
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{
                fontSize: "var(--font-size-base)",
                color: "var(--text-primary)",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}>
                {extractLabel(cp.message)}
              </div>
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: "2px" }}>
                stash@{"{" + cp.index + "}"} · {cp.oid.slice(0, 8)}
              </div>
            </div>

            {/* Restore button */}
            <button
              onClick={() => setConfirmRestore(cp)}
              disabled={restoring === cp.index || deleting === cp.index}
              className="panel-btn panel-btn-secondary"
              style={{ whiteSpace: "nowrap", flexShrink: 0 }}
            >
              {restoring === cp.index ? "…" : "Restore"}
            </button>

            {/* Delete button */}
            <button
              onClick={() => setConfirmDelete(cp)}
              disabled={deleting === cp.index || restoring === cp.index}
              title="Delete checkpoint"
              className="panel-btn panel-btn-danger"
              style={{ flexShrink: 0, lineHeight: 1, padding: "3px 8px" }}
            >
              {deleting === cp.index ? <Loader2 size={12} className="spin" /> : <X size={12} />}
            </button>
          </div>
        ))}
      </div>

      {/* Confirm delete modal */}
      {confirmDelete && (
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
            borderRadius: "var(--radius-sm-alt)",
            padding: "20px",
            maxWidth: "320px",
            width: "90%",
          }}>
            <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: "12px", color: "var(--text-primary)" }}>
              Delete Checkpoint?
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: "16px", lineHeight: 1.5 }}>
              Permanently delete <strong style={{ color: "var(--text-primary)" }}>
                "{extractLabel(confirmDelete.message)}"
              </strong>? This cannot be undone.
            </div>
            <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
              <button
                onClick={() => setConfirmDelete(null)}
                className="panel-btn panel-btn-secondary"
              >
                Cancel
              </button>
              <button
                onClick={() => deleteCheckpoint(confirmDelete.index)}
                className="panel-btn panel-btn-danger"
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}

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
            borderRadius: "var(--radius-sm-alt)",
            padding: "20px",
            maxWidth: "320px",
            width: "90%",
          }}>
            <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: "12px", color: "var(--text-primary)" }}>
              Restore Checkpoint?
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: "16px", lineHeight: 1.5 }}>
              This will apply stash <strong style={{ color: "var(--text-primary)" }}>
                "{extractLabel(confirmRestore.message)}"
              </strong> to your current working tree. Uncommitted changes may conflict.
            </div>
            <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
              <button
                onClick={() => setConfirmRestore(null)}
                className="panel-btn panel-btn-secondary"
              >
                Cancel
              </button>
              <button
                onClick={() => restoreCheckpoint(confirmRestore.index)}
                className="panel-btn panel-btn-primary"
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
