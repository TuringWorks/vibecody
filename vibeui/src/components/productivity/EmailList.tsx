import { Mail, MailOpen, Tag } from "lucide-react";
import type { Email } from "../../types/productivity";

interface Props {
  emails: Email[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function formatDate(raw: string): string {
  if (!raw) return "";
  const d = new Date(raw);
  if (isNaN(d.getTime())) return raw.slice(0, 16);
  const now = new Date();
  const sameDay =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();
  return sameDay
    ? d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })
    : d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function senderName(from: string): string {
  const match = from.match(/^"?([^"<]+?)"?\s*<.+>$/);
  return (match ? match[1] : from).trim();
}

export function EmailList({ emails, selectedId, onSelect }: Props) {
  if (emails.length === 0) {
    return (
      <div
        style={{
          padding: 20,
          color: "var(--text-secondary)",
          textAlign: "center",
          fontSize: "var(--font-size-sm)",
        }}
      >
        No messages.
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", overflowY: "auto", flex: 1 }}>
      {emails.map((e) => {
        const active = e.id === selectedId;
        const Icon = e.is_read ? MailOpen : Mail;
        const interesting = e.labels.filter(
          (l) => l !== "INBOX" && l !== "UNREAD" && !l.startsWith("CATEGORY_"),
        );
        return (
          <button
            key={e.id}
            onClick={() => onSelect(e.id)}
            className="panel-card panel-card--clickable"
            style={{
              display: "grid",
              gridTemplateColumns: "20px 1fr auto",
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
              fontFamily: "inherit",
              color: "inherit",
              fontSize: "var(--font-size-sm)",
              width: "100%",
            }}
          >
            <Icon
              size={13}
              strokeWidth={e.is_read ? 1.5 : 2}
              color="currentColor"
              style={{ flexShrink: 0, opacity: e.is_read ? 0.5 : 1 }}
            />
            <span style={{ overflow: "hidden", display: "flex", flexDirection: "column", gap: 2 }}>
              <span
                style={{
                  fontWeight: e.is_read ? 400 : 600,
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {senderName(e.from) || "(no sender)"}
              </span>
              <span
                style={{
                  color: e.is_read ? "var(--text-secondary)" : "var(--text-primary)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {e.subject || "(no subject)"}
              </span>
              {e.snippet && (
                <span
                  style={{
                    color: "var(--text-secondary)",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    fontSize: "calc(var(--font-size-sm) - 1px)",
                  }}
                >
                  {e.snippet}
                </span>
              )}
              {interesting.length > 0 && (
                <span style={{ display: "flex", gap: 4, alignItems: "center", flexWrap: "wrap" }}>
                  {interesting.slice(0, 3).map((l) => (
                    <span
                      key={l}
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 2,
                        padding: "1px 5px",
                        borderRadius: "var(--radius-xs-plus)",
                        background: "var(--bg-secondary)",
                        color: "var(--text-secondary)",
                        fontSize: "calc(var(--font-size-sm) - 2px)",
                      }}
                    >
                      <Tag size={8} />
                      {l}
                    </span>
                  ))}
                </span>
              )}
            </span>
            <span
              style={{
                color: "var(--text-secondary)",
                fontSize: "calc(var(--font-size-sm) - 1px)",
                whiteSpace: "nowrap",
              }}
            >
              {formatDate(e.date)}
            </span>
          </button>
        );
      })}
    </div>
  );
}
