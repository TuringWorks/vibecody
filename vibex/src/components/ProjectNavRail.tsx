import { MessageSquarePlus, Search, Sparkles, Plug, Workflow, Folder, Settings, PanelLeftClose } from "lucide-react";
import type { Task } from "../hooks/useTasks";

interface ProjectNavRailProps {
  daemonUrl: string;
  daemonOnline: boolean;
  tasks: Task[];
  onToggle: () => void;
}

/**
 * VX-102 — left project/chat navigator (Codex screenshots 1, 4, 8).
 * Fixed top items, then a Projects→chats tree, then Settings pinned bottom.
 * Currently renders the static structure; chats wire to /api/tasks in VX-112.
 */
const TOP_ITEMS = [
  { icon: MessageSquarePlus, label: "New chat" },
  { icon: Search, label: "Search" },
  { icon: Sparkles, label: "Skills" },
  { icon: Plug, label: "Plugins" },
  { icon: Workflow, label: "Automations" },
];

/** Group live tasks by their project path's last path segment. */
function groupByProject(tasks: Task[]): { name: string; tasks: Task[] }[] {
  const byProject = new Map<string, Task[]>();
  for (const t of tasks) {
    const name = t.project_path.split("/").filter(Boolean).pop() || "workspace";
    const arr = byProject.get(name) ?? [];
    arr.push(t);
    byProject.set(name, arr);
  }
  return [...byProject.entries()].map(([name, tasks]) => ({ name, tasks }));
}

const STATUS_DOT: Record<string, string> = {
  running: "var(--accent-green)",
  reviewing: "var(--accent-blue)",
  completed: "var(--text-tertiary)",
  failed: "var(--error-color)",
  queued: "var(--accent-gold)",
  draft: "var(--text-tertiary)",
};

export function ProjectNavRail({ tasks, onToggle }: ProjectNavRailProps) {
  const projects = groupByProject(tasks);
  return (
    <nav className="vx-nav">
      <div className="vx-nav__header">
        <span className="vx-nav__brand">VibeX</span>
        <button className="vx-icon-btn" title="Collapse" onClick={onToggle} aria-label="Collapse navigation">
          <PanelLeftClose size={15} />
        </button>
      </div>

      <ul className="vx-nav__list">
        {TOP_ITEMS.map(({ icon: Icon, label }) => (
          <li key={label}>
            <button className="vx-nav__item" aria-label={label}>
              <Icon size={15} />
              <span>{label}</span>
            </button>
          </li>
        ))}
      </ul>

      <div className="vx-nav__section">Projects</div>
      <ul className="vx-nav__list">
        {projects.length === 0 && (
          <li className="vx-nav__empty">No tasks yet — type one below.</li>
        )}
        {projects.map((p) => (
          <li key={p.name}>
            <button className="vx-nav__item vx-nav__item--project" aria-label={p.name}>
              <Folder size={14} />
              <span>{p.name}</span>
            </button>
            <ul className="vx-nav__chats">
              {p.tasks.map((t, i) => (
                <li key={t.id}>
                  <button className="vx-nav__chat" aria-label={t.title} title={`${t.status} · ${t.branch || "no branch"}`}>
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
      <button className="vx-nav__item vx-nav__item--settings" aria-label="Settings">
        <Settings size={15} />
        <span>Settings</span>
      </button>
    </nav>
  );
}
