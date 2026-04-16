// useWatchSync.ts — polls sessions.db for Watch-originated messages and
// notifies VibeUI when new messages arrive (Watch → VibeUI live sync).
//
// Usage: call useWatchSync(sessionId, onNewMessages) in a chat component.
// The hook polls every POLL_INTERVAL_MS while the window is focused.
// onNewMessages is called with messages that have id > lastSeenId.

import { useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

const POLL_INTERVAL_MS = 3000;

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
    // Initial fetch to set lastId without surfacing existing messages
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
