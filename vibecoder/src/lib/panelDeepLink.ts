/**
 * Opening one panel's subtab from somewhere else in the app.
 *
 * `vibecoder:open-tab` already switches which panel is on screen; a
 * `panelId/tabId` key — the same key Settings uses to move a tab between
 * panels — additionally names a subtab inside it. `TabbedPanel` listens for
 * the event, so a panel that is already mounted switches immediately.
 *
 * The event alone is not enough, because panels are lazy: the very first deep
 * link into a panel dispatches before that panel's `TabbedPanel` exists, and a
 * listener that is not mounted yet hears nothing. So the request is also
 * parked here, and `TabbedPanel` claims it as it mounts. One slot, not a
 * queue: two deep links racing means the later one is what the user asked for.
 */

/** The unclaimed request, if any. Cleared by whoever acts on it. */
let pending: string | null = null;

/**
 * Ask for `panelId/tabId`, or a bare panel id for "just show this panel".
 *
 * The event fires either way; only a key with a subtab is parked, since a bare
 * panel id is `App`'s to act on and it is always listening.
 */
export function openPanelTab(key: string): void {
  pending = key.includes("/") ? key : null;
  window.dispatchEvent(new CustomEvent("vibecoder:open-tab", { detail: key }));
}

/**
 * The parked request for this panel, consumed. Returns null when the pending
 * request belongs to a different panel — a deep link into Security must not be
 * eaten by whichever panel happens to mount next.
 */
export function takePendingTab(panelId: string): string | null {
  if (pending === null || !pending.startsWith(`${panelId}/`)) return null;
  const key = pending;
  pending = null;
  return key;
}
