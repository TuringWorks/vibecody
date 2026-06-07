import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Task row mirrored from the daemon `/api/tasks` shape (task_store::TaskRow). */
export interface Task {
  id: string;
  title: string;
  status: string;
  provider: string;
  model: string;
  branch: string;
  worktree_path: string;
  session_id: string;
  project_path: string;
  created_at: number;
  updated_at: number;
  /** Lifecycle timestamps (worktree-lifecycle). Present only when set. */
  archived_at?: number;
  trashed_at?: number;
  reaped_at?: number;
}

/** Raw agent event as persisted in the daemon's durable log (job_events). */
export interface AgentEventPayload {
  type: string; // "user" | "chunk" | "step" | "system" | "complete" | "error"
  content?: string | null;
  step_num?: number | null;
  tool_name?: string | null;
  success?: boolean | null;
}

/** A finished chat's reconstructed conversation, from `/api/tasks/:id/history`. */
export interface TaskHistory {
  id: string;
  title: string;
  status: string;
  session_id: string;
  events: AgentEventPayload[];
}

/**
 * Live task list + create/update, backed by the daemon `/api/tasks` API
 * (VX-112). The daemon is the source of truth; this hook is a thin client.
 */
export function useTasks(daemonUrl: string, daemonOnline: boolean) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!daemonOnline) return;
    try {
      const rows = await invoke<Task[]>("list_tasks", { url: daemonUrl });
      setTasks(rows);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [daemonUrl, daemonOnline]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  /** Create a task (and its worktree). Returns the new task row. */
  const createTask = useCallback(
    async (title: string, provider: string, model?: string, projectPath?: string): Promise<Task> => {
      const task = await invoke<Task>("create_task", {
        url: daemonUrl,
        title,
        provider,
        model,
        projectPath,
      });
      await refresh();
      return task;
    },
    [daemonUrl, refresh]
  );

  /**
   * Update a task's lifecycle status and/or link a started agent run's session
   * id. An empty `sessionId` sends a status-only update (the daemon's
   * update_task only writes session_id when non-empty), so this doubles as the
   * VX-201 status-transition call.
   */
  const linkSession = useCallback(
    async (id: string, sessionId: string, status = "running") => {
      await invoke<Task>("update_task", {
        url: daemonUrl,
        id,
        status,
        sessionId: sessionId || undefined,
      });
      await refresh();
    },
    [daemonUrl, refresh]
  );

  /**
   * Delete a task. With the worktree-lifecycle backend this SOFT-deletes by
   * default (moves the chat to Trash, recoverable via {@link restoreTask}); the
   * worktree is reclaimed later by the daemon's reaper. `removeWorktree` maps to
   * the daemon's `?purge=true` (permanent now — still preserves unmerged work).
   */
  const deleteTask = useCallback(
    async (id: string, removeWorktree = false): Promise<void> => {
      await invoke("delete_task", { url: daemonUrl, id, removeWorktree });
      await refresh();
    },
    [daemonUrl, refresh]
  );

  /** Archive a task: keep its branch, free the worktree dir. Recoverable. */
  const archiveTask = useCallback(
    async (id: string): Promise<void> => {
      await invoke("archive_task", { url: daemonUrl, id });
      await refresh();
    },
    [daemonUrl, refresh]
  );

  /** Restore a trashed/archived task to Active (re-materializes its worktree). */
  const restoreTask = useCallback(
    async (id: string): Promise<void> => {
      await invoke("restore_task", { url: daemonUrl, id });
      await refresh();
    },
    [daemonUrl, refresh]
  );

  /** Permanently remove a task now (safe purge — preserves unmerged work). */
  const purgeTask = useCallback(
    async (id: string): Promise<void> => {
      await invoke("purge_task", { url: daemonUrl, id });
      await refresh();
    },
    [daemonUrl, refresh]
  );

  /**
   * Fetch tasks by lifecycle state for the recovery views WITHOUT touching the
   * live `tasks` list: `"trashed"` → the Trash view, `"archived"` → the Archive
   * view, `"all"` → everything. The default list (`refresh`) excludes Trashed.
   */
  const listInState = useCallback(
    async (state: "trashed" | "archived" | "all"): Promise<Task[]> => {
      return invoke<Task[]>("list_tasks_by_state", { url: daemonUrl, state });
    },
    [daemonUrl]
  );

  /**
   * Merge a task's worktree branch back, then delete the task on success.
   * Rejects (with the daemon's conflict detail) when the merge conflicts — the
   * task is left intact in that case.
   */
  const mergeTask = useCallback(
    async (id: string): Promise<void> => {
      await invoke("merge_task", { url: daemonUrl, id });
      await refresh();
    },
    [daemonUrl, refresh]
  );

  /** Fetch a finished chat's reconstructed conversation for display. */
  const getHistory = useCallback(
    async (id: string): Promise<TaskHistory> => {
      return invoke<TaskHistory>("get_task_history", { url: daemonUrl, id });
    },
    [daemonUrl]
  );

  return {
    tasks,
    error,
    refresh,
    createTask,
    linkSession,
    deleteTask,
    mergeTask,
    archiveTask,
    restoreTask,
    purgeTask,
    listInState,
    getHistory,
  };
}
