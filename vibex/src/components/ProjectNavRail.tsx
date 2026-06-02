import { MessageSquarePlus, FolderPlus, Search, Sparkles, Plug, Workflow, Folder, Settings, PanelLeftClose } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import type { Task } from "../hooks/useTasks";

interface ProjectNavRailProps {
  daemonUrl: string;
  daemonOnline: boolean;
  tasks: Task[];
  activeChatId: string | null;
  activeProject: string | null;
  onNewChat: () => void;
  onNewProject: (path: string) => void;
  onSelectProject: (path: string) => void;
  onSelectChat: (id: string) => void;
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

/** Group live tasks by their project path; keep the full path for selection. */
function groupByProject(tasks: Task[]): { name: string; path: string; tasks: Task[] }[] {
  const byPath = new Map<string, Task[]>();
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
  activeChatId,
  activeProject,
  onNewChat,
  onNewProject,
  onSelectProject,
  onSelectChat,
  onOpenSettings,
  onToggle,
}: ProjectNavRailProps) {
  const projects = groupByProject(tasks);

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
            <button
              className={`vx-nav__item vx-nav__item--project${activeProject === p.path ? " is-active" : ""}`}
              aria-label={p.name}
              title={p.path}
              onClick={() => onSelectProject(p.path)}
            >
              <Folder size={14} />
              <span>{p.name}</span>
            </button>
            <ul className="vx-nav__chats">
              {p.tasks.map((t, i) => (
                <li key={t.id}>
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
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>

      <div className="vx-nav__spacer" />
      <button className="vx-nav__item vx-nav__item--settings" aria-label="Settings" onClick={onOpenSettings}>
        <Settings size={15} />
        <span>Settings</span>
      </button>
    </nav>
  );
}
