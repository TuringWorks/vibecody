/**
 * useToast — lightweight toast notification hook for VibeUI.
 *
 * Usage:
 *   const { toast } = useToast();
 *   toast.success("File saved!");
 *   toast.error("Failed to open: " + err);
 *   toast.info("Workspace loaded.");
 *   toast.warn("No active provider set.");
 */

import { useState, useCallback, useEffect, useRef } from "react";

export type ToastVariant = "success" | "error" | "info" | "warn";

export interface Toast {
  id: number;
  message: string;
  variant: ToastVariant;
}

let _nextId = 1;

export interface ToastApi {
  success: (message: string) => void;
  error:   (message: string) => void;
  info:    (message: string) => void;
  warn:    (message: string) => void;
}

/** Duration (ms) before a toast auto-dismisses. Errors stay 6 s, others 3 s. */
const DURATION: Record<ToastVariant, number> = {
  success: 3000,
  info:    3000,
  warn:    4000,
  error:   6000,
};

export function useToast(): { toasts: Toast[]; toast: ToastApi; dismiss: (id: number) => void } {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timersRef = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  useEffect(() => {
    const timers = timersRef.current;
    return () => {
      timers.forEach(t => clearTimeout(t));
      timers.clear();
    };
  }, []);

  const dismiss = useCallback((id: number) => {
    setToasts(prev => prev.filter(t => t.id !== id));
    const timer = timersRef.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timersRef.current.delete(id);
    }
  }, []);

  const add = useCallback((message: string, variant: ToastVariant) => {
    const id = _nextId++;
    setToasts(prev => [...prev, { id, message, variant }]);
    const timer = setTimeout(() => {
      timersRef.current.delete(id);
      dismiss(id);
    }, DURATION[variant]);
    timersRef.current.set(id, timer);
  }, [dismiss]);

  const toast: ToastApi = {
    success: (msg) => add(msg, "success"),
    error:   (msg) => add(msg, "error"),
    info:    (msg) => add(msg, "info"),
    warn:    (msg) => add(msg, "warn"),
  };

  return { toasts, toast, dismiss };
}
