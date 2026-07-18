/**
 * One-time migration of pre-rename (VibeX) localStorage keys to their VibeDesk
 * names, so an upgraded install keeps its theme preference instead of resetting
 * to the default.
 *
 * Runs before the app renders (see main.tsx) so the theme loader reads the
 * migrated value on first paint. Idempotent and non-destructive: it only copies
 * when the new key is absent and the old key exists, and leaves the old keys in
 * place. The models cache is intentionally omitted — it is transient and re-
 * populates from the daemon on the next fetch.
 */
const KEY_MAP: ReadonlyArray<readonly [oldKey: string, newKey: string]> = [
  ["vibex-theme-id", "vibedesk-theme-id"],
  ["vibex-theme", "vibedesk-theme"],
];

export function migrateLegacyVibexStorage(): void {
  try {
    for (const [oldKey, newKey] of KEY_MAP) {
      if (localStorage.getItem(newKey) === null) {
        const legacy = localStorage.getItem(oldKey);
        if (legacy !== null) localStorage.setItem(newKey, legacy);
      }
    }
  } catch {
    /* localStorage unavailable (private mode / disabled) — nothing to migrate */
  }
}
