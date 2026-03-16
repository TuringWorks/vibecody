/**
 * usePersistentState — A drop-in replacement for useState that persists
 * to localStorage. Survives tab switches (via keep-alive PanelHost) AND
 * full app restarts.
 *
 * Usage:
 *   const [result, setResult] = usePersistentState<MyType>("coverage.result", null);
 *
 * The key is scoped automatically with a "vibeui-panel:" prefix.
 * Values are JSON-serialized. Non-serializable values (functions, refs) should
 * NOT be persisted — use regular useState for those.
 */
import { useState, useCallback, useRef, useEffect } from "react";

const PREFIX = "vibeui-panel:";

function readStorage<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    if (raw === null) return fallback;
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function writeStorage(key: string, value: unknown): void {
  try {
    if (value === null || value === undefined) {
      localStorage.removeItem(PREFIX + key);
    } else {
      localStorage.setItem(PREFIX + key, JSON.stringify(value));
    }
  } catch {
    // localStorage full or unavailable — silently ignore
  }
}

export function usePersistentState<T>(
  key: string,
  initialValue: T,
): [T, (value: T | ((prev: T) => T)) => void] {
  // Lazy initializer reads from localStorage once
  const [state, setState] = useState<T>(() => readStorage(key, initialValue));
  const keyRef = useRef(key);
  keyRef.current = key;

  const setPersistentState = useCallback(
    (value: T | ((prev: T) => T)) => {
      setState((prev) => {
        const next = typeof value === "function" ? (value as (prev: T) => T)(prev) : value;
        writeStorage(keyRef.current, next);
        return next;
      });
    },
    [],
  );

  // If key changes (rare), re-read from storage
  useEffect(() => {
    const stored = readStorage(key, initialValue);
    setState(stored);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  return [state, setPersistentState];
}

/**
 * Clear all persisted panel state. Useful for a "reset all panels" action.
 */
export function clearAllPanelState(): void {
  const keys: string[] = [];
  for (let i = 0; i < localStorage.length; i++) {
    const k = localStorage.key(i);
    if (k?.startsWith(PREFIX)) keys.push(k);
  }
  keys.forEach((k) => localStorage.removeItem(k));
}
