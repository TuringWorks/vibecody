/**
 * GeneratedFileList — the reviewable result of any design-to-code generator.
 *
 * Figma import, screenshot-to-app and the Import tab all produce the same
 * thing: a list of proposed files that the user reads and then chooses to
 * write. Each of them had grown its own copy of this list, and they had
 * drifted — one could write files, one could only copy them, and one wrote
 * every file the model named without asking.
 *
 * Nothing here writes on its own. `write_file` resolves the destination
 * against the open workspace and refuses anything that escapes it, so a
 * model-chosen `../../.ssh/config` is rejected on the Rust side rather than
 * trusted here.
 */
import { useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface GeneratedFile {
  path: string;
  content: string;
  language?: string;
  /** True when the generator named no path and `path` is a placeholder. */
  path_inferred?: boolean;
}

/** Where one file stands. `idle` is not a state a file is *left* in after a write. */
type WriteState = "idle" | "writing" | "done" | "error";

interface Props {
  files: GeneratedFile[];
  workspacePath: string | null;
  /** Called with the paths actually written, once a write batch finishes. */
  onWritten?: (paths: string[]) => void;
  /** Surface an error to the host panel's toast/banner. */
  onError?: (message: string) => void;
  /** Open a written file in the editor. */
  onOpenFile?: (path: string, line?: number) => void;
}

const LANG_TONE: Record<string, string> = {
  tsx: "var(--accent-color)",
  jsx: "var(--accent-color)",
  ts: "var(--accent-color)",
  typescript: "var(--accent-color)",
  js: "var(--warning-color)",
  javascript: "var(--warning-color)",
  vue: "var(--success-color)",
  svelte: "var(--error-color)",
  html: "var(--error-color)",
  css: "var(--accent-blue)",
};

/** Extension-derived language label when the generator did not supply one. */
function languageOf(file: GeneratedFile): string {
  if (file.language) return file.language;
  const ext = file.path.split(".").pop();
  return ext && ext !== file.path ? ext : "text";
}

/** Join a workspace root and a relative destination without doubling the slash. */
function joinPath(root: string, relative: string): string {
  return root.endsWith("/") ? root + relative : `${root}/${relative}`;
}

export function GeneratedFileList({ files, workspacePath, onWritten, onError, onOpenFile }: Props) {
  const [expanded, setExpanded] = useState<number | null>(files.length === 1 ? 0 : null);
  const [status, setStatus] = useState<Record<number, WriteState>>({});
  const [pathEdits, setPathEdits] = useState<Record<number, string>>({});
  const [busy, setBusy] = useState(false);

  const unnamed = useMemo(() => files.filter((f) => f.path_inferred).length, [files]);

  const destinationFor = useCallback(
    (idx: number) => (pathEdits[idx] ?? files[idx]?.path ?? "").trim(),
    [pathEdits, files],
  );

  /** Write one file. Returns the path written, or null if it did not land. */
  const writeOne = useCallback(
    async (idx: number, root: string): Promise<string | null> => {
      const destination = destinationFor(idx);
      if (!destination) {
        setStatus((prev) => ({ ...prev, [idx]: "error" }));
        onError?.("Give the file a destination path before writing it.");
        return null;
      }
      setStatus((prev) => ({ ...prev, [idx]: "writing" }));
      try {
        await invoke("write_file", {
          path: joinPath(root, destination),
          content: files[idx].content,
        });
        setStatus((prev) => ({ ...prev, [idx]: "done" }));
        return destination;
      } catch (e) {
        setStatus((prev) => ({ ...prev, [idx]: "error" }));
        onError?.(`Failed to write ${destination}: ${e}`);
        return null;
      }
    },
    [destinationFor, files, onError],
  );

  const writeAll = async () => {
    if (!workspacePath) {
      onError?.("No workspace folder open.");
      return;
    }
    setBusy(true);
    // Sequential on purpose: two generated files can share a directory, and a
    // partial failure should stop at a known point rather than interleave.
    const written: string[] = [];
    for (let i = 0; i < files.length; i++) {
      const path = await writeOne(i, workspacePath);
      if (path) written.push(path);
    }
    setBusy(false);
    window.dispatchEvent(new Event("vibecoder:refresh-files"));
    onWritten?.(written);
  };

  const writeSingle = async (idx: number) => {
    if (!workspacePath) {
      onError?.("No workspace folder open.");
      return;
    }
    const path = await writeOne(idx, workspacePath);
    window.dispatchEvent(new Event("vibecoder:refresh-files"));
    if (path) onWritten?.([path]);
  };

  const copy = (text: string) => {
    navigator.clipboard
      .writeText(text)
      .catch((e) => onError?.(`Copy failed: ${e}`));
  };

  if (files.length === 0) return null;

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "var(--space-2)", marginBottom: "var(--space-2)", flexWrap: "wrap" }}>
        <span style={{ color: "var(--text-success)", fontWeight: 600, fontSize: "var(--font-size-base)" }}>
          {files.length} file{files.length === 1 ? "" : "s"} generated
        </span>
        <button
          type="button"
          className="panel-btn panel-btn-primary panel-btn-sm"
          onClick={writeAll}
          disabled={!workspacePath || busy}
          title={workspacePath ? `Write all ${files.length} files into ${workspacePath}` : "Open a workspace folder first"}
        >
          {busy ? "Writing…" : "Write All to Project"}
        </button>
      </div>

      {!workspacePath && (
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--warning-color)", marginBottom: "var(--space-2)" }}>
          No workspace folder is open — files can be copied but not written.
        </div>
      )}

      {unnamed > 0 && (
        <div style={{ color: "var(--text-secondary)", marginBottom: "var(--space-2)", fontSize: "var(--font-size-sm)", lineHeight: 1.5 }}>
          {unnamed} file{unnamed === 1 ? "" : "s"} came back without a path and{" "}
          {unnamed === 1 ? "was" : "were"} named after the code {unnamed === 1 ? "it declares" : "they declare"}.
          Edit the destination before writing.
        </div>
      )}

      {files.map((file, idx) => {
        const state = status[idx] ?? "idle";
        const lang = languageOf(file);
        return (
          <div key={`${file.path}:${idx}`} style={{ border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", marginBottom: "var(--space-2)", overflow: "hidden" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "8px 10px", background: "var(--bg-secondary)" }}>
              <button
                type="button"
                aria-expanded={expanded === idx}
                aria-label={`${expanded === idx ? "Collapse" : "Expand"} ${file.path}`}
                onClick={() => setExpanded(expanded === idx ? null : idx)}
                style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", padding: 0, width: 14 }}
              >
                {expanded === idx ? "▾" : "▸"}
              </button>
              <span style={{ background: LANG_TONE[lang] ?? "var(--text-secondary)", color: "var(--bg-primary)", padding: "1px 6px", borderRadius: 3, fontSize: "var(--font-size-xs)", fontWeight: 700 }}>
                {lang.toUpperCase()}
              </span>
              <input
                aria-label={`Destination for generated file ${idx + 1}`}
                value={destinationFor(idx)}
                onChange={(e) => setPathEdits((prev) => ({ ...prev, [idx]: e.target.value }))}
                placeholder="path/to/file.tsx"
                title="Relative to the workspace root — edit to retarget an existing file"
                style={{ flex: 1, minWidth: 0, background: "var(--bg-primary)", color: "var(--text-primary)", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", padding: "2px 6px", border: "1px solid var(--border-color)", borderRadius: 3 }}
              />
              <button type="button" className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => copy(file.content)}>
                Copy
              </button>
              <button
                type="button"
                className={`panel-btn panel-btn-sm ${state === "error" ? "panel-btn-secondary" : "panel-btn-primary"}`}
                onClick={() => writeSingle(idx)}
                disabled={!workspacePath || state === "writing" || busy}
              >
                {state === "writing" ? "…" : state === "done" ? "Written" : state === "error" ? "Retry" : "Write"}
              </button>
              {state === "done" && onOpenFile && workspacePath && (
                <button
                  type="button"
                  className="panel-btn panel-btn-secondary panel-btn-sm"
                  onClick={() => onOpenFile(joinPath(workspacePath, destinationFor(idx)))}
                >
                  Open
                </button>
              )}
            </div>
            {expanded === idx && (
              <pre style={{ margin: 0, padding: "10px", background: "var(--bg-primary)", overflow: "auto", maxHeight: 320, fontSize: "var(--font-size-sm)", lineHeight: 1.5, color: "var(--text-primary)", whiteSpace: "pre" }}>
                <code>{file.content}</code>
              </pre>
            )}
          </div>
        );
      })}
    </div>
  );
}
