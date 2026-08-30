/**
 * "This file lives inside an archive — extract it to edit?"
 *
 * Files opened from inside a `.zip` / `.tar.gz` / … are read-only: writing a
 * member back in place would mean re-encoding a container we do not
 * necessarily understand, and a half-understood rewrite of someone's `.vsix`
 * or `.apk` is a worse outcome than refusing. So an attempted edit routes
 * here, and the escape hatch is an extraction: the whole archive is unpacked
 * into a sibling folder named after it, and the file the user was trying to
 * edit opens from there — writable, with its neighbours intact so relative
 * imports still resolve.
 *
 * The plan is fetched *before* anything is written (`plan_archive_extraction`)
 * so the prompt can name the exact folder that is about to appear, including
 * the `-1` suffix when a previous extraction is still sitting there. Nothing
 * on disk changes until the user picks Extract.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./Icon";
import { errorMessage } from "../utils/errorMessage";
import "./Modal.css";

/** Mirror of `commands::ArchiveExtractPlan`. */
export interface ArchiveExtractPlan {
  archive: string;
  archive_name: string;
  member: string;
  destination: string;
  destination_name: string;
  member_destination: string | null;
  renamed_to_avoid_collision: boolean;
}

/** Mirror of `commands::ArchiveExtractResult`. */
export interface ArchiveExtractResult {
  destination: string;
  files: number;
  directories: number;
  skipped: number;
  bytes: number;
  opened_path: string | null;
}

interface ArchiveExtractModalProps {
  /** Virtual path of the member the user tried to edit (`a.zip!/src/x.rs`),
   *  or the archive itself when extraction was asked for directly. `null`
   *  keeps the modal closed. */
  path: string | null;
  onCancel: () => void;
  onExtracted: (result: ArchiveExtractResult) => void;
}

/** `loading → ready → extracting`, or `failed` from either. One field, so the
 *  "spinner and an error at the same time" state cannot be represented. */
type Phase =
  | { kind: "loading" }
  | { kind: "ready"; plan: ArchiveExtractPlan }
  | { kind: "extracting"; plan: ArchiveExtractPlan }
  | { kind: "failed"; error: string };

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
};

export function ArchiveExtractModal({ path, onCancel, onExtracted }: ArchiveExtractModalProps) {
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  const overlayRef = useRef<HTMLDivElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!path) return;
    let cancelled = false;
    setPhase({ kind: "loading" });
    invoke<ArchiveExtractPlan>("plan_archive_extraction", { path })
      .then(plan => {
        if (!cancelled) setPhase({ kind: "ready", plan });
      })
      .catch(e => {
        if (!cancelled) setPhase({ kind: "failed", error: errorMessage(e) ?? "Could not read the archive" });
      });
    // A plan is a read of the archive's header, not a write, so an abandoned
    // one costs nothing beyond the read already in flight.
    return () => {
      cancelled = true;
    };
  }, [path]);

  useEffect(() => {
    if (phase.kind === "ready") confirmRef.current?.focus();
  }, [phase.kind]);

  const extract = useCallback(async () => {
    if (phase.kind !== "ready") return;
    const plan = phase.plan;
    setPhase({ kind: "extracting", plan });
    try {
      const result = await invoke<ArchiveExtractResult>("extract_archive", { path });
      onExtracted(result);
    } catch (e) {
      setPhase({ kind: "failed", error: errorMessage(e) ?? "Extraction failed" });
    }
  }, [phase, path, onExtracted]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
        return;
      }
      if (e.key !== "Tab" || !overlayRef.current) return;
      const focusable = overlayRef.current.querySelectorAll<HTMLElement>(
        'button, [href], [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    },
    [onCancel],
  );

  if (!path) return null;

  const busy = phase.kind === "extracting";

  return (
    <div
      className="modal-overlay"
      ref={overlayRef}
      role="dialog"
      aria-modal="true"
      aria-labelledby="archive-extract-title"
      onKeyDown={handleKeyDown}
    >
      <div className="modal-content" style={{ width: 520 }}>
        <h3 id="archive-extract-title" style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Icon name="package" size={16} />
          Extract to edit?
        </h3>

        {phase.kind === "loading" && <p>Reading the archive…</p>}

        {phase.kind === "failed" && (
          <p style={{ color: "var(--accent-red, #f14c4c)" }}>{phase.error}</p>
        )}

        {(phase.kind === "ready" || phase.kind === "extracting") && (
          <>
            <p style={{ marginBottom: 12 }}>
              {phase.plan.member ? (
                <>
                  <code>{phase.plan.member}</code> is inside{" "}
                  <strong>{phase.plan.archive_name}</strong>, so it is open read-only.
                  Files inside an archive cannot be edited in place.
                </>
              ) : (
                <>
                  <strong>{phase.plan.archive_name}</strong> will be unpacked so its
                  contents can be edited.
                </>
              )}
            </p>
            <div
              style={{
                background: "var(--bg-primary)",
                border: "1px solid var(--border-color)",
                borderRadius: 4,
                padding: "10px 12px",
                marginBottom: 12,
                fontSize: 13,
              }}
            >
              <div style={{ color: "var(--text-secondary)", marginBottom: 4 }}>
                Extract the whole archive into
              </div>
              <div style={{ fontFamily: "var(--font-mono, monospace)", wordBreak: "break-all" }}>
                {phase.plan.destination}
              </div>
              {phase.plan.member_destination && (
                <div style={{ color: "var(--text-secondary)", marginTop: 8 }}>
                  and open{" "}
                  <span style={{ fontFamily: "var(--font-mono, monospace)" }}>
                    {phase.plan.member}
                  </span>{" "}
                  from there for editing.
                </div>
              )}
            </div>
            {phase.plan.renamed_to_avoid_collision && (
              <p style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                <Icon name="info" size={12} />{" "}
                <strong>{stripSuffix(phase.plan.destination_name)}</strong> already exists —
                this extraction goes to <strong>{phase.plan.destination_name}</strong> rather
                than writing over it.
              </p>
            )}
            <p style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              The archive itself is not modified.
            </p>
          </>
        )}

        <div className="modal-actions">
          <button className="btn-secondary" onClick={onCancel} disabled={busy}>
            {phase.kind === "failed" ? "Close" : "Keep read-only"}
          </button>
          {phase.kind !== "failed" && (
            <button
              className="btn-primary"
              ref={confirmRef}
              onClick={extract}
              disabled={phase.kind !== "ready"}
            >
              {busy ? "Extracting…" : "Extract and edit"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/** `bundle-3` → `bundle`. Only used to name the folder that forced the rename. */
function stripSuffix(name: string): string {
  return name.replace(/-\d+$/, "");
}

/** Human-readable summary of a finished extraction, for the toast. */
export function describeExtraction(result: ArchiveExtractResult): string {
  const parts = [`${result.files} file${result.files === 1 ? "" : "s"}`];
  if (result.directories > 0) {
    parts.push(`${result.directories} folder${result.directories === 1 ? "" : "s"}`);
  }
  parts.push(formatBytes(result.bytes));
  const skipped =
    result.skipped > 0 ? ` (${result.skipped} unsafe or non-file entries skipped)` : "";
  return `Extracted ${parts.join(", ")}${skipped}`;
}
