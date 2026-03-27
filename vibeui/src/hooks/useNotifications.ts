/**
 * useNotifications — centralized notification state for VibeUI.
 *
 * Stores a list of app-level notifications (API key health, build failures, etc.)
 * that persist across panel switches and can be viewed in the NotificationCenter.
 *
 * Usage:
 *   const { notifications, add, markRead, markAllRead, dismiss, unreadCount } = useNotifications();
 *   add({ title: "API key expired", body: "OpenAI key returned 401", severity: "error", category: "api-keys" });
 */

import { useState, useCallback, useMemo } from "react";

export type NotificationSeverity = "info" | "warn" | "error" | "success";
export type NotificationCategory = "api-keys" | "system" | "build" | "git" | "provider" | "general";

export interface AppNotification {
  id: number;
  title: string;
  body: string;
  severity: NotificationSeverity;
  category: NotificationCategory;
  timestamp: number;
  read: boolean;
  /** Optional action label + callback */
  action?: { label: string; onClick: () => void };
}

export interface AddNotificationOpts {
  title: string;
  body: string;
  severity: NotificationSeverity;
  category: NotificationCategory;
  action?: { label: string; onClick: () => void };
}

let _nextNotifId = 1;

/** Maximum number of notifications retained in memory. */
const MAX_NOTIFICATIONS = 100;

export function useNotifications() {
  const [notifications, setNotifications] = useState<AppNotification[]>([]);

  const add = useCallback((opts: AddNotificationOpts): AppNotification => {
    const notif: AppNotification = {
      id: _nextNotifId++,
      title: opts.title,
      body: opts.body,
      severity: opts.severity,
      category: opts.category,
      timestamp: Date.now(),
      read: false,
      action: opts.action,
    };
    setNotifications(prev => [notif, ...prev].slice(0, MAX_NOTIFICATIONS));
    return notif;
  }, []);

  const markRead = useCallback((id: number) => {
    setNotifications(prev => prev.map(n => n.id === id ? { ...n, read: true } : n));
  }, []);

  const markAllRead = useCallback(() => {
    setNotifications(prev => prev.map(n => ({ ...n, read: true })));
  }, []);

  const dismiss = useCallback((id: number) => {
    setNotifications(prev => prev.filter(n => n.id !== id));
  }, []);

  const clearCategory = useCallback((category: NotificationCategory) => {
    setNotifications(prev => prev.filter(n => n.category !== category));
  }, []);

  const unreadCount = useMemo(() => notifications.filter(n => !n.read).length, [notifications]);

  return {
    notifications,
    add,
    markRead,
    markAllRead,
    dismiss,
    clearCategory,
    unreadCount,
  };
}
