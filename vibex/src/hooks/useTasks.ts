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

  return { tasks, error, refresh, createTask, linkSession };
}
