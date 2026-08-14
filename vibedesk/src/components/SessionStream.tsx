import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { toWire } from "../lib/sandbox";
import { ArrowDown, RotateCcw } from "lucide-react";
import { TaskPrompt, type ComposerSubmit } from "./TaskPrompt";
import { ToolUseBlock } from "./ToolUseBlock";
import { CopyButton, Markdown } from "@vibe/shared/markdown/Markdown";
import { ReasoningBlock } from "./ReasoningBlock";
import { splitThinking } from "@vibe/shared/lib/thinking";
import { effortParam } from "../lib/effort";
import { ApprovalPrompt } from "./ApprovalPrompt";
import { useAgentStream, eventsToStreamItems } from "../hooks/useAgentStream";
import { useApprovals } from "../hooks/useApprovals";
import { formatCost, formatTokens, useJobUsage } from "../hooks/useJobUsage";
import type { Task, TaskHistory } from "../hooks/useTasks";
import type { ComposerPrefs } from "../hooks/useComposerPrefs";
import type { Attachment } from "../lib/attachments";
import type { QuickAction } from "./QuickActionDrawer";
import { helpText, type SlashAction } from "./slashCommands";

interface SessionStreamProps {
  daemonUrl: string;
  daemonOnline: boolean;
  /** Project the next task is scoped to (null → daemon workspace default). */
  projectPath: string | null;
  /** When set, this chat is loaded into the pane and follow-ups resume its
   *  session instead of starting a fresh one. */
  selectedTask: Task | null;
  prefs: ComposerPrefs;
  onPref: <K extends keyof ComposerPrefs>(key: K, value: ComposerPrefs[K]) => void;
  onProviderModel: (provider: string, model: string | undefined) => void;
  draft: string;
  onDraft: (text: string) => void;
  attachments: Attachment[];
  onAttachments: (next: Attachment[]) => void;
  createTask: (
    title: string,
    provider: string,
    model?: string,
    projectPath?: string,
    createWorktree?: boolean,
  ) => Promise<Task>;
  linkSession: (id: string, sessionId: string, status?: string) => Promise<void>;
  getHistory: (id: string) => Promise<TaskHistory>;
  onQuickAction: (action: QuickAction) => void;
  /** Slash command picked in the composer; the shell owns what they do. */
  onSlash: (action: SlashAction) => void;
  /** Fired when a run reaches a terminal state, so the parent can refresh env. */
  onRunFinished: () => void;
}

/** A chat title is a one-liner — the prompt that started it, clipped. */
function titleFrom(task: string): string {
  const firstLine = task.split("\n").find((l) => l.trim()) ?? task;
  const trimmed = firstLine.trim();
  return trimmed.length > 72 ? `${trimmed.slice(0, 71)}…` : trimmed;
}

/** `1m 04s` / `12s` — the elapsed readout next to a running turn. */
function formatElapsed(ms: number): string {
  const total = Math.floor(ms / 1000);
  const mins = Math.floor(total / 60);
  const secs = total % 60;
  return mins > 0 ? `${mins}m ${String(secs).padStart(2, "0")}s` : `${secs}s`;
}

/**
 * VX-103 + VX-105b — center linear conversation (Codex screenshots 1, 8).
 * User messages are right-aligned chips; agent output is left-aligned markdown
 * streamed live from the daemon via `useAgentStream`. The composer (TaskPrompt)
 * is pinned at the bottom; this component orchestrates the full submit flow:
 * create task (+worktree) → run agent → link session → reflect lifecycle.
 */
export function SessionStream({
  daemonUrl,
  daemonOnline,
  projectPath,
  selectedTask,
  prefs,
  onPref,
  onProviderModel,
  draft,
  onDraft,
  attachments,
  onAttachments,
  createTask,
  linkSession,
  getHistory,
  onQuickAction,
  onSlash,
  onRunFinished,
}: SessionStreamProps) {
  const { items, state, sessionId, startedAt, runTask, attach, stop, loadItems, appendSystem } =
    useAgentStream();
  const usage = useJobUsage(daemonUrl, sessionId, state === "running");
  const approvals = useApprovals(daemonUrl, daemonOnline);
  const [title, setTitle] = useState<string>(selectedTask?.title ?? "New chat");
  const [elapsed, setElapsed] = useState(0);
  // The task whose run is currently streaming — used to PATCH its lifecycle
  // status when the run reaches a terminal state (VX-201).
  const activeTaskId = useRef<string | null>(selectedTask?.id ?? null);
  // When resuming a selected chat, the session id to continue.
  const resumeSessionId = useRef<string | null>(null);
  // Last submitted payload, so a failed turn can be retried verbatim.
  const lastSubmit = useRef<ComposerSubmit | null>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  // Sticky-bottom: follow the stream while the user is at the bottom, and stop
  // following the moment they scroll up to read something.
  const [stuck, setStuck] = useState(true);

  // The directory a run executes in: a task's own worktree when it has one,
  // otherwise the selected project. Without this the daemon ran every task in
  // its own cwd, so switching projects changed nothing about what the agent saw.
  const runRoot = selectedTask?.worktree_path || selectedTask?.project_path || projectPath || undefined;

  // Load the selected chat. A chat whose run is still in flight is re-attached
  // to its live stream (replaying the durable log first) rather than rendered
  // as a static transcript — otherwise leaving and returning to a running chat
  // showed a frozen pane and a status that never advanced.
  useEffect(() => {
    const task = selectedTask;
    if (!task?.session_id) return;
    resumeSessionId.current = task.session_id;
    let alive = true;
    (async () => {
      // A chat marked "running" gets re-attached to its live stream. That can
      // still fail — the daemon drops the stream when a run ends, and a task
      // can be stale-marked (app closed mid-run, daemon restarted) — so fall
      // through to the static transcript rather than showing an error.
      if (task.status === "running") {
        const attached = await attach(daemonUrl, task.session_id).catch(() => false);
        if (attached || !alive) return;
      }
      try {
        const hist = await getHistory(task.id);
        if (alive) loadItems(eventsToStreamItems(hist.events), task.session_id);
      } catch (e) {
        console.error("load chat history failed", e);
      }
    })();
    return () => {
      alive = false;
    };
    // Only on (re)mount for this selected task — the pane is keyed per chat.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // VX-201: reflect the run's terminal state onto the task card. A completed
  // run moves the task to "reviewing" (user reviews the diff); an error moves
  // it to "failed"; a user-stopped run returns to "reviewing" with whatever
  // work landed. Mirrors the VibeDesk state machine in pdm/08 §7.
  //
  // A `partial` run is not "reviewing" either: the agent stopped with planned
  // work outstanding, so the card must not present it as a finished change
  // awaiting review. It carries its own status so the board shows the task
  // still needs work.
  useEffect(() => {
    const id = activeTaskId.current;
    if (!id || state === "idle" || state === "running") return;
    // Claim the task before doing anything else. This effect depends on the
    // parent's `onRunFinished`, and calling it re-renders the parent — which
    // mints a new callback identity and re-runs this effect. Without releasing
    // the id here that cycle never terminates, PATCHing the task every pass.
    activeTaskId.current = null;
    const status = state === "error" ? "failed" : state === "partial" ? "partial" : "reviewing";
    linkSession(id, "", status).catch((e) => console.error("status update failed", e));
    onRunFinished();
  }, [state, linkSession, onRunFinished]);

  // Elapsed-time readout while a turn is in flight. Codex and Claude Desktop
  // both show one; without it a long tool loop is indistinguishable from a hang.
  useEffect(() => {
    if (state !== "running" || startedAt == null) return;
    setElapsed(Date.now() - startedAt);
    const id = setInterval(() => setElapsed(Date.now() - startedAt), 1000);
    return () => clearInterval(id);
  }, [state, startedAt]);

  // Follow the tail of the stream, but only while the user hasn't scrolled up.
  useLayoutEffect(() => {
    if (!stuck) return;
    const el = bodyRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [items, state, stuck]);

  const onScroll = useCallback(() => {
    const el = bodyRef.current;
    if (!el) return;
    // 24px of slack so a near-bottom position still counts as "following".
    setStuck(el.scrollHeight - el.scrollTop - el.clientHeight < 24);
  }, []);

  const jumpToLatest = useCallback(() => {
    const el = bodyRef.current;
    if (el) el.scrollTop = el.scrollHeight;
    setStuck(true);
  }, []);

  const submit = useCallback(
    async (p: ComposerSubmit) => {
      lastSubmit.current = p;
      setStuck(true);

      // Resume path — continue the selected chat's session instead of creating
      // a fresh task. The new turn + reply append to the loaded history.
      if (resumeSessionId.current && selectedTask) {
        activeTaskId.current = selectedTask.id;
        try {
          await linkSession(selectedTask.id, "", "running");
        } catch (e) {
          console.error("status update failed", e);
        }
        await runTask({
          daemonUrl,
          task: p.task,
          provider: p.provider,
          model: p.model,
          approval: p.approval,
          reasoning: effortParam(p.reasoning),
          mode: p.mode,
          sandbox: p.mode === "sandbox" ? toWire(p.sandbox) : undefined,
          resumeSessionId: resumeSessionId.current,
          workspaceRoot: runRoot,
        });
        return;
      }

      // First message becomes the chat title.
      if (items.length === 0) setTitle(titleFrom(p.typed || p.task));

      // VX-112/113: create the task card before the agent starts. A worktree
      // branch is only forked when the user opted in via the composer's Branch
      // toggle (p.isolate) — a plain chat stays in place (no branch).
      let task: Task | null = null;
      try {
        task = await createTask(titleFrom(p.typed || p.task), p.provider, p.model, projectPath ?? undefined, p.isolate);
        activeTaskId.current = task.id;
      } catch (e) {
        // A task card is bookkeeping, not the run itself — surface the failure
        // but still run, rather than silently dropping the user's request.
        console.error("create task failed", e);
      }

      // A fresh isolated task runs inside the worktree the daemon just forked.
      const sessionId = await runTask({
        daemonUrl,
        task: p.task,
        provider: p.provider,
        model: p.model,
        approval: p.approval,
        reasoning: effortParam(p.reasoning),
        mode: p.mode,
          sandbox: p.mode === "sandbox" ? toWire(p.sandbox) : undefined,
        workspaceRoot: task?.worktree_path || runRoot,
      });

      // VX-201: link the run's session and reflect lifecycle on the task. The
      // chat is now resumable, so follow-ups continue this session.
      if (sessionId) resumeSessionId.current = sessionId;
      if (task && sessionId) {
        try {
          await linkSession(task.id, sessionId, "running");
        } catch (e) {
          console.error("link session failed", e);
        }
      }
    },
    [createTask, daemonUrl, items.length, linkSession, projectPath, runRoot, runTask, selectedTask]
  );

  const statusLabel =
    state === "running"
      ? `running · ${formatElapsed(elapsed)}`
      : state === "error"
        ? "failed"
        : state === "cancelled"
          ? "stopped"
          : state === "partial"
            ? "incomplete"
            : state === "done"
              ? "reviewing"
              : "";

  // A partial run left planned work undone, so it is exactly the case the
  // retry affordance exists for — offering it only on error/cancel left the
  // user with no in-app way to finish the task.
  const canRetry =
    (state === "error" || state === "cancelled" || state === "partial") && !!lastSubmit.current;

  return (
    <div className="vx-stream">
      <header className="vx-stream__header">
        <span className="vx-stream__title" title={title}>
          {title}
        </span>
        {statusLabel && (
          <span className={`vx-stream__status vx-stream__status--${state}`}>{statusLabel}</span>
        )}
        {usage && usage.tokens_used > 0 && (
          <span
            className="vx-stream__usage"
            title={`${usage.steps_completed} step${usage.steps_completed === 1 ? "" : "s"} · ${usage.tokens_used.toLocaleString()} tokens · ${formatCost(usage.cost_cents)}`}
          >
            {usage.steps_completed}⋅steps · {formatTokens(usage.tokens_used)} tok ·{" "}
            {formatCost(usage.cost_cents)}
          </span>
        )}
        {canRetry && (
          <button
            className="vx-stream__retry"
            onClick={() => lastSubmit.current && submit(lastSubmit.current)}
            title="Run the last message again"
          >
            <RotateCcw size={12} /> Retry
          </button>
        )}
      </header>

      <div className="vx-stream__body" ref={bodyRef} onScroll={onScroll}>
        {items.length === 0 && (
          <div className="vx-stream__empty">Welcome to VibeDesk</div>
        )}
        {items.map((item, i) => {
          switch (item.kind) {
            case "user":
              return (
                <div key={i} className="vx-msg vx-msg--user">
                  <div className="vx-msg__chip">{item.text}</div>
                </div>
              );
            case "agent": {
              // `<thinking>` is transport markup, not prose. Rendering the raw
              // turn put the tags on screen as literal text.
              const { reasoning, visible } = splitThinking(item.text);
              return (
                <div key={i} className="vx-msg vx-msg--agent">
                  <ReasoningBlock
                    reasoning={reasoning}
                    only={reasoning.length > 0 && !visible}
                  />
                  {visible && (
                    <div className="vx-msg__prose">
                      <Markdown text={visible} />
                    </div>
                  )}
                  <div className="vx-msg__tools">
                    {/* Copy the answer, not the internal markup. */}
                    <CopyButton text={visible || item.text} />
                  </div>
                </div>
              );
            }
            case "system":
              // Rendered as markdown so `/help` reads as a list; daemon system
              // notes are plain text, which passes through unchanged.
              return (
                <div key={i} className="vx-msg vx-msg--system">
                  <div className="vx-msg__system">
                    <Markdown text={item.text} />
                  </div>
                </div>
              );
            case "tool":
              return <ToolUseBlock key={i} tool={item.tool} summary={item.summary} />;
            case "error":
              return (
                <div key={i} className="vx-msg vx-msg--agent">
                  <div className="vx-msg__error">{item.text}</div>
                </div>
              );
          }
        })}
        {state === "running" && <div className="vx-stream__typing">●●●</div>}
      </div>

      {!stuck && (
        <button className="vx-stream__jump" onClick={jumpToLatest} aria-label="Jump to latest">
          <ArrowDown size={14} /> Latest
        </button>
      )}

      <ApprovalPrompt pending={approvals.pending} onRespond={approvals.respond} />

      <TaskPrompt
        daemonUrl={daemonUrl}
        daemonOnline={daemonOnline}
        busy={state === "running"}
        prefs={prefs}
        onPref={onPref}
        onProviderModel={onProviderModel}
        draft={draft}
        onDraft={onDraft}
        attachments={attachments}
        onAttachments={onAttachments}
        scopePath={runRoot}
        onSubmit={submit}
        onStop={() => stop(daemonUrl)}
        onQuickAction={onQuickAction}
        onSlash={(action) => {
          // Three commands act on this pane; the rest are the shell's.
          if (action === "stop") stop(daemonUrl);
          else if (action === "branch") onPref("isolate", !prefs.isolate);
          else if (action === "help") appendSystem(helpText());
          else onSlash(action);
        }}
      />
    </div>
  );
}
