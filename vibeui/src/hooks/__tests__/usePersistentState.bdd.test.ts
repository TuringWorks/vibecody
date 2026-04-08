/**
 * BDD tests for usePersistentState — localStorage-backed useState.
 *
 * Scenarios:
 *  1. Initial value is used when nothing is in localStorage
 *  2. Persisted value is loaded from localStorage on mount
 *  3. Setting a value persists it to localStorage under the "vibeui-panel:" prefix
 *  4. Setting null removes the key from localStorage
 *  5. Functional updater receives the previous value
 *  6. Changing the key re-reads from the new key's storage slot
 *  7. Corrupt JSON falls back to the initial value
 *  8. clearAllPanelState removes all "vibeui-panel:" keys
 *  9. clearAllPanelState does not touch unrelated localStorage keys
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeEach } from 'vitest';
import { usePersistentState, clearAllPanelState } from '../usePersistentState';

const PREFIX = 'vibeui-panel:';

beforeEach(() => {
  localStorage.clear();
});

// ── Scenario 1: No stored value → initial value is used ───────────────────────

describe('Given localStorage has no entry for the key', () => {
  it('When usePersistentState mounts, Then the state equals the initial value', () => {
    const { result } = renderHook(() => usePersistentState('test.counter', 0));
    expect(result.current[0]).toBe(0);
  });

  it('When the initial value is a complex object, Then it is returned as-is', () => {
    const init = { mode: 'dark', fontSize: 14 };
    const { result } = renderHook(() => usePersistentState('test.settings', init));
    expect(result.current[0]).toEqual(init);
  });
});

// ── Scenario 2: Pre-existing storage entry is loaded ─────────────────────────

describe('Given localStorage has a stored value for the key', () => {
  it('When usePersistentState mounts, Then the stored value is loaded instead of the initial', () => {
    localStorage.setItem(PREFIX + 'test.theme', JSON.stringify('forest'));
    const { result } = renderHook(() => usePersistentState('test.theme', 'default'));
    expect(result.current[0]).toBe('forest');
  });

  it('When the stored value is a number, Then it is correctly deserialized', () => {
    localStorage.setItem(PREFIX + 'test.zoom', JSON.stringify(150));
    const { result } = renderHook(() => usePersistentState('test.zoom', 100));
    expect(result.current[0]).toBe(150);
  });

  it('When the stored value is an array, Then it is correctly deserialized', () => {
    localStorage.setItem(PREFIX + 'test.list', JSON.stringify(['a', 'b', 'c']));
    const { result } = renderHook(() => usePersistentState('test.list', [] as string[]));
    expect(result.current[0]).toEqual(['a', 'b', 'c']);
  });
});

// ── Scenario 3: Setting a value persists to localStorage ─────────────────────

describe('Given a mounted usePersistentState hook', () => {
  it('When setState is called with a new value, Then localStorage is updated', () => {
    const { result } = renderHook(() => usePersistentState('test.count', 0));
    act(() => { result.current[1](42); });
    expect(result.current[0]).toBe(42);
    expect(JSON.parse(localStorage.getItem(PREFIX + 'test.count')!)).toBe(42);
  });

  it('When setState is called, Then the key is stored with the "vibeui-panel:" prefix', () => {
    const { result } = renderHook(() => usePersistentState('my.panel.value', ''));
    act(() => { result.current[1]('hello'); });
    expect(localStorage.getItem(PREFIX + 'my.panel.value')).toBe(JSON.stringify('hello'));
    expect(localStorage.getItem('my.panel.value')).toBeNull();
  });

  it('When setState is called multiple times, Then each update is reflected in state and storage', () => {
    const { result } = renderHook(() => usePersistentState('test.step', 1));
    act(() => { result.current[1](2); });
    act(() => { result.current[1](3); });
    expect(result.current[0]).toBe(3);
    expect(JSON.parse(localStorage.getItem(PREFIX + 'test.step')!)).toBe(3);
  });
});

// ── Scenario 4: Setting null removes the key ─────────────────────────────────

describe('Given a previously stored value', () => {
  it('When setState(null) is called, Then the key is removed from localStorage', () => {
    localStorage.setItem(PREFIX + 'test.nullable', JSON.stringify('exists'));
    const { result } = renderHook(() => usePersistentState<string | null>('test.nullable', null));
    act(() => { result.current[1](null); });
    expect(localStorage.getItem(PREFIX + 'test.nullable')).toBeNull();
  });
});

// ── Scenario 5: Functional updater ───────────────────────────────────────────

describe('Given a counter in state', () => {
  it('When setState is called with a function, Then it receives the previous value', () => {
    const { result } = renderHook(() => usePersistentState('test.counter2', 10));
    act(() => { result.current[1](prev => prev + 5); });
    expect(result.current[0]).toBe(15);
  });

  it('When the functional updater is called twice, Then each increment builds on the previous', () => {
    const { result } = renderHook(() => usePersistentState('test.counter3', 0));
    act(() => { result.current[1](n => n + 1); });
    act(() => { result.current[1](n => n + 1); });
    expect(result.current[0]).toBe(2);
  });
});

// ── Scenario 6: Corrupt JSON falls back to initial value ─────────────────────

describe('Given localStorage has corrupt JSON for the key', () => {
  it('When usePersistentState mounts, Then it falls back to the initial value', () => {
    localStorage.setItem(PREFIX + 'test.broken', '{not valid json}}}');
    const { result } = renderHook(() => usePersistentState('test.broken', 'fallback'));
    expect(result.current[0]).toBe('fallback');
  });
});

// ── Scenario 7: clearAllPanelState ───────────────────────────────────────────

describe('Given multiple panel state keys are in localStorage', () => {
  it('When clearAllPanelState is called, Then all "vibeui-panel:" keys are removed', () => {
    localStorage.setItem(PREFIX + 'panel.a', '"one"');
    localStorage.setItem(PREFIX + 'panel.b', '"two"');
    localStorage.setItem(PREFIX + 'panel.c', '"three"');
    clearAllPanelState();
    expect(localStorage.getItem(PREFIX + 'panel.a')).toBeNull();
    expect(localStorage.getItem(PREFIX + 'panel.b')).toBeNull();
    expect(localStorage.getItem(PREFIX + 'panel.c')).toBeNull();
  });

  it('When clearAllPanelState is called, Then unrelated keys are preserved', () => {
    localStorage.setItem('other-app:setting', 'untouched');
    localStorage.setItem(PREFIX + 'panel.x', '"gone"');
    clearAllPanelState();
    expect(localStorage.getItem('other-app:setting')).toBe('untouched');
    expect(localStorage.getItem(PREFIX + 'panel.x')).toBeNull();
  });

  it('When clearAllPanelState is called on an empty store, Then no error is thrown', () => {
    expect(() => clearAllPanelState()).not.toThrow();
  });
});

// ── Scenario 8: State is consistent between read and write ───────────────────

describe('Given usePersistentState is used as a plain useState replacement', () => {
  it('When value is set, Then re-reading state returns the latest value', () => {
    const { result } = renderHook(() => usePersistentState('test.sync', 'initial'));
    expect(result.current[0]).toBe('initial');
    act(() => { result.current[1]('updated'); });
    expect(result.current[0]).toBe('updated');
  });
});
