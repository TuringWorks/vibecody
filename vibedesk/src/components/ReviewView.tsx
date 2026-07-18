import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";

interface ReviewViewProps {
  daemonUrl: string;
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
export function ReviewView({ daemonUrl, onClose }: ReviewViewProps) {
  const [files, setFiles] = useState<FileDiff[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await invoke<{ diff: string }>("git_diff", { url: daemonUrl });
        if (!cancelled) setFiles(splitDiff(res.diff));
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [daemonUrl]);

  return (
    <div className="vx-review">
      <div className="vx-review__head">
        <span>Review changes</span>
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
        {files?.map((f) => (
          <div key={f.path} className="vx-diff">
            <div className="vx-diff__file">{f.path}</div>
            <pre className="vx-diff__code">
              {f.lines.map((line, i) => (
                <div key={i} className={lineClass(line)}>
                  {line || " "}
                </div>
              ))}
            </pre>
          </div>
        ))}
      </div>
    </div>
  );
}
