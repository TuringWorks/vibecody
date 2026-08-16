import { useEffect, useState } from "react";
import {
  loadLayoutPrefs,
  subscribeLayoutPrefs,
  type LayoutPrefs,
} from "../lib/layoutPrefs";

/**
 * The current layout preferences, kept in step with edits made elsewhere.
 *
 * Reading `loadLayoutPrefs()` once at mount is not enough: the nav and the open
 * composites mount long before Settings does, and Settings is a modal drawn on
 * top of them. Subscribing is what makes hiding a tab take effect on the panel
 * already behind the dialog rather than at the next restart.
 */
export function useLayoutPrefs(): LayoutPrefs {
  const [prefs, setPrefs] = useState<LayoutPrefs>(loadLayoutPrefs);
  useEffect(() => subscribeLayoutPrefs(setPrefs), []);
  return prefs;
}
