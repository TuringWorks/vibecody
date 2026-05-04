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
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { DiffReviewPanel } from "./DiffReviewPanel";

interface AdditionalFile {
  path: string;
  content: string;
}

/**
 * Classify a raw error string into a user-actionable message + hint.
 *
 * The backend surfaces failures as plain strings (Tauri command boundary —
 * we don't carry typed errors across the IPC). This mapper inspects
 * stable substrings and attaches a short next-step hint so users see
 * "what do I do about this" instead of just a stack-trace line.
 *
 * Exported for testing.
 */
export function classifyDiffCompleteError(raw: string): { message: string; hint?: string } {
  const lower = raw.toLowerCase();
  if (lower.includes("no active ai provider")) {
    return {
      message: raw,
      hint: "Open Settings → API Keys and add a provider key. Diffcomplete needs at least one configured provider.",
    };
  }
  if (lower.includes("did not contain a diff")) {
    return {
      message: raw,
      hint: "The model didn't return a unified diff. Try a different provider or simplify your instruction.",
    };
  }
  if (lower.includes("could not be applied cleanly")) {
    return {
      message: raw,
      // Avoid the phrase "try again" so the hint copy doesn't collide
      // with the "Try again" button label in the same view (would cause
      // tests using getByText(/Try again/) to find two matches).
      hint: "The diff didn't match your file. Use Regenerate with a refinement to nudge the model.",
    };
  }
  if (
    lower.includes("not available") ||
    lower.includes("network") ||
    lower.includes("connection") ||
    lower.includes("timeout")
  ) {
    return {
      message: raw,
      hint: "Check your internet connection or switch to a different provider.",
    };
  }
  if (lower.includes("rate limit") || lower.includes("quota") || lower.includes("429")) {
    return {
      message: raw,
      hint: "Provider rate limit hit — wait a moment, or switch to a different provider.",
    };
  }
  if (lower.includes("401") || lower.includes("unauthorized") || lower.includes("invalid api key")) {
    return {
      message: raw,
      hint: "The provider rejected the API key. Check it in Settings → API Keys.",
    };
  }
  return { message: raw };
}

/**
 * Cycle focus among the modal's focusable elements when Tab/Shift+Tab
 * would otherwise leave the modal. Keeps keyboard users inside the
 * dialog until they explicitly close it (Escape or click backdrop).
 *
 * Exported for testing.
 */
export function trapFocusInside(container: HTMLElement, e: KeyboardEvent | React.KeyboardEvent): boolean {
  if (e.key !== "Tab") return false;
  // Selector covers the standard focusable elements. We deliberately don't
  // filter by offsetParent / getClientRects — jsdom returns null/empty for
  // both even on visible elements, breaking tests. The modal renders a
  // controlled set of focusables so visibility filtering would be
  // redundant in practice.
  const focusables = Array.from(
    container.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
    )
  );
  if (focusables.length === 0) return false;
  const first = focusables[0];
  const last = focusables[focusables.length - 1];
  const active = document.activeElement as HTMLElement | null;
  if (e.shiftKey) {
    if (active === first || !container.contains(active)) {
      e.preventDefault();
      last.focus();
      return true;
    }
  } else {
    if (active === last) {
      e.preventDefault();
      first.focus();
      return true;
    }
  }
  return false;
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
type ProviderStatus = "unknown" | "ready" | "no_providers";

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
  const [providerStatus, setProviderStatus] = useState<ProviderStatus>("unknown");
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const refineRef = useRef<HTMLTextAreaElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);

  // Memoize the classified error so we don't re-classify on every render
  // and so the message + hint stay stable for the aria-live announcer.
  const classifiedError = useMemo(
    () => (error ? classifyDiffCompleteError(error) : null),
    [error]
  );

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
      setProviderStatus("unknown");
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  // Empty-state probe: when the modal opens, confirm at least one AI
  // provider is configured. If the parent passed a non-empty `provider`
  // prop we trust it; otherwise we ask the daemon's `/health` endpoint
  // which exposes `features.diffcomplete.available` as the canonical
  // signal (see serve.rs `health()`). Failure → assume no providers and
  // render the empty-state — better than letting submit fail with a
  // bare "No active AI provider configured" string.
  useEffect(() => {
    if (!open) return;
    if (provider && provider.length > 0) {
      setProviderStatus("ready");
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const res = await fetch("http://127.0.0.1:7878/health", {
          signal: AbortSignal.timeout(800),
        });
        if (cancelled) return;
        if (!res.ok) {
          setProviderStatus("no_providers");
          return;
        }
        const body = await res.json() as { features?: { diffcomplete?: { available?: boolean } } };
        const ok = body?.features?.diffcomplete?.available === true;
        setProviderStatus(ok ? "ready" : "no_providers");
      } catch {
        if (!cancelled) setProviderStatus("no_providers");
      }
    })();
    return () => { cancelled = true; };
  }, [open, provider]);

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

  // aria-live announcer text — describes the current phase for screen
  // readers. Using `polite` avoids interrupting longer announcements.
  // Errors are NOT announced here: the visible error block uses
  // role="alert" which is implicitly aria-live=assertive and gets read
  // automatically. Duplicating it here would also break tests that use
  // getByText(/error message/) by producing two matching nodes.
  const liveMessage = (() => {
    if (phase === "loading") return "Generating diff…";
    if (phase === "review") return "Diff ready for review.";
    if (phase === "prompt" && providerStatus === "no_providers") {
      return "No AI provider is configured. Open Settings to add one.";
    }
    return "";
  })();

  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal="true"
      aria-label="AI edit (diff)"
      onKeyDown={e => {
        if (e.key === "Escape") { e.preventDefault(); onClose(); return; }
        if (dialogRef.current) trapFocusInside(dialogRef.current, e);
      }}
      style={{
        position: "fixed", inset: 0, background: "rgba(0,0,0,0.5)",
        zIndex: 1000, display: "flex", alignItems: "center", justifyContent: "center",
      }}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      {/* Screen-reader-only live region — visually hidden but announced
          on every state change. Keeps blind users in sync with phase
          transitions without forcing focus changes. */}
      <div
        aria-live="polite"
        aria-atomic="true"
        style={{
          position: "absolute", width: 1, height: 1,
          margin: -1, padding: 0, overflow: "hidden",
          clip: "rect(0,0,0,0)", whiteSpace: "nowrap", border: 0,
        }}
      >
        {liveMessage}
      </div>
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
      {phase === "prompt" && providerStatus === "no_providers" && (
        <div
          className="panel-card"
          style={{ display: "flex", flexDirection: "column", gap: 12, padding: 20 }}
        >
          <h4 style={{ margin: 0, color: "var(--text-primary)" }}>
            No AI provider configured
          </h4>
          <p style={{ margin: 0, color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
            Diffcomplete needs at least one AI provider with an API key.
          </p>
          <p style={{ margin: 0, color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
            Open <strong>Settings → API Keys</strong> in the app, or run
            <code style={{ marginLeft: 6, padding: "2px 6px", background: "var(--bg-tertiary)", borderRadius: 4 }}>
              vibecli set-key &lt;provider&gt; &lt;value&gt;
            </code>
            in your terminal.
          </p>
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
            <button className="panel-btn panel-btn-primary" onClick={onClose} autoFocus>
              Close
            </button>
          </div>
        </div>
      )}

      {phase === "prompt" && providerStatus !== "no_providers" && (
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
          <div className="panel-error" role="alert">{classifiedError?.message ?? error}</div>
          {classifiedError?.hint && (
            <div
              data-testid="diffcomplete-error-hint"
              style={{
                color: "var(--text-secondary)",
                fontSize: "var(--font-size-sm)",
                fontStyle: "italic",
                padding: "6px 10px",
                background: "var(--bg-tertiary)",
                borderRadius: "var(--radius-sm)",
                borderLeft: "3px solid var(--accent-color)",
              }}
            >
              <strong>Hint:</strong> {classifiedError.hint}
            </div>
          )}
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
            <button className="panel-btn panel-btn-secondary" onClick={onClose}>Close</button>
            <button className="panel-btn panel-btn-primary" onClick={() => setPhase("prompt")} autoFocus>Try again</button>
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
