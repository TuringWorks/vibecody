import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Send, Sparkles } from "lucide-react";
import { ComposeModal } from "./ComposeModal";
import { PROVIDER_DEFAULT_MODEL } from "../../hooks/useModelRegistry";

interface DraftReplyResult {
  draft: string;
  provider: string;
  model: string;
  duration_ms: number;
}

interface Props {
  onClose: () => void;
  onSent?: (id: string) => void;
  initialTo?: string;
  initialSubject?: string;
  initialBody?: string;
  /** When set, enables the "AI draft" button that calls productivity_draft_reply. */
  replyToId?: string;
  /** Provider from the toolbar dropdown — required for the AI-draft path.
   *  When unset, the AI Draft button is disabled with a hint. */
  provider?: string;
}

export function EmailComposer({
  onClose,
  onSent,
  initialTo = "",
  initialSubject = "",
  initialBody = "",
  replyToId,
  provider,
}: Props) {
  const [to, setTo] = useState(initialTo);
  const [subject, setSubject] = useState(initialSubject);
  const [body, setBody] = useState(initialBody);
  const [sending, setSending] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [showDraftRow, setShowDraftRow] = useState(false);
  const [instructions, setInstructions] = useState("");
  const [drafting, setDrafting] = useState(false);
  const [draftMeta, setDraftMeta] = useState<string | null>(null);

  const draftModel = provider ? PROVIDER_DEFAULT_MODEL[provider] : undefined;
  const canDraft = !!replyToId && !!provider && !!draftModel;

  async function aiDraft() {
    if (!replyToId) return;
    if (!provider || !draftModel) {
      setErr("Pick a provider/model from the toolbar dropdown first.");
      return;
    }
    setDrafting(true);
    setErr(null);
    try {
      const r = await invoke<DraftReplyResult>("productivity_draft_reply", {
        emailId: replyToId,
        instructions: instructions.trim() || null,
        provider,
        model: draftModel,
      });
      const quoted = initialBody.trim() ? `\n\n${initialBody}` : "";
      setBody(`${r.draft}${quoted}`);
      setDraftMeta(`${r.provider} · ${r.model} · ${(r.duration_ms / 1000).toFixed(1)}s`);
    } catch (e) {
      setErr(String(e));
    } finally {
      setDrafting(false);
    }
  }

  async function send() {
    if (!to.trim() || !subject.trim()) return;
    setSending(true);
    setErr(null);
    try {
      const id = await invoke<string>("productivity_email_send", {
        to: to.trim(),
        subject: subject.trim(),
        body,
      });
      onSent?.(id);
      onClose();
    } catch (e) {
      setErr(String(e));
    } finally {
      setSending(false);
    }
  }

  return (
    <ComposeModal
      title="Compose email"
      onClose={onClose}
      width={600}
      footer={
        <>
          <button
            className="panel-btn panel-btn-secondary"
            onClick={onClose}
            disabled={sending}
          >
            Cancel
          </button>
          <button
            className="panel-btn panel-btn-primary"
            onClick={send}
            disabled={sending || !to.trim() || !subject.trim()}
            style={{ display: "flex", alignItems: "center", gap: 5 }}
          >
            <Send size={12} />
            {sending ? "Sending…" : "Send"}
          </button>
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <label style={{ display: "flex", flexDirection: "column", gap: 3 }}>
          <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
            To
          </span>
          <input
            className="panel-input"
            type="email"
            placeholder="name@example.com"
            value={to}
            onChange={(e) => setTo(e.target.value)}
            disabled={sending}
            autoFocus
          />
        </label>
        <label style={{ display: "flex", flexDirection: "column", gap: 3 }}>
          <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
            Subject
          </span>
          <input
            className="panel-input"
            placeholder="Subject"
            value={subject}
            onChange={(e) => setSubject(e.target.value)}
            disabled={sending}
          />
        </label>
        <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", flex: 1 }}>
              Body
            </span>
            {replyToId && (
              <button
                className={`panel-btn panel-btn-secondary${showDraftRow ? " active" : ""}`}
                onClick={() => setShowDraftRow((s) => !s)}
                disabled={sending || drafting}
                title="Draft a reply with AI"
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 4,
                  fontSize: "calc(var(--font-size-sm) - 1px)",
                }}
              >
                <Sparkles size={11} />
                AI draft
              </button>
            )}
          </div>
          {showDraftRow && replyToId && (
            <div
              style={{
                display: "flex",
                gap: 6,
                padding: 6,
                background: "var(--bg-secondary)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-xs-plus)",
              }}
            >
              <input
                className="panel-input"
                style={{ flex: 1 }}
                placeholder="Optional tone/intent (e.g. 'accept, suggest Thursday 2pm')"
                value={instructions}
                onChange={(e) => setInstructions(e.target.value)}
                disabled={drafting}
                onKeyDown={(e) => {
                  if (e.key === "Enter") aiDraft();
                }}
              />
              <button
                className="panel-btn panel-btn-primary"
                onClick={aiDraft}
                disabled={drafting || !canDraft}
                title={
                  canDraft
                    ? `Draft using ${provider} · ${draftModel}`
                    : "Pick a provider/model from the toolbar dropdown first"
                }
                style={{ display: "flex", alignItems: "center", gap: 4 }}
              >
                {drafting ? (
                  <Loader2 size={11} style={{ animation: "spin 1s linear infinite" }} />
                ) : (
                  <Sparkles size={11} />
                )}
                {drafting ? "Drafting…" : "Generate"}
              </button>
            </div>
          )}
          {draftMeta && (
            <span
              style={{
                color: "var(--text-secondary)",
                fontSize: "calc(var(--font-size-sm) - 2px)",
              }}
            >
              Drafted by {draftMeta} — edit as needed before sending
            </span>
          )}
          <textarea
            className="panel-input"
            rows={12}
            placeholder="Write your message…"
            value={body}
            onChange={(e) => setBody(e.target.value)}
            disabled={sending}
            style={{ resize: "vertical", fontFamily: "inherit" }}
          />
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
