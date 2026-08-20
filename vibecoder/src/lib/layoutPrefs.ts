/**
 * Which features are shown, and in what order.
 *
 * The app ships 45 panels holding 234 subfeature tabs, grouped into 11
 * categories. Nobody uses all of it, and the order that suits a person doing
 * infrastructure work is not the one that suits someone doing code review. This
 * module is where that choice lives.
 *
 * Two design rules the rest of the app depends on:
 *
 * **Preferences are a diff, not a copy.** They store only what the user
 * changed — an explicit order for the groups they reordered, a list of the
 * things they hid. Anything unmentioned keeps its shipped position. Storing a
 * full ordering instead would freeze the layout at the version it was saved:
 * every panel added afterwards would be invisible, because it would be absent
 * from the stored list rather than present-and-unranked. `applyLayout` treats
 * absence as "not yet ranked", never as "excluded".
 *
 * **The ordering is a pure function.** `applyLayout` takes the shipped list and
 * the preferences and returns the list to render. It touches no storage and no
 * React, so the ordering rules can be tested directly rather than through a
 * component that happens to call them.
 *
 * **A move is stored against the thing's original home, never its new one.**
 * `moves.tabs["design/sketch"] = "security"` reads "the Sketch tab that ships
 * in Design is hosted by Security". Re-keying it to its destination would make
 * the identity move too, and then hiding it, ordering it, or moving it a second
 * time would all be looking for a key that no longer exists.
 */

/** Storage key. Versioned so a future incompatible shape can be migrated. */
const STORAGE_KEY = "vibecoder-layout-prefs:v1";

export interface LayoutPrefs {
  /** Explicit order for the ids named. Unnamed ids keep their shipped rank. */
  order: {
    groups: string[];
    /** Keyed by group label. */
    panels: Record<string, string[]>;
    /** Keyed by panel id. Entries are tab ids, or `panelId/tabId` keys for a
     *  tab moved in from another panel. */
    tabs: Record<string, string[]>;
  };
  /** Ids the user switched off. Tabs are namespaced `panelId/tabId`. */
  hidden: {
    groups: string[];
    panels: string[];
    tabs: string[];
  };
  /** Things re-homed somewhere other than where they ship. Keyed by the
   *  original id, valued by the destination. */
  moves: {
    /** Panel id → the group label that now shows it. */
    panels: Record<string, string>;
    /** `panelId/tabId` → the panel id that now hosts that tab. */
    tabs: Record<string, string>;
  };
}

export const EMPTY_PREFS: LayoutPrefs = {
  order: { groups: [], panels: {}, tabs: {} },
  hidden: { groups: [], panels: [], tabs: [] },
  moves: { panels: {}, tabs: {} },
};

/** A tab's key in `hidden.tabs`. Tab ids repeat across panels — "dashboard"
 *  appears in many — so hiding one must not hide its namesakes. */
export function tabKey(panelId: string, tabId: string): string {
  return `${panelId}/${tabId}`;
}

/** The panel a tab key belongs to, and the tab's own id. */
export function splitTabKey(key: string): { panelId: string; tabId: string } {
  const at = key.indexOf("/");
  // No separator means it is a bare tab id, which only happens for an order
  // list written before moves existed. Those are always the host's own tabs.
  if (at < 0) return { panelId: "", tabId: key };
  return { panelId: key.slice(0, at), tabId: key.slice(at + 1) };
}

/**
 * Order `items` by `order`, then drop anything in `hidden`.
 *
 * Items named in `order` come first, in that order. Everything else follows in
 * its original relative order — which is what lets a panel added in a later
 * release show up for someone whose preferences predate it, instead of
 * silently vanishing because it was not on a list written before it existed.
 */
export function applyLayout<T>(
  items: readonly T[],
  keyOf: (item: T) => string,
  order: readonly string[],
  hidden: readonly string[],
): T[] {
  const rank = new Map(order.map((id, i) => [id, i]));
  const hide = new Set(hidden);
  const ranked: { item: T; at: number }[] = [];
  const rest: T[] = [];

  for (const item of items) {
    const key = keyOf(item);
    if (hide.has(key)) continue;
    const at = rank.get(key);
    if (at === undefined) rest.push(item);
    else ranked.push({ item, at });
  }
  ranked.sort((a, b) => a.at - b.at);
  return [...ranked.map(r => r.item), ...rest];
}

/**
 * Move `id` one step within `ids`, returning the explicit order to store.
 *
 * Reordering is expressed as up/down rather than drag because the result has to
 * be reachable by keyboard, and because a drag target that is one row tall is
 * a poor way to move an item through a list of 45.
 */
export function moveWithin(ids: readonly string[], id: string, delta: -1 | 1): string[] {
  const from = ids.indexOf(id);
  if (from < 0) return [...ids];
  const to = from + delta;
  if (to < 0 || to >= ids.length) return [...ids];
  const next = [...ids];
  [next[from], next[to]] = [next[to], next[from]];
  return next;
}

/** Add or remove `key` from a hidden list. */
export function toggleHidden(hidden: readonly string[], key: string, hide: boolean): string[] {
  const has = hidden.includes(key);
  if (hide === has) return [...hidden];
  return hide ? [...hidden, key] : hidden.filter(k => k !== key);
}

// ── Moves ───────────────────────────────────────────────────────────────────

/**
 * Record (or clear) where a thing is hosted.
 *
 * Setting the destination back to where it ships **deletes** the entry rather
 * than storing an identity move. Preferences are a diff; an entry that says
 * "Design's Sketch tab lives in Design" is not a preference, and keeping it
 * would pin the tab to a group it might be moved out of in a later release.
 */
function setMove(
  moves: Readonly<Record<string, string>>,
  key: string,
  destination: string,
  shippedHome: string,
): Record<string, string> {
  const next = { ...moves };
  if (destination === shippedHome) delete next[key];
  else next[key] = destination;
  return next;
}

/** Host `panelId` in `group`, or send it home by passing its shipped group. */
export function movePanelToGroup(
  prefs: LayoutPrefs,
  panelId: string,
  group: string,
  shippedGroup: string,
): LayoutPrefs {
  return {
    ...prefs,
    moves: { ...prefs.moves, panels: setMove(prefs.moves.panels, panelId, group, shippedGroup) },
  };
}

/**
 * Host a tab in `destinationPanel`.
 *
 * The tab keeps its origin key, so its visibility and its position are still
 * addressed the same way wherever it lands — and the rank it held in the panel
 * it left is kept, not cleared, so moving it away and back puts it where it
 * was. A rank naming a tab that is elsewhere is inert: `applyLayout` ranks only
 * the items it is given.
 */
export function moveTabToPanel(
  prefs: LayoutPrefs,
  panelId: string,
  tabId: string,
  destinationPanel: string,
): LayoutPrefs {
  const key = tabKey(panelId, tabId);
  return {
    ...prefs,
    moves: { ...prefs.moves, tabs: setMove(prefs.moves.tabs, key, destinationPanel, panelId) },
  };
}

/** The panel currently hosting the tab named by `key`. */
export function tabHost(key: string, moves: Readonly<Record<string, string>>): string {
  return moves[key] ?? splitTabKey(key).panelId;
}

/** The keys of tabs moved *into* `panelId` from somewhere else. */
export function tabsMovedInto(
  panelId: string,
  moves: Readonly<Record<string, string>>,
): string[] {
  return Object.entries(moves)
    .filter(([key, dest]) => dest === panelId && splitTabKey(key).panelId !== panelId)
    .map(([key]) => key);
}

/**
 * Re-home panels according to `moves`, keeping each group's relative order.
 *
 * A destination that is not a real group is ignored. A stale preference — a
 * group renamed or dropped in a later release — must leave the panel where it
 * ships, not delete it from the layout: a panel nobody can reach is a worse
 * outcome than a move that stopped applying.
 */
export function applyPanelMoves(
  groups: readonly { label: string; tabs: readonly string[] }[],
  moves: Readonly<Record<string, string>>,
): { label: string; tabs: string[] }[] {
  const known = new Set(groups.map((g) => g.label));
  const homeOf = (panelId: string, shipped: string) => {
    const dest = moves[panelId];
    return dest && known.has(dest) ? dest : shipped;
  };
  const byGroup = new Map(groups.map((g) => [g.label, [] as string[]]));
  for (const group of groups) {
    for (const panelId of group.tabs) {
      byGroup.get(homeOf(panelId, group.label))!.push(panelId);
    }
  }
  return groups.map((g) => ({ label: g.label, tabs: byGroup.get(g.label) ?? [] }));
}

/**
 * The groups to render: moves applied, then ordered, then hidden entries
 * dropped.
 *
 * Shared by the sidebar and the settings screen so the two cannot disagree
 * about what the layout is. They differ in one thing only: settings shows what
 * is switched off, because it is where you go to switch it back on.
 */
export function resolveGroups(
  groups: readonly { label: string; tabs: readonly string[] }[],
  prefs: LayoutPrefs,
  opts: { includeHidden?: boolean } = {},
): { label: string; tabs: string[] }[] {
  const hiddenGroups = opts.includeHidden ? [] : prefs.hidden.groups;
  const hiddenPanels = opts.includeHidden ? [] : prefs.hidden.panels;
  return applyLayout(
    applyPanelMoves(groups, prefs.moves.panels),
    (g) => g.label,
    prefs.order.groups,
    hiddenGroups,
  ).map((g) => ({
    label: g.label,
    tabs: applyLayout(g.tabs, (t) => t, prefs.order.panels[g.label] ?? [], hiddenPanels),
  }));
}

// ── Storage ─────────────────────────────────────────────────────────────────

/**
 * Fill in any missing branch of a stored object.
 *
 * Preferences written by an older build lack the keys added since, and a
 * missing `hidden.tabs` must read as "nothing hidden" rather than crash the
 * whole layout on a `.includes` of undefined — a settings page is exactly the
 * place where a stored value is most likely to be from a different version.
 */
function coerce(raw: unknown): LayoutPrefs {
  const o = (raw ?? {}) as Partial<LayoutPrefs>;
  const arr = (v: unknown): string[] => (Array.isArray(v) ? v.filter(x => typeof x === "string") : []);
  const rec = (v: unknown): Record<string, string[]> => {
    if (!v || typeof v !== "object") return {};
    return Object.fromEntries(
      Object.entries(v as Record<string, unknown>).map(([k, val]) => [k, arr(val)]),
    );
  };
  const strRec = (v: unknown): Record<string, string> => {
    if (!v || typeof v !== "object") return {};
    return Object.fromEntries(
      Object.entries(v as Record<string, unknown>).filter(
        (e): e is [string, string] => typeof e[1] === "string" && e[1].length > 0,
      ),
    );
  };
  return {
    order: {
      groups: arr(o.order?.groups),
      panels: rec(o.order?.panels),
      tabs: rec(o.order?.tabs),
    },
    hidden: {
      groups: arr(o.hidden?.groups),
      panels: arr(o.hidden?.panels),
      tabs: arr(o.hidden?.tabs),
    },
    moves: {
      panels: strRec(o.moves?.panels),
      tabs: strRec(o.moves?.tabs),
    },
  };
}

export function loadLayoutPrefs(): LayoutPrefs {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === null) return EMPTY_PREFS;
    return coerce(JSON.parse(raw));
  } catch {
    // Unparseable or unavailable storage falls back to the shipped layout.
    // Refusing to render the app because a preference is corrupt would be a
    // worse outcome than ignoring the preference.
    return EMPTY_PREFS;
  }
}

type Listener = (prefs: LayoutPrefs) => void;
const listeners = new Set<Listener>();

/**
 * Watch for changes.
 *
 * The nav and every open composite read these preferences, and they are edited
 * in a modal on top of them. Without a notification the panels behind the
 * dialog keep the layout they mounted with, so a change appears to have been
 * ignored until the app restarts.
 */
export function subscribeLayoutPrefs(fn: Listener): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function saveLayoutPrefs(prefs: LayoutPrefs): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
  } catch {
    // Full or unavailable storage: the change still applies to this session
    // via the listeners below. Losing it at restart beats losing it now.
  }
  for (const fn of listeners) fn(prefs);
}

/** Restore the shipped layout. */
export function resetLayoutPrefs(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* nothing to remove */
  }
  for (const fn of listeners) fn(EMPTY_PREFS);
}
