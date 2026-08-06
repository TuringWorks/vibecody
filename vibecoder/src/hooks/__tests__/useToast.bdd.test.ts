/**
 * BDD tests for useToast — toast queue lifecycle.
 *
 * Scenarios:
 *  1. Adding a toast makes it appear in the queue
 *  2. Each variant (success/error/info/warn) is stored with correct variant
 *  3. Toasts auto-dismiss after their duration (errors 6s, others 3/4s)
 *  4. Manually dismissing removes the toast immediately
 *  5. Dismissing a non-existent id is a no-op
 *  6. Multiple toasts can coexist in the queue
 *  7. Dismissed toast timer is cleared (no stale dismissal)
 *  8. Unmounting clears all pending timers
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useToast } from '../useToast';

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

// ── Scenario 1: Adding a toast ─────────────────────────────────────────────────

describe('Given the toast queue is empty', () => {
  it('When toast.success is called, Then the toast appears in the queue', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.success('File saved!'); });
    expect(result.current.toasts).toHaveLength(1);
    expect(result.current.toasts[0].message).toBe('File saved!');
    expect(result.current.toasts[0].variant).toBe('success');
  });

  it('When toast.error is called, Then variant is "error"', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.error('Build failed'); });
    expect(result.current.toasts[0].variant).toBe('error');
  });

  it('When toast.info is called, Then variant is "info"', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.info('Workspace loaded'); });
    expect(result.current.toasts[0].variant).toBe('info');
  });

  it('When toast.warn is called, Then variant is "warn"', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.warn('No provider set'); });
    expect(result.current.toasts[0].variant).toBe('warn');
  });
});

// ── Scenario 2: Each toast gets a unique id ────────────────────────────────────

describe('Given two toasts are added', () => {
  it('When both are added, Then they have distinct ids', () => {
    const { result } = renderHook(() => useToast());
    act(() => {
      result.current.toast.success('First');
      result.current.toast.success('Second');
    });
    const [a, b] = result.current.toasts;
    expect(a.id).not.toBe(b.id);
  });
});

// ── Scenario 3: Auto-dismiss timing ───────────────────────────────────────────

describe('Given a success toast is in the queue (auto-dismisses at 3000ms)', () => {
  it('When 2999ms elapses, Then the toast is still visible', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.success('Saving…'); });
    act(() => { vi.advanceTimersByTime(2999); });
    expect(result.current.toasts).toHaveLength(1);
  });

  it('When 3000ms elapses, Then the toast is gone', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.success('Saved'); });
    act(() => { vi.advanceTimersByTime(3000); });
    expect(result.current.toasts).toHaveLength(0);
  });
});

describe('Given an error toast is in the queue (auto-dismisses at 6000ms)', () => {
  it('When 5999ms elapses, Then the error toast is still visible', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.error('Compile failed'); });
    act(() => { vi.advanceTimersByTime(5999); });
    expect(result.current.toasts).toHaveLength(1);
  });

  it('When 6000ms elapses, Then the error toast is gone', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.error('Compile failed'); });
    act(() => { vi.advanceTimersByTime(6000); });
    expect(result.current.toasts).toHaveLength(0);
  });
});

describe('Given a warn toast is in the queue (auto-dismisses at 4000ms)', () => {
  it('When 4000ms elapses, Then the warn toast is gone', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.warn('Low memory'); });
    act(() => { vi.advanceTimersByTime(4000); });
    expect(result.current.toasts).toHaveLength(0);
  });
});

// ── Scenario 4: Manual dismiss ────────────────────────────────────────────────

describe('Given a toast is visible', () => {
  it('When dismiss(id) is called, Then the toast is removed immediately', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.info('Check this out'); });
    const { id } = result.current.toasts[0];
    act(() => { result.current.dismiss(id); });
    expect(result.current.toasts).toHaveLength(0);
  });

  it('When dismiss(id) is called, Then the auto-dismiss timer does not fire later', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.info('Ephemeral'); });
    const { id } = result.current.toasts[0];
    act(() => { result.current.dismiss(id); });
    // Advance past the auto-dismiss duration — should still be 0, no side effects
    act(() => { vi.advanceTimersByTime(5000); });
    expect(result.current.toasts).toHaveLength(0);
  });
});

// ── Scenario 5: Dismissing non-existent id is a no-op ────────────────────────

describe('Given one toast is in the queue', () => {
  it('When dismiss is called with an unknown id, Then the queue is unchanged', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.success('Still here'); });
    act(() => { result.current.dismiss(99999); });
    expect(result.current.toasts).toHaveLength(1);
  });
});

// ── Scenario 6: Multiple coexisting toasts ────────────────────────────────────

describe('Given three toasts are added rapidly', () => {
  it('When all three are added, Then all three are in the queue', () => {
    const { result } = renderHook(() => useToast());
    act(() => {
      result.current.toast.success('A');
      result.current.toast.warn('B');
      result.current.toast.error('C');
    });
    expect(result.current.toasts).toHaveLength(3);
  });

  it('When the success toast timer fires (3s), Then only success is removed', () => {
    const { result } = renderHook(() => useToast());
    act(() => {
      result.current.toast.success('Short');
      result.current.toast.error('Long'); // stays 6s
    });
    act(() => { vi.advanceTimersByTime(3000); });
    expect(result.current.toasts).toHaveLength(1);
    expect(result.current.toasts[0].variant).toBe('error');
  });

  it('When dismiss removes the middle toast, Then others remain', () => {
    const { result } = renderHook(() => useToast());
    act(() => {
      result.current.toast.success('First');
      result.current.toast.info('Middle');
      result.current.toast.warn('Last');
    });
    const middleId = result.current.toasts[1].id;
    act(() => { result.current.dismiss(middleId); });
    expect(result.current.toasts).toHaveLength(2);
    expect(result.current.toasts.find(t => t.id === middleId)).toBeUndefined();
  });
});

// ── Scenario 7: Toast id is a number, message is preserved ───────────────────

describe('Given a toast is added', () => {
  it('Then its id is a positive number', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.success('ping'); });
    expect(typeof result.current.toasts[0].id).toBe('number');
    expect(result.current.toasts[0].id).toBeGreaterThan(0);
  });

  it('Then its message matches exactly what was passed', () => {
    const { result } = renderHook(() => useToast());
    const msg = 'Saved ./src/main.ts in 12ms';
    act(() => { result.current.toast.success(msg); });
    expect(result.current.toasts[0].message).toBe(msg);
  });
});

// ── Scenario 8: Stable API identity ───────────────────────────────────────────
// Panels list `toast` in useCallback/useEffect dependency arrays. A fresh
// object every render makes an error path self-retrigger: render → fetch →
// error toast → state change → render → …

describe('Given a component holds on to the toast API', () => {
  it('When the component re-renders, Then the toast object identity is unchanged', () => {
    const { result, rerender } = renderHook(() => useToast());
    const first = result.current.toast;
    rerender();
    expect(result.current.toast).toBe(first);
  });

  it('When a toast is raised (state change), Then the toast object identity is unchanged', () => {
    const { result } = renderHook(() => useToast());
    const first = result.current.toast;
    act(() => { result.current.toast.error('boom'); });
    expect(result.current.toast).toBe(first);
  });

  it('When toasts auto-dismiss, Then dismiss keeps its identity too', () => {
    const { result } = renderHook(() => useToast());
    const first = result.current.dismiss;
    act(() => { result.current.toast.info('hi'); });
    act(() => { vi.advanceTimersByTime(3000); });
    expect(result.current.dismiss).toBe(first);
  });
});

// ── Scenario 9: Duplicate collapsing ──────────────────────────────────────────

describe('Given the same message is raised repeatedly', () => {
  it('When it repeats while visible, Then one toast carries a count', () => {
    const { result } = renderHook(() => useToast());
    act(() => {
      result.current.toast.error('Failed to list goals');
      result.current.toast.error('Failed to list goals');
      result.current.toast.error('Failed to list goals');
    });
    expect(result.current.toasts).toHaveLength(1);
    expect(result.current.toasts[0].count).toBe(3);
  });

  it('When the same text is raised under a different variant, Then they are separate toasts', () => {
    const { result } = renderHook(() => useToast());
    act(() => {
      result.current.toast.error('same');
      result.current.toast.info('same');
    });
    expect(result.current.toasts.map(t => t.variant)).toEqual(['error', 'info']);
  });

  it('When a repeat arrives, Then the auto-dismiss countdown restarts', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.error('flaky'); });
    act(() => { vi.advanceTimersByTime(5000); });
    act(() => { result.current.toast.error('flaky'); });
    act(() => { vi.advanceTimersByTime(5000); });
    expect(result.current.toasts).toHaveLength(1);
    act(() => { vi.advanceTimersByTime(1000); });
    expect(result.current.toasts).toHaveLength(0);
  });

  it('When the message is dismissed and raised again, Then it starts a fresh toast', () => {
    const { result } = renderHook(() => useToast());
    act(() => { result.current.toast.error('gone'); });
    act(() => { result.current.dismiss(result.current.toasts[0].id); });
    act(() => { result.current.toast.error('gone'); });
    expect(result.current.toasts).toHaveLength(1);
    expect(result.current.toasts[0].count).toBe(1);
  });
});

// ── Scenario 10: Stack cap ────────────────────────────────────────────────────

describe('Given more distinct toasts than the visible cap are raised', () => {
  it('When six are raised, Then only the four newest remain', () => {
    const { result } = renderHook(() => useToast());
    act(() => {
      ['a', 'b', 'c', 'd', 'e', 'f'].forEach(m => result.current.toast.error(m));
    });
    expect(result.current.toasts.map(t => t.message)).toEqual(['c', 'd', 'e', 'f']);
  });

  it('When an evicted message is raised again, Then it reappears with count 1', () => {
    const { result } = renderHook(() => useToast());
    act(() => {
      ['a', 'b', 'c', 'd', 'e'].forEach(m => result.current.toast.error(m));
      result.current.toast.error('a');
    });
    const revived = result.current.toasts.find(t => t.message === 'a');
    expect(revived?.count).toBe(1);
  });
});
