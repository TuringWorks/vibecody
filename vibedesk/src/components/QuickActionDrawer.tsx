import {
  FolderTree,
  MessagesSquare,
  Globe,
  GitCompare,
  TerminalSquare,
  Paperclip,
  AudioLines,
} from "lucide-react";

export type QuickAction = "files" | "review" | "side-chat" | "browser" | "terminal";

interface QuickActionDrawerProps {
  onAction: (action: QuickAction) => void;
  onClose: () => void;
  /** Attach files to the next message — the other half of "+ adds something". */
  onAttach: () => void;
  /** Full-duplex voice: the standing opt-in, not the start/stop. */
  voice: {
    enabled: boolean;
    supported: boolean;
    /** Why it is unavailable, when it is. Shown instead of a dead switch. */
    unsupportedHint?: string;
    onEnabledChange: (on: boolean) => void;
  };
}

/**
 * VX-110 — the "+" menu (Codex screenshots 6, 7).
 *
 * Codex's progressive-disclosure model: summon Files / Side chat / Browser /
 * Review / Terminal on demand. It used to be a three-column card grid that
 * covered the conversation; a plain menu anchored to the button reads as "this
 * is what + does" instead of as a modal, and leaves room for the two things
 * that belong here rather than on the toolbar — attaching a file, and the
 * voice-conversation switch.
 */
const ACTIONS: { id: QuickAction; icon: typeof FolderTree; label: string; sub: string }[] = [
  { id: "files", icon: FolderTree, label: "Files", sub: "Browse project files" },
  { id: "review", icon: GitCompare, label: "Review", sub: "View code changes" },
  { id: "side-chat", icon: MessagesSquare, label: "Side chat", sub: "Ask without making a task" },
  { id: "browser", icon: Globe, label: "Browser", sub: "Open a website" },
  { id: "terminal", icon: TerminalSquare, label: "Terminal", sub: "Run a command in the project" },
];

export function QuickActionDrawer({ onAction, onClose, onAttach, voice }: QuickActionDrawerProps) {
  // Click-away and Escape are owned by the composer, which holds both this
  // menu and the "+" that opens it: closing here would fire before the
  // button's own click and reopen the menu on every attempt to dismiss it.
  return (
    <div className="vx-drawer" role="menu" aria-label="Quick actions">
      <div className="vx-drawer__group">Add to this message</div>
      <button
        className="vx-drawer__item"
        role="menuitem"
        onClick={() => {
          onClose();
          onAttach();
        }}
      >
        <Paperclip size={15} />
        <span className="vx-drawer__label">Attach files</span>
        <span className="vx-drawer__sub">Send file contents with the prompt</span>
      </button>

      <div className="vx-drawer__group">Open a panel</div>
      {ACTIONS.map(({ id, icon: Icon, label, sub }) => (
        <button key={id} className="vx-drawer__item" role="menuitem" onClick={() => onAction(id)}>
          <Icon size={15} />
          <span className="vx-drawer__label">{label}</span>
          <span className="vx-drawer__sub">{sub}</span>
        </button>
      ))}

      <div className="vx-drawer__group">Voice</div>
      <button
        className="vx-drawer__item"
        role="menuitemcheckbox"
        aria-checked={voice.enabled && voice.supported}
        disabled={!voice.supported}
        title={voice.supported ? undefined : voice.unsupportedHint}
        onClick={() => voice.onEnabledChange(!voice.enabled)}
      >
        <AudioLines size={15} />
        <span className="vx-drawer__label">Voice conversation</span>
        <span className="vx-drawer__sub">
          {!voice.supported
            ? (voice.unsupportedHint ?? "Not available here")
            : voice.enabled
              ? "On — start it from the toolbar"
              : "Off — talk with the model, hands free"}
        </span>
        {voice.supported && (
          <span className={`vx-drawer__switch${voice.enabled ? " is-on" : ""}`} aria-hidden />
        )}
      </button>
    </div>
  );
}
