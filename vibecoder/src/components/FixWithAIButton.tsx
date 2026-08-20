/**
 * The "Fix with AI" button every finding panel shares.
 *
 * It writes the change request into the chat composer and says so — the
 * composer is usually behind another tab, so a button that silently succeeded
 * looked broken and got clicked again. Nothing is edited here; the user reads
 * the request and presses send.
 */
import React, { useEffect, useState } from "react";
import {
  FIX_BATCH_LIMIT,
  fixLabel,
  sendFixToChat,
  type FixItem,
  type FixRequestOptions,
} from "../lib/fixWithAI";

export interface FixWithAIButtonProps extends FixRequestOptions {
  /** What to hand over. Empty disables the button rather than sending nothing. */
  items: FixItem[];
  /**
   * Changes when a new run replaces the findings, clearing "Sent to chat ✓".
   * Without it the button of a fresh run still claims the last one was sent.
   */
  resetKey?: unknown;
  /**
   * Overrides the label. For a panel showing a per-item button *and* a
   * hand-off-everything button at once: with one finding both would otherwise
   * read "Fix with AI", and two identical buttons a line apart do different
   * things.
   */
  label?: string;
  title?: string;
  style?: React.CSSProperties;
}

/** An outline, never a primary action — the primary action is running the tool. */
const baseStyle: React.CSSProperties = {
  padding: "2px 8px",
  fontSize: "var(--font-size-xs)",
  borderRadius: 3,
  border: "1px solid var(--accent-blue)",
  background: "none",
  color: "var(--accent-blue)",
  cursor: "pointer",
  flexShrink: 0,
  whiteSpace: "nowrap",
};

export function FixWithAIButton({
  items,
  source,
  total,
  instructions,
  resetKey,
  label,
  title,
  style,
}: FixWithAIButtonProps) {
  const [sent, setSent] = useState(false);

  useEffect(() => setSent(false), [resetKey]);

  const empty = items.length === 0;
  const batch = items.slice(0, FIX_BATCH_LIMIT);
  const count = total ?? items.length;

  return (
    <button
      onClick={(e) => {
        e.stopPropagation();
        if (sendFixToChat(batch, { source, total: count, instructions })) setSent(true);
      }}
      disabled={empty}
      title={
        title ??
        (items.length > FIX_BATCH_LIMIT
          ? `Write a fix request for the first ${FIX_BATCH_LIMIT} of these ${items.length} findings into the chat composer`
          : "Write a fix request into the chat composer — you read it and press send")
      }
      style={{ ...baseStyle, ...(empty ? { opacity: 0.5, cursor: "default" } : null), ...style }}
    >
      {sent ? "Sent to chat ✓" : label ?? fixLabel(items.length)}
    </button>
  );
}

export default FixWithAIButton;
