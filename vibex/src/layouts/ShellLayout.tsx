import { useState } from "react";
import { PanelLeftOpen, PanelRightOpen } from "lucide-react";
import { ask, confirm, message } from "@tauri-apps/plugin-dialog";
import { ProjectNavRail } from "../components/ProjectNavRail";
import { SessionStream } from "../components/SessionStream";
import { EnvironmentInspector } from "../components/EnvironmentInspector";
import { ReviewView } from "../components/ReviewView";
import { FilesView } from "../components/FilesView";
import { SettingsView } from "../components/SettingsView";
import { RecoveryView } from "../components/RecoveryView";
import type { QuickAction } from "../components/QuickActionDrawer";
import { useProjects } from "../hooks/useProjects";
import type { Task, useTasks } from "../hooks/useTasks";

type TasksApi = ReturnType<typeof useTasks>;
type Overlay = null | "review" | "files" | "settings" | "trash";

interface ShellLayoutProps {
  daemonUrl: string;
  daemonOnline: boolean;
  tasks: TasksApi;
}

/**
 * VX-101 — the Codex-faithful three-column shell:
 *   left ProjectNavRail · center SessionStream · right EnvironmentInspector.
 * Side rails collapse to a thin strip with an expand button (so collapse is
 * reversible). No persistent editor pane — code is summoned via the
 * Review/Files quick-actions, which open as a center overlay.
 */
export function ShellLayout({ daemonUrl, daemonOnline, tasks }: ShellLayoutProps) {
  const [navCollapsed, setNavCollapsed] = useState(false);
  const [envCollapsed, setEnvCollapsed] = useState(false);
  const [overlay, setOverlay] = useState<Overlay>(null);
  // Bumped when a run finishes so the Environment inspector refetches git status.
  const [envRefresh, setEnvRefresh] = useState(0);
  // Remounting SessionStream on a fresh nonce is how "New chat" resets the
  // conversation (the live stream state lives inside SessionStream).
  const [chatNonce, setChatNonce] = useState(0);
  // The active project the next task is scoped to (set by "New project" /
  // selecting a project). null → daemon workspace_root default.
  const [activeProject, setActiveProject] = useState<string | null>(null);
  // The currently selected chat/task id (visual highlight in the nav).
  const [activeChatId, setActiveChatId] = useState<string | null>(null);
  // The selected chat's full task row — loaded into SessionStream so its
  // conversation renders and follow-ups resume its session (VX bug-3).
  const [selectedTask, setSelectedTask] = useState<Task | null>(null);
  // Persisted project list so empty projects show + survive restart (bug-1).
  const projects = useProjects();

  function handleQuickAction(action: QuickAction) {
    if (action === "review") setOverlay("review");
    else if (action === "files") setOverlay("files");
    // side-chat / browser / terminal: Phase 3.
  }

  function newChat() {
    setOverlay(null);
    setActiveChatId(null);
    setSelectedTask(null);
    setChatNonce((n) => n + 1);
  }

  // VX bug-3: selecting a chat loads it into the center pane. Remounting
  // SessionStream (fresh nonce) makes it fetch + render the chat's history.
  function selectChat(id: string) {
    const t = tasks.tasks.find((x) => x.id === id) ?? null;
    setActiveChatId(id);
    setSelectedTask(t);
    if (t?.project_path) setActiveProject(t.project_path);
    setOverlay(null);
    setChatNonce((n) => n + 1);
  }

  // VX bug-2 + worktree-lifecycle: delete a chat → move it to Trash (reversible
  // from the Trash & Archive view; the daemon reclaims its worktree after the
  // grace window). For a chat with a worktree, still offer to merge its branch
  // back first as the alternative to trashing.
  async function deleteChat(task: Task) {
    try {
      if (task.worktree_path) {
        const merge = await ask(
          "Merge this chat's worktree branch back into the project, or move the chat to Trash?\n\nTrashed chats are recoverable; their worktree is reclaimed later.",
          { title: "Merge or Trash?", okLabel: "Merge & delete", cancelLabel: "Move to Trash" }
        );
        if (merge) await tasks.mergeTask(task.id);
        else await tasks.deleteTask(task.id, false);
      } else {
        const ok = await confirm(`Move chat “${task.title}” to Trash?`, {
          title: "Move to Trash",
        });
        if (!ok) return;
        await tasks.deleteTask(task.id, false);
      }
      if (activeChatId === task.id) newChat();
    } catch (e) {
      await message(String(e), { title: "Delete failed", kind: "error" });
    }
  }

  // Archive a chat: keep its branch forever, free the worktree dir. Recoverable
  // from the Trash & Archive view (restore re-materializes the worktree).
  async function archiveChat(task: Task) {
    try {
      await tasks.archiveTask(task.id);
      if (activeChatId === task.id) newChat();
    } catch (e) {
      await message(String(e), { title: "Archive failed", kind: "error" });
    }
  }

  // Delete a project: forget the picked path and move its chats to Trash. The
  // Projects rail is the union of task paths + picked paths (groupByProject), so
  // the project only fully disappears once its tasks are gone — hence we trash
  // them too. Chats are recoverable from the Trash & Archive view; their
  // worktrees are reclaimed later by the daemon's reaper.
  async function deleteProject(path: string) {
    const name = path.split("/").filter(Boolean).pop() || path;
    const chats = tasks.tasks.filter((t) => t.project_path === path);
    const ok = await confirm(
      chats.length > 0
        ? `Delete project “${name}” and move its ${chats.length} chat${chats.length === 1 ? "" : "s"} to Trash?\n\nChats are recoverable from Trash & Archive; worktrees are reclaimed later.`
        : `Delete project “${name}”?`,
      { title: "Delete project", kind: "warning" }
    );
    if (!ok) return;
    try {
      for (const t of chats) {
        await tasks.deleteTask(t.id, false);
      }
      projects.removeProject(path);
      if (activeProject === path) {
        setActiveProject(null);
        newChat();
      }
    } catch (e) {
      await message(String(e), { title: "Delete failed", kind: "error" });
    }
  }

  return (
    <div
      className="vibex-shell"
      data-nav={navCollapsed ? "collapsed" : "expanded"}
      data-env={envCollapsed ? "collapsed" : "expanded"}
    >
      <div className="vibex-col vibex-col--nav">
        {navCollapsed ? (
          <div className="vx-rail vx-rail--left">
            <button
              className="vx-icon-btn"
              title="Expand navigation"
              aria-label="Expand navigation"
              onClick={() => setNavCollapsed(false)}
            >
              <PanelLeftOpen size={16} />
            </button>
          </div>
        ) : (
          <ProjectNavRail
            daemonUrl={daemonUrl}
            daemonOnline={daemonOnline}
            tasks={tasks.tasks}
            projectPaths={projects.projectPaths}
            activeChatId={activeChatId}
            activeProject={activeProject}
            onNewChat={newChat}
            onNewProject={(path) => {
              projects.addProject(path);
              setActiveProject(path);
              newChat();
            }}
            onSelectProject={(path) => setActiveProject(path)}
            onDeleteProject={deleteProject}
            onSelectChat={selectChat}
            onDeleteChat={deleteChat}
            onArchiveChat={archiveChat}
            onOpenTrash={() => setOverlay("trash")}
            onOpenSettings={() => setOverlay("settings")}
            onToggle={() => setNavCollapsed(true)}
          />
        )}
      </div>

      <div className="vibex-col vibex-col--stream">
        {overlay === "settings" ? (
          <SettingsView onClose={() => setOverlay(null)} />
        ) : overlay === "trash" ? (
          <RecoveryView
            tasks={tasks}
            onClose={() => setOverlay(null)}
            onChanged={(id) => {
              if (activeChatId === id) newChat();
            }}
          />
        ) : overlay === "review" ? (
          <ReviewView daemonUrl={daemonUrl} onClose={() => setOverlay(null)} />
        ) : overlay === "files" ? (
          <FilesView daemonUrl={daemonUrl} onClose={() => setOverlay(null)} />
        ) : (
          <SessionStream
            key={chatNonce}
            daemonUrl={daemonUrl}
            daemonOnline={daemonOnline}
            projectPath={activeProject}
            selectedTask={selectedTask}
            createTask={tasks.createTask}
            linkSession={tasks.linkSession}
            getHistory={tasks.getHistory}
            onQuickAction={handleQuickAction}
            onRunFinished={() => setEnvRefresh((n) => n + 1)}
          />
        )}
      </div>

      <div className="vibex-col vibex-col--env">
        {envCollapsed ? (
          <div className="vx-rail vx-rail--right">
            <button
              className="vx-icon-btn"
              title="Expand environment"
              aria-label="Expand environment"
              onClick={() => setEnvCollapsed(false)}
            >
              <PanelRightOpen size={16} />
            </button>
          </div>
        ) : (
          <EnvironmentInspector
            daemonUrl={daemonUrl}
            daemonOnline={daemonOnline}
            refreshKey={envRefresh}
            onOpenReview={() => setOverlay("review")}
            onToggle={() => setEnvCollapsed(true)}
          />
        )}
      </div>
    </div>
  );
}
