import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Circle, Flame, Plus, Zap } from "lucide-react";
import type { TodoistTask } from "../../types/productivity";
import { ComposeModal } from "./ComposeModal";

interface Props {
  onClose: () => void;
  onCreated?: (task: TodoistTask) => void;
}

const PRIORITIES: { value: number; label: string; icon: React.ReactNode }[] = [
  { value: 4, label: "P1 · Urgent", icon: <Flame size={12} color="var(--color-error, #d63e3e)" /> },
  { value: 3, label: "P2 · High", icon: <Zap size={12} color="var(--color-warn, #c69023)" /> },
  { value: 2, label: "P3 · Medium", icon: <Circle size={10} color="var(--text-secondary)" strokeWidth={2} /> },
  { value: 1, label: "P4 · Normal", icon: <span style={{ color: "var(--text-secondary)", width: 12, textAlign: "center" }}>·</span> },
];

const DUE_PRESETS = ["today", "tomorrow", "this weekend", "next week"];

export function TaskComposer({ onClose, onCreated }: Props) {
  const [content, setContent] = useState("");
  const [due, setDue] = useState("");
  const [priority, setPriority] = useState(1);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function create() {
    if (!content.trim()) return;
    setBusy(true);
    setErr(null);
    try {
      const t = await invoke<TodoistTask>("productivity_tasks_add_full", {
        content: content.trim(),
        due: due.trim() || null,
        priority,
      });
      onCreated?.(t);
      onClose();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <ComposeModal
      title="New task"
      onClose={onClose}
      footer={
        <>
          <button className="panel-btn panel-btn-secondary" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button
            className="panel-btn panel-btn-primary"
            onClick={create}
            disabled={busy || !content.trim()}
            style={{ display: "flex", alignItems: "center", gap: 5 }}
          >
            <Plus size={12} />
            {busy ? "Adding…" : "Add task"}
          </button>
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        <label style={{ display: "flex", flexDirection: "column", gap: 3 }}>
          <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
            Task
          </span>
          <input
            className="panel-input"
            placeholder="What needs to be done?"
            value={content}
            onChange={(e) => setContent(e.target.value)}
            disabled={busy}
            autoFocus
          />
        </label>
        <div>
          <div
            style={{
              color: "var(--text-secondary)",
              fontSize: "var(--font-size-sm)",
              marginBottom: 3,
            }}
          >
            Due
          </div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 6 }}>
            {DUE_PRESETS.map((p) => (
              <button
                key={p}
                className={`panel-btn panel-btn-secondary${due === p ? " active" : ""}`}
                onClick={() => setDue(due === p ? "" : p)}
                disabled={busy}
                style={{ fontSize: "calc(var(--font-size-sm) - 1px)" }}
              >
                {p}
              </button>
            ))}
          </div>
          <input
            className="panel-input"
            placeholder="Natural language (e.g. every Monday, 2026-04-25, next Friday at 3pm)"
            value={due}
            onChange={(e) => setDue(e.target.value)}
            disabled={busy}
          />
        </div>
        <div>
          <div
            style={{
              color: "var(--text-secondary)",
              fontSize: "var(--font-size-sm)",
              marginBottom: 3,
            }}
          >
            Priority
          </div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {PRIORITIES.map((p) => (
              <button
                key={p.value}
                className={`panel-btn panel-btn-secondary${priority === p.value ? " active" : ""}`}
                onClick={() => setPriority(p.value)}
                disabled={busy}
                style={{ display: "flex", alignItems: "center", gap: 4 }}
              >
                {p.icon}
                {p.label}
              </button>
            ))}
          </div>
        </div>
        {err && (
          <div style={{ color: "var(--color-error, #d63e3e)", fontSize: "var(--font-size-sm)" }}>
            {err}
          </div>
        )}
      </div>
    </ComposeModal>
  );
}
