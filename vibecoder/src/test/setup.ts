import '@testing-library/jest-dom/vitest';
import { configure } from '@testing-library/react';

// CI runners are 3-4× slower than dev macOS for async render + state
// propagation. Default `waitFor` timeout (1000ms) flakes on otherwise-
// correct tests. Match the file-level test timeout from vitest.config.ts.
configure({ asyncUtilTimeout: 5000 });

/**
 * Does the ambient `localStorage` actually store anything?
 *
 * Presence of the methods is not enough. Two different environments break in
 * two different ways:
 *
 *  - jsdom 29 + vitest 4 exposes a `localStorage` missing `setItem`/`getItem`/
 *    `clear` entirely.
 *  - Node 26 ships a *native* Web Storage `localStorage` that has every method
 *    but is inert unless the process was started with `--localstorage-file`
 *    ("localStorage is not available because --localstorage-file was not
 *    provided"). It looks correct and silently discards every write.
 *
 * The second case is why this probes a real round-trip instead of checking for
 * a method: a shape check passes and the tests still fail. Verify the
 * behaviour, not the signature.
 */
function localStorageWorks(): boolean {
  try {
    const store = globalThis.localStorage as Storage | undefined;
    if (!store || typeof store.setItem !== 'function') return false;
    const probeKey = '__vibecoder_storage_probe__';
    store.setItem(probeKey, 'ok');
    const roundTripped = store.getItem(probeKey) === 'ok';
    store.removeItem(probeKey);
    return roundTripped;
  } catch {
    return false;
  }
}

// Install a minimal in-memory polyfill so panels and tests can
// `localStorage.setItem(...)` etc. The store resets per file; tests that need
// finer isolation should still `localStorage.clear()` in beforeEach.
if (!localStorageWorks()) {
  const memoryStore = new Map<string, string>();
  const polyfill: Storage = {
    get length() { return memoryStore.size; },
    clear() { memoryStore.clear(); },
    getItem(key: string) { return memoryStore.has(key) ? memoryStore.get(key)! : null; },
    setItem(key: string, value: string) { memoryStore.set(String(key), String(value)); },
    removeItem(key: string) { memoryStore.delete(key); },
    key(index: number) {
      const keys = Array.from(memoryStore.keys());
      return keys[index] ?? null;
    },
  };
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    writable: true,
    value: polyfill,
  });
}
