import { useEffect, useRef } from "react";
import "./Modal.css";

interface ConfirmationDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  busy?: boolean;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/** Consistent, keyboard-accessible replacement for blocking browser confirm(). */
export function ConfirmationDialog({
  open,
  title,
  message,
  confirmLabel = "Confirm",
  busy = false,
  danger = false,
  onConfirm,
  onCancel,
}: ConfirmationDialogProps) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;
    previousFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    cancelRef.current?.focus();
    return () => previousFocus.current?.focus();
  }, [open]);

  if (!open) return null;
  return (
    <div
      className="modal-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirmation-dialog-title"
      aria-describedby="confirmation-dialog-message"
      onKeyDown={(event) => {
        if (event.key === "Escape" && !busy) onCancel();
      }}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onCancel();
      }}
    >
      <div className="modal-content">
        <h3 id="confirmation-dialog-title">{title}</h3>
        <p id="confirmation-dialog-message">{message}</p>
        <div className="modal-actions">
          <button ref={cancelRef} type="button" className="panel-btn panel-btn-secondary" onClick={onCancel} disabled={busy}>
            Cancel
          </button>
          <button
            type="button"
            className={`panel-btn ${danger ? "panel-btn-danger" : "panel-btn-primary"}`}
            onClick={onConfirm}
            disabled={busy}
          >
            {busy ? "Working…" : confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
