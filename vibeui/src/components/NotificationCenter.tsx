/**
 * NotificationCenter — bell icon + dropdown panel showing recent app notifications.
 *
 * Rendered in the App header. Shows unread badge. Clicking opens a dropdown
 * with notification list grouped by severity. Each notification can be dismissed
 * or acted upon.
 */

import { useState, useRef, useEffect } from "react";
import { Bell, X, CheckCheck, AlertTriangle, AlertCircle, Info, CheckCircle } from "lucide-react";
import type { AppNotification } from "../hooks/useNotifications";
import "./NotificationCenter.css";

interface NotificationCenterProps {
  notifications: AppNotification[];
  unreadCount: number;
  onMarkRead: (id: number) => void;
  onMarkAllRead: () => void;
  onDismiss: (id: number) => void;
  onAction?: (id: number) => void;
}

const SEVERITY_ICON: Record<string, typeof AlertCircle> = {
  error: AlertCircle,
  warn: AlertTriangle,
  info: Info,
  success: CheckCircle,
};

const SEVERITY_COLOR: Record<string, string> = {
  error: "var(--error-color, #f87171)",
  warn: "var(--warning-color, #fbbf24)",
  info: "var(--info-color, #60a5fa)",
  success: "var(--success-color, #34d399)",
};

function formatTimeAgo(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 60_000) return "just now";
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)}h ago`;
  return `${Math.floor(diff / 86400_000)}d ago`;
}

export function NotificationCenter({
  notifications,
  unreadCount,
  onMarkRead,
  onMarkAllRead,
  onDismiss,
}: NotificationCenterProps) {
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [open]);

  return (
    <div className="notification-center" ref={panelRef}>
      <button
        className="notification-center__bell"
        onClick={() => setOpen(!open)}
        aria-label={`Notifications${unreadCount > 0 ? ` (${unreadCount} unread)` : ""}`}
        title="Notifications"
      >
        <Bell size={16} strokeWidth={1.5} />
        {unreadCount > 0 && (
          <span className="notification-center__badge">
            {unreadCount > 99 ? "99+" : unreadCount}
          </span>
        )}
      </button>

      {open && (
        <div className="notification-center__panel" role="region" aria-label="Notifications">
          <div className="notification-center__header">
            <span className="notification-center__title">Notifications</span>
            <div className="notification-center__actions">
              {unreadCount > 0 && (
                <button
                  className="notification-center__action-btn"
                  onClick={onMarkAllRead}
                  title="Mark all as read"
                >
                  <CheckCheck size={14} /> Read all
                </button>
              )}
            </div>
          </div>

          <div className="notification-center__list">
            {notifications.length === 0 ? (
              <div className="notification-center__empty">
                <Bell size={24} strokeWidth={1} style={{ opacity: 0.3 }} />
                <span>No notifications</span>
              </div>
            ) : (
              notifications.map(n => {
                const Icon = SEVERITY_ICON[n.severity] || Info;
                return (
                  <div
                    key={n.id}
                    className={`notification-center__item ${n.read ? "" : "notification-center__item--unread"}`}
                    onClick={() => { if (!n.read) onMarkRead(n.id); }}
                  >
                    <div className="notification-center__item-icon" style={{ color: SEVERITY_COLOR[n.severity] }}>
                      <Icon size={14} />
                    </div>
                    <div className="notification-center__item-content">
                      <div className="notification-center__item-title">{n.title}</div>
                      <div className="notification-center__item-body">{n.body}</div>
                      <div className="notification-center__item-meta">
                        <span className="notification-center__item-time">{formatTimeAgo(n.timestamp)}</span>
                        <span className="notification-center__item-category">{n.category}</span>
                      </div>
                      {n.action && (
                        <button
                          className="notification-center__item-action"
                          onClick={(e) => { e.stopPropagation(); n.action!.onClick(); }}
                        >
                          {n.action.label}
                        </button>
                      )}
                    </div>
                    <button
                      className="notification-center__item-dismiss"
                      onClick={(e) => { e.stopPropagation(); onDismiss(n.id); }}
                      aria-label="Dismiss notification"
                    >
                      <X size={12} />
                    </button>
                  </div>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
}
