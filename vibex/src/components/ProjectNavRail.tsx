import { MessageSquarePlus, Search, Sparkles, Plug, Workflow, Folder, Settings, PanelLeftClose } from "lucide-react";

interface ProjectNavRailProps {
  daemonUrl: string;
  daemonOnline: boolean;
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

// Placeholder until VX-112 wires the daemon's project/task list.
const MOCK_PROJECTS = [
  { name: "vibecody", chats: ["fix the auth timeout", "import vibecody relate…"] },
  { name: "website", chats: ["build and run the site"] },
];

export function ProjectNavRail({ onToggle }: ProjectNavRailProps) {
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
        {MOCK_PROJECTS.map((p) => (
          <li key={p.name}>
            <button className="vx-nav__item vx-nav__item--project" aria-label={p.name}>
              <Folder size={14} />
              <span>{p.name}</span>
            </button>
            <ul className="vx-nav__chats">
              {p.chats.map((c, i) => (
                <li key={c}>
                  <button className="vx-nav__chat" aria-label={c}>
                    <span className="vx-nav__chat-title">{c}</span>
                    <kbd className="vx-nav__kbd">⌘{i + 1}</kbd>
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
