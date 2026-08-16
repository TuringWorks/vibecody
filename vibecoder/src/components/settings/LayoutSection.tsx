import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, ArrowUp, ArrowDown, RotateCcw, Search } from "lucide-react";
import { TAB_GROUPS } from "../../constants/tabGroups";
import { TAB_META, DEFAULT_TAB_META } from "../../constants/tabMeta";
import { PANEL_CATALOG } from "../../constants/panelCatalog";
import {
  applyLayout,
  moveWithin,
  resetLayoutPrefs,
  saveLayoutPrefs,
  tabKey,
  toggleHidden,
  type LayoutPrefs,
} from "../../lib/layoutPrefs";
import { useLayoutPrefs } from "../../hooks/useLayoutPrefs";

/**
 * Panels & Tabs — turn features off and put the rest in the order you use them.
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
 * button twice.
 */
export function LayoutSection() {
  const prefs = useLayoutPrefs();
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const [query, setQuery] = useState("");

  const update = (next: LayoutPrefs) => saveLayoutPrefs(next);

  // Ordering with nothing filtered out. `applyLayout` drops hidden entries,
  // which is right for the nav and wrong here — this is the screen where you
  // go to un-hide something.
  const groups = useMemo(
    () =>
      applyLayout(TAB_GROUPS, (g) => g.label, prefs.order.groups, []).map((g) => ({
        ...g,
        tabs: applyLayout(g.tabs, (t) => t, prefs.order.panels[g.label] ?? [], []),
      })),
    [prefs],
  );

  const q = query.trim().toLowerCase();
  const matches = (panelId: string, groupLabel: string) => {
    if (!q) return true;
    const meta = TAB_META[panelId] || DEFAULT_TAB_META;
    return (
      panelId.includes(q) ||
      meta.label.toLowerCase().includes(q) ||
      groupLabel.toLowerCase().includes(q) ||
      (PANEL_CATALOG[panelId] ?? []).some((t) => t.label.toLowerCase().includes(q))
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

  const moveTab = (panelId: string, tabId: string, delta: -1 | 1) => {
    const current = applyLayout(
      PANEL_CATALOG[panelId] ?? [],
      (t) => t.id,
      prefs.order.tabs[panelId] ?? [],
      [],
    ).map((t) => t.id);
    update({
      ...prefs,
      order: { ...prefs.order, tabs: { ...prefs.order.tabs, [panelId]: moveWithin(current, tabId, delta) } },
    });
  };

  const setGroupHidden = (label: string, hide: boolean) =>
    update({ ...prefs, hidden: { ...prefs.hidden, groups: toggleHidden(prefs.hidden.groups, label, hide) } });

  const setPanelHidden = (panelId: string, hide: boolean) =>
    update({ ...prefs, hidden: { ...prefs.hidden, panels: toggleHidden(prefs.hidden.panels, panelId, hide) } });

  const setTabHidden = (panelId: string, tabId: string, hide: boolean) =>
    update({
      ...prefs,
      hidden: { ...prefs.hidden, tabs: toggleHidden(prefs.hidden.tabs, tabKey(panelId, tabId), hide) },
    });

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
        shown. Hidden features stay listed here so you can turn them back on.
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
              const subtabs = applyLayout(
                PANEL_CATALOG[panelId] ?? [],
                (t) => t.id,
                prefs.order.tabs[panelId] ?? [],
                [],
              );
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
                      const hidden = prefs.hidden.tabs.includes(tabKey(panelId, t.id));
                      return (
                        <div
                          key={t.id}
                          style={{ display: "flex", alignItems: "center", gap: 8, paddingLeft: 56, marginTop: 3, opacity: hidden ? 0.55 : 1 }}
                        >
                          <input
                            type="checkbox"
                            checked={!hidden}
                            onChange={(e) => setTabHidden(panelId, t.id, !e.target.checked)}
                            aria-label={`Show the ${t.label} tab in ${meta.label}`}
                          />
                          <span style={{ fontSize: "var(--font-size-sm)" }}>{t.label}</span>
                          <Reorder
                            label={`${t.label} tab`}
                            onUp={() => moveTab(panelId, t.id, -1)}
                            onDown={() => moveTab(panelId, t.id, 1)}
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
    <span style={{ marginLeft: "auto", display: "flex", gap: 2 }}>
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
