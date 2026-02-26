/**
 * Toaster — renders the active toast stack in the bottom-right corner.
 *
 * Usage (in App.tsx):
 *   const { toasts, toast, dismiss } = useToast();
 *   ...
 *   <Toaster toasts={toasts} onDismiss={dismiss} />
 */

import type { Toast } from "../hooks/useToast";
import "./Toaster.css";

interface ToasterProps {
  toasts: Toast[];
  onDismiss: (id: number) => void;
}

const ICONS: Record<string, string> = {
  success: "✓",
  error:   "✕",
  warn:    "⚠",
  info:    "ℹ",
};

export function Toaster({ toasts, onDismiss }: ToasterProps) {
  if (toasts.length === 0) return null;
  return (
    <div className="toaster" role="region" aria-label="Notifications" aria-live="polite">
      {toasts.map(t => (
        <div key={t.id} className={`toast toast--${t.variant}`} role="alert">
          <span className="toast__icon">{ICONS[t.variant]}</span>
          <span className="toast__message">{t.message}</span>
          <button
            className="toast__close"
            onClick={() => onDismiss(t.id)}
            aria-label="Dismiss"
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
