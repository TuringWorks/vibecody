// useWatchSync.ts — real-time Watch ↔ VibeUI bidirectional session sync.
//
// Two mechanisms:
//  1. Polling: checks sessions.db for new Watch messages every POLL_INTERVAL_MS.
//  2. Events (optional): subscribes to /watch/events SSE for instant push.
//
// Usage: call useWatchSync(sessionId, onNewMessages, onSessionChange) in AIChat.

import { useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

const POLL_INTERVAL_MS = 1000; // 1 second — near real-time like Google Docs

export interface WatchMessage {
  id: number;
  role: 'user' | 'assistant' | 'system';
  content: string;
  created_at: number;
}

interface WatchSessionMessages {
  session_id: string;
  messages: WatchMessage[];
}

export function useWatchSync(
  sessionId: string | undefined,
  onNewMessages: (msgs: WatchMessage[]) => void,
) {
  const lastIdRef = useRef<number>(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const onNewMessagesRef = useRef(onNewMessages);
  onNewMessagesRef.current = onNewMessages;

  const poll = useCallback(async () => {
    if (!sessionId) return;
    try {
      const result = await invoke<WatchSessionMessages>('watch_get_session_messages', {
        sessionId,
        afterId: lastIdRef.current,
      });
      if (result.messages.length > 0) {
        const maxId = Math.max(...result.messages.map(m => m.id));
        lastIdRef.current = maxId;
        onNewMessagesRef.current(result.messages);
      }
    } catch {
      // Silently ignore — daemon may not be running
    }
  }, [sessionId]);

  useEffect(() => {
    // Reset cursor when session changes
    lastIdRef.current = 0;
  }, [sessionId]);

  useEffect(() => {
    if (!sessionId) return;
    // Initial fetch to set lastId without surfacing existing messages to caller
    invoke<WatchSessionMessages>('watch_get_session_messages', {
      sessionId,
      afterId: null,
    }).then(result => {
      if (result.messages.length > 0) {
        lastIdRef.current = Math.max(...result.messages.map(m => m.id));
      }
    }).catch(() => {});

    const interval = setInterval(poll, POLL_INTERVAL_MS);
    timerRef.current = interval;
    return () => clearInterval(interval);
  }, [sessionId, poll]);
}

// ── Watch active-session hook ─────────────────────────────────────────────────
// Polls the daemon for which session the Watch is currently viewing.
// Call this at the top level (e.g. ChatTabManager) to auto-switch tabs.

export interface WatchActiveSession {
  session_id: string | null;
}

const ACTIVE_SESSION_POLL_MS = 2000;

export function useWatchActiveSession(
  onSessionChange: (sessionId: string) => void,
) {
  const lastSessionRef = useRef<string | null>(null);
  const onChangeRef = useRef(onSessionChange);
  onChangeRef.current = onSessionChange;

  useEffect(() => {
    const poll = async () => {
      try {
        const result = await invoke<WatchActiveSession>('watch_get_active_session');
        const sid = result.session_id;
        if (sid && sid !== lastSessionRef.current) {
          lastSessionRef.current = sid;
          onChangeRef.current(sid);
        }
      } catch {
        // Daemon not running or command not available
      }
    };

    const interval = setInterval(poll, ACTIVE_SESSION_POLL_MS);
    return () => clearInterval(interval);
  }, []);
}
