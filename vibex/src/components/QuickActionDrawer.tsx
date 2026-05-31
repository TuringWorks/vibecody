import { FolderTree, MessagesSquare, Globe, GitCompare, TerminalSquare, X } from "lucide-react";

interface QuickActionDrawerProps {
  daemonUrl: string;
  onClose: () => void;
}

/**
 * VX-110 — the "+" quick-action drawer (Codex screenshots 6, 7).
 * This is Codex's progressive-disclosure model: summon Files / Side chat /
 * Browser / Review / Terminal on demand, instead of a time-gated level ramp.
 * Files + Terminal are wired first (VX-110); the rest land in Phase 3.
 */
const ACTIONS = [
  { icon: FolderTree, label: "Files", sub: "Browse project files", phase: 1 },
  { icon: MessagesSquare, label: "Side chat", sub: "Start a side conversation", phase: 3 },
  { icon: Globe, label: "Browser", sub: "Open a website", phase: 3 },
  { icon: GitCompare, label: "Review", sub: "View code changes", phase: 2 },
  { icon: TerminalSquare, label: "Terminal", sub: "Start an interactive shell", phase: 1 },
];

export function QuickActionDrawer({ onClose }: QuickActionDrawerProps) {
  return (
    <div className="vx-drawer">
      <div className="vx-drawer__head">
        <span>Quick actions</span>
        <button className="vx-icon-btn" aria-label="Close drawer" onClick={onClose}>
          <X size={14} />
        </button>
      </div>
      <div className="vx-drawer__grid">
        {ACTIONS.map(({ icon: Icon, label, sub, phase }) => (
          <button key={label} className="vx-drawer__action" disabled={phase > 1} aria-label={label}>
            <Icon size={18} />
            <span className="vx-drawer__action-label">{label}</span>
            <span className="vx-drawer__action-sub">{sub}</span>
            {phase > 1 && <span className="vx-drawer__soon">Phase {phase}</span>}
          </button>
        ))}
      </div>
    </div>
  );
}
