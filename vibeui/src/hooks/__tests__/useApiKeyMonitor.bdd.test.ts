/**
 * BDD tests for useApiKeyMonitor — API key health polling with change-based notifications.
 *
 * Scenarios:
 *  1. On first run, failing keys trigger a warn toast and notification
 *  2. On first run, valid keys produce no toast
 *  3. "No key configured" is silently skipped on all runs
 *  4. Subsequent runs only fire notifications on state changes (valid→invalid)
 *  5. Subsequent runs fire success toast when a key recovers (invalid→valid)
 *  6. A key going invalid between polls fires error toast + notification
 *  7. vibeui:api-key-validations event is dispatched on every validation run
 *  8. "vibeui:providers-updated" event triggers a re-validation
 *  9. Multiple failing keys on first run batch warn toast and add individual notifications
 * 10. lastChecked is null before first poll and a number after
 * 11. revalidate() can be called manually
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { useApiKeyMonitor } from '../useApiKeyMonitor';
import type { ApiKeyValidation } from '../useApiKeyMonitor';

// ── Fixture helpers ───────────────────────────────────────────────────────────

function validResult(provider: string, latency = 80): ApiKeyValidation {
  return { provider, valid: true, error: null, latency_ms: latency };
}
function invalidResult(provider: string, error = 'HTTP 401'): ApiKeyValidation {
  return { provider, valid: false, error, latency_ms: 0 };
}
function unconfiguredResult(provider: string): ApiKeyValidation {
  return { provider, valid: false, error: 'No key configured', latency_ms: 0 };
}

function makeToast() {
  return {
    success: vi.fn(),
    warn:    vi.fn(),
    error:   vi.fn(),
    info:    vi.fn(),
  };
}
function makeAddNotification() {
  return vi.fn();
}

// ── Setup / teardown ──────────────────────────────────────────────────────────

beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const INITIAL_DELAY = 4000; // ms — from source

// ── Scenario 1: First run — failing keys trigger warn ─────────────────────────

describe('Given one API key is failing on first validation run', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue([invalidResult('openai', 'HTTP 401 Unauthorized')]);
  });

  it('When the initial delay fires, Then a warn toast is shown', async () => {
    const toast = makeToast();
    renderHook(() => useApiKeyMonitor({ toast, addNotification: makeAddNotification() }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    expect(toast.warn).toHaveBeenCalledOnce();
    expect(toast.warn.mock.calls[0][0]).toMatch(/OpenAI|401/i);
  });

  it('When the initial delay fires, Then a notification is added', async () => {
    const toast = makeToast();
    const addNotification = makeAddNotification();
    renderHook(() => useApiKeyMonitor({ toast, addNotification }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    expect(addNotification).toHaveBeenCalledOnce();
    const notif = addNotification.mock.calls[0][0];
    expect(notif.category).toBe('api-keys');
    expect(notif.severity).toBe('warn');
  });
});

// ── Scenario 2: First run — valid keys are silent ─────────────────────────────

describe('Given all API keys are valid on first run', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue([
      validResult('openai'),
      validResult('anthropic'),
    ]);
  });

  it('When the initial delay fires, Then no toast is shown', async () => {
    const toast = makeToast();
    renderHook(() => useApiKeyMonitor({ toast, addNotification: makeAddNotification() }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    expect(toast.warn).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
    expect(toast.success).not.toHaveBeenCalled();
  });
});

// ── Scenario 3: "No key configured" is silently skipped ───────────────────────

describe('Given a provider has "No key configured"', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue([
      unconfiguredResult('mistral'),
      validResult('openai'),
    ]);
  });

  it('When the validation runs, Then no toast is shown for the unconfigured key', async () => {
    const toast = makeToast();
    renderHook(() => useApiKeyMonitor({ toast, addNotification: makeAddNotification() }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    expect(toast.warn).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
  });
});

// ── Scenario 4: Subsequent run — no duplicate toast if already known-bad ─────

describe('Given a key was already failing on first run', () => {
  it('When the same key is still failing on the next poll, Then no new toast is added', async () => {
    mockInvoke.mockResolvedValue([invalidResult('groq', 'HTTP 401')]);

    const toast = makeToast();
    renderHook(() => useApiKeyMonitor({ toast, addNotification: makeAddNotification() }));

    // First run
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    const toastsAfterFirst = toast.warn.mock.calls.length;

    // Second run (5 minutes later)
    await act(async () => { vi.advanceTimersByTime(5 * 60 * 1000); });
    await act(async () => {});

    // warn count should not have increased
    expect(toast.warn.mock.calls.length).toBe(toastsAfterFirst);
    expect(toast.error).not.toHaveBeenCalled();
  });
});

// ── Scenario 5: Key recovers → success toast ─────────────────────────────────

describe('Given a key was failing on first run but recovers on the next poll', () => {
  it('When the key recovers, Then a success toast is shown', async () => {
    // First run: key failing
    mockInvoke.mockResolvedValueOnce([invalidResult('gemini')]);
    // Second run: key recovered
    mockInvoke.mockResolvedValueOnce([validResult('gemini', 120)]);

    const toast = makeToast();
    renderHook(() => useApiKeyMonitor({ toast, addNotification: makeAddNotification() }));

    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});

    await act(async () => { vi.advanceTimersByTime(5 * 60 * 1000); });
    await act(async () => {});

    expect(toast.success).toHaveBeenCalledOnce();
    expect(toast.success.mock.calls[0][0]).toMatch(/Gemini|recover/i);
  });
});

// ── Scenario 6: Key goes from valid to invalid → error toast ─────────────────

describe('Given a key was valid on first run but fails on the next poll', () => {
  it('When the key fails, Then an error toast is shown', async () => {
    mockInvoke.mockResolvedValueOnce([validResult('anthropic')]);
    mockInvoke.mockResolvedValueOnce([invalidResult('anthropic', 'HTTP 403')]);

    const toast = makeToast();
    renderHook(() => useApiKeyMonitor({ toast, addNotification: makeAddNotification() }));

    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    expect(toast.error).not.toHaveBeenCalled();

    await act(async () => { vi.advanceTimersByTime(5 * 60 * 1000); });
    await act(async () => {});

    expect(toast.error).toHaveBeenCalledOnce();
    expect(toast.error.mock.calls[0][0]).toMatch(/Anthropic|403/i);
  });

  it('When the key fails, Then a notification with severity "error" is added', async () => {
    mockInvoke.mockResolvedValueOnce([validResult('anthropic')]);
    mockInvoke.mockResolvedValueOnce([invalidResult('anthropic', 'HTTP 403')]);

    const addNotification = makeAddNotification();
    renderHook(() => useApiKeyMonitor({ toast: makeToast(), addNotification }));

    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    await act(async () => { vi.advanceTimersByTime(5 * 60 * 1000); });
    await act(async () => {});

    type NotifArg = { severity: string; category: string };
    const call = addNotification.mock.calls.find(
      (c: unknown[]) => ((c as [NotifArg])[0]).severity === 'error'
    );
    expect(call).toBeDefined();
    expect(((call as [NotifArg])![0]).category).toBe('api-keys');
  });
});

// ── Scenario 7: vibeui:api-key-validations event is dispatched ───────────────

describe('Given a validation run completes', () => {
  it('When the run fires, Then "vibeui:api-key-validations" custom event is dispatched', async () => {
    mockInvoke.mockResolvedValue([validResult('openai')]);
    const events: CustomEvent[] = [];
    window.addEventListener('vibeui:api-key-validations', (e) => events.push(e as CustomEvent));

    renderHook(() => useApiKeyMonitor({ toast: makeToast(), addNotification: makeAddNotification() }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});

    expect(events.length).toBeGreaterThanOrEqual(1);
    expect(events[0].detail).toBeDefined();
  });

  it('When the event fires, Then its detail contains the provider validation map', async () => {
    mockInvoke.mockResolvedValue([validResult('openai')]);
    const events: CustomEvent[] = [];
    window.addEventListener('vibeui:api-key-validations', (e) => events.push(e as CustomEvent));

    renderHook(() => useApiKeyMonitor({ toast: makeToast(), addNotification: makeAddNotification() }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});

    expect(events[0].detail.openai).toBeDefined();
    expect(events[0].detail.openai.valid).toBe(true);
  });
});

// ── Scenario 8: vibeui:providers-updated triggers re-validation ──────────────

describe('Given the user saves new API keys (vibeui:providers-updated fires)', () => {
  it('When the event fires, Then a re-validation is triggered', async () => {
    mockInvoke.mockResolvedValue([validResult('openai')]);
    renderHook(() => useApiKeyMonitor({ toast: makeToast(), addNotification: makeAddNotification() }));

    // First scheduled run
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    const callsAfterFirst = mockInvoke.mock.calls.length;

    // Dispatch providers-updated — triggers a 1500ms debounced re-validation
    act(() => { window.dispatchEvent(new CustomEvent('vibeui:providers-updated')); });
    await act(async () => { vi.advanceTimersByTime(1500); });
    await act(async () => {});

    expect(mockInvoke.mock.calls.length).toBeGreaterThan(callsAfterFirst);
  });
});

// ── Scenario 9: Multiple failing keys on first run ───────────────────────────

describe('Given 3 API keys are failing on first run', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue([
      invalidResult('openai', 'HTTP 401'),
      invalidResult('anthropic', 'HTTP 401'),
      invalidResult('gemini', 'HTTP 403'),
    ]);
  });

  it('When the initial delay fires, Then a single batched warn toast summarises all failures', async () => {
    const toast = makeToast();
    renderHook(() => useApiKeyMonitor({ toast, addNotification: makeAddNotification() }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    expect(toast.warn).toHaveBeenCalledOnce();
    expect(toast.warn.mock.calls[0][0]).toMatch(/3|three/i);
  });

  it('When the initial delay fires, Then one notification is added per failing key', async () => {
    const addNotification = makeAddNotification();
    renderHook(() => useApiKeyMonitor({ toast: makeToast(), addNotification }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    expect(addNotification).toHaveBeenCalledTimes(3);
  });
});

// ── Scenario 10: lastChecked and return values ────────────────────────────────

describe('Return values', () => {
  it('lastChecked is null before any validation runs', () => {
    mockInvoke.mockResolvedValue([]);
    const { result } = renderHook(() =>
      useApiKeyMonitor({ toast: makeToast(), addNotification: makeAddNotification() }));
    expect(result.current.lastChecked).toBeNull();
  });

  it('lastChecked is a number after the first validation completes', async () => {
    mockInvoke.mockResolvedValue([validResult('openai')]);
    const { result } = renderHook(() =>
      useApiKeyMonitor({ toast: makeToast(), addNotification: makeAddNotification() }));
    await act(async () => { vi.advanceTimersByTime(INITIAL_DELAY); });
    await act(async () => {});
    expect(typeof result.current.lastChecked).toBe('number');
  });
});

// ── Scenario 11: Manual revalidate ───────────────────────────────────────────

describe('Given the hook exposes a revalidate() function', () => {
  it('When revalidate() is called, Then invoke("validate_all_api_keys") is called', async () => {
    mockInvoke.mockResolvedValue([validResult('openai')]);
    const { result } = renderHook(() =>
      useApiKeyMonitor({ toast: makeToast(), addNotification: makeAddNotification() }));
    await act(async () => { await result.current.revalidate(); });
    expect(mockInvoke).toHaveBeenCalledWith('validate_all_api_keys');
  });
});
