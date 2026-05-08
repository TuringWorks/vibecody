import '@testing-library/jest-dom/vitest';

// jsdom 29 + vitest 4 in this repo's pool config exposes a `localStorage`
// global that's missing the Storage methods (`setItem`, `getItem`, `clear`),
// breaking any test that touches localStorage. Install a minimal in-memory
// polyfill so panels and tests can `localStorage.setItem(...)` etc. The
// store resets per file; tests that need finer isolation should still
// `localStorage.clear()` in beforeEach.
if (typeof globalThis.localStorage === 'undefined' ||
    typeof (globalThis.localStorage as Storage | undefined)?.clear !== 'function') {
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
