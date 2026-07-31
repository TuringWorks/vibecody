import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Plus, ArrowUp, GitBranch, Square } from "lucide-react";
import { ApprovalPill, type ApprovalTier } from "./ApprovalPill";
import { ProviderPill } from "./ProviderPill";
import { ReasoningPill, type ReasoningEffort } from "./ReasoningPill";
import { QuickActionDrawer, type QuickAction } from "./QuickActionDrawer";
import type { ComposerPrefs } from "../hooks/useComposerPrefs";

/** The composer's submit payload, bubbled up to SessionStream for orchestration. */
export interface ComposerSubmit {
  task: string;
  provider: string;
  model?: string;
  approval: ApprovalTier;
  reasoning: ReasoningEffort;
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
  onSubmit: (payload: ComposerSubmit) => void;
  onStop: () => void;
  onQuickAction: (action: QuickAction) => void;
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
  onSubmit,
  onStop,
  onQuickAction,
}: TaskPromptProps) {
  const [drawerOpen, setDrawerOpen] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  // Sent messages, newest last — recalled with ↑/↓ on an empty composer, the
  // way every shell and chat client behaves.
  const history = useRef<string[]>([]);
  const historyPos = useRef<number | null>(null);

  const canSubmit = !!draft.trim() && !busy && daemonOnline;

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
    const task = draft.trim();
    history.current = [...history.current, task];
    historyPos.current = null;
    onSubmit({ task, ...prefs });
    onDraft("");
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
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
      <textarea
        ref={inputRef}
        className="vx-composer__input"
        placeholder={daemonOnline ? "Describe a task, or ask a question" : "Waiting for the daemon…"}
        value={draft}
        rows={1}
        onChange={(e) => {
          historyPos.current = null;
          onDraft(e.target.value);
        }}
        onKeyDown={onKeyDown}
      />
      <div className="vx-composer__bar">
        <button
          className="vx-icon-btn"
          aria-label="Quick actions"
          title="Quick actions"
          onClick={() => setDrawerOpen((v) => !v)}
        >
          <Plus size={16} />
        </button>
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
