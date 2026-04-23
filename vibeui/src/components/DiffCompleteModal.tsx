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
import { DiffReviewPanel } from "./DiffReviewPanel";

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
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (open) {
      setPhase("prompt");
      setInstruction("");
      setError("");
      setModified("");
      setExplanation(null);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  const submit = useCallback(async () => {
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
      });

      const applied = applyUnifiedDiff(originalContent, res.unified_diff);
      if (applied === null) {
        setError("Model returned a diff that could not be applied cleanly.");
        setPhase("error");
        return;
      }
      setModified(applied);
      setExplanation(res.explanation);
      setPhase("review");
    } catch (e) {
      setError(String(e));
      setPhase("error");
    }
  }, [instruction, originalContent, selectionText, selectionStartLine,
      selectionEndLine, filePath, language, provider]);

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
          <div style={{ flex: 1, minHeight: 0 }}>
            <DiffReviewPanel
              original={originalContent}
              modified={modified}
              filePath={filePath}
              onApply={handleReviewApply}
            />
          </div>
        </div>
      )}
        </div>
      </div>
    </div>
  );
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
