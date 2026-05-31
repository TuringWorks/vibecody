import { GitBranch, FileDiff, Monitor, GitCommit, FolderGit2, PanelRightClose, Github as GithubIcon } from "lucide-react";

interface EnvironmentInspectorProps {
  daemonUrl: string;
  daemonOnline: boolean;
  onToggle: () => void;
}

/**
 * VX-109 — right-rail Environment inspector (Codex screenshots 1, 4, 8).
 * Shows Changes / Local / branch / Commit / Sources. This replaces the spec's
 * left "Project Hub" — Codex keeps project state in a right inspector, always
 * available. Live git data wires in VX-112/VX-113; static for now.
 */
export function EnvironmentInspector({ onToggle }: EnvironmentInspectorProps) {
  return (
    <aside className="vx-env">
      <div className="vx-env__head">
        <span className="vx-env__title">Environment</span>
        <button className="vx-icon-btn" aria-label="Collapse environment" onClick={onToggle}>
          <PanelRightClose size={15} />
        </button>
      </div>

      <ul className="vx-env__list">
        <li className="vx-env__item">
          <FileDiff size={14} /> <span>Changes</span> <span className="vx-env__badge">+0</span>
        </li>
        <li className="vx-env__item">
          <Monitor size={14} /> <span>Local</span>
        </li>
        <li className="vx-env__item">
          <GitBranch size={14} /> <span>main</span>
        </li>
        <li className="vx-env__item vx-env__item--muted">
          <GitCommit size={14} /> <span>Commit</span>
        </li>
        <li className="vx-env__item vx-env__item--muted">
          <GithubIcon size={14} /> <span>GitHub CLI unavailable</span>
        </li>
      </ul>

      <button className="vx-env__sources" aria-label="Sources">
        <FolderGit2 size={14} /> <span>Sources</span> <span className="vx-env__chevron">›</span>
      </button>
    </aside>
  );
}
