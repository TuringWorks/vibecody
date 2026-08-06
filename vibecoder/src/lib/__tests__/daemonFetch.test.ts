/**
 * Tests for daemonFetch — bearer auth for daemon routes, resilient to the
 * daemon's per-restart token rotation.
 *
 * The bugs these pin down were both live:
 *  - Background Jobs called `/jobs`, `/agent`, `/v1/resume` with a plain
 *    `fetch()`. Every one returned 401 (verified against a running daemon),
 *    so the panel was entirely non-functional.
 *  - The tainted-argument confirmation modal took its token from a
 *    `VITE_DAEMON_TOKEN` env var nothing set, so the security prompt never
 *    appeared and the user's decision never reached the daemon.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { daemonFetch, getDaemonToken, resetDaemonTokenCache } from '../daemonFetch';

function res(status: number): Response {
  return { ok: status >= 200 && status < 300, status } as Response;
}

/** Authorization header of the Nth fetch call. */
function authOf(call: number): string | null {
  const init = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[call][1] as
    | RequestInit
    | undefined;
  return new Headers(init?.headers).get('Authorization');
}

beforeEach(() => {
  mockInvoke.mockReset();
  resetDaemonTokenCache();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('Given the daemon is running with a token', () => {
  it('When a daemon route is called, Then the bearer token is attached', async () => {
    mockInvoke.mockResolvedValue('tok-abc');
    const fetchMock = vi.fn().mockResolvedValue(res(200));
    vi.stubGlobal('fetch', fetchMock);

    const r = await daemonFetch('http://localhost:7878/jobs');

    expect(r.status).toBe(200);
    expect(mockInvoke).toHaveBeenCalledWith('daemon_auth_token');
    expect(authOf(0)).toBe('Bearer tok-abc');
  });

  it('When several calls are made, Then the token is read once and reused', async () => {
    mockInvoke.mockResolvedValue('tok-abc');
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(res(200)));

    await daemonFetch('http://localhost:7878/jobs');
    await daemonFetch('http://localhost:7878/v1/metrics/jobs');

    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it('When concurrent calls race, Then only one token read is issued', async () => {
    mockInvoke.mockResolvedValue('tok-abc');
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(res(200)));

    await Promise.all([
      daemonFetch('http://localhost:7878/jobs'),
      daemonFetch('http://localhost:7878/jobs'),
      daemonFetch('http://localhost:7878/jobs'),
    ]);

    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it('When caller supplies headers, Then they survive alongside the token', async () => {
    mockInvoke.mockResolvedValue('tok-abc');
    const fetchMock = vi.fn().mockResolvedValue(res(200));
    vi.stubGlobal('fetch', fetchMock);

    await daemonFetch('http://localhost:7878/agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    });

    const init = fetchMock.mock.calls[0][1] as RequestInit;
    const headers = new Headers(init.headers);
    expect(headers.get('Content-Type')).toBe('application/json');
    expect(headers.get('Authorization')).toBe('Bearer tok-abc');
    expect(init.method).toBe('POST');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Token rotation: `vibecli serve` mints a new random token on every start, and
// VibeCoder itself restarts the daemon on autostart. A cached token therefore
// goes stale routinely — not as an edge case.
// ─────────────────────────────────────────────────────────────────────────────

describe('Given the daemon restarted and rotated its token', () => {
  it('When a call 401s, Then the token is re-read and the call retried', async () => {
    mockInvoke.mockResolvedValueOnce('stale').mockResolvedValueOnce('fresh');
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(res(401))
      .mockResolvedValueOnce(res(200));
    vi.stubGlobal('fetch', fetchMock);

    const r = await daemonFetch('http://localhost:7878/jobs');

    expect(r.status).toBe(200);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(authOf(0)).toBe('Bearer stale');
    expect(authOf(1)).toBe('Bearer fresh');
  });

  it('When the re-read returns the same token, Then it does not retry forever', async () => {
    // A genuine auth failure, not a rotation. One retry attempt at most.
    mockInvoke.mockResolvedValue('same');
    const fetchMock = vi.fn().mockResolvedValue(res(401));
    vi.stubGlobal('fetch', fetchMock);

    const r = await daemonFetch('http://localhost:7878/jobs');

    expect(r.status).toBe(401);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('When a later call happens after a rotation, Then it uses the new token', async () => {
    mockInvoke.mockResolvedValueOnce('stale').mockResolvedValueOnce('fresh');
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(res(401))
      .mockResolvedValue(res(200));
    vi.stubGlobal('fetch', fetchMock);

    await daemonFetch('http://localhost:7878/jobs');
    await daemonFetch('http://localhost:7878/v1/metrics/jobs');

    expect(authOf(2)).toBe('Bearer fresh');
  });
});

describe('Given the daemon is not running', () => {
  it('When the token read fails, Then the request still goes out unauthenticated', async () => {
    // Surfacing the real transport error beats inventing an auth error.
    mockInvoke.mockRejectedValue(new Error('no token file'));
    const fetchMock = vi.fn().mockResolvedValue(res(503));
    vi.stubGlobal('fetch', fetchMock);

    const r = await daemonFetch('http://localhost:7878/jobs');

    expect(r.status).toBe(503);
    expect(authOf(0)).toBeNull();
  });

  it('When the token is empty, Then getDaemonToken reports null', async () => {
    mockInvoke.mockResolvedValue('');
    expect(await getDaemonToken()).toBeNull();
  });
});
