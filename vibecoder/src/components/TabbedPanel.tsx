import { useState, useRef, useMemo, type ReactNode } from "react";
import { applyLayout, tabKey } from "../lib/layoutPrefs";
import { useLayoutPrefs } from "../hooks/useLayoutPrefs";

export interface SubTab {
  id: string;
  label: string;
  content: ReactNode;
}

interface TabbedPanelProps {
  tabs: SubTab[];
  defaultTab?: string;
  /** Controlled active tab — when provided, external callers can switch tabs. */
  activeTab?: string;
  /** Called when user manually clicks a tab (for controlled mode). */
  onTabChange?: (id: string) => void;
  /**
   * The panel these tabs belong to, e.g. `"security"`. When set, Settings can
   * reorder and hide them.
   *
   * Applied here rather than in `createComposite` because two composites
   * (Chat, Diagnostics) build their tab lists by hand and never call that
   * factory. Putting it in the one component both paths render through is what
   * keeps the preference from silently not applying to those two.
   *
   * Opt-in: `TabbedPanel` is also used for tab strips nested *inside* a panel,
   * and those are not user-configurable features. No id, no reordering.
   */
  panelId?: string;
}

/**
 * Reusable sub-tab panel with keep-alive behavior.
 * Sub-panels are only mounted once visited, then kept alive (hidden) when inactive.
 * Pass activeTab + onTabChange for fully controlled mode (e.g. Watch-driven tab switching).
 */
export function TabbedPanel({ tabs, defaultTab, activeTab, onTabChange, panelId }: TabbedPanelProps) {
  const prefs = useLayoutPrefs();

  // Hiding every tab in a panel would leave a strip with nothing behind it and
  // no way to recover from inside the panel, so an all-hidden preference falls
  // back to the shipped tabs. A panel you can still open must still show
  // something.
  const shown = useMemo(() => {
    if (!panelId) return tabs;
    const kept = applyLayout(
      tabs,
      (t) => tabKey(panelId, t.id),
      (prefs.order.tabs[panelId] ?? []).map((id) => tabKey(panelId, id)),
      prefs.hidden.tabs,
    );
    return kept.length > 0 ? kept : tabs;
  }, [tabs, panelId, prefs]);

  const [internalActive, setInternalActive] = useState(defaultTab || tabs[0]?.id || "");
  const setActive = (id: string) => {
    setInternalActive(id);
    onTabChange?.(id);
  };
  // Hiding the tab that happened to be open must not blank the panel: fall
  // back to the first one still showing.
  const requested = activeTab ?? internalActive;
  const active = shown.some((t) => t.id === requested) ? requested : (shown[0]?.id ?? requested);

  const visitedRef = useRef<Set<string>>(new Set([defaultTab || tabs[0]?.id || ""]));
  visitedRef.current.add(active);

  return (
    <div className="panel-container">
      <div className="panel-tab-bar panel-tab-bar--primary" style={{ overflowX: "auto" }}>
        {shown.map((t) => (
          <button
            key={t.id}
            onClick={() => setActive(t.id)}
            className={`panel-tab ${active === t.id ? "active" : ""}`}
            style={{ whiteSpace: "nowrap" }}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column" }}>
        {shown.map((t) =>
          visitedRef.current.has(t.id) ? (
            <div
              key={t.id}
              style={{
                display: active === t.id ? "flex" : "none",
                flexDirection: "column",
                flex: 1,
                minHeight: 0,
              }}
            >
              {t.content}
            </div>
          ) : null,
        )}
      </div>
    </div>
  );
}
