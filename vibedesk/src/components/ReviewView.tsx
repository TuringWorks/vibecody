import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, ChevronRight, X } from "lucide-react";

interface ReviewViewProps {
  daemonUrl: string;
  /** Repo to diff — the active project or the selected task's worktree.
   *  Undefined → the daemon's own workspace root. */
  path?: string;
  onClose: () => void;
}

interface FileDiff {
  path: string;
  lines: string[];
}

/**
 * Split a unified `git diff` into per-file hunks. Each `diff --git a/x b/x`
 * marker starts a new file section.
 */
function splitDiff(diff: string): FileDiff[] {
  if (!diff.trim()) return [];
  const files: FileDiff[] = [];
  let current: FileDiff | null = null;
  for (const line of diff.split("\n")) {
    if (line.startsWith("diff --git")) {
      if (current) files.push(current);
      const m = line.match(/ b\/(.+)$/);
      current = { path: m ? m[1] : line, lines: [] };
    } else if (current) {
      current.lines.push(line);
    }
  }
  if (current) files.push(current);
  return files;
}

function lineClass(line: string): string {
  if (line.startsWith("+") && !line.startsWith("+++")) return "vx-diff__line vx-diff__line--add";
  if (line.startsWith("-") && !line.startsWith("---")) return "vx-diff__line vx-diff__line--del";
  if (line.startsWith("@@")) return "vx-diff__line vx-diff__line--hunk";
  return "vx-diff__line";
}

/**
 * VX-202 — the Review diff viewer, summoned by the `+` drawer Review action or
 * the Environment "Changes" badge. Renders the working-tree diff per file with
 * add/del/hunk highlighting. Read-only; targeted edits go through the ⌘.
 * DiffCompleteModal surface (pdm/08 §1), never an inline-completion overlay.
 */
/** Added / removed line counts for a file's hunk, for the per-file stat chip. */
function countChanges(lines: string[]): { added: number; removed: number } {
  return lines.reduce(
    (acc, l) => ({
      added: acc.added + (l.startsWith("+") && !l.startsWith("+++") ? 1 : 0),
      removed: acc.removed + (l.startsWith("-") && !l.startsWith("---") ? 1 : 0),
    }),
    { added: 0, removed: 0 }
  );
}

export function ReviewView({ daemonUrl, path, onClose }: ReviewViewProps) {
  const [files, setFiles] = useState<FileDiff[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Files start expanded; a large review is navigable by collapsing what's
  // already been read.
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await invoke<{ diff: string }>("git_diff", { url: daemonUrl, path });
        if (!cancelled) setFiles(splitDiff(res.diff));
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [daemonUrl, path]);

  const toggle = (p: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (!next.delete(p)) next.add(p);
      return next;
    });

  const totals = (files ?? []).reduce(
    (acc, f) => {
      const c = countChanges(f.lines);
      return { added: acc.added + c.added, removed: acc.removed + c.removed };
    },
    { added: 0, removed: 0 }
  );

  return (
    <div className="vx-review">
      <div className="vx-review__head">
        <span>Review changes</span>
        {files && files.length > 0 && (
          <span className="vx-review__totals">
            {files.length} file{files.length === 1 ? "" : "s"}
            <span className="vx-diff__stat-add">+{totals.added}</span>
            <span className="vx-diff__stat-del">−{totals.removed}</span>
          </span>
        )}
        <div className="vx-review__spacer" />
        <button className="vx-icon-btn" aria-label="Close review" onClick={onClose}>
          <X size={14} />
        </button>
      </div>
      <div className="vx-review__body">
        {error && <div className="vx-review__empty">Failed to load diff: {error}</div>}
        {!error && files === null && <div className="vx-review__empty">Loading diff…</div>}
        {!error && files !== null && files.length === 0 && (
          <div className="vx-review__empty">No changes in the working tree.</div>
        )}
        {files?.map((f) => {
          const { added, removed } = countChanges(f.lines);
          const isOpen = !collapsed.has(f.path);
          return (
            <div key={f.path} className="vx-diff">
              <button
                className="vx-diff__file"
                onClick={() => toggle(f.path)}
                aria-expanded={isOpen}
                title={f.path}
              >
                {isOpen ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                <span className="vx-diff__file-path">{f.path}</span>
                <span className="vx-diff__stat-add">+{added}</span>
                <span className="vx-diff__stat-del">−{removed}</span>
              </button>
              {isOpen && (
                <pre className="vx-diff__code">
                  {f.lines.map((line, i) => (
                    <div key={i} className={lineClass(line)}>
                      {line || " "}
                    </div>
                  ))}
                </pre>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
