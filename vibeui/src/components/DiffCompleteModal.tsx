/**
 * DiffCompleteModal — ⌘. diff-mode AI editing.
 *
 * Explicit-chord alternative to keystroke-driven ghost-text completion:
 * user opens the modal, types an instruction, the backend returns a unified
 * diff, and the review hands off to DiffReviewPanel for per-hunk accept/reject.
 *
 * The modal is never opened automatically — only via Monaco's ⌘. chord or
 * the equivalent command-palette entry.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { DiffReviewPanel } from "./DiffReviewPanel";

interface AdditionalFile {
  path: string;
  content: string;
}

export interface DiffCompleteModalProps {
  open: boolean;
  onClose: () => void;
  filePath: string;
  language: string;
  /** Full current file content, used as the "before" side for review. */
  originalContent: string;
  /** Selected text (empty string if nothing is selected). */
  selectionText: string;
  /** 1-based line numbers for the selection range. 0 if no selection. */
  selectionStartLine: number;
  selectionEndLine: number;
  /** Active provider id (e.g. "claude", "openai"). */
  provider: string;
  /** Called with the modified file content on apply; null means cancelled. */
  onApply: (modified: string | null) => void;
}

interface BackendResponse {
  unified_diff: string;
  explanation: string | null;
  model_name: string;
}

type Phase = "prompt" | "loading" | "review" | "error";

export function DiffCompleteModal(props: DiffCompleteModalProps) {
  const { open, onClose, filePath, language, originalContent, selectionText,
    selectionStartLine, selectionEndLine, provider, onApply } = props;

  const [phase, setPhase] = useState<Phase>("prompt");
  const [instruction, setInstruction] = useState("");
  const [error, setError] = useState<string>("");
  const [modified, setModified] = useState<string>("");
  const [explanation, setExplanation] = useState<string | null>(null);
  const [additionalFiles, setAdditionalFiles] = useState<AdditionalFile[]>([]);
  const [pickerBusy, setPickerBusy] = useState(false);
  const [lastDiff, setLastDiff] = useState<string>("");
  const [refinement, setRefinement] = useState<string>("");
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const refineRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (open) {
      setPhase("prompt");
      setInstruction("");
      setError("");
      setModified("");
      setExplanation(null);
      setAdditionalFiles([]);
      setPickerBusy(false);
      setLastDiff("");
      setRefinement("");
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  const addContextFiles = useCallback(async () => {
    setPickerBusy(true);
    try {
      const selected = await openDialog({
        multiple: true,
        title: "Add files as context",
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      const existing = new Set(additionalFiles.map(f => f.path));
      const additions: AdditionalFile[] = [];
      for (const path of paths) {
        if (existing.has(path)) continue;
        try {
          const content = await invoke<string>("read_file_sandbox", { path });
          additions.push({ path, content });
        } catch (e) {
          setError(`Failed to read "${path}": ${e}`);
        }
      }
      if (additions.length) setAdditionalFiles(prev => [...prev, ...additions]);
    } catch (e) {
      console.error("File picker error:", e);
    } finally {
      setPickerBusy(false);
    }
  }, [additionalFiles]);

  const removeContextFile = useCallback((path: string) => {
    setAdditionalFiles(prev => prev.filter(f => f.path !== path));
  }, []);

  const runGenerate = useCallback(async (opts: { previousDiff?: string; refinement?: string } = {}) => {
    if (!instruction.trim()) return;
    setPhase("loading");
    setError("");

    // Context window: 200 lines on each side of the selection (or around the file).
    const lines = originalContent.split("\n");
    const contextRadius = 200;
    const hasSelection = selectionText.length > 0;
    const startIdx = hasSelection ? Math.max(0, selectionStartLine - 1 - contextRadius) : 0;
    const selStartIdx = hasSelection ? Math.max(0, selectionStartLine - 1) : 0;
    const selEndIdx = hasSelection ? Math.min(lines.length, selectionEndLine) : 0;
    const endIdx = hasSelection
      ? Math.min(lines.length, selectionEndLine + contextRadius)
      : lines.length;

    const beforeContext = hasSelection ? lines.slice(startIdx, selStartIdx).join("\n") : lines.join("\n");
    const afterContext = hasSelection ? lines.slice(selEndIdx, endIdx).join("\n") : "";

    try {
      const res = await invoke<BackendResponse>("diffcomplete_generate", {
        filePath,
        language,
        selectionText: hasSelection ? selectionText : null,
        selectionStartLine: hasSelection ? selectionStartLine : null,
        selectionEndLine: hasSelection ? selectionEndLine : null,
        beforeContext,
        afterContext,
        instruction: instruction.trim(),
        provider,
        additionalFiles: additionalFiles.length ? additionalFiles : null,
        previousDiff: opts.previousDiff && opts.previousDiff.length ? opts.previousDiff : null,
        refinement: opts.refinement && opts.refinement.trim().length ? opts.refinement.trim() : null,
      });

      const applied = applyUnifiedDiff(originalContent, res.unified_diff);
      if (applied === null) {
        setError("Model returned a diff that could not be applied cleanly.");
        setPhase("error");
        return;
      }
      setModified(applied);
      setExplanation(res.explanation);
      setLastDiff(res.unified_diff);
      setPhase("review");
    } catch (e) {
      setError(String(e));
      setPhase("error");
    }
  }, [instruction, originalContent, selectionText, selectionStartLine,
      selectionEndLine, filePath, language, provider, additionalFiles]);

  const submit = useCallback(() => runGenerate(), [runGenerate]);

  const regenerate = useCallback(async () => {
    if (!refinement.trim() || !lastDiff) return;
    const refineNow = refinement.trim();
    const prevNow = lastDiff;
    setRefinement("");
    await runGenerate({ previousDiff: prevNow, refinement: refineNow });
  }, [refinement, lastDiff, runGenerate]);

  const handleReviewApply = useCallback((result: string | null) => {
    onApply(result);
    onClose();
  }, [onApply, onClose]);

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="AI edit (diff)"
      onKeyDown={e => { if (e.key === "Escape") { e.preventDefault(); onClose(); } }}
      style={{
        position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)",
        zIndex: 1000, display: "flex", alignItems: "center", justifyContent: "center",
      }}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div
        style={{
          background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
          borderRadius: "var(--radius-md)", width: "min(840px, 92vw)",
          maxHeight: "88vh", display: "flex", flexDirection: "column",
          boxShadow: "var(--shadow-xl, 0 24px 48px rgba(0,0,0,0.4))",
        }}
      >
        <div className="panel-header" style={{ minHeight: 40 }}>
          <h3 style={{ margin: 0, fontSize: "var(--font-size-lg)" }}>AI edit (diff)</h3>
          <button
            className="panel-btn panel-btn-secondary panel-btn-sm"
            style={{ marginLeft: "auto" }}
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </div>
        <div style={{ flex: 1, minHeight: 0, overflow: "auto", padding: 16 }}>
      {phase === "prompt" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div className="panel-label" style={{ color: "var(--text-secondary)" }}>
            {selectionText
              ? `Editing selection: lines ${selectionStartLine}-${selectionEndLine} of ${filePath}`
              : `Editing whole file: ${filePath}`}
          </div>
          <textarea
            ref={inputRef}
            className="panel-input panel-input-full panel-textarea"
            style={{ minHeight: 90, fontFamily: "var(--font-sans)" }}
            placeholder="Describe the change you want (e.g. 'extract this into a helper function')"
            value={instruction}
            onChange={e => setInstruction(e.target.value)}
            onKeyDown={e => {
              if ((e.metaKey || e.ctrlKey) && e.key === "Enter") { e.preventDefault(); submit(); }
              if (e.key === "Escape") { e.preventDefault(); onClose(); }
            }}
          />
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <button
                className="panel-btn panel-btn-secondary panel-btn-sm"
                onClick={addContextFiles}
                disabled={pickerBusy}
                aria-label="Add files as context"
              >
                {pickerBusy ? "Adding…" : "+ Add file…"}
              </button>
              <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                {additionalFiles.length === 0
                  ? "Optional: attach related files as extra context."
                  : `${additionalFiles.length} file${additionalFiles.length === 1 ? "" : "s"} attached`}
              </span>
            </div>
            {additionalFiles.length > 0 && (
              <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                {additionalFiles.map(f => (
                  <span
                    key={f.path}
                    className="panel-card"
                    style={{
                      display: "inline-flex", alignItems: "center", gap: 6,
                      padding: "2px 6px", fontSize: "var(--font-size-xs)",
                      fontFamily: "var(--font-mono)",
                    }}
                    title={f.path}
                  >
                    {shortPath(f.path)}
                    <button
                      className="panel-btn panel-btn-secondary panel-btn-sm"
                      style={{ padding: "0 4px", minHeight: 0, lineHeight: 1 }}
                      onClick={() => removeContextFile(f.path)}
                      aria-label={`Remove ${f.path}`}
                    >
                      ×
                    </button>
                  </span>
                ))}
              </div>
            )}
          </div>
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
            <button className="panel-btn panel-btn-secondary" onClick={onClose}>Cancel</button>
            <button
              className="panel-btn panel-btn-primary"
              disabled={!instruction.trim()}
              onClick={submit}
            >
              Generate diff (⌘⏎)
            </button>
          </div>
        </div>
      )}

      {phase === "loading" && (
        <div className="panel-loading" style={{ padding: 32, textAlign: "center" }}>
          Generating diff…
        </div>
      )}

      {phase === "error" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div className="panel-error">{error}</div>
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
            <button className="panel-btn panel-btn-secondary" onClick={onClose}>Close</button>
            <button className="panel-btn panel-btn-primary" onClick={() => setPhase("prompt")}>Try again</button>
          </div>
        </div>
      )}

      {phase === "review" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8, minHeight: 420 }}>
          {explanation && (
            <div className="panel-card" style={{ padding: 8, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
              {explanation}
            </div>
          )}
          <div className="panel-card" style={{
            padding: 8, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)",
            display: "flex", flexDirection: "column", gap: 2,
          }}>
            <div><strong>Original instruction:</strong> {instruction}</div>
          </div>
          <div style={{ flex: 1, minHeight: 0 }}>
            <DiffReviewPanel
              original={originalContent}
              modified={modified}
              filePath={filePath}
              language={language}
              onApply={handleReviewApply}
            />
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <label
              htmlFor="diffcomplete-refinement"
              className="panel-label"
              style={{ color: "var(--text-secondary)" }}
            >
              Refine this diff (layered on the original instruction)
            </label>
            <div style={{ display: "flex", gap: 8 }}>
              <textarea
                id="diffcomplete-refinement"
                ref={refineRef}
                className="panel-input panel-input-full panel-textarea"
                style={{ minHeight: 60, fontFamily: "var(--font-sans)" }}
                placeholder="e.g. tighten the error path, use the helper from utils"
                value={refinement}
                onChange={e => setRefinement(e.target.value)}
                onKeyDown={e => {
                  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
                    e.preventDefault();
                    void regenerate();
                  }
                }}
              />
              <button
                className="panel-btn panel-btn-secondary"
                disabled={!refinement.trim() || !lastDiff}
                onClick={regenerate}
                aria-label="Regenerate with refinement"
                style={{ alignSelf: "flex-end" }}
              >
                Regenerate (⌘⏎)
              </button>
            </div>
          </div>
        </div>
      )}
        </div>
      </div>
    </div>
  );
}

function shortPath(p: string): string {
  const parts = p.split(/[\\/]/).filter(Boolean);
  if (parts.length <= 2) return p;
  return `…/${parts.slice(-2).join("/")}`;
}

// ── Unified-diff applier ────────────────────────────────────────────────────

interface Hunk {
  oldStart: number;
  oldCount: number;
  lines: string[];
}

/**
 * Apply a unified diff to `original` and return the modified content, or null
 * if the diff cannot be applied cleanly (context mismatch, malformed hunks).
 *
 * Intentionally lenient on file-header lines (`--- a/..`, `+++ b/..`) — we
 * only care about hunks. Strict on context matching.
 */
export function applyUnifiedDiff(original: string, unifiedDiff: string): string | null {
  const hunks = parseHunks(unifiedDiff);
  if (hunks.length === 0) return null;

  const origLines = original.split("\n");
  const out: string[] = [];
  let cursor = 0; // 0-based index into origLines

  for (const hunk of hunks) {
    const hunkStart = Math.max(0, hunk.oldStart - 1);
    if (hunkStart < cursor) return null; // overlapping / out-of-order
    while (cursor < hunkStart) out.push(origLines[cursor++]);

    for (const line of hunk.lines) {
      if (line.length === 0) continue;
      const marker = line[0];
      const body = line.slice(1);
      if (marker === " ") {
        if (origLines[cursor] !== body) return null;
        out.push(origLines[cursor++]);
      } else if (marker === "-") {
        if (origLines[cursor] !== body) return null;
        cursor++;
      } else if (marker === "+") {
        out.push(body);
      } else if (marker === "\\") {
        // "\ No newline at end of file" — ignore
      } else {
        return null;
      }
    }
  }

  while (cursor < origLines.length) out.push(origLines[cursor++]);
  return out.join("\n");
}

function parseHunks(diff: string): Hunk[] {
  const hunks: Hunk[] = [];
  const lines = diff.split("\n");
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    const m = line.match(/^@@ -(\d+)(?:,(\d+))? \+\d+(?:,\d+)? @@/);
    if (!m) { i++; continue; }
    const oldStart = parseInt(m[1], 10);
    const oldCount = m[2] !== undefined ? parseInt(m[2], 10) : 1;
    i++;
    const body: string[] = [];
    while (i < lines.length && !lines[i].startsWith("@@ ") && !lines[i].startsWith("--- ")) {
      body.push(lines[i]);
      i++;
    }
    hunks.push({ oldStart, oldCount, lines: body });
  }
  return hunks;
}
