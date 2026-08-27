import { useEffect, type RefObject } from "react";

/**
 * Close a popover on an outside click or Escape.
 *
 * Every composer menu needs this and only the model picker had it: a menu that
 * closes only by re-clicking the control it hangs off swallows the next click
 * meant for the conversation behind it, which reads as the app ignoring you.
 *
 * `active` gates the listeners so a closed menu costs nothing.
 */
export function useClickAway(
  active: boolean,
  ref: RefObject<HTMLElement | null>,
  onAway: () => void
) {
  useEffect(() => {
    if (!active) return;
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) onAway();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onAway();
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [active, ref, onAway]);
}
