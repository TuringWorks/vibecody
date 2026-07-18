import { classifyMemoryError } from "../lib/memoryError";

/**
 * Standard error display for the Memory panels — shows the raw message
 * with role="alert" so AT users hear it, plus a classified hint card
 * (when one applies). One implementation across MemoryPanel,
 * OpenMemoryPanel, MemoryProjectionsPanel, etc., so the user sees the
 * same shape no matter which panel surfaced the error.
 */
export function MemoryErrorCard({ error }: { error: string | null }) {
  if (!error) return null;
  const c = classifyMemoryError(error);
  return (
    <div role="alert" style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 12 }}>
      <div className="panel-error">{c.message}</div>
      {c.hint && (
        <div
          data-testid="memory-error-hint"
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
          <strong>Hint:</strong> {c.hint}
        </div>
      )}
    </div>
  );
}
