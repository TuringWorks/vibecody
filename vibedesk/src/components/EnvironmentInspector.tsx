import { GitBranch, FileDiff, Monitor, GitCommit, FolderGit2, PanelRightClose } from "lucide-react";
import { useEnvironment } from "../hooks/useEnvironment";

interface EnvironmentInspectorProps {
  daemonUrl: string;
  daemonOnline: boolean;
  /** Repo to inspect — the active project or the selected task's worktree.
   *  Undefined → the daemon's own workspace root. */
  path?: string;
  /** Bumped by the parent after a run finishes, to force a status refetch. */
  refreshKey?: number;
  /** Open the Review diff viewer (VX-202). */
  onOpenReview?: () => void;
  /** Open the Files list — what the "Sources" row means. */
  onOpenFiles?: () => void;
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
  path,
  refreshKey,
  onOpenReview,
  onOpenFiles,
  onToggle,
}: EnvironmentInspectorProps) {
  const { status } = useEnvironment(daemonUrl, daemonOnline, path, refreshKey);

  const branch = status?.branch || (status?.is_git_repo === false ? "(no repo)" : "…");
  const changedCount = status?.changed_count ?? 0;
  const repoName = path ? path.split("/").filter(Boolean).pop() : null;
  const head = status?.head ?? null;
  // The daemon parks task worktrees under `<repo>/.vibecli/worktrees/` (see
  // create_task in serve.rs); a run in one is isolated on its own branch, which
  // is worth saying out loud rather than leaving "Local" as a bare label.
  const isWorktree = !!path && path.includes("/.vibecli/worktrees/");

  return (
    <aside className="vx-env">
      <div className="vx-env__head">
        <span className="vx-env__title">Environment</span>
        <button className="vx-icon-btn" aria-label="Collapse environment" onClick={onToggle}>
          <PanelRightClose size={15} />
        </button>
      </div>

      {/* Which repo these numbers describe — ambiguous the moment more than
          one project is open, which is the whole point of the app. */}
      <div className="vx-env__scope" title={path ?? "Daemon workspace root"}>
        {repoName ?? "Daemon workspace"}
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
        <li className="vx-env__item" title={path ?? "The daemon's own workspace root"}>
          <Monitor size={14} /> <span>Local</span>
          <span className="vx-env__badge">{isWorktree ? "worktree" : "in place"}</span>
        </li>
        <li className="vx-env__item" title={branch}>
          <GitBranch size={14} /> <span className="vx-env__branch">{branch}</span>
        </li>
        {/* HEAD, not a dead "Commit" label — the row now says what's checked out. */}
        <li
          className={`vx-env__item${head ? "" : " vx-env__item--muted"}`}
          title={head ? `${head.hash} — ${head.message}` : "No commits yet"}
        >
          <GitCommit size={14} />
          {head ? (
            <>
              <span className="vx-env__sha">{head.hash}</span>
              <span className="vx-env__commit-msg">{head.message}</span>
            </>
          ) : (
            <span>No commits yet</span>
          )}
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

      {/* Was a chevron button that did nothing; it now opens the file list,
          which is what "Sources" was always implying. */}
      <button
        className="vx-env__sources"
        aria-label="Browse project sources"
        onClick={onOpenFiles}
        disabled={!onOpenFiles}
      >
        <FolderGit2 size={14} /> <span>Sources</span> <span className="vx-env__chevron">›</span>
      </button>
    </aside>
  );
}
