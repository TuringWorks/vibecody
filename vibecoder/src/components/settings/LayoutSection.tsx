import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, ArrowUp, ArrowDown, RotateCcw, Search } from "lucide-react";
import { TAB_GROUPS } from "../../constants/tabGroups";
import { TAB_META, DEFAULT_TAB_META } from "../../constants/tabMeta";
import { MOVABLE_TABS, PANEL_CATALOG } from "../../constants/panelCatalog";
import {
  applyLayout,
  moveTabToPanel,
  movePanelToGroup,
  moveWithin,
  resetLayoutPrefs,
  resolveGroups,
  saveLayoutPrefs,
  splitTabKey,
  tabHost,
  tabKey,
  tabsMovedInto,
  toggleHidden,
  type LayoutPrefs,
} from "../../lib/layoutPrefs";
import { useLayoutPrefs } from "../../hooks/useLayoutPrefs";

/**
 * Panels & Tabs — turn features off, put the rest in the order you use them,
 * and move them to where you look for them.
 *
 * The app ships 45 panels holding 234 subfeature tabs. That is a lot to leave
 * arranged by whichever order they happened to be written in, and most people
 * need a small fraction of it.
 *
 * The list shows **everything**, including what is currently hidden, because a
 * settings page whose disabled entries disappear from it has no way back. A
 * hidden row is dimmed and keeps its position; unchecking is reversible from
 * the same place it was done.
 *
 * Reordering is ↑/↓ rather than drag: the result has to be reachable from the
 * keyboard, and dragging a row through a list of 45 is worse than pressing a
 * button twice. Moving between parents is a select for the same reason — a
 * drop target three levels down a scrolling tree is not a keyboard control.
 *
 * A moved row is listed under its new parent, not its old one. Settings is
 * where you go to find something, so it has to agree with where the thing is.
 */
export function LayoutSection() {
  const prefs = useLayoutPrefs();
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [query, setQuery] = useState("");

  const update = (next: LayoutPrefs) => saveLayoutPrefs(next);

  // Ordering with nothing filtered out. `resolveGroups` drops hidden entries by
  // default, which is right for the nav and wrong here — this is the screen
  // where you go to un-hide something.
  const groups = useMemo(() => resolveGroups(TAB_GROUPS, prefs, { includeHidden: true }), [prefs]);

  /** Where a panel ships, so a move back there clears the preference. */
  const shippedGroup = useMemo(() => {
    const out: Record<string, string> = {};
    for (const g of TAB_GROUPS) for (const p of g.tabs) out[p] = g.label;
    return out;
  }, []);

  /** Panels that can host a tab: the ones that render a tab strip at all. */
  const destinations = useMemo(
    () =>
      TAB_GROUPS.map((g) => ({
        label: g.label,
        panels: g.tabs.filter((p) => (PANEL_CATALOG[p] ?? []).length > 0),
      })).filter((g) => g.panels.length > 0),
    [],
  );

  const movable = useMemo(() => new Set(MOVABLE_TABS), []);

  /**
   * The tabs a panel shows: its own minus those moved out, plus those moved in,
   * in the stored order. Same rule `TabbedPanel` applies at runtime — both go
   * through `tabHost` / `tabsMovedInto` rather than each deciding for itself.
   */
  const hostedTabs = useMemo(() => {
    const metaOf = (key: string) => {
      const { panelId, tabId } = splitTabKey(key);
      const meta = (PANEL_CATALOG[panelId] ?? []).find((t) => t.id === tabId);
      return meta ? { key, originPanel: panelId, tabId, label: meta.label } : null;
    };
    return (panelId: string) => {
      const own = (PANEL_CATALOG[panelId] ?? [])
        .map((t) => tabKey(panelId, t.id))
        .filter((key) => tabHost(key, prefs.moves.tabs) === panelId);
      const all = [...own, ...tabsMovedInto(panelId, prefs.moves.tabs)]
        .map(metaOf)
        .filter((t): t is NonNullable<ReturnType<typeof metaOf>> => t !== null);
      const order = (prefs.order.tabs[panelId] ?? []).map((id) =>
        id.includes("/") ? id : tabKey(panelId, id),
      );
      return applyLayout(all, (t) => t.key, order, []);
    };
  }, [prefs]);

  const q = query.trim().toLowerCase();
  const matches = (panelId: string, groupLabel: string) => {
    if (!q) return true;
    const meta = TAB_META[panelId] || DEFAULT_TAB_META;
    return (
      panelId.includes(q) ||
      meta.label.toLowerCase().includes(q) ||
      groupLabel.toLowerCase().includes(q) ||
      hostedTabs(panelId).some((t) => t.label.toLowerCase().includes(q))
    );
  };

  const shown = groups
    .map((g) => ({ ...g, tabs: g.tabs.filter((t) => matches(t, g.label)) }))
    .filter((g) => g.tabs.length > 0);

  const totals = useMemo(() => {
    const panels = TAB_GROUPS.flatMap((g) => g.tabs);
    const tabs = panels.flatMap((p) => (PANEL_CATALOG[p] ?? []).map((t) => tabKey(p, t.id)));
    return {
      panels: panels.length,
      panelsOn: panels.filter((p) => !prefs.hidden.panels.includes(p)).length,
      tabs: tabs.length,
      tabsOn: tabs.filter((k) => !prefs.hidden.tabs.includes(k)).length,
      moved: Object.keys(prefs.moves.panels).length + Object.keys(prefs.moves.tabs).length,
    };
  }, [prefs]);

  // ── Mutations ────────────────────────────────────────────────────────────

  const moveGroup = (label: string, delta: -1 | 1) =>
    update({
      ...prefs,
      order: { ...prefs.order, groups: moveWithin(groups.map((g) => g.label), label, delta) },
    });

  const movePanel = (groupLabel: string, panelId: string, delta: -1 | 1) => {
    const current = groups.find((g) => g.label === groupLabel)?.tabs ?? [];
    update({
      ...prefs,
      order: {
        ...prefs.order,
        panels: { ...prefs.order.panels, [groupLabel]: moveWithin(current, panelId, delta) },
      },
    });
  };

  const moveTab = (hostPanelId: string, key: string, delta: -1 | 1) => {
    const current = hostedTabs(hostPanelId).map((t) => t.key);
    update({
      ...prefs,
      order: {
        ...prefs.order,
        tabs: { ...prefs.order.tabs, [hostPanelId]: moveWithin(current, key, delta) },
      },
    });
  };

  const rehomePanel = (panelId: string, group: string) =>
    update(movePanelToGroup(prefs, panelId, group, shippedGroup[panelId] ?? group));

  const rehomeTab = (originPanel: string, tabId: string, destination: string) =>
    update(moveTabToPanel(prefs, originPanel, tabId, destination));

  const setGroupHidden = (label: string, hide: boolean) =>
    update({ ...prefs, hidden: { ...prefs.hidden, groups: toggleHidden(prefs.hidden.groups, label, hide) } });

  const setPanelHidden = (panelId: string, hide: boolean) =>
    update({ ...prefs, hidden: { ...prefs.hidden, panels: toggleHidden(prefs.hidden.panels, panelId, hide) } });

  const setTabHidden = (key: string, hide: boolean) =>
    update({ ...prefs, hidden: { ...prefs.hidden, tabs: toggleHidden(prefs.hidden.tabs, key, hide) } });

  const toggleExpanded = (panelId: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(panelId)) next.delete(panelId);
      else next.add(panelId);
      return next;
    });

  // ── Render ───────────────────────────────────────────────────────────────

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 4, flexWrap: "wrap" }}>
        <h3 style={{ margin: 0, fontSize: "var(--font-size-lg)" }}>Panels &amp; Tabs</h3>
        <button
          className="panel-btn"
          onClick={resetLayoutPrefs}
          style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 6 }}
        >
          <RotateCcw size={13} /> Reset to defaults
        </button>
      </div>
      <p style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", marginTop: 0 }}>
        {totals.panelsOn} of {totals.panels} panels and {totals.tabsOn} of {totals.tabs} tabs are
        shown{totals.moved > 0 ? `, and ${totals.moved} ${totals.moved === 1 ? "is" : "are"} moved from where ${totals.moved === 1 ? "it ships" : "they ship"}` : ""}.
        Hidden features stay listed here so you can turn them back on.
      </p>

      <div style={{ display: "flex", alignItems: "center", gap: 6, margin: "12px 0", padding: "4px 10px", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)" }}>
        <Search size={14} />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Find a panel or tab"
          aria-label="Find a panel or tab"
          style={{ flex: 1, background: "none", border: "none", outline: "none", color: "var(--text-primary)", fontSize: "var(--font-size-base)" }}
        />
      </div>

      {shown.length === 0 && (
        <div style={{ color: "var(--text-secondary)" }}>Nothing matches that.</div>
      )}

      {shown.map((group, gi) => {
        const groupHidden = prefs.hidden.groups.includes(group.label);
        return (
          <section key={group.label} style={{ marginBottom: 18, opacity: groupHidden ? 0.55 : 1 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, borderBottom: "1px solid var(--border-color)", paddingBottom: 4 }}>
              <input
                type="checkbox"
                checked={!groupHidden}
                onChange={(e) => setGroupHidden(group.label, !e.target.checked)}
                aria-label={`Show the ${group.label} group`}
              />
              <span style={{ fontWeight: 700 }}>{group.label}</span>
              <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
                {group.tabs.length} panel{group.tabs.length === 1 ? "" : "s"}
              </span>
              <Reorder
                label={group.label}
                onUp={() => moveGroup(group.label, -1)}
                onDown={() => moveGroup(group.label, 1)}
                first={gi === 0}
                last={gi === shown.length - 1}
                disabled={Boolean(q)}
              />
            </div>

            {group.tabs.map((panelId, pi) => {
              const meta = TAB_META[panelId] || DEFAULT_TAB_META;
              const panelHidden = prefs.hidden.panels.includes(panelId);
              const subtabs = hostedTabs(panelId);
              const isOpen = expanded.has(panelId);
              return (
                <div key={panelId} style={{ marginTop: 6, opacity: panelHidden ? 0.55 : 1 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, paddingLeft: 18 }}>
                    <input
                      type="checkbox"
                      checked={!panelHidden}
                      onChange={(e) => setPanelHidden(panelId, !e.target.checked)}
                      aria-label={`Show the ${meta.label} panel`}
                    />
                    {subtabs.length > 0 ? (
                      <button
                        className="panel-btn"
                        onClick={() => toggleExpanded(panelId)}
                        aria-expanded={isOpen}
                        aria-label={`${isOpen ? "Hide" : "Show"} the tabs in ${meta.label}`}
                        style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 0, display: "flex", alignItems: "center", gap: 4 }}
                      >
                        {isOpen ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                        <span style={{ color: "var(--text-primary)" }}>{meta.label}</span>
                      </button>
                    ) : (
                      <span style={{ paddingLeft: 17 }}>{meta.label}</span>
                    )}
                    <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
                      {subtabs.length > 0 ? `${subtabs.length} tabs` : "single view"}
                    </span>
                    <select
                      className="panel-input"
                      value={group.label}
                      onChange={(e) => rehomePanel(panelId, e.target.value)}
                      aria-label={`Group that shows the ${meta.label} panel`}
                      style={selectStyle}
                    >
                      {TAB_GROUPS.map((g) => (
                        <option key={g.label} value={g.label}>
                          {g.label}
                          {shippedGroup[panelId] === g.label ? " (default)" : ""}
                        </option>
                      ))}
                    </select>
                    <Reorder
                      label={meta.label}
                      onUp={() => movePanel(group.label, panelId, -1)}
                      onDown={() => movePanel(group.label, panelId, 1)}
                      first={pi === 0}
                      last={pi === group.tabs.length - 1}
                      disabled={Boolean(q)}
                    />
                  </div>

                  {isOpen &&
                    subtabs.map((t, ti) => {
                      const hidden = prefs.hidden.tabs.includes(t.key);
                      const canMove = movable.has(t.key);
                      const homeLabel = (TAB_META[t.originPanel] || DEFAULT_TAB_META).label;
                      return (
                        <div
                          key={t.key}
                          style={{ display: "flex", alignItems: "center", gap: 8, paddingLeft: 56, marginTop: 3, opacity: hidden ? 0.55 : 1 }}
                        >
                          <input
                            type="checkbox"
                            checked={!hidden}
                            onChange={(e) => setTabHidden(t.key, !e.target.checked)}
                            aria-label={`Show the ${t.label} tab in ${meta.label}`}
                          />
                          <span style={{ fontSize: "var(--font-size-sm)" }}>{t.label}</span>
                          {t.originPanel !== panelId && (
                            <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                              from {homeLabel}
                            </span>
                          )}
                          <select
                            className="panel-input"
                            value={panelId}
                            disabled={!canMove}
                            onChange={(e) => rehomeTab(t.originPanel, t.tabId, e.target.value)}
                            aria-label={`Panel that shows the ${t.label} tab`}
                            title={
                              canMove
                                ? undefined
                                : `${t.label} needs settings only ${homeLabel} can give it, so it cannot be moved.`
                            }
                            style={{ ...selectStyle, marginLeft: "auto" }}
                          >
                            {destinations.map((g) => (
                              <optgroup key={g.label} label={g.label}>
                                {g.panels.map((p) => (
                                  <option key={p} value={p}>
                                    {(TAB_META[p] || DEFAULT_TAB_META).label}
                                    {t.originPanel === p ? " (default)" : ""}
                                  </option>
                                ))}
                              </optgroup>
                            ))}
                          </select>
                          <Reorder
                            label={`${t.label} tab`}
                            onUp={() => moveTab(panelId, t.key, -1)}
                            onDown={() => moveTab(panelId, t.key, 1)}
                            first={ti === 0}
                            last={ti === subtabs.length - 1}
                            disabled={Boolean(q)}
                          />
                        </div>
                      );
                    })}
                </div>
              );
            })}
          </section>
        );
      })}
    </div>
  );
}

const selectStyle = {
  fontSize: "var(--font-size-xs)",
  padding: "1px 4px",
  maxWidth: 160,
} as const;

/**
 * Move-up / move-down for one row.
 *
 * Disabled while a search is active: the arrows move an item relative to what
 * is on screen, and with most of the list filtered away that is not the move
 * anyone means.
 */
function Reorder({
  label,
  onUp,
  onDown,
  first,
  last,
  disabled,
}: {
  label: string;
  onUp: () => void;
  onDown: () => void;
  first: boolean;
  last: boolean;
  disabled?: boolean;
}) {
  const style = { background: "none", border: "none", padding: 2, cursor: "pointer", color: "var(--text-secondary)" };
  return (
    <span style={{ display: "flex", gap: 2 }}>
      <button
        className="panel-btn"
        style={style}
        onClick={onUp}
        disabled={first || disabled}
        title={disabled ? "Clear the search to reorder" : undefined}
        aria-label={`Move ${label} up`}
      >
        <ArrowUp size={13} />
      </button>
      <button
        className="panel-btn"
        style={style}
        onClick={onDown}
        disabled={last || disabled}
        title={disabled ? "Clear the search to reorder" : undefined}
        aria-label={`Move ${label} down`}
      >
        <ArrowDown size={13} />
      </button>
    </span>
  );
}

export default LayoutSection;
