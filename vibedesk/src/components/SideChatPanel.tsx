import { useCallback, useLayoutEffect, useMemo, useRef, useState } from "react";
import { toWire } from "../lib/sandbox";
import { effortParam } from "../lib/effort";
import { ArrowUp, Square, Trash2 } from "lucide-react";
import { Markdown } from "@vibe/shared/markdown/Markdown";
import { useVoiceInput } from "@vibe/shared/voice/useVoiceInput";
import { VoiceButton } from "@vibe/shared/voice/VoiceButton";
import { tauriTranscriber } from "@vibe/shared/voice/transcribers";
import { ToolUseBlock } from "./ToolUseBlock";
import { useAgentStream } from "../hooks/useAgentStream";
import type { ComposerPrefs } from "../hooks/useComposerPrefs";

interface SideChatPanelProps {
  daemonUrl: string;
  daemonOnline: boolean;
  /** Directory the side chat runs in — the same scope as the main pane. */
  workspaceRoot?: string;
  /** Provider/model/approval/reasoning, shared with the main composer. */
  prefs: ComposerPrefs;
}

/**
 * The `+` drawer's Side chat action: a throwaway conversation that runs against
 * the daemon *without* creating a task card.
 *
 * The main pane is task-shaped — every message creates or continues a task with
 * a lifecycle, a status dot in the rail, and possibly a worktree branch. That's
 * right for work and wrong for "what does this function do?", which was
 * previously either polluting the task list or not worth asking. This is the
 * escape hatch: same provider and scope, no bookkeeping.
 *
 * Ephemeral by construction — it is never linked to a task, so it isn't in
 * `/api/tasks`, isn't restored on relaunch, and Clear simply drops it. The
 * daemon still keeps its own durable event log for the session, as it does for
 * every run; nothing is hidden from it, it just isn't surfaced as a chat.
 */
export function SideChatPanel({
  daemonUrl,
  daemonOnline,
  workspaceRoot,
  prefs,
}: SideChatPanelProps) {
  const { items, state, runTask, stop, loadItems } = useAgentStream();
  const [text, setText] = useState("");
  const bodyRef = useRef<HTMLDivElement>(null);
  // Follow-ups continue the same session, so the side chat has memory of
  // itself without ever becoming a task.
  const sessionRef = useRef<string | null>(null);

  const busy = state === "running";
  const canSend = !!text.trim() && !busy && daemonOnline;

  useLayoutEffect(() => {
    const el = bodyRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [items, state]);

  const send = useCallback(async () => {
    if (!canSend) return;
    const task = text.trim();
    setText("");
    const sid = await runTask({
      daemonUrl,
      task,
      provider: prefs.provider,
      model: prefs.model,
      approval: prefs.approval,
      reasoning: effortParam(prefs.reasoning),
      mode: prefs.mode,
      sandbox: prefs.mode === "sandbox" ? toWire(prefs.sandbox) : undefined,
      resumeSessionId: sessionRef.current ?? undefined,
      workspaceRoot,
    });
    if (sid) sessionRef.current = sid;
  }, [canSend, text, runTask, daemonUrl, prefs, workspaceRoot]);

  function clear() {
    sessionRef.current = null;
    loadItems([]);
  }

  // Dictated text extends the draft; the functional update keeps successive
  // Web Speech chunks from overwriting each other.
  const appendTranscript = useCallback((chunk: string) => {
    const trimmed = chunk.trim();
    if (!trimmed) return;
    setText((prev) => (prev ? `${prev.replace(/\s+$/, "")} ${trimmed}` : trimmed));
  }, []);
  const transcribe = useMemo(() => tauriTranscriber(daemonUrl), [daemonUrl]);
  const voice = useVoiceInput({ onTranscript: appendTranscript, transcribe });

  return (
    <div className="vx-side">
      <div className="vx-side__body" ref={bodyRef}>
        {items.length === 0 && (
          <div className="vx-side__empty">
            A quick question, kept out of your task list. Runs with the same model and
            project as the main conversation, but never creates a task.
          </div>
        )}
        {items.map((item, i) => {
          switch (item.kind) {
            case "user":
              return (
                <div key={i} className="vx-side__msg vx-side__msg--user">
                  {item.text}
                </div>
              );
            case "agent":
              return (
                <div key={i} className="vx-side__msg vx-side__msg--agent">
                  <Markdown text={item.text} />
                </div>
              );
            case "tool":
              return <ToolUseBlock key={i} tool={item.tool} summary={item.summary} />;
            case "system":
              return (
                <div key={i} className="vx-side__msg vx-side__msg--system">
                  {item.text}
                </div>
              );
            case "error":
              return (
                <div key={i} className="vx-side__msg vx-side__msg--error">
                  {item.text}
                </div>
              );
          }
        })}
        {busy && <div className="vx-stream__typing">●●●</div>}
      </div>

      <div className="vx-side__composer">
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
          className="vx-side__input"
          placeholder={daemonOnline ? "Ask something…" : "Waiting for the daemon…"}
          value={text}
          rows={2}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send();
            }
          }}
        />
        <div className="vx-side__bar">
          <button
            className="vx-side__clear"
            onClick={clear}
            disabled={items.length === 0 || busy}
            title="Clear this side chat"
            aria-label="Clear side chat"
          >
            <Trash2 size={13} />
          </button>
          <VoiceButton voice={voice} disabled={busy} size={13} />
          <span className="vx-side__model" title={`${prefs.provider} · ${prefs.model ?? "default"}`}>
            {prefs.model ?? prefs.provider}
          </span>
          {busy ? (
            <button
              className="vx-composer__submit vx-composer__submit--stop"
              onClick={() => stop(daemonUrl)}
              aria-label="Stop"
              title="Stop"
            >
              <Square size={12} fill="currentColor" />
            </button>
          ) : (
            <button
              className="vx-composer__submit"
              onClick={send}
              disabled={!canSend}
              aria-label="Send"
              title="Send (Enter)"
            >
              <ArrowUp size={14} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
