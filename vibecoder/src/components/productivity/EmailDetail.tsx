import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Archive,
  CheckCheck,
  Circle,
  Loader2,
  Mail,
  Reply,
  X,
} from "lucide-react";
import type { EmailBody } from "../../types/productivity";
import { EmailComposer } from "./EmailComposer";

interface Props {
  id: string;
  onClose: () => void;
  onArchived: (id: string) => void;
  onReadChanged: (id: string, read: boolean) => void;
  /** Provider from the toolbar dropdown — forwarded to the reply composer
   *  so AI Draft uses the user's selected model. */
  provider?: string;
}

const EMAIL_RE = /<([^>]+)>/;

function extractAddress(from: string): string {
  const m = from.match(EMAIL_RE);
  return m ? m[1] : from.trim();
}

function replySubject(subject: string): string {
  const s = subject.trim();
  return /^re:/i.test(s) ? s : `Re: ${s || "(no subject)"}`;
}

function quoteBody(body: EmailBody): string {
  const header = `\n\n\nOn ${body.date}, ${body.from} wrote:\n`;
  const text = body.body_text || "";
  const quoted = text
    .split("\n")
    .map((l) => `> ${l}`)
    .join("\n");
  return header + quoted;
}

export function EmailDetail({ id, onClose, onArchived, onReadChanged, provider }: Props) {
  const [body, setBody] = useState<EmailBody | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<"archive" | "mark" | null>(null);
  const [replying, setReplying] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setErr(null);
    setBody(null);
    invoke<EmailBody>("productivity_email_read", { id })
      .then((b) => {
        if (cancelled) return;
        setBody(b);
        // Auto-mark-read on open if currently unread
        if (!b.is_read) {
          invoke("productivity_email_mark_read", { id, read: true })
            .then(() => onReadChanged(id, true))
            .catch(() => {});
        }
      })
      .catch((e: unknown) => {
        if (!cancelled) setErr(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [id, onReadChanged]);

  async function archive() {
    setBusy("archive");
    try {
      await invoke("productivity_email_archive", { id });
      onArchived(id);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function toggleRead() {
    if (!body) return;
    const next = !body.is_read;
    setBusy("mark");
    try {
      await invoke("productivity_email_mark_read", { id, read: next });
      setBody({ ...body, is_read: next });
      onReadChanged(id, next);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(null);
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
          onClick={toggleRead}
          disabled={!body || busy !== null}
          title={body?.is_read ? "Mark as unread" : "Mark as read"}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {body?.is_read ? <Circle size={12} /> : <CheckCheck size={12} />}
          {body?.is_read ? "Unread" : "Read"}
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={archive}
          disabled={!body || busy !== null}
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          {busy === "archive" ? (
            <Loader2 size={12} style={{ animation: "spin 1s linear infinite" }} />
          ) : (
            <Archive size={12} />
          )}
          Archive
        </button>
        <button
          className="panel-btn panel-btn-secondary"
          onClick={() => setReplying(true)}
          disabled={!body}
          title="Reply"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <Reply size={12} />
          Reply
        </button>
        <span style={{ flex: 1 }} />
        <button
          className="panel-btn panel-btn-secondary"
          onClick={onClose}
          title="Close reader"
          style={{ display: "flex", alignItems: "center", gap: 4 }}
        >
          <X size={12} />
        </button>
      </div>

      {replying && body && (
        <EmailComposer
          onClose={() => setReplying(false)}
          replyToId={id}
          initialTo={extractAddress(body.from)}
          initialSubject={replySubject(body.subject)}
          initialBody={quoteBody(body)}
          provider={provider}
        />
      )}

      <div style={{ flex: 1, overflowY: "auto", padding: 14 }}>
        {loading && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              color: "var(--text-secondary)",
              fontSize: "var(--font-size-sm)",
            }}
          >
            <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
            Loading…
          </div>
        )}
        {err && (
          <div
            style={{
              color: "var(--color-error, #d63e3e)",
              fontSize: "var(--font-size-sm)",
              whiteSpace: "pre-wrap",
            }}
          >
            {err}
          </div>
        )}
        {body && (
          <>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                marginBottom: 12,
              }}
            >
              <Mail size={16} strokeWidth={1.75} />
              <h3 style={{ margin: 0, fontSize: "var(--font-size-lg, 15px)" }}>
                {body.subject || "(no subject)"}
              </h3>
            </div>
            <div
              style={{
                fontSize: "calc(var(--font-size-sm) - 1px)",
                color: "var(--text-secondary)",
                marginBottom: 14,
                display: "grid",
                gridTemplateColumns: "auto 1fr",
                gap: "2px 10px",
              }}
            >
              <span>From:</span>
              <span style={{ color: "var(--text-primary)" }}>{body.from}</span>
              {body.to && (
                <>
                  <span>To:</span>
                  <span>{body.to}</span>
                </>
              )}
              {body.cc && (
                <>
                  <span>Cc:</span>
                  <span>{body.cc}</span>
                </>
              )}
              {body.date && (
                <>
                  <span>Date:</span>
                  <span>{body.date}</span>
                </>
              )}
            </div>
            {body.body_html ? (
              <iframe
                title="email-body"
                sandbox=""
                srcDoc={body.body_html}
                style={{
                  width: "100%",
                  minHeight: 400,
                  border: "1px solid var(--border-color)",
                  borderRadius: "var(--radius-xs-plus)",
                  background: "#fff",
                }}
              />
            ) : (
              <pre
                style={{
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                  fontFamily: "inherit",
                  fontSize: "var(--font-size-sm)",
                  lineHeight: 1.55,
                  margin: 0,
                  color: "var(--text-primary)",
                }}
              >
                {body.body_text || "(no body)"}
              </pre>
            )}
          </>
        )}
      </div>
    </div>
  );
}
