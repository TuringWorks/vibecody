/**
 * ToggleSwitch — shared primitive for custom on/off toggles (US-012, A-4).
 *
 * Renders a `role="switch"` element with `aria-checked`, keyboard
 * activation (Space and Enter), and focus-visible outline inherited
 * from the global `:focus-visible` rule. Use this anywhere a panel
 * would otherwise build a custom `<div>`-styled iOS-style toggle.
 *
 * Pass `label` for accessible name (required — an unnamed switch is a
 * WCAG 4.1.2 violation). Visual label can live elsewhere; this prop
 * becomes `aria-label`.
 */
import type { CSSProperties, KeyboardEvent } from "react";

export interface ToggleSwitchProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
  disabled?: boolean;
  /** Optional override style for the track. */
  style?: CSSProperties;
}

const TRACK_WIDTH = 32;
const TRACK_HEIGHT = 18;
const THUMB_SIZE = 14;
const THUMB_INSET = (TRACK_HEIGHT - THUMB_SIZE) / 2;

export function ToggleSwitch({
  checked,
  onChange,
  label,
  disabled = false,
  style,
}: ToggleSwitchProps) {
  const handleActivate = () => {
    if (disabled) return;
    onChange(!checked);
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (disabled) return;
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      onChange(!checked);
    }
  };

  const trackStyle: CSSProperties = {
    display: "inline-block",
    position: "relative",
    width: TRACK_WIDTH,
    height: TRACK_HEIGHT,
    borderRadius: TRACK_HEIGHT / 2,
    background: checked ? "var(--accent-color)" : "var(--bg-tertiary)",
    border: "1px solid var(--border-color)",
    cursor: disabled ? "not-allowed" : "pointer",
    opacity: disabled ? 0.5 : 1,
    transition: "background var(--transition-fast, 150ms)",
    verticalAlign: "middle",
    ...style,
  };

  const thumbStyle: CSSProperties = {
    position: "absolute",
    top: THUMB_INSET,
    left: checked ? TRACK_WIDTH - THUMB_SIZE - THUMB_INSET - 1 : THUMB_INSET,
    width: THUMB_SIZE,
    height: THUMB_SIZE,
    borderRadius: "50%",
    background: "var(--bg-primary)",
    boxShadow: "0 1px 2px rgba(0, 0, 0, 0.2)",
    transition: "left var(--transition-fast, 150ms)",
  };

  return (
    <div
      role="switch"
      aria-checked={checked}
      aria-label={label}
      aria-disabled={disabled || undefined}
      tabIndex={0}
      style={trackStyle}
      onClick={handleActivate}
      onKeyDown={handleKeyDown}
    >
      <span style={thumbStyle} />
    </div>
  );
}
