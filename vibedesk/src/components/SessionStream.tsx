import { useEffect, useRef, useState } from "react";
import { TaskPrompt, type ComposerSubmit } from "./TaskPrompt";
import { ToolUseBlock } from "./ToolUseBlock";
import { useAgentStream, eventsToStreamItems } from "../hooks/useAgentStream";
import type { Task, TaskHistory } from "../hooks/useTasks";
import type { QuickAction } from "./QuickActionDrawer";

interface SessionStreamProps {
  daemonUrl: string;
  daemonOnline: boolean;
  /** Project the next task is scoped to (null → daemon workspace default). */
  projectPath: string | null;
  /** When set, this finished chat is loaded into the pane and follow-ups resume
   *  its session instead of starting a fresh one (VX bug-3). */
  selectedTask: Task | null;
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
  /** Fired when a run reaches a terminal state, so the parent can refresh env. */
  onRunFinished: () => void;
}

/**
 * VX-103 + VX-105b — center linear conversation (Codex screenshots 1, 8).
 * User messages are right-aligned chips; agent output is left-aligned prose
 * streamed live from the daemon via `useAgentStream`. The composer (TaskPrompt)
 * is pinned at the bottom; this component orchestrates the full submit flow:
 * create task (+worktree) → run agent → link session → reflect lifecycle.
 */
export function SessionStream({
  daemonUrl,
  daemonOnline,
  projectPath,
  selectedTask,
  createTask,
  linkSession,
  getHistory,
  onQuickAction,
  onRunFinished,
}: SessionStreamProps) {
  const { items, state, runTask, loadItems } = useAgentStream();
  const [title, setTitle] = useState<string>(selectedTask?.title ?? "New task");
  // The task whose run is currently streaming — used to PATCH its lifecycle
  // status when the run reaches a terminal state (VX-201).
  const activeTaskId = useRef<string | null>(null);
  // When resuming a selected chat, the session id to continue (VX bug-3).
  const resumeSessionId = useRef<string | null>(null);

  // VX bug-3: when mounted onto a selected chat, load its prior conversation
  // from the daemon and arm follow-ups to resume that session. SessionStream is
  // remounted (keyed on a nonce) per selection, so a mount-time load is correct.
  useEffect(() => {
    const task = selectedTask;
    if (!task || !task.session_id) return;
    resumeSessionId.current = task.session_id;
    let alive = true;
    (async () => {
      try {
        const hist = await getHistory(task.id);
        if (!alive) return;
        loadItems(eventsToStreamItems(hist.events));
      } catch (e) {
        console.error("load chat history failed", e);
      }
    })();
    return () => {
      alive = false;
    };
    // Only on (re)mount for this selected task.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // VX-201: reflect the run's terminal state onto the task card. A completed
  // run moves the task to "reviewing" (user reviews the diff); an error moves
  // it to "failed". Mirrors the VibeDesk state machine in pdm/08 §7.
  useEffect(() => {
    const id = activeTaskId.current;
    if (!id) return;
    if (state === "done") {
      linkSession(id, "", "reviewing").catch((e) => console.error("status update failed", e));
      activeTaskId.current = null;
      onRunFinished();
    } else if (state === "error") {
      linkSession(id, "", "failed").catch((e) => console.error("status update failed", e));
      activeTaskId.current = null;
      onRunFinished();
    }
  }, [state, linkSession, onRunFinished]);

  async function handleSubmit(p: ComposerSubmit) {
    // VX bug-3: resume path — continue the selected chat's session instead of
    // creating a fresh task. The new turn + reply append to the loaded history.
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
        reasoning: p.reasoning,
        resumeSessionId: resumeSessionId.current,
      });
      return;
    }

    // First task message becomes the session title.
    if (state === "idle") setTitle(p.task);

    // VX-112/113: create the task card before the agent starts. A worktree
    // branch is only forked when the user opted in via the composer's Branch
    // toggle (p.isolate) — a plain chat stays in place (no branch).
    let task: Task | null = null;
    try {
      task = await createTask(p.task, p.provider, p.model, projectPath ?? undefined, p.isolate);
      activeTaskId.current = task.id;
    } catch (e) {
      console.error("create task failed", e);
    }

    // VX-105b: run the agent and stream its output live.
    const sessionId = await runTask({
      daemonUrl,
      task: p.task,
      provider: p.provider,
      model: p.model,
      approval: p.approval,
      reasoning: p.reasoning,
    });

    // VX-201: link the run's session and reflect lifecycle on the task.
    if (task && sessionId) {
      try {
        await linkSession(task.id, sessionId, "running");
      } catch (e) {
        console.error("link session failed", e);
      }
    }
  }

  const statusLabel =
    state === "running" ? "running" : state === "error" ? "failed" : state === "done" ? "reviewing" : "";

  return (
    <div className="vx-stream">
      <header className="vx-stream__header">
        <span className="vx-stream__title">{title}</span>
        {statusLabel && <span className={`vx-stream__status vx-stream__status--${state}`}>{statusLabel}</span>}
      </header>

      <div className="vx-stream__body">
        {items.length === 0 && (
          <div className="vx-stream__empty">
            Type a message below — VibeDesk runs the agent and streams the result here. Toggle
            Branch in the composer to isolate a coding task on its own git worktree.
          </div>
        )}
        {items.map((item, i) => {
          switch (item.kind) {
            case "user":
              return (
                <div key={i} className="vx-msg vx-msg--user">
                  <div className="vx-msg__chip">{item.text}</div>
                </div>
              );
            case "agent":
              return (
                <div key={i} className="vx-msg vx-msg--agent">
                  <div className="vx-msg__prose">{item.text}</div>
                </div>
              );
            case "system":
              return (
                <div key={i} className="vx-msg vx-msg--system">
                  <div className="vx-msg__system">{item.text}</div>
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

      <TaskPrompt
        daemonUrl={daemonUrl}
        daemonOnline={daemonOnline}
        busy={state === "running"}
        onSubmit={handleSubmit}
        onQuickAction={onQuickAction}
      />
    </div>
  );
}
