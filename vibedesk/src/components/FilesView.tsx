import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X, FileCode } from "lucide-react";

interface FilesViewProps {
  daemonUrl: string;
  onClose: () => void;
}

/**
 * VX-110 — the Files quick-action: a flat, searchable list of the project's
 * tracked files (gitignore-correct, from the daemon's `git ls-files`). Codex
 * summons this on demand rather than keeping a persistent tree.
 */
export function FilesView({ daemonUrl, onClose }: FilesViewProps) {
  const [files, setFiles] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const res = await invoke<{ files: string[] }>("list_files", { url: daemonUrl });
        if (!cancelled) setFiles(res.files);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [daemonUrl]);

  const shown = (files ?? []).filter((f) => f.toLowerCase().includes(filter.toLowerCase())).slice(0, 500);

  return (
    <div className="vx-files">
      <div className="vx-files__head">
        <span>Files</span>
        <button className="vx-icon-btn" aria-label="Close files" onClick={onClose}>
          <X size={14} />
        </button>
      </div>
      <input
        className="vx-files__filter"
        placeholder="Filter files…"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
      />
      <div className="vx-files__body">
        {error && <div className="vx-files__empty">Failed to load files: {error}</div>}
        {!error && files === null && <div className="vx-files__empty">Loading…</div>}
        {!error && files !== null && shown.length === 0 && (
          <div className="vx-files__empty">No matching files.</div>
        )}
        <ul className="vx-files__list">
          {shown.map((f) => (
            <li key={f} className="vx-files__item" title={f}>
              <FileCode size={13} />
              <span className="vx-files__path">{f}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
