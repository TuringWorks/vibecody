/**
 * The Figma personal access token — one place that reads and writes it.
 *
 * Two panels offer the Figma import (DesignHubPanel, and DesignMode's Figma
 * tab) and both used to own the storage. DesignMode kept the token in
 * `localStorage`, which is a plaintext file in the webview's profile directory:
 * readable by anything running as the user, and copied by every backup and file
 * sync. Credentials belong in the encrypted ProfileStore, like every other key.
 *
 * `loadFigmaToken` also drains a leftover localStorage copy into the store, so
 * the plaintext one disappears the first time either panel is opened.
 */
import { invoke } from "@tauri-apps/api/core";

const PROFILE_ID = "default";
const PROVIDER = "figma";

/** Pre-ProfileStore key. Read once so the token survives, then removed. */
const LEGACY_LOCALSTORAGE_KEY = "figma_token";

/** Both spellings: which one a Tauri command sees depends on its own signature. */
const keyArgs = () => ({ profile_id: PROFILE_ID, profileId: PROFILE_ID, provider: PROVIDER });

/** The stored token, or `""` when none is stored. Never reads a value from localStorage without moving it into the store first. */
export async function loadFigmaToken(): Promise<string> {
  const stored = await invoke<string | null>("profile_api_key_get", keyArgs());
  if (typeof stored === "string" && stored.length > 0) {
    // The store is authoritative; a leftover plaintext copy is only a
    // liability now, so it goes even though it is not what we return.
    clearLegacy();
    return stored;
  }
  const legacy = readLegacy();
  if (!legacy) return "";
  // Move it rather than drop it: deleting it silently would leave the user
  // with a failing import and no hint that we discarded a token they gave us.
  // `saveFigmaToken` clears the plaintext copy only once the encrypted write
  // lands — on a failure it stays put and this throws, rather than losing it.
  await saveFigmaToken(legacy);
  return legacy;
}

export async function saveFigmaToken(token: string): Promise<void> {
  await invoke("profile_api_key_set", { ...keyArgs(), api_key: token, apiKey: token });
  clearLegacy();
}

export async function deleteFigmaToken(): Promise<void> {
  await invoke("profile_api_key_delete", keyArgs());
  clearLegacy();
}

function readLegacy(): string {
  try { return localStorage.getItem(LEGACY_LOCALSTORAGE_KEY) ?? ""; } catch { return ""; }
}

function clearLegacy(): void {
  try { localStorage.removeItem(LEGACY_LOCALSTORAGE_KEY); } catch { /* storage disabled */ }
}
