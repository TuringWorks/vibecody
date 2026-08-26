import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { Plus, ArrowUp, GitBranch, Square, FileCode, Paperclip, X } from "lucide-react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useVoiceInput } from "@vibe/shared/voice/useVoiceInput";
import { VoiceButton } from "@vibe/shared/voice/VoiceButton";
import { useVoiceDuplex } from "@vibe/shared/voice/useVoiceDuplex";
import { DuplexVoiceButton } from "@vibe/shared/voice/DuplexVoiceButton";
import { tauriTranscriber } from "@vibe/shared/voice/transcribers";
import { ApprovalPill, type ApprovalTier } from "./ApprovalPill";
import { ProviderPill } from "./ProviderPill";
import { ReasoningPill, type ReasoningEffort } from "./ReasoningPill";
import { ModePill, type RunMode } from "./ModePill";
import { SandboxSettings } from "./SandboxSettings";
import { describe as describeSandbox, type SandboxPolicy } from "../lib/sandbox";
import { QuickActionDrawer, type QuickAction } from "./QuickActionDrawer";
import type { ComposerPrefs } from "../hooks/useComposerPrefs";
import { findMention, rankFiles, useProjectFiles } from "../hooks/useProjectFiles";
import { findSlash, matchSlash, type SlashAction } from "./slashCommands";
import { composeWithAttachments, formatBytes, type Attachment } from "../lib/attachments";

/** The composer's submit payload, bubbled up to SessionStream for orchestration. */
export interface ComposerSubmit {
  /** The full prompt sent to the agent — attachments prepended, if any. */
  task: string;
  /** Just what the user typed. The chat title and the ↑-history come from this:
   *  titling a chat after the first line of an attached file would name it
   *  "Attached file: /Users/…". */
  typed: string;
  provider: string;
  model?: string;
  approval: ApprovalTier;
  reasoning: ReasoningEffort;
  mode: RunMode;
  sandbox: SandboxPolicy;
  /** When true, this run gets its own git worktree branch for isolation. Off by
   *  default — a plain chat/question should not fork a branch. Opt in per-run
   *  via the composer's Branch toggle for isolated coding tasks. */
  isolate: boolean;
}

interface TaskPromptProps {
  daemonUrl: string;
  daemonOnline: boolean;
  /** True while a run is in flight — swaps Submit for Stop. */
  busy: boolean;
  /** Run controls, owned above the conversation pane so they survive a chat
   *  switch (which remounts this subtree). */
  prefs: ComposerPrefs;
  onPref: <K extends keyof ComposerPrefs>(key: K, value: ComposerPrefs[K]) => void;
  onProviderModel: (provider: string, model: string | undefined) => void;
  /** Draft text, lifted for the same reason — switching chats must not eat a
   *  half-typed message. Keyed per chat by the parent. */
  draft: string;
  onDraft: (text: string) => void;
  /** Files attached to the next message. Lifted for the same reason as the
   *  draft — switching chats must not silently discard them. */
  attachments: Attachment[];
  onAttachments: (next: Attachment[]) => void;
  /** Repo whose files back @-mention completion. */
  scopePath?: string;
  onSubmit: (payload: ComposerSubmit) => void;
  onStop: () => void;
  onQuickAction: (action: QuickAction) => void;
  /** Run a slash command. Handled by the shell — the composer only picks it. */
  onSlash: (action: SlashAction) => void;
}

/** Grow the textarea with its content, up to a scrollable ceiling. */
const MIN_ROWS_PX = 44;
const MAX_ROWS_PX = 260;

/**
 * VX-105 — the composer (Codex screenshots 1, 2, 7). Carries all run controls
 * inline: + quick-action drawer, approval pill, provider pill, reasoning pill,
 * submit/stop. This is the only primary input (P3: conversation is the
 * interface). Orchestration (create task → run agent → link session) lives in
 * the parent SessionStream; this component only gathers input and bubbles it up.
 * NOTE: there is intentionally NO Cmd+K inline edit — targeted edits use the
 * ⌘. diffcomplete surface (see pdm/08 §1).
 */
export function TaskPrompt({
  daemonUrl,
  daemonOnline,
  busy,
  prefs,
  onPref,
  onProviderModel,
  draft,
  onDraft,
  attachments,
  onAttachments,
  scopePath,
  onSubmit,
  onStop,
  onQuickAction,
  onSlash,
}: TaskPromptProps) {
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [sandboxOpen, setSandboxOpen] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  // Sent messages, newest last — recalled with ↑/↓ on an empty composer, the
  // way every shell and chat client behaves.
  const history = useRef<string[]>([]);
  const historyPos = useRef<number | null>(null);
  // @-mention completion over the project's tracked files: naming a file in a
  // prompt shouldn't mean retyping its path from memory.
  const projectFiles = useProjectFiles(daemonUrl, daemonOnline, scopePath);
  const [caret, setCaret] = useState(0);
  const [mentionCursor, setMentionCursor] = useState(0);

  const mention = useMemo(() => findMention(draft, caret), [draft, caret]);
  const mentionMatches = useMemo(
    () => (mention ? rankFiles(projectFiles, mention.query) : []),
    [mention, projectFiles]
  );
  const mentionOpen = !!mention && mentionMatches.length > 0;

  // Slash commands: only when the whole composer is one `/word`, so paths and
  // URLs inside a real prompt never pop the menu.
  const slash = findSlash(draft);
  const slashMatches = useMemo(
    () => (slash === null ? [] : matchSlash(slash, busy)),
    [slash, busy]
  );
  const slashOpen = slash !== null && slashMatches.length > 0;
  const [slashCursor, setSlashCursor] = useState(0);

  // A shrinking match list must not leave the highlight past the end.
  useEffect(() => setMentionCursor(0), [mention?.query]);
  useEffect(() => setSlashCursor(0), [slash]);

  /** Run the picked command and clear the composer. */
  function runSlash(action: SlashAction) {
    onDraft("");
    onSlash(action);
  }

  /** Replace the live `@…` token with the picked path and close the picker. */
  function applyMention(path: string) {
    if (!mention) return;
    const next = `${draft.slice(0, mention.start)}@${path} ${draft.slice(caret)}`;
    onDraft(next);
    const pos = mention.start + path.length + 2;
    setCaret(pos);
    requestAnimationFrame(() => {
      inputRef.current?.setSelectionRange(pos, pos);
      inputRef.current?.focus();
    });
  }

  const [attachError, setAttachError] = useState<string | null>(null);

  /** Native file picker → read each file → append as attachment chips. */
  async function pickAttachments() {
    setAttachError(null);
    try {
      const picked = await openDialog({ multiple: true, title: "Attach files" });
      const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
      if (paths.length === 0) return;
      const results = await Promise.allSettled(
        paths.map((path) => invoke<Attachment>("read_attachment", { path }))
      );
      const ok = results.flatMap((r) => (r.status === "fulfilled" ? [r.value] : []));
      // A binary or unreadable file is a normal mistake, not a failure of the
      // whole action — attach what worked and say what didn't.
      const failed = results.flatMap((r) => (r.status === "rejected" ? [String(r.reason)] : []));
      if (failed.length > 0) setAttachError(failed[0]);
      if (ok.length > 0) {
        const byPath = new Map(attachments.map((a) => [a.path, a]));
        for (const a of ok) byPath.set(a.path, a);
        onAttachments([...byPath.values()]);
      }
    } catch (e) {
      setAttachError(String(e));
    }
  }

  // Dictation appends to the draft rather than replacing it, so speaking a
  // second sentence — or speaking after typing — extends the prompt instead of
  // discarding what's already there. `draft` is owned by the parent, so read
  // the live value through a ref: Web Speech fires several final chunks per
  // utterance and a stale closure would make each one overwrite the last.
  const draftRef = useRef(draft);
  useEffect(() => {
    draftRef.current = draft;
  }, [draft]);

  const appendTranscript = useCallback(
    (text: string) => {
      const chunk = text.trim();
      if (!chunk) return;
      const prev = draftRef.current;
      const next = prev ? `${prev.replace(/\s+$/, "")} ${chunk}` : chunk;
      draftRef.current = next;
      onDraft(next);
    },
    [onDraft],
  );
  const transcribe = useMemo(() => tauriTranscriber(daemonUrl), [daemonUrl]);
  const voice = useVoiceInput({ onTranscript: appendTranscript, transcribe });

  // Full-duplex conversation, on the same provider/model the composer will use
  // for a typed task. Push-to-talk above stays: it dictates *into* the
  // composer, where this speaks a whole turn without touching the draft.
  const duplex = useVoiceDuplex({
    daemonUrl,
    provider: prefs.provider,
    model: prefs.model,
    language: "en",
  });

  const canSubmit = (!!draft.trim() || attachments.length > 0) && !busy && daemonOnline;

  // Auto-grow: a fixed 2-row box made anything longer than a sentence a
  // 2-line peephole, which is the most-hit edge of the composer.
  useLayoutEffect(() => {
    const el = inputRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(Math.max(el.scrollHeight, MIN_ROWS_PX), MAX_ROWS_PX)}px`;
  }, [draft]);

  // Focus the composer when a run ends, so the next follow-up can be typed
  // without reaching for the mouse.
  useEffect(() => {
    if (!busy) inputRef.current?.focus();
  }, [busy]);

  function submit() {
    if (!canSubmit) return;
    // Clicking the send arrow on a bare `/command` should run it, not send the
    // literal text to the model.
    const exact = slashMatches.find((c) => c.name === slash);
    if (exact) {
      runSlash(exact.id);
      return;
    }
    const typed = draft.trim();
    // Attachments lead the prompt; see composeWithAttachments.
    const task = composeWithAttachments(typed, attachments);
    history.current = [...history.current, typed];
    historyPos.current = null;
    onSubmit({ task, typed, ...prefs });
    onDraft("");
    onAttachments([]);
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    // The slash menu takes precedence: it only opens when the composer holds
    // nothing but the command, so there's no prompt text to protect.
    if (slashOpen) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSlashCursor((c) => Math.min(slashMatches.length - 1, c + 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSlashCursor((c) => Math.max(0, c - 1));
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        runSlash(slashMatches[slashCursor].id);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        onDraft("");
        return;
      }
    }
    // The mention picker owns the arrows / Enter / Tab / Esc while it's open,
    // so completing a path never submits the message by accident.
    if (mentionOpen) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setMentionCursor((c) => Math.min(mentionMatches.length - 1, c + 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setMentionCursor((c) => Math.max(0, c - 1));
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        applyMention(mentionMatches[mentionCursor]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        // Move the caret off the token so `findMention` stops matching.
        setCaret(-1);
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
      return;
    }
    // ↑ on an empty (or history-navigated) composer walks back through sent
    // messages; ↓ walks forward and back out to an empty box.
    const atStart = e.currentTarget.selectionStart === 0 && e.currentTarget.selectionEnd === 0;
    if (e.key === "ArrowUp" && (draft === "" || historyPos.current !== null) && atStart) {
      const items = history.current;
      if (items.length === 0) return;
      const next = historyPos.current === null ? items.length - 1 : Math.max(0, historyPos.current - 1);
      e.preventDefault();
      historyPos.current = next;
      onDraft(items[next]);
    } else if (e.key === "ArrowDown" && historyPos.current !== null) {
      const items = history.current;
      const next = historyPos.current + 1;
      e.preventDefault();
      if (next >= items.length) {
        historyPos.current = null;
        onDraft("");
      } else {
        historyPos.current = next;
        onDraft(items[next]);
      }
    }
  }

  return (
    <div className="vx-composer">
      {drawerOpen && (
        <QuickActionDrawer
          onAction={(a) => {
            setDrawerOpen(false);
            onQuickAction(a);
          }}
          onClose={() => setDrawerOpen(false)}
        />
      )}
      {slashOpen && (
        <ul className="vx-mention" role="listbox" aria-label="Slash commands">
          {slashMatches.map((c, i) => (
            <li key={c.id}>
              <button
                role="option"
                aria-selected={i === slashCursor}
                className={`vx-mention__item${i === slashCursor ? " is-active" : ""}`}
                onMouseEnter={() => setSlashCursor(i)}
                onMouseDown={(e) => {
                  e.preventDefault();
                  runSlash(c.id);
                }}
              >
                <span className="vx-slash__name">/{c.name}</span>
                <span className="vx-slash__hint">{c.hint}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      {mentionOpen && (
        <ul className="vx-mention" role="listbox" aria-label="Project files">
          {mentionMatches.map((f, i) => (
            <li key={f}>
              <button
                role="option"
                aria-selected={i === mentionCursor}
                className={`vx-mention__item${i === mentionCursor ? " is-active" : ""}`}
                onMouseEnter={() => setMentionCursor(i)}
                // mousedown, not click: the textarea must not lose focus first.
                onMouseDown={(e) => {
                  e.preventDefault();
                  applyMention(f);
                }}
              >
                <FileCode size={12} />
                <span className="vx-mention__path">{f}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      {(attachments.length > 0 || attachError) && (
        <div className="vx-attach">
          {attachments.map((a) => (
            <span key={a.path} className="vx-attach__chip" title={`${a.path} · ${formatBytes(a.bytes)}`}>
              <FileCode size={11} />
              <span className="vx-attach__name">{a.name}</span>
              <span className="vx-attach__size">
                {formatBytes(a.bytes)}
                {a.truncated ? " · truncated" : ""}
              </span>
              <button
                className="vx-attach__remove"
                aria-label={`Remove ${a.name}`}
                onClick={() => onAttachments(attachments.filter((x) => x.path !== a.path))}
              >
                <X size={11} />
              </button>
            </span>
          ))}
          {attachError && <span className="vx-attach__error">{attachError}</span>}
        </div>
      )}
      {voice.interimText && (
        <div className="vx-voice-interim" aria-live="polite">
          {voice.interimText}
        </div>
      )}
      {voice.error && (
        <div className="vx-voice-error" role="status">
          {voice.error}
        </div>
      )}
      <textarea
        ref={inputRef}
        className="vx-composer__input"
        placeholder={
          daemonOnline
            ? "Describe a task, or ask a question — @ for a file, / for commands"
            : "Waiting for the daemon…"
        }
        value={draft}
        rows={1}
        onChange={(e) => {
          historyPos.current = null;
          setCaret(e.target.selectionStart ?? e.target.value.length);
          onDraft(e.target.value);
        }}
        onSelect={(e) => setCaret(e.currentTarget.selectionStart ?? 0)}
        onKeyDown={onKeyDown}
      />
      {sandboxOpen && (
        <SandboxSettings
          value={prefs.sandbox}
          onChange={(next) => onPref("sandbox", next)}
          onClose={() => setSandboxOpen(false)}
        />
      )}
      <div className="vx-composer__bar">
        <button
          className="vx-icon-btn"
          aria-label="Quick actions"
          title="Quick actions"
          onClick={() => setDrawerOpen((v) => !v)}
        >
          <Plus size={16} />
        </button>
        <button
          className="vx-icon-btn"
          aria-label="Attach files"
          title="Attach files"
          onClick={pickAttachments}
        >
          <Paperclip size={15} />
        </button>
        <VoiceButton voice={voice} disabled={busy} />
        <DuplexVoiceButton
          state={duplex.state}
          active={duplex.active}
          supported={duplex.supported && daemonOnline}
          onStart={duplex.start}
          onStop={duplex.stop}
          unsupportedHint={
            daemonOnline ? "This webview cannot capture audio" : "The daemon is offline"
          }
        />
        <ApprovalPill value={prefs.approval} onChange={(v) => onPref("approval", v)} />
        <button
          type="button"
          className={`vx-pill vx-pill--branch${prefs.isolate ? " vx-pill--branch-on" : ""}`}
          aria-pressed={prefs.isolate}
          title={
            prefs.isolate
              ? "This run will get its own git worktree branch"
              : "Run in place (no branch). Click to isolate this run in a git worktree branch."
          }
          onClick={() => onPref("isolate", !prefs.isolate)}
        >
          <GitBranch size={13} />
          <span>Branch: {prefs.isolate ? "on" : "off"}</span>
        </button>
        <div className="vx-composer__spacer" />
        <ProviderPill
          daemonUrl={daemonUrl}
          daemonOnline={daemonOnline}
          provider={prefs.provider}
          model={prefs.model}
          onSelect={onProviderModel}
        />
        <ModePill value={prefs.mode} onChange={(v) => onPref("mode", v)} />
        {prefs.mode === "sandbox" && (
          <button
            className="vx-pill"
            onClick={() => setSandboxOpen(true)}
            title={`Sandbox access: ${describeSandbox(prefs.sandbox)}`}
          >
            Access: {describeSandbox(prefs.sandbox)}
          </button>
        )}
        <ReasoningPill
          provider={prefs.provider}
          value={prefs.reasoning}
          onChange={(v) => onPref("reasoning", v)}
        />
        {busy ? (
          <button
            className="vx-composer__submit vx-composer__submit--stop"
            aria-label="Stop the running task"
            title="Stop"
            onClick={onStop}
          >
            <Square size={13} fill="currentColor" />
          </button>
        ) : (
          <button
            className="vx-composer__submit"
            aria-label="Submit task"
            title="Send (Enter)"
            disabled={!canSubmit}
            onClick={submit}
          >
            <ArrowUp size={16} />
          </button>
        )}
      </div>
    </div>
  );
}
