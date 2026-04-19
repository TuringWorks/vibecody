/**
 * PanelError — shared primitive for error containers (US-015, A-2).
 *
 * Wraps error text in a `role="alert"` + `aria-live="assertive"`
 * container so screen readers announce the failure the moment it
 * mounts. Keeps the existing `.panel-error` class so visual styling
 * continues to come from App.css.
 *
 * Returns `null` for empty/falsy children so callers can write:
 *   <PanelError>{error}</PanelError>
 * without a surrounding `&&` — matching the ergonomics of the old
 * `error && <div className="panel-error">{error}</div>` pattern.
 *
 * Pass `onDismiss` to render a keyboard-accessible "Dismiss" button.
 */
import type { CSSProperties, ReactNode } from "react";

export interface PanelErrorProps {
  children: ReactNode;
  onDismiss?: () => void;
  style?: CSSProperties;
}

export function PanelError({ children, onDismiss, style }: PanelErrorProps) {
  if (!children) return null;
  return (
    <div role="alert" aria-live="assertive" className="panel-error" style={style}>
      <span>{children}</span>
      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss error"
        >
          ✕
        </button>
      )}
    </div>
  );
}
