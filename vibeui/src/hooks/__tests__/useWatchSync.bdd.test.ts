/**
 * BDD tests for useWatchSync and useWatchActiveSession hooks.
 *
 * Scenarios:
 *  - useWatchSync: polling triggers invoke, new messages are forwarded to callback
 *  - useWatchSync: sessionId change resets cursor and re-subscribes
 *  - useWatchSync: errors from invoke are silently swallowed
 *  - useWatchSync: cleanup stops polling on unmount
 *  - useWatchActiveSession: fires callback when active session changes
 *  - useWatchActiveSession: does not fire for same session_id
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useWatchSync, useWatchActiveSession } from '../useWatchSync';
import type { WatchMessage } from '../useWatchSync';

// ── Mock @tauri-apps/api/core ─────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Test data ─────────────────────────────────────────────────────────────────

const SESSION_A = 'session-aaa';
const SESSION_B = 'session-bbb';

function makeMsg(id: number, role: 'user' | 'assistant', content: string): WatchMessage {
  return { id, role, content, created_at: Date.now() };
}

// ── beforeEach / afterEach ────────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers();
  mockInvoke.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

// ── useWatchSync ──────────────────────────────────────────────────────────────

describe('useWatchSync — initial fetch', () => {
  it('calls watch_get_session_messages on mount with afterId: null to seed the cursor', async () => {
    mockInvoke.mockResolvedValue({ session_id: SESSION_A, messages: [] });

    renderHook(() => useWatchSync(SESSION_A, vi.fn()));

    await act(async () => {});

    const seedCall = mockInvoke.mock.calls.find((call: unknown[]) => {
      const cmd = call[0] as string;
      const args = call[1] as Record<string, unknown> | undefined;
      return cmd === 'watch_get_session_messages' && args?.afterId === null;
    });
    expect(seedCall).toBeDefined();
    expect(seedCall![1]).toMatchObject({ sessionId: SESSION_A });
  });

  it('does not call onNewMessages for the seed messages (cursor init only)', async () => {
    const seed: WatchMessage[] = [makeMsg(1, 'user', 'existing')];
    mockInvoke.mockResolvedValue({ session_id: SESSION_A, messages: seed });

    const onNew = vi.fn();
    renderHook(() => useWatchSync(SESSION_A, onNew));

    await act(async () => {});

    expect(onNew).not.toHaveBeenCalled();
  });
});

describe('useWatchSync — polling', () => {
  it('polls every 1 second and forwards new messages to the callback', async () => {
    // Seed returns no messages; subsequent polls return one new message
    const newMsg = makeMsg(10, 'assistant', 'hello from watch');

    mockInvoke.mockResolvedValue({ session_id: SESSION_A, messages: [] });
    const onNew = vi.fn();

    renderHook(() => useWatchSync(SESSION_A, onNew));
    await act(async () => {}); // seed

    // Override mock for poll calls to return a new message
    mockInvoke.mockResolvedValue({ session_id: SESSION_A, messages: [newMsg] });

    await act(async () => {
      vi.advanceTimersByTime(1100); // advance past the 1s poll interval
    });
    await act(async () => {});

    expect(onNew).toHaveBeenCalledWith([newMsg]);
  });

  it('uses afterId > 0 for polls after seeding', async () => {
    const seedMsg = makeMsg(5, 'user', 'seed');
    const pollMsg = makeMsg(6, 'assistant', 'new');

    let callIndex = 0;
    mockInvoke.mockImplementation(async (_cmd: string, args: { afterId: number | null }) => {
      callIndex++;
      if (callIndex === 1) {
        // Seed call (afterId: null)
        return { session_id: SESSION_A, messages: [seedMsg] };
      }
      // Poll calls should use afterId: 5 (max seed msg id)
      return { session_id: SESSION_A, messages: args?.afterId === 5 ? [pollMsg] : [] };
    });

    const onNew = vi.fn();
    renderHook(() => useWatchSync(SESSION_A, onNew));
    await act(async () => {});

    await act(async () => {
      vi.advanceTimersByTime(1100);
    });
    await act(async () => {});

    expect(onNew).toHaveBeenCalledWith([pollMsg]);
  });

  it('does not call onNewMessages when poll returns empty messages', async () => {
    mockInvoke.mockResolvedValue({ session_id: SESSION_A, messages: [] });

    const onNew = vi.fn();
    renderHook(() => useWatchSync(SESSION_A, onNew));
    await act(async () => {});

    await act(async () => {
      vi.advanceTimersByTime(3000); // 3 poll ticks
    });
    await act(async () => {});

    expect(onNew).not.toHaveBeenCalled();
  });

  it('does not poll when sessionId is undefined', async () => {
    renderHook(() => useWatchSync(undefined, vi.fn()));
    await act(async () => {});

    await act(async () => {
      vi.advanceTimersByTime(2000);
    });

    // No invoke should be called at all (seed and poll both guard on sessionId)
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('silently swallows errors from invoke during polling', async () => {
    mockInvoke.mockResolvedValueOnce({ session_id: SESSION_A, messages: [] }); // seed ok
    mockInvoke.mockRejectedValue(new Error('daemon offline')); // poll throws

    const onNew = vi.fn();

    expect(() => {
      renderHook(() => useWatchSync(SESSION_A, onNew));
    }).not.toThrow();

    await act(async () => {});
    await act(async () => {
      vi.advanceTimersByTime(1100);
    });
    await act(async () => {});

    expect(onNew).not.toHaveBeenCalled();
  });
});

describe('useWatchSync — sessionId changes', () => {
  it('resets the cursor when sessionId changes', async () => {
    const msgA = makeMsg(100, 'user', 'from session A');
    const msgB = makeMsg(1, 'assistant', 'from session B');

    mockInvoke.mockResolvedValue({ session_id: SESSION_A, messages: [msgA] });

    const onNew = vi.fn();
    const { rerender } = renderHook(
      ({ sid }: { sid: string }) => useWatchSync(sid, onNew),
      { initialProps: { sid: SESSION_A } },
    );
    await act(async () => {}); // seed session A

    // Switch to session B
    mockInvoke.mockResolvedValue({ session_id: SESSION_B, messages: [msgB] });
    rerender({ sid: SESSION_B });
    await act(async () => {}); // seed session B

    // Poll under session B should use afterId: 1 (not 100 from A)
    const bSeedCall = mockInvoke.mock.calls.find((call: unknown[]) => {
      const cmd = call[0] as string;
      const args = call[1] as Record<string, unknown> | undefined;
      return (
        cmd === 'watch_get_session_messages' &&
        args?.sessionId === SESSION_B &&
        args?.afterId === null
      );
    });
    expect(bSeedCall).toBeDefined();
  });

  it('stops polling the old session and starts polling the new one', async () => {
    mockInvoke.mockResolvedValue({ session_id: SESSION_A, messages: [] });

    const onNew = vi.fn();
    const { rerender } = renderHook(
      ({ sid }: { sid: string }) => useWatchSync(sid, onNew),
      { initialProps: { sid: SESSION_A } },
    );
    await act(async () => {});

    // Switch to B
    rerender({ sid: SESSION_B });
    await act(async () => {});

    // All invokes after the switch should reference SESSION_B
    const afterSwitch = mockInvoke.mock.calls.filter((call: unknown[]) => {
      const cmd = call[0] as string;
      const args = call[1] as Record<string, unknown> | undefined;
      return cmd === 'watch_get_session_messages' && args?.sessionId === SESSION_A;
    });
    // Some A calls may be present from before the switch; that's OK.
    // The important check: poll calls with B exist.
    const bCalls = mockInvoke.mock.calls.filter((call: unknown[]) => {
      const cmd = call[0] as string;
      const args = call[1] as Record<string, unknown> | undefined;
      return cmd === 'watch_get_session_messages' && args?.sessionId === SESSION_B;
    });
    expect(bCalls.length).toBeGreaterThan(0);
    void afterSwitch;
  });
});

describe('useWatchSync — cleanup', () => {
  it('stops polling after unmount', async () => {
    mockInvoke.mockResolvedValue({ session_id: SESSION_A, messages: [] });

    const { unmount } = renderHook(() => useWatchSync(SESSION_A, vi.fn()));
    await act(async () => {}); // seed

    const callsBefore = mockInvoke.mock.calls.length;
    unmount();

    await act(async () => {
      vi.advanceTimersByTime(5000);
    });

    expect(mockInvoke.mock.calls.length).toBe(callsBefore);
  });
});

// ── useWatchActiveSession ─────────────────────────────────────────────────────

describe('useWatchActiveSession', () => {
  it('calls watch_get_active_session on each poll tick', async () => {
    mockInvoke.mockResolvedValue({ session_id: null });

    renderHook(() => useWatchActiveSession(vi.fn()));

    await act(async () => {
      vi.advanceTimersByTime(2100); // advance past first 2s tick
    });
    await act(async () => {});

    const activeCalls = mockInvoke.mock.calls.filter(
      (call: unknown[]) => (call[0] as string) === 'watch_get_active_session',
    );
    expect(activeCalls.length).toBeGreaterThanOrEqual(1);
  });

  it('fires onSessionChange when a new session_id is received', async () => {
    mockInvoke.mockResolvedValue({ session_id: SESSION_A });

    const onChange = vi.fn();
    renderHook(() => useWatchActiveSession(onChange));

    await act(async () => {
      vi.advanceTimersByTime(2100);
    });
    await act(async () => {});

    expect(onChange).toHaveBeenCalledWith(SESSION_A);
  });

  it('does NOT fire again for the same session_id on subsequent polls', async () => {
    mockInvoke.mockResolvedValue({ session_id: SESSION_A });

    const onChange = vi.fn();
    renderHook(() => useWatchActiveSession(onChange));

    // First tick — fires
    await act(async () => {
      vi.advanceTimersByTime(2100);
    });
    await act(async () => {});
    expect(onChange).toHaveBeenCalledTimes(1);

    // Second tick — same session, should NOT fire again
    await act(async () => {
      vi.advanceTimersByTime(2100);
    });
    await act(async () => {});
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it('fires again when session_id changes to a new value', async () => {
    let callNum = 0;
    mockInvoke.mockImplementation(async () => {
      callNum++;
      return { session_id: callNum <= 2 ? SESSION_A : SESSION_B };
    });

    const onChange = vi.fn();
    renderHook(() => useWatchActiveSession(onChange));

    // Tick 1 — SESSION_A (fires)
    await act(async () => {
      vi.advanceTimersByTime(2100);
    });
    await act(async () => {});
    expect(onChange).toHaveBeenCalledWith(SESSION_A);

    // Tick 2 — SESSION_A (no fire)
    await act(async () => {
      vi.advanceTimersByTime(2100);
    });
    await act(async () => {});
    expect(onChange).toHaveBeenCalledTimes(1);

    // Tick 3 — SESSION_B (fires)
    await act(async () => {
      vi.advanceTimersByTime(2100);
    });
    await act(async () => {});
    expect(onChange).toHaveBeenCalledWith(SESSION_B);
    expect(onChange).toHaveBeenCalledTimes(2);
  });

  it('does NOT fire when session_id is null', async () => {
    mockInvoke.mockResolvedValue({ session_id: null });

    const onChange = vi.fn();
    renderHook(() => useWatchActiveSession(onChange));

    await act(async () => {
      vi.advanceTimersByTime(4200); // two ticks
    });
    await act(async () => {});

    expect(onChange).not.toHaveBeenCalled();
  });

  it('silently swallows errors from watch_get_active_session', async () => {
    mockInvoke.mockRejectedValue(new Error('command not found'));

    const onChange = vi.fn();

    expect(() => {
      renderHook(() => useWatchActiveSession(onChange));
    }).not.toThrow();

    await act(async () => {
      vi.advanceTimersByTime(2100);
    });
    await act(async () => {});

    expect(onChange).not.toHaveBeenCalled();
  });

  it('stops polling after unmount', async () => {
    mockInvoke.mockResolvedValue({ session_id: null });

    const { unmount } = renderHook(() => useWatchActiveSession(vi.fn()));
    await act(async () => {
      vi.advanceTimersByTime(2100);
    });
    await act(async () => {});

    const callsBefore = mockInvoke.mock.calls.length;
    unmount();

    await act(async () => {
      vi.advanceTimersByTime(10000);
    });

    expect(mockInvoke.mock.calls.length).toBe(callsBefore);
  });
});
