import { MessageSquarePlus, FolderPlus, Search, Sparkles, Plug, Workflow, Folder, Settings, PanelLeftClose, Trash2, Archive } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import type { Task } from "../hooks/useTasks";

interface ProjectNavRailProps {
  daemonUrl: string;
  daemonOnline: boolean;
  tasks: Task[];
  /** Explicitly-added project paths (persisted) — shown even with no chats. */
  projectPaths: string[];
  activeChatId: string | null;
  activeProject: string | null;
  onNewChat: () => void;
  onNewProject: (path: string) => void;
  onSelectProject: (path: string) => void;
  onDeleteProject: (path: string) => void;
  onSelectChat: (id: string) => void;
  onDeleteChat: (task: Task) => void;
  onArchiveChat: (task: Task) => void;
  onOpenTrash: () => void;
  onOpenSettings: () => void;
  onToggle: () => void;
}

/**
 * VX-102 — left project/chat navigator (Codex screenshots 1, 4, 8).
 * Fixed top items (New chat / New project / Search / Skills / Plugins /
 * Automations), then a Projects→chats tree grouped from live tasks, then
 * Settings pinned at the bottom. Rows are clickable; "New project" opens a
 * native folder picker and scopes the next task to it.
 */
const STATUS_DOT: Record<string, string> = {
  running: "var(--accent-green)",
  reviewing: "var(--accent-blue)",
  completed: "var(--text-tertiary)",
  failed: "var(--error-color)",
  queued: "var(--accent-gold)",
  draft: "var(--text-tertiary)",
};

/**
 * Group live tasks by their project path, unioned with explicitly-added
 * project paths so a freshly-picked project with no chats still renders (VX
 * bug-1). Seed the map with `projectPaths` first to preserve add order.
 */
function groupByProject(
  tasks: Task[],
  projectPaths: string[]
): { name: string; path: string; tasks: Task[] }[] {
  const byPath = new Map<string, Task[]>();
  for (const p of projectPaths) {
    if (p && !byPath.has(p)) byPath.set(p, []);
  }
  for (const t of tasks) {
    const arr = byPath.get(t.project_path) ?? [];
    arr.push(t);
    byPath.set(t.project_path, arr);
  }
  return [...byPath.entries()].map(([path, tasks]) => ({
    name: path.split("/").filter(Boolean).pop() || "workspace",
    path,
    tasks,
  }));
}

export function ProjectNavRail({
  tasks,
  projectPaths,
  activeChatId,
  activeProject,
  onNewChat,
  onNewProject,
  onSelectProject,
  onDeleteProject,
  onSelectChat,
  onDeleteChat,
  onArchiveChat,
  onOpenTrash,
  onOpenSettings,
  onToggle,
}: ProjectNavRailProps) {
  const projects = groupByProject(tasks, projectPaths);

  async function pickProject() {
    try {
      const picked = await open({ directory: true, multiple: false, title: "Open a project folder" });
      if (typeof picked === "string" && picked) onNewProject(picked);
    } catch (e) {
      console.error("folder picker failed", e);
    }
  }

  return (
    <nav className="vx-nav">
      <div className="vx-nav__header">
        <span className="vx-nav__brand">VibeX</span>
        <button className="vx-icon-btn" title="Collapse" onClick={onToggle} aria-label="Collapse navigation">
          <PanelLeftClose size={15} />
        </button>
      </div>

      <ul className="vx-nav__list">
        <li>
          <button className="vx-nav__item" aria-label="New chat" onClick={onNewChat}>
            <MessageSquarePlus size={15} />
            <span>New chat</span>
          </button>
        </li>
        <li>
          <button className="vx-nav__item" aria-label="New project" onClick={pickProject}>
            <FolderPlus size={15} />
            <span>New project</span>
          </button>
        </li>
        <li>
          <button className="vx-nav__item vx-nav__item--soon" aria-label="Search" disabled title="Coming soon">
            <Search size={15} />
            <span>Search</span>
          </button>
        </li>
        <li>
          <button className="vx-nav__item vx-nav__item--soon" aria-label="Skills" disabled title="Coming soon">
            <Sparkles size={15} />
            <span>Skills</span>
          </button>
        </li>
        <li>
          <button className="vx-nav__item vx-nav__item--soon" aria-label="Plugins" disabled title="Coming soon">
            <Plug size={15} />
            <span>Plugins</span>
          </button>
        </li>
        <li>
          <button className="vx-nav__item vx-nav__item--soon" aria-label="Automations" disabled title="Coming soon">
            <Workflow size={15} />
            <span>Automations</span>
          </button>
        </li>
      </ul>

      <div className="vx-nav__section">Projects</div>
      <ul className="vx-nav__list">
        {projects.length === 0 && (
          <li className="vx-nav__empty">No tasks yet — type one below or add a project.</li>
        )}
        {projects.map((p) => (
          <li key={p.path}>
            <div className="vx-nav__project-row">
              <button
                className={`vx-nav__item vx-nav__item--project${activeProject === p.path ? " is-active" : ""}`}
                aria-label={p.name}
                title={p.path}
                onClick={() => onSelectProject(p.path)}
              >
                <Folder size={14} />
                <span className="vx-nav__project-name">{p.name}</span>
              </button>
              <button
                className="vx-nav__chat-del vx-nav__project-del"
                aria-label={`Delete project ${p.name}`}
                title="Delete project"
                onClick={(e) => {
                  e.stopPropagation();
                  onDeleteProject(p.path);
                }}
              >
                <Trash2 size={13} />
              </button>
            </div>
            <ul className="vx-nav__chats">
              {p.tasks.length === 0 && (
                <li className="vx-nav__chats-empty">No chats yet — type a task below.</li>
              )}
              {p.tasks.map((t, i) => (
                <li key={t.id} className="vx-nav__chat-row">
                  <button
                    className={`vx-nav__chat${activeChatId === t.id ? " is-active" : ""}`}
                    aria-label={t.title}
                    title={`${t.status} · ${t.branch || "no branch"}`}
                    onClick={() => onSelectChat(t.id)}
                  >
                    <span
                      className="vx-nav__chat-dot"
                      style={{ background: STATUS_DOT[t.status] ?? "var(--text-tertiary)" }}
                    />
                    <span className="vx-nav__chat-title">{t.title}</span>
                    {i < 9 && <kbd className="vx-nav__kbd">⌘{i + 1}</kbd>}
                  </button>
                  <button
                    className="vx-nav__chat-del"
                    aria-label={`Archive chat ${t.title}`}
                    title="Archive chat"
                    onClick={(e) => {
                      e.stopPropagation();
                      onArchiveChat(t);
                    }}
                  >
                    <Archive size={13} />
                  </button>
                  <button
                    className="vx-nav__chat-del"
                    aria-label={`Delete chat ${t.title}`}
                    title="Move chat to Trash"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDeleteChat(t);
                    }}
                  >
                    <Trash2 size={13} />
                  </button>
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>

      <div className="vx-nav__spacer" />
      <button className="vx-nav__item" aria-label="Trash and Archive" onClick={onOpenTrash}>
        <Trash2 size={15} />
        <span>Trash &amp; Archive</span>
      </button>
      <button className="vx-nav__item vx-nav__item--settings" aria-label="Settings" onClick={onOpenSettings}>
        <Settings size={15} />
        <span>Settings</span>
      </button>
    </nav>
  );
}
