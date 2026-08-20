import { useState, useRef, useMemo, useEffect, lazy, Suspense, type ComponentType, type ReactNode } from "react";
import { applyLayout, tabHost, tabKey, tabsMovedInto } from "../lib/layoutPrefs";
import { useLayoutPrefs } from "../hooks/useLayoutPrefs";
import type { RegisteredTab } from "../constants/tabRegistry";

export interface SubTab {
  id: string;
  label: string;
  content: ReactNode;
}

/** Props a tab moved in from another panel is rendered with. */
export interface HostProps {
  workspacePath?: string | null;
  provider?: string;
  onOpenFile?: (path: string, line?: number) => void;
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
   * reorder, hide, and re-home them.
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
  /**
   * Standard props for tabs hosted here after being moved from another panel.
   * Without them a moved-in tab renders with no workspace and no provider,
   * which for most panels is an empty screen rather than an error.
   */
  hostProps?: HostProps;
}

/** One tab as this panel renders it: its identity, its label, its content. */
interface Entry {
  /** `panelId/tabId` of where the tab *ships*. Stable across moves, which is
   *  what lets hiding and ordering keep pointing at the same tab. */
  key: string;
  /** The id the outside world knows it by (`activeTab` / `onTabChange`). */
  id: string;
  label: string;
  content: ReactNode;
}

const Loading = () => (
  <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>Loading...</div>
);

/**
 * Lazy component per moved-in tab, created once.
 *
 * `lazy()` returns a new component type on every call, and a new type remounts
 * its subtree — calling it during render would reset the tab's state on every
 * keystroke anywhere in the panel.
 */
const movedTabComponents = new Map<string, ComponentType<HostProps>>();

function movedTabComponent(tab: RegisteredTab): ComponentType<HostProps> {
  const cached = movedTabComponents.get(tabKey(tab.panelId, tab.tabId));
  if (cached) return cached;
  const Comp = lazy(() =>
    tab.load().then((mod) => {
      if (tab.exportName && tab.exportName in mod) {
        return { default: (mod as Record<string, ComponentType<HostProps>>)[tab.exportName] };
      }
      if ("default" in mod) return mod as { default: ComponentType<HostProps> };
      return { default: Object.values(mod)[0] as ComponentType<HostProps> };
    }),
  ) as ComponentType<HostProps>;
  movedTabComponents.set(tabKey(tab.panelId, tab.tabId), Comp);
  return Comp;
}

/**
 * Reusable sub-tab panel with keep-alive behavior.
 * Sub-panels are only mounted once visited, then kept alive (hidden) when inactive.
 * Pass activeTab + onTabChange for fully controlled mode (e.g. Watch-driven tab switching).
 */
export function TabbedPanel({ tabs, defaultTab, activeTab, onTabChange, panelId, hostProps }: TabbedPanelProps) {
  const prefs = useLayoutPrefs();

  // Keys of tabs Settings has re-homed *into* this panel. Read from the
  // preferences alone, so a panel nobody has moved anything into never pays
  // for the registry below.
  const incomingKeys = useMemo(
    () => (panelId ? tabsMovedInto(panelId, prefs.moves.tabs) : []),
    [panelId, prefs],
  );

  const [registry, setRegistry] = useState<Record<string, RegisteredTab> | null>(null);
  const [registryFailed, setRegistryFailed] = useState(false);
  useEffect(() => {
    if (incomingKeys.length === 0 || registry) return;
    let cancelled = false;
    import("../constants/tabRegistry")
      .then((m) => !cancelled && setRegistry(m.TAB_REGISTRY))
      .catch(() => !cancelled && setRegistryFailed(true));
    return () => {
      cancelled = true;
    };
  }, [incomingKeys.length, registry]);

  // Everything this panel hosts: its own tabs minus the ones moved out, plus
  // the ones moved in. Hiding and ordering are applied after, on the result.
  const present = useMemo<Entry[]>(() => {
    const own: Entry[] = tabs.map((t) => ({
      key: panelId ? tabKey(panelId, t.id) : t.id,
      id: t.id,
      label: t.label,
      content: t.content,
    }));
    if (!panelId) return own;

    const stayed = own.filter((e) => tabHost(e.key, prefs.moves.tabs) === panelId);
    const movedIn = incomingKeys
      .map((key) => registry?.[key])
      .filter((t): t is RegisteredTab => Boolean(t))
      .map((t) => {
        const Comp = movedTabComponent(t);
        return {
          key: tabKey(t.panelId, t.tabId),
          id: t.tabId,
          label: t.label,
          content: (
            <Suspense fallback={<Loading />}>
              <Comp
                workspacePath={hostProps?.workspacePath ?? null}
                provider={hostProps?.provider}
                onOpenFile={hostProps?.onOpenFile}
              />
            </Suspense>
          ),
        };
      });
    return [...stayed, ...movedIn];
  }, [tabs, panelId, prefs, incomingKeys, registry, hostProps]);

  // Hiding every tab in a panel would leave a strip with nothing behind it and
  // no way to recover from inside the panel, so an all-hidden preference falls
  // back to showing them. A panel you can still open must still show something.
  //
  // Moving every tab out is *not* that case: those tabs are somewhere else now,
  // and re-showing them here would put the same tab in two panels at once.
  const shown = useMemo(() => {
    if (!panelId) return present;
    const order = (prefs.order.tabs[panelId] ?? []).map((id) =>
      id.includes("/") ? id : tabKey(panelId, id),
    );
    const kept = applyLayout(present, (e) => e.key, order, prefs.hidden.tabs);
    return kept.length > 0 ? kept : present;
  }, [present, panelId, prefs]);

  // Tracked by key, not id. A tab moved in from another panel can share an id
  // with one already here — "dashboard" is a tab in several panels — and with
  // ids alone, clicking one of them would open the other.
  const [activeKey, setActiveKey] = useState<string | null>(null);
  const setActive = (entry: Entry) => {
    setActiveKey(entry.key);
    onTabChange?.(entry.id);
  };
  // A controlled `activeTab` names a tab id, which is the contract the outside
  // world (Watch-driven switching) already speaks. Hiding or moving away the
  // tab that happened to be open must not blank the panel: fall back to the
  // first one still showing.
  const activeEntry =
    (activeTab !== undefined ? shown.find((e) => e.id === activeTab) : null) ??
    shown.find((e) => e.key === activeKey) ??
    (defaultTab ? shown.find((e) => e.id === defaultTab) : null) ??
    shown[0];

  const visitedRef = useRef<Set<string>>(new Set());
  if (activeEntry) visitedRef.current.add(activeEntry.key);

  if (shown.length === 0) {
    return (
      <div className="panel-container">
        <div className="empty-state" style={{ padding: 24 }}>
          <p>
            Every tab in this panel is now hosted somewhere else. Settings →
            Panels &amp; Tabs is where they went, and where to bring them back.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="panel-container">
      <div className="panel-tab-bar panel-tab-bar--primary" style={{ overflowX: "auto" }}>
        {shown.map((e) => (
          <button
            key={e.key}
            onClick={() => setActive(e)}
            className={`panel-tab ${activeEntry?.key === e.key ? "active" : ""}`}
            style={{ whiteSpace: "nowrap" }}
          >
            {e.label}
          </button>
        ))}
        {registryFailed && (
          <span
            role="alert"
            style={{ alignSelf: "center", padding: "0 8px", color: "var(--error-color)", fontSize: "var(--font-size-sm)", whiteSpace: "nowrap" }}
          >
            {incomingKeys.length} moved tab{incomingKeys.length === 1 ? "" : "s"} could not be loaded
          </span>
        )}
      </div>
      <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column" }}>
        {shown.map((e) =>
          visitedRef.current.has(e.key) ? (
            <div
              key={e.key}
              style={{
                display: activeEntry?.key === e.key ? "flex" : "none",
                flexDirection: "column",
                flex: 1,
                minHeight: 0,
              }}
            >
              {e.content}
            </div>
          ) : null,
        )}
      </div>
    </div>
  );
}
