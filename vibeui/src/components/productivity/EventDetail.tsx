import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Calendar as CalIcon,
  Clock,
  Loader2,
  MapPin,
  Trash2,
  Users,
  X,
} from "lucide-react";
import type { CalendarEvent } from "../../types/productivity";

interface Props {
  event: CalendarEvent;
  onClose: () => void;
  onDeleted: (id: string) => void;
}

function formatDateTime(iso: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function EventDetail({ event, onClose, onDeleted }: Props) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function remove() {
    setBusy(true);
    setErr(null);
    try {
      await invoke("productivity_cal_delete", { id: event.id });
      onDeleted(event.id);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        background: "var(--bg-primary)",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "8px 10px",
          borderBottom: "1px solid var(--border-color)",
          flexShrink: 0,
        }}
      >
        <button
          className="panel-btn panel-btn-secondary"
          onClick={remove}
          disabled={busy}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {busy ? (
            <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
          ) : (
            <Trash2 size={12} />
          )}
          Delete
        </button>
        <span style={{ flex: 1 }} />
        <button
          className="panel-btn panel-btn-secondary"
          onClick={onClose}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <X size={12} />
        </button>
      </div>
      <div style={{ flex: 1, overflowY: "auto", padding: 14 }}>
        {err && (
          <div
            style={{
              color: "var(--color-error, #d63e3e)",
              fontSize: "var(--font-size-sm)",
              marginBottom: 10,
            }}
          >
            {err}
          </div>
        )}
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 14 }}>
          <CalIcon size={16} strokeWidth={1.75} />
          <h3 style={{ margin: 0, fontSize: "var(--font-size-lg, 15px)" }}>
            {event.summary || "(untitled)"}
          </h3>
        </div>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "16px 1fr",
            gap: "6px 10px",
            fontSize: "var(--font-size-sm)",
            marginBottom: 12,
            alignItems: "center",
          }}
        >
          <Clock size={12} color="var(--text-secondary)" />
          <span>
            {formatDateTime(event.start)} — {formatDateTime(event.end)}
          </span>
          {event.location && (
            <>
              <MapPin size={12} color="var(--text-secondary)" />
              <span>{event.location}</span>
            </>
          )}
          {event.attendees.length > 0 && (
            <>
              <Users size={12} color="var(--text-secondary)" />
              <span>
                {event.attendees.length} attendee
                {event.attendees.length === 1 ? "" : "s"}
                <br />
                <span style={{ color: "var(--text-secondary)" }}>
                  {event.attendees.slice(0, 8).join(", ")}
                  {event.attendees.length > 8 && ` +${event.attendees.length - 8} more`}
                </span>
              </span>
            </>
          )}
        </div>
        {event.description && (
          <pre
            style={{
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              fontFamily: "inherit",
              fontSize: "var(--font-size-sm)",
              lineHeight: 1.55,
              margin: 0,
              padding: 10,
              background: "var(--bg-secondary)",
              border: "1px solid var(--border-color)",
              borderRadius: "var(--radius-xs-plus)",
            }}
          >
            {event.description}
          </pre>
        )}
        <div
          style={{
            marginTop: 14,
            fontSize: "calc(var(--font-size-sm) - 1px)",
            color: "var(--text-secondary)",
          }}
        >
          Status: {event.status}
        </div>
      </div>
    </div>
  );
}
