import { GitBranch, FileDiff, Monitor, GitCommit, FolderGit2, PanelRightClose } from "lucide-react";
import { useEnvironment } from "../hooks/useEnvironment";

interface EnvironmentInspectorProps {
  daemonUrl: string;
  daemonOnline: boolean;
  /** Bumped by the parent after a run finishes, to force a status refetch. */
  refreshKey?: number;
  /** Open the Review diff viewer (VX-202). */
  onOpenReview?: () => void;
  onToggle: () => void;
}

/**
 * VX-109 — right-rail Environment inspector (Codex screenshots 1, 4, 8).
 * Shows live Changes / Local / branch from the daemon's git status. Replaces
 * the spec's left "Project Hub" — Codex keeps project state in a right
 * inspector, always available.
 */
export function EnvironmentInspector({
  daemonUrl,
  daemonOnline,
  refreshKey,
  onOpenReview,
  onToggle,
}: EnvironmentInspectorProps) {
  const { status } = useEnvironment(daemonUrl, daemonOnline, refreshKey);

  const branch = status?.branch || (status?.is_git_repo === false ? "(no repo)" : "…");
  const changedCount = status?.changed_count ?? 0;

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
          <button
            className="vx-env__changes"
            onClick={onOpenReview}
            disabled={changedCount === 0 || !onOpenReview}
            aria-label="Review changes"
          >
            <FileDiff size={14} /> <span>Changes</span>
            <span className="vx-env__badge">{changedCount > 0 ? `+${changedCount}` : "0"}</span>
          </button>
        </li>
        <li className="vx-env__item">
          <Monitor size={14} /> <span>Local</span>
        </li>
        <li className="vx-env__item" title={branch}>
          <GitBranch size={14} /> <span className="vx-env__branch">{branch}</span>
        </li>
        <li className="vx-env__item vx-env__item--muted">
          <GitCommit size={14} /> <span>Commit</span>
        </li>
      </ul>

      {(status?.changed?.length ?? 0) > 0 && (
        <ul className="vx-env__changed-list">
          {status!.changed.slice(0, 12).map((f) => (
            <li key={f.path} className="vx-env__changed" title={`${f.status}: ${f.path}`}>
              <span className={`vx-env__changed-dot vx-env__changed-dot--${f.status}`} />
              <span className="vx-env__changed-path">{f.path}</span>
            </li>
          ))}
          {status!.changed.length > 12 && (
            <li className="vx-env__changed vx-env__changed--more">
              +{status!.changed.length - 12} more
            </li>
          )}
        </ul>
      )}

      <button className="vx-env__sources" aria-label="Sources">
        <FolderGit2 size={14} /> <span>Sources</span> <span className="vx-env__chevron">›</span>
      </button>
    </aside>
  );
}
