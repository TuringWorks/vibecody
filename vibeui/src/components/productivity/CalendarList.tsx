import { Calendar as CalIcon, MapPin, Users } from "lucide-react";
import type { CalendarEvent } from "../../types/productivity";

interface Props {
  events: CalendarEvent[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function formatTime(iso: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso.slice(11, 16);
  return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

function dateKey(iso: string): string {
  if (!iso) return "unknown";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso.slice(0, 10);
  return d.toDateString();
}

function statusColor(status: string): string {
  if (status === "cancelled") return "var(--text-secondary)";
  if (status === "tentative") return "var(--color-warn, #c69023)";
  return "var(--text-primary)";
}

export function CalendarList({ events, selectedId, onSelect }: Props) {
  if (events.length === 0) {
    return (
      <div
        style={{
          padding: 20,
          color: "var(--text-secondary)",
          textAlign: "center",
          fontSize: "var(--font-size-sm)",
        }}
      >
        No events in this range.
      </div>
    );
  }

  const grouped = new Map<string, CalendarEvent[]>();
  for (const e of events) {
    const key = dateKey(e.start);
    const arr = grouped.get(key) ?? [];
    arr.push(e);
    grouped.set(key, arr);
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", overflowY: "auto", flex: 1 }}>
      {Array.from(grouped.entries()).map(([day, items]) => (
        <div key={day}>
          <div
            style={{
              padding: "6px 10px",
              fontSize: "calc(var(--font-size-sm) - 1px)",
              color: "var(--text-secondary)",
              background: "var(--bg-secondary)",
              borderBottom: "1px solid var(--border-color)",
              position: "sticky",
              top: 0,
            }}
          >
            {day}
          </div>
          {items.map((e) => {
            const active = e.id === selectedId;
            const cancelled = e.status === "cancelled";
            return (
              <button
                key={e.id}
                onClick={() => onSelect(e.id)}
                className="panel-card panel-card--clickable"
                style={{
                  display: "grid",
                  gridTemplateColumns: "80px 1fr",
                  alignItems: "baseline",
                  gap: 10,
                  padding: "8px 10px",
                  border: "none",
                  borderBottom: "1px solid var(--border-color)",
                  borderLeft: active
                    ? "2px solid var(--accent-color, var(--text-primary))"
                    : "2px solid transparent",
                  background: active ? "var(--bg-hover, var(--bg-secondary))" : "transparent",
                  textAlign: "left",
                  cursor: "pointer",
                  width: "100%",
                  fontFamily: "inherit",
                  color: "inherit",
                  fontSize: "var(--font-size-sm)",
                  textDecoration: cancelled ? "line-through" : "none",
                  opacity: cancelled ? 0.6 : 1,
                }}
              >
                <span
                  style={{
                    color: "var(--text-secondary)",
                    fontVariantNumeric: "tabular-nums",
                    fontSize: "calc(var(--font-size-sm) - 1px)",
                  }}
                >
                  {formatTime(e.start)} – {formatTime(e.end)}
                </span>
                <span style={{ display: "flex", flexDirection: "column", gap: 2, overflow: "hidden" }}>
                  <span
                    style={{
                      fontWeight: 500,
                      color: statusColor(e.status),
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    <CalIcon
                      size={11}
                      style={{ display: "inline", verticalAlign: "-1px", marginRight: 5 }}
                    />
                    {e.summary || "(untitled)"}
                  </span>
                  {(e.location || e.attendees.length > 0) && (
                    <span
                      style={{
                        display: "flex",
                        gap: 10,
                        color: "var(--text-secondary)",
                        fontSize: "calc(var(--font-size-sm) - 1px)",
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                      }}
                    >
                      {e.location && (
                        <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
                          <MapPin size={10} />
                          {e.location}
                        </span>
                      )}
                      {e.attendees.length > 0 && (
                        <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
                          <Users size={10} />
                          {e.attendees.length}
                        </span>
                      )}
                    </span>
                  )}
                </span>
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}
