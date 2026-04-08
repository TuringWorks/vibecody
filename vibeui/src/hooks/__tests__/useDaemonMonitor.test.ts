/**
 * BDD / TDD tests for useDaemonMonitor.
 *
 * Given/When/Then comments map directly to the scenario names in the hook's
 * README and the feature specification above each describe block.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useDaemonMonitor } from '../useDaemonMonitor';

// ── Mock @tauri-apps/api/core ─────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeToast() {
  return {
    success: vi.fn(),
    warn:    vi.fn(),
    error:   vi.fn(),
    info:    vi.fn(),
  };
}

function makeNotify() {
  return vi.fn();
}

/** Mock fetch to return HTTP 200 (daemon online) or throw (daemon offline). */
function mockFetchOnline() {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true }));
}
function mockFetchOffline() {
  vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('ECONNREFUSED')));
}

// ── Setup / teardown ──────────────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers();
  mockInvoke.mockReset();
  // Default: daemon starts online
  mockFetchOnline();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

// ─────────────────────────────────────────────────────────────────────────────
// BDD Scenario 1: Daemon already online on first check
// ─────────────────────────────────────────────────────────────────────────────

describe('Given the daemon is already running when VibeUI starts', () => {
  it('When the first health check fires, Then a success toast is shown', async () => {
    const toast = makeToast();
    const notify = makeNotify();
    renderHook(() => useDaemonMonitor({ toast, addNotification: notify }));

    // Advance past the initial 3-second delay
    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(toast.success).toHaveBeenCalledOnce();
    expect(toast.success.mock.calls[0][0]).toContain('7878');
  });

  it('When the first check fires, Then invoke("start_daemon") is NOT called', async () => {
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('When a subsequent poll finds daemon still online, Then no additional toast fires', async () => {
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    const firstCount = toast.success.mock.calls.length;

    // Advance another full poll interval
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    expect(toast.success.mock.calls.length).toBe(firstCount);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BDD Scenario 2: Daemon is offline on first check — auto-start
// ─────────────────────────────────────────────────────────────────────────────

describe('Given the daemon is NOT running when VibeUI starts', () => {
  beforeEach(() => {
    mockFetchOffline();
    mockInvoke.mockResolvedValue('started');
  });

  it('When the first health check fires, Then invoke("start_daemon") is called', async () => {
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(mockInvoke).toHaveBeenCalledWith('start_daemon');
  });

  it('When start_daemon returns "started", Then no error toast is shown', async () => {
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(toast.error).not.toHaveBeenCalled();
    expect(toast.warn).not.toHaveBeenCalled();
  });

  it('When still offline on second tick while starting, Then start_daemon is NOT called again (guard)', async () => {
    // Return "starting" from the very first call so startingRef stays true.
    mockInvoke.mockResolvedValue('starting');

    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    // First tick — triggers start, returns "starting" → startingRef remains true
    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    const callsAfterFirst = mockInvoke.mock.calls.length;
    expect(callsAfterFirst).toBe(1);

    // Second tick — startingRef is still true, must NOT call start_daemon again
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    expect(mockInvoke.mock.calls.length).toBe(callsAfterFirst);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BDD Scenario 3: vibecli binary not installed
// ─────────────────────────────────────────────────────────────────────────────

describe('Given vibecli is not installed (start_daemon throws)', () => {
  beforeEach(() => {
    mockFetchOffline();
    mockInvoke.mockRejectedValue(new Error('vibecli not found'));
  });

  it('When start_daemon throws, Then a warning notification is added', async () => {
    const toast = makeToast();
    const notify = makeNotify();
    renderHook(() => useDaemonMonitor({ toast, addNotification: notify }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(notify).toHaveBeenCalledOnce();
    const call = notify.mock.calls[0][0];
    expect(call.severity).toBe('warn');
    expect(call.title).toContain('unavailable');
  });

  it('When start_daemon throws, Then a warn toast is shown with install hint', async () => {
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(toast.warn).toHaveBeenCalledOnce();
    expect(toast.warn.mock.calls[0][0]).toMatch(/vibecli|install/i);
  });

  it('When start_daemon throws, Then startingRef is reset so next poll retries', async () => {
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    const callsAfterFirst = mockInvoke.mock.calls.length;

    // Reset mock to succeed this time
    mockInvoke.mockResolvedValue('started');

    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    // Should have retried on next tick
    expect(mockInvoke.mock.calls.length).toBeGreaterThan(callsAfterFirst);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BDD Scenario 4: Daemon was online, then goes offline
// ─────────────────────────────────────────────────────────────────────────────

describe('Given daemon was running but went offline during a session', () => {
  it('When daemon goes offline after being online, Then start_daemon is invoked', async () => {
    mockFetchOnline();
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    // First check — online
    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    expect(mockInvoke).not.toHaveBeenCalled();

    // Daemon goes down
    mockFetchOffline();
    mockInvoke.mockResolvedValue('started');

    // Next poll
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    expect(mockInvoke).toHaveBeenCalledWith('start_daemon');
  });

  it('When daemon comes back online after recovery, Then success toast fires', async () => {
    mockFetchOnline();
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    // First check — online
    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    // Goes offline
    mockFetchOffline();
    mockInvoke.mockResolvedValue('started');
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    // Comes back
    mockFetchOnline();
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    const successCalls = toast.success.mock.calls.map(c => c[0] as string);
    const recoveryMsg = successCalls.find(m => m.toLowerCase().includes('back') || m.toLowerCase().includes('recover'));
    expect(recoveryMsg).toBeDefined();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BDD Scenario 5: Hook return values
// ─────────────────────────────────────────────────────────────────────────────

describe('Return values', () => {
  it('online reflects current daemon health', async () => {
    mockFetchOnline();
    const { result } = renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    expect(result.current.online).toBe(false); // not yet checked
    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    expect(result.current.online).toBe(true);
  });

  it('lastChecked is null before first poll and a number after', async () => {
    const { result } = renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    expect(result.current.lastChecked).toBeNull();
    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    expect(typeof result.current.lastChecked).toBe('number');
  });

  it('recheck() triggers an immediate health check', async () => {
    mockFetchOffline();
    mockInvoke.mockResolvedValue('started');
    const { result } = renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    const callsAfter = mockInvoke.mock.calls.length;

    // Manual recheck while still offline
    mockFetchOffline();
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue('started');
    // Reset startingRef by forcing online briefly
    // (simulate daemon coming back and going offline again)
    mockFetchOnline();
    await act(async () => { result.current.recheck(); });
    await act(async () => {});
    // Online now, so no invoke
    expect(result.current.online).toBe(true);
    void callsAfter; // used to avoid lint warning
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// TDD: vibeui:daemon-status custom event
// ─────────────────────────────────────────────────────────────────────────────

describe('vibeui:daemon-status custom event', () => {
  it('is dispatched on every check with online and checkedAt', async () => {
    mockFetchOnline();
    const events: CustomEvent[] = [];
    window.addEventListener('vibeui:daemon-status', (e) => events.push(e as CustomEvent));

    renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(events.length).toBeGreaterThanOrEqual(1);
    expect(events[0].detail.online).toBe(true);
    expect(typeof events[0].detail.checkedAt).toBe('number');
  });

  it('carries online: false when daemon is down', async () => {
    mockFetchOffline();
    mockInvoke.mockResolvedValue('started');
    const events: CustomEvent[] = [];
    window.addEventListener('vibeui:daemon-status', (e) => events.push(e as CustomEvent));

    renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(events.length).toBeGreaterThanOrEqual(1);
    expect(events[0].detail.online).toBe(false);
  });
});
