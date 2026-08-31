/**
 * Panel visibility, and polling that respects it.
 *
 * `TabbedPanel` hides an inactive tab with `display: none` rather than
 * unmounting it, so its state survives a tab switch. The cost is that every
 * timer the tab started keeps running: nineteen panels held an ungated
 * `setInterval`, several at two seconds, each one invoking a Tauri command that
 * reaches the daemon or the filesystem. Open ten panels over a session and ten
 * invisible pollers run for as long as the window is open.
 *
 * `useVisibleInterval` ties a poll to whether anyone can actually see it: the
 * tab is the active one, and the OS window is not hidden or in another space.
 */
import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";

/**
 * Whether the containing tab is the visible one.
 *
 * Defaults to `true`: a panel rendered outside a `TabbedPanel` is on screen
 * whenever it is mounted, and must keep polling as it always did.
 */
export const PanelVisibilityContext = createContext<boolean>(true);

/** Whether the document itself is visible (window minimised, tab in the background). */
function useDocumentVisible(): boolean {
  const [visible, setVisible] = useState(
    () => typeof document === "undefined" || document.visibilityState !== "hidden",
  );
  useEffect(() => {
    if (typeof document === "undefined") return;
    const onChange = () => setVisible(document.visibilityState !== "hidden");
    document.addEventListener("visibilitychange", onChange);
    return () => document.removeEventListener("visibilitychange", onChange);
  }, []);
  return visible;
}

/** True when this panel's tab is active *and* the window is not hidden. */
export function useIsPanelVisible(): boolean {
  const tabVisible = useContext(PanelVisibilityContext);
  // Both hooks run on every render. Writing this as
  // `tabVisible && useDocumentVisible()` short-circuits the second hook away
  // whenever the tab is hidden, which changes the hook order between renders —
  // React throws "Should have a queue" the moment a tab is switched.
  const documentVisible = useDocumentVisible();
  return tabVisible && documentVisible;
}

/**
 * Run `fn` every `ms` while the panel is visible.
 *
 * - Pass `null` for `ms` to disable the poll entirely; the caller keeps its own
 *   gating (a job that is running, auto-refresh switched off) that way.
 * - `fn` is held in a ref, so a callback that changes identity every render
 *   does not restart the timer — the usual way a "5 second poll" becomes a
 *   poll on every render.
 * - Becoming visible again refreshes immediately rather than showing data up to
 *   `ms` old while the next tick is awaited. `runOnShow: false` opts out for a
 *   `fn` that is a tick counter rather than a fetch.
 */
export function useVisibleInterval(
  fn: () => void,
  ms: number | null,
  options: { runOnShow?: boolean } = {},
): void {
  const { runOnShow = true } = options;
  const visible = useIsPanelVisible();
  const saved = useRef(fn);
  saved.current = fn;

  useEffect(() => {
    if (!visible || ms === null || ms <= 0) return;
    if (runOnShow) saved.current();
    const id = setInterval(() => saved.current(), ms);
    return () => clearInterval(id);
  }, [visible, ms, runOnShow]);
}
