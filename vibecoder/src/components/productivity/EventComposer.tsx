import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CalendarPlus } from "lucide-react";
import type { CalendarEvent } from "../../types/productivity";
import { ComposeModal } from "./ComposeModal";

interface Props {
  onClose: () => void;
  onCreated?: (event: CalendarEvent) => void;
}

function toLocalInput(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function defaults() {
  const now = new Date();
  now.setMinutes(now.getMinutes() - (now.getMinutes() % 15) + 15, 0, 0);
  const end = new Date(now.getTime() + 60 * 60 * 1000);
  return { start: toLocalInput(now), end: toLocalInput(end) };
}

export function EventComposer({ onClose, onCreated }: Props) {
  const initial = useMemo(defaults, []);
  const [summary, setSummary] = useState("");
  const [start, setStart] = useState(initial.start);
  const [end, setEnd] = useState(initial.end);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function create() {
    if (!summary.trim() || !start || !end) return;
    const startIso = new Date(start).toISOString();
    const endIso = new Date(end).toISOString();
    if (new Date(endIso) <= new Date(startIso)) {
      setErr("End time must be after start time.");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      const ev = await invoke<CalendarEvent>("productivity_cal_create", {
        summary: summary.trim(),
        start: startIso,
        end: endIso,
      });
      onCreated?.(ev);
      onClose();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <ComposeModal
      title="New event"
      onClose={onClose}
      footer={
        <>
          <button
            className="panel-btn panel-btn-secondary"
            onClick={onClose}
            disabled={busy}
          >
            Cancel
          </button>
          <button
            className="panel-btn panel-btn-primary"
            onClick={create}
            disabled={busy || !summary.trim()}
            style={{ display: "flex", alignItems: "center", gap: 5 }}
          >
            <CalendarPlus size={12} />
            {busy ? "Creating…" : "Create"}
          </button>
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <label style={{ display: "flex", flexDirection: "column", gap: 3 }}>
          <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
            Title
          </span>
          <input
            className="panel-input"
            placeholder="Event summary"
            value={summary}
            onChange={(e) => setSummary(e.target.value)}
            disabled={busy}
            autoFocus
          />
        </label>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          <label style={{ display: "flex", flexDirection: "column", gap: 3 }}>
            <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
              Start
            </span>
            <input
              className="panel-input"
              type="datetime-local"
              value={start}
              onChange={(e) => setStart(e.target.value)}
              disabled={busy}
            />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 3 }}>
            <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
              End
            </span>
            <input
              className="panel-input"
              type="datetime-local"
              value={end}
              onChange={(e) => setEnd(e.target.value)}
              disabled={busy}
            />
          </label>
        </div>
        {err && (
          <div
            style={{
              color: "var(--color-error, #d63e3e)",
              fontSize: "var(--font-size-sm)",
            }}
          >
            {err}
          </div>
        )}
      </div>
    </ComposeModal>
  );
}
