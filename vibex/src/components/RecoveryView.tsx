import { useCallback, useEffect, useState } from "react";
import { X, RotateCcw, Trash2, Archive as ArchiveIcon } from "lucide-react";
import { confirm, message } from "@tauri-apps/plugin-dialog";
import type { Task, useTasks } from "../hooks/useTasks";

type TasksApi = ReturnType<typeof useTasks>;

interface RecoveryViewProps {
  tasks: TasksApi;
  onClose: () => void;
  /** Called after a restore/purge so the shell can reconcile the active chat. */
  onChanged?: (affectedId: string) => void;
}

type Tab = "trashed" | "archived";

function whenLabel(ts?: number): string {
  if (!ts) return "";
  return new Date(ts * 1000).toLocaleString();
}

function projectName(path: string): string {
  return path.split("/").filter(Boolean).pop() || "workspace";
}

/**
 * Worktree-lifecycle slice 2 — the Trash & Archive recovery view. Trashed chats
 * are restorable for the daemon's grace window (then the reaper reclaims their
 * worktree); archived chats keep their branch forever and re-materialize on
 * restore. "Delete forever" routes through the daemon's safe purge, which still
 * preserves any unmerged work under `refs/trash/<id>`.
 */
export function RecoveryView({ tasks, onClose, onChanged }: RecoveryViewProps) {
  const [tab, setTab] = useState<Tab>("trashed");
  const [rows, setRows] = useState<Task[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setRows(null);
    setError(null);
    try {
      setRows(await tasks.listInState(tab));
    } catch (e) {
      setError(String(e));
    }
  }, [tasks, tab]);

  useEffect(() => {
    load();
  }, [load]);

  async function restore(t: Task) {
    try {
      await tasks.restoreTask(t.id);
      onChanged?.(t.id);
      await load();
    } catch (e) {
      await message(String(e), { title: "Restore failed", kind: "error" });
    }
  }

  async function purge(t: Task) {
    const ok = await confirm(
      `Permanently delete “${t.title}”?\n\nThis removes the chat for good. Any unmerged work on its branch is still preserved under refs/trash/ and recoverable with git.`,
      { title: "Delete forever", kind: "warning" }
    );
    if (!ok) return;
    try {
      await tasks.purgeTask(t.id);
      onChanged?.(t.id);
      await load();
    } catch (e) {
      await message(String(e), { title: "Delete failed", kind: "error" });
    }
  }

  return (
    <div className="vx-files vx-trash">
      <div className="vx-files__head">
        <span>Trash &amp; Archive</span>
        <button className="vx-icon-btn" aria-label="Close" onClick={onClose}>
          <X size={14} />
        </button>
      </div>

      <div className="vx-trash__tabs" role="tablist">
        <button
          role="tab"
          aria-selected={tab === "trashed"}
          className={`vx-trash__tab${tab === "trashed" ? " is-active" : ""}`}
          onClick={() => setTab("trashed")}
        >
          <Trash2 size={13} /> Trash
        </button>
        <button
          role="tab"
          aria-selected={tab === "archived"}
          className={`vx-trash__tab${tab === "archived" ? " is-active" : ""}`}
          onClick={() => setTab("archived")}
        >
          <ArchiveIcon size={13} /> Archive
        </button>
      </div>

      <div className="vx-files__body">
        {error && <div className="vx-files__empty">Failed to load: {error}</div>}
        {!error && rows === null && <div className="vx-files__empty">Loading…</div>}
        {!error && rows !== null && rows.length === 0 && (
          <div className="vx-files__empty">
            {tab === "trashed" ? "Trash is empty." : "No archived chats."}
          </div>
        )}
        <ul className="vx-files__list">
          {(rows ?? []).map((t) => (
            <li key={t.id} className="vx-trash__item">
              <div className="vx-trash__meta">
                <span className="vx-trash__title" title={t.title}>
                  {t.title}
                </span>
                <span className="vx-trash__sub">
                  {projectName(t.project_path)}
                  {tab === "trashed" && t.trashed_at ? ` · trashed ${whenLabel(t.trashed_at)}` : ""}
                  {tab === "archived" && t.archived_at ? ` · archived ${whenLabel(t.archived_at)}` : ""}
                </span>
              </div>
              <div className="vx-trash__actions">
                <button className="vx-trash__btn" title="Restore" onClick={() => restore(t)}>
                  <RotateCcw size={13} /> Restore
                </button>
                {tab === "trashed" && (
                  <button
                    className="vx-trash__btn vx-trash__btn--danger"
                    title="Delete forever"
                    onClick={() => purge(t)}
                  >
                    <Trash2 size={13} /> Delete forever
                  </button>
                )}
              </div>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
