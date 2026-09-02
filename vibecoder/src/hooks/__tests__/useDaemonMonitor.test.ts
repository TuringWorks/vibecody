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

/**
 * How many times the hook asked the backend to *start* a daemon.
 *
 * These assertions used to read `expect(mockInvoke).not.toHaveBeenCalled()`,
 * which was only ever true by accident: `start_daemon` happened to be the
 * hook's one `invoke`. It now also asks `daemon_readiness_probe` whether the
 * token it holds will authenticate — a question no amount of `/health` polling
 * can answer — so the assertion has to name the command it actually means.
 */
function startDaemonCalls(): number {
  return mockInvoke.mock.calls.filter(([cmd]) => cmd === 'start_daemon').length;
}
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

/**
 * Mock fetch to look like the real daemon: HTTP 200 *and* a `/health` body
 * identifying the service. The hook requires both — `ok` alone is liveness,
 * not identity.
 */
function mockFetchOnline() {
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ service: 'vibecli', status: 'ok', version: '0.5.7' }),
    })
  );
}
function mockFetchOffline() {
  vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('ECONNREFUSED')));
}
/**
 * A different local service answering 200 JSON on the daemon's port. Must be
 * treated as "daemon offline", not as a healthy daemon.
 */
function mockFetchForeignService() {
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ status: 'ok', service: 'some-other-app' }),
    })
  );
}

/**
 * Advance to the point where a *sustained* outage has been recognised.
 *
 * `OFFLINE_AFTER_FAILURES` is 2. One missed probe is indistinguishable from a
 * daemon that is merely busy — a code-graph build pegging a core, a cold model
 * load — so the hook holds the first failure back and only reports offline when
 * the next probe fails too. That removed a class of false "offline"/"back
 * online" toast pairs, and a redundant `start_daemon` fired at a daemon that
 * was already running.
 *
 * Tests of the offline path therefore need two ticks: the initial delay, then
 * one poll interval. Tests of the *online* path still need only one.
 */
async function tickUntilOffline() {
  await act(async () => { vi.advanceTimersByTime(3100); });
  await act(async () => {});
  await act(async () => { vi.advanceTimersByTime(30_100); });
  await act(async () => {});
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

describe('Given the daemon is already running when VibeCoder starts', () => {
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

    expect(startDaemonCalls()).toBe(0);
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

describe('Given the daemon is NOT running when VibeCoder starts', () => {
  beforeEach(() => {
    mockFetchOffline();
    mockInvoke.mockResolvedValue('started');
  });

  it('When the first health check fires, Then invoke("start_daemon") is called', async () => {
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await tickUntilOffline();

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

    // Two ticks to be *believed* offline; the start fires on the second.
    await tickUntilOffline();
    const callsAfterFirst = mockInvoke.mock.calls.length;
    expect(callsAfterFirst).toBe(1);

    // Third tick — startingRef is still true, must NOT call start_daemon again
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

    await tickUntilOffline();

    expect(notify).toHaveBeenCalledOnce();
    const call = notify.mock.calls[0][0];
    expect(call.severity).toBe('warn');
    expect(call.title).toContain('unavailable');
  });

  it('When start_daemon throws, Then a warn toast is shown with install hint', async () => {
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await tickUntilOffline();

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

describe('Given a single probe fails while the daemon is merely busy', () => {
  it('When only one probe misses, Then nothing is reported and no restart is attempted', async () => {
    mockFetchOnline();
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    // One miss — a code-graph build pegging a core, not a dead daemon.
    mockFetchOffline();
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    expect(startDaemonCalls()).toBe(0);
    expect(toast.warn).not.toHaveBeenCalled();

    // It answers again on the next tick, and the user was never told anything.
    mockFetchOnline();
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    const backOnline = toast.success.mock.calls
      .map(c => c[0] as string)
      .filter(m => m.toLowerCase().includes('back'));
    expect(backOnline).toHaveLength(0);
  });
});

describe('Given daemon was running but went offline during a session', () => {
  it('When daemon goes offline after being online, Then start_daemon is invoked', async () => {
    mockFetchOnline();
    const toast = makeToast();
    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    // First check — online
    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    expect(startDaemonCalls()).toBe(0);

    // Daemon goes down
    mockFetchOffline();
    mockInvoke.mockResolvedValue('started');

    // Two failing polls: the first is held back as possible busyness.
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});
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

    // Goes offline — two failing polls before it is believed.
    mockFetchOffline();
    mockInvoke.mockResolvedValue('started');
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});
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
// TDD: vibecoder:daemon-status custom event
// ─────────────────────────────────────────────────────────────────────────────

describe('vibecoder:daemon-status custom event', () => {
  it('is dispatched on every check with online and checkedAt', async () => {
    mockFetchOnline();
    const events: CustomEvent[] = [];
    window.addEventListener('vibecoder:daemon-status', (e) => events.push(e as CustomEvent));

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
    window.addEventListener('vibecoder:daemon-status', (e) => events.push(e as CustomEvent));

    renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    await tickUntilOffline();

    // The held-back first failure emits no status at all — reporting "offline"
    // on one missed probe is exactly what the two-strike rule removed.
    expect(events.length).toBeGreaterThanOrEqual(1);
    expect(events[events.length - 1].detail.online).toBe(false);
  });
});

describe('Given an older daemon that predates the `service` field', () => {
  it('When the health check fires, Then it is still reported online', async () => {
    // Upgrade path: rejecting the legacy body made VibeCoder call a working
    // daemon offline, then blame "another program" for holding the port.
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ status: 'ok', version: '0.5.7' }),
      })
    );
    const events: CustomEvent[] = [];
    window.addEventListener('vibecoder:daemon-status', (e) => events.push(e as CustomEvent));

    renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(events[0].detail.online).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BDD Scenario: the daemon is healthy and our token will not authenticate
//
// The two-and-a-half-day outage this hook could not see. A second daemon on
// port 7979 bound its own free port, wrote its token into the shared,
// port-agnostic `~/.vibecli/daemon.token`, and exited. The daemon on 7878 kept
// answering `/health` perfectly — so this monitor said "online" on every tick,
// while every authenticated route in the app returned 401 and each panel
// invented its own explanation. Health polling cannot see a credential; the
// readiness probe compares the token's fingerprint against the daemon's.
// ─────────────────────────────────────────────────────────────────────────────

describe('Given a reachable daemon whose token no longer authenticates', () => {
  /** Readiness reporting a running daemon and a token that will not work. */
  function mockStaleToken() {
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === 'daemon_readiness_probe'
        ? Promise.resolve({
            port: 7878,
            ready: false,
            daemonRunning: true,
            daemonVersion: '0.5.11',
            clientVersion: '0.5.11',
            versionMatches: true,
            tokenState: 'stale',
            features: null,
            message:
              'The saved daemon token (1b7d…) is not the one the daemon on port 7878 is ' +
              'accepting (f392…). Restart the daemon on port 7878.',
          })
        : Promise.resolve('started'),
    );
  }

  it('When the daemon is online but the token is stale, Then the user is warned', async () => {
    mockFetchOnline();
    mockStaleToken();
    const toast = makeToast();

    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(toast.warn).toHaveBeenCalled();
    expect(toast.warn.mock.calls[0][0]).toContain('Restart the daemon');
    // And the daemon is still reported online, because it is.
    expect(startDaemonCalls()).toBe(0);
  });

  it('When the condition persists, Then it is said once, not every 30 seconds', async () => {
    mockFetchOnline();
    mockStaleToken();
    const toast = makeToast();

    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    // A stale token does not fix itself; repeating it is noise, and noise is
    // how a warning that matters gets ignored.
    expect(toast.warn).toHaveBeenCalledTimes(1);
  });

  it('When the token becomes valid again, Then a later failure is reported afresh', async () => {
    mockFetchOnline();
    mockStaleToken();
    const toast = makeToast();

    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));
    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});
    expect(toast.warn).toHaveBeenCalledTimes(1);

    // The user restarts the daemon; readiness goes good.
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === 'daemon_readiness_probe'
        ? Promise.resolve({ ready: true, tokenState: 'valid', message: 'ok' })
        : Promise.resolve('started'),
    );
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    // And it goes bad again later — the "said once" latch must have cleared,
    // or the second outage would be silent.
    mockStaleToken();
    await act(async () => { vi.advanceTimersByTime(30_100); });
    await act(async () => {});

    expect(toast.warn).toHaveBeenCalledTimes(2);
  });

  it('When the shell cannot answer a readiness probe, Then nothing is claimed', async () => {
    // An older shell without the command. Saying nothing is right; inventing a
    // credential problem would be the same substitution in reverse.
    mockFetchOnline();
    mockInvoke.mockRejectedValue(new Error('no such command'));
    const toast = makeToast();

    renderHook(() => useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(toast.warn).not.toHaveBeenCalled();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BDD Scenario: another program occupies the daemon port
//
// A bare `res.ok` check treated any 200 on port 7878 as a healthy daemon, so
// every panel then failed against a service that isn't VibeCLI. `/health`
// reports `service: "vibecli"` so clients can tell the two apart.
// ─────────────────────────────────────────────────────────────────────────────

describe('Given a different service is listening on the daemon port', () => {
  it('When the health check fires, Then the daemon is reported offline', async () => {
    mockFetchForeignService();
    mockInvoke.mockResolvedValue('started');
    const events: CustomEvent[] = [];
    window.addEventListener('vibecoder:daemon-status', (e) => events.push(e as CustomEvent));

    renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    await tickUntilOffline();

    expect(events.length).toBeGreaterThanOrEqual(1);
    expect(events[events.length - 1].detail.online).toBe(false);
  });

  it('When the health check fires, Then no "daemon is running" success toast is shown', async () => {
    mockFetchForeignService();
    mockInvoke.mockResolvedValue('started');
    const toast = makeToast();

    renderHook(() =>
      useDaemonMonitor({ toast, addNotification: makeNotify() }));

    await act(async () => { vi.advanceTimersByTime(3100); });
    await act(async () => {});

    expect(toast.success).not.toHaveBeenCalled();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BDD Scenario: /health answers 200 but the body is not JSON
// ─────────────────────────────────────────────────────────────────────────────

describe('Given the port answers 200 with a non-JSON body', () => {
  it('When the health check fires, Then it is treated as offline rather than throwing', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => {
          throw new SyntaxError('Unexpected token < in JSON');
        },
      })
    );
    mockInvoke.mockResolvedValue('started');
    const events: CustomEvent[] = [];
    window.addEventListener('vibecoder:daemon-status', (e) => events.push(e as CustomEvent));

    renderHook(() =>
      useDaemonMonitor({ toast: makeToast(), addNotification: makeNotify() }));

    await tickUntilOffline();

    expect(events.length).toBeGreaterThanOrEqual(1);
    expect(events[events.length - 1].detail.online).toBe(false);
  });
});
