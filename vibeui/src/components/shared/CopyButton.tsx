import { useState } from "react";

interface CopyButtonProps {
  /** Text to copy to clipboard */
  text: string;
  /** Optional label shown before the copy icon */
  label?: string;
  /** Milliseconds to show "copied" feedback (default: 1500) */
  feedbackMs?: number;
}

/**
 * Shared copy-to-clipboard button used across panels.
 * Shows a brief "Copied" confirmation after clicking.
 */
export function CopyButton({ text, label = "", feedbackMs = 1500 }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  const handleClick = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), feedbackMs);
  };

  return (
    <button
      onClick={handleClick}
      title="Copy to clipboard"
      style={{
        fontSize: 10,
        padding: "2px 8px",
        cursor: "pointer",
        background: copied ? "var(--accent-color)" : "var(--bg-tertiary, #333)",
        color: copied ? "#fff" : "var(--text-secondary)",
        border: "1px solid var(--border-color)",
        borderRadius: 3,
        fontFamily: "inherit",
      }}
    >
      {copied ? "Copied" : label || "Copy"}
    </button>
  );
}
