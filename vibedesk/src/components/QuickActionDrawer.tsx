import { FolderTree, MessagesSquare, Globe, GitCompare, TerminalSquare, X } from "lucide-react";

export type QuickAction = "files" | "review" | "side-chat" | "browser" | "terminal";

interface QuickActionDrawerProps {
  onAction: (action: QuickAction) => void;
  onClose: () => void;
}

/**
 * VX-110 — the "+" quick-action drawer (Codex screenshots 6, 7).
 * Codex's progressive-disclosure model: summon Files / Side chat / Browser /
 * Review / Terminal on demand, not a time-gated level ramp. Files + Review are
 * wired (VX-110/202); the rest land in Phase 3.
 */
const ACTIONS: { id: QuickAction; icon: typeof FolderTree; label: string; sub: string; ready: boolean }[] = [
  { id: "files", icon: FolderTree, label: "Files", sub: "Browse project files", ready: true },
  { id: "review", icon: GitCompare, label: "Review", sub: "View code changes", ready: true },
  { id: "side-chat", icon: MessagesSquare, label: "Side chat", sub: "Start a side conversation", ready: false },
  { id: "browser", icon: Globe, label: "Browser", sub: "Open a website", ready: false },
  { id: "terminal", icon: TerminalSquare, label: "Terminal", sub: "Start an interactive shell", ready: false },
];

export function QuickActionDrawer({ onAction, onClose }: QuickActionDrawerProps) {
  return (
    <div className="vx-drawer">
      <div className="vx-drawer__head">
        <span>Quick actions</span>
        <button className="vx-icon-btn" aria-label="Close drawer" onClick={onClose}>
          <X size={14} />
        </button>
      </div>
      <div className="vx-drawer__grid">
        {ACTIONS.map(({ id, icon: Icon, label, sub, ready }) => (
          <button
            key={id}
            className="vx-drawer__action"
            disabled={!ready}
            aria-label={label}
            onClick={() => ready && onAction(id)}
          >
            <Icon size={18} />
            <span className="vx-drawer__action-label">{label}</span>
            <span className="vx-drawer__action-sub">{sub}</span>
            {!ready && <span className="vx-drawer__soon">soon</span>}
          </button>
        ))}
      </div>
    </div>
  );
}
