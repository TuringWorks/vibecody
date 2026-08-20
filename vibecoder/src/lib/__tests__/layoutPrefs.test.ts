/**
 * The ordering rules, tested directly.
 *
 * `applyLayout` decides what the nav and every tab strip render. It is a pure
 * function of (shipped list, preferences), so the interesting cases — a
 * preference written before a panel existed, an all-hidden panel, a stored
 * value from a different version — are reachable here without mounting
 * anything.
 */
import { describe, it, expect, beforeEach } from "vitest";
import {
  applyLayout,
  applyPanelMoves,
  moveWithin,
  movePanelToGroup,
  moveTabToPanel,
  resolveGroups,
  splitTabKey,
  tabHost,
  tabsMovedInto,
  toggleHidden,
  tabKey,
  loadLayoutPrefs,
  saveLayoutPrefs,
  resetLayoutPrefs,
  subscribeLayoutPrefs,
  EMPTY_PREFS,
  type LayoutPrefs,
} from "../layoutPrefs";

const id = (s: string) => s;

describe("applyLayout", () => {
  it("keeps the shipped order when nothing is configured", () => {
    expect(applyLayout(["a", "b", "c"], id, [], [])).toEqual(["a", "b", "c"]);
  });

  it("puts explicitly ordered items first, in that order", () => {
    expect(applyLayout(["a", "b", "c"], id, ["c", "a"], [])).toEqual(["c", "a", "b"]);
  });

  it("drops hidden items", () => {
    expect(applyLayout(["a", "b", "c"], id, [], ["b"])).toEqual(["a", "c"]);
  });

  // The rule the whole design rests on. Preferences are a diff: a panel added
  // after they were saved is absent from the stored order, and absent must mean
  // "not ranked yet", never "excluded" — otherwise every release would be
  // invisible to anyone who had ever touched this screen.
  it("shows items the stored preferences have never heard of", () => {
    const storedBeforeDAndEExisted = ["c", "a"];
    expect(applyLayout(["a", "b", "c", "d", "e"], id, storedBeforeDAndEExisted, [])).toEqual([
      "c",
      "a",
      "b",
      "d",
      "e",
    ]);
  });

  it("keeps unranked items in their original relative order", () => {
    expect(applyLayout(["a", "b", "c", "d"], id, ["d"], [])).toEqual(["d", "a", "b", "c"]);
  });

  it("ignores an order naming things that are gone", () => {
    // A panel removed in a later release must not leave a hole or throw.
    expect(applyLayout(["a", "b"], id, ["deleted-panel", "b"], [])).toEqual(["b", "a"]);
  });

  it("hides and orders together", () => {
    expect(applyLayout(["a", "b", "c"], id, ["c", "b", "a"], ["b"])).toEqual(["c", "a"]);
  });

  it("can empty a list entirely", () => {
    // Callers decide what to do about this; TabbedPanel falls back to the
    // shipped tabs rather than render a panel with nothing in it.
    expect(applyLayout(["a"], id, [], ["a"])).toEqual([]);
  });

  it("works on objects via the key function", () => {
    const items = [{ id: "x" }, { id: "y" }];
    expect(applyLayout(items, (i) => i.id, ["y"], [])).toEqual([{ id: "y" }, { id: "x" }]);
  });
});

describe("tabKey", () => {
  it("namespaces a tab by its panel", () => {
    // "dashboard" is a tab id in several panels. Hiding one must not hide the
    // others, so the key carries the panel.
    expect(tabKey("agent-os", "dashboard")).not.toEqual(tabKey("observability", "dashboard"));
  });
});

describe("moveWithin", () => {
  it("swaps with the neighbour above", () => {
    expect(moveWithin(["a", "b", "c"], "b", -1)).toEqual(["b", "a", "c"]);
  });

  it("swaps with the neighbour below", () => {
    expect(moveWithin(["a", "b", "c"], "b", 1)).toEqual(["a", "c", "b"]);
  });

  it("is a no-op at either end", () => {
    expect(moveWithin(["a", "b"], "a", -1)).toEqual(["a", "b"]);
    expect(moveWithin(["a", "b"], "b", 1)).toEqual(["a", "b"]);
  });

  it("is a no-op for an id that is not there", () => {
    expect(moveWithin(["a", "b"], "zzz", 1)).toEqual(["a", "b"]);
  });

  it("does not mutate its input", () => {
    const input = ["a", "b"];
    moveWithin(input, "a", 1);
    expect(input).toEqual(["a", "b"]);
  });
});

describe("toggleHidden", () => {
  it("adds and removes", () => {
    expect(toggleHidden([], "a", true)).toEqual(["a"]);
    expect(toggleHidden(["a"], "a", false)).toEqual([]);
  });

  it("is idempotent", () => {
    expect(toggleHidden(["a"], "a", true)).toEqual(["a"]);
    expect(toggleHidden([], "a", false)).toEqual([]);
  });
});

describe("storage", () => {
  beforeEach(() => localStorage.clear());

  it("round-trips", () => {
    const prefs: LayoutPrefs = {
      order: { groups: ["AI"], panels: { AI: ["chat"] }, tabs: { chat: ["sandbox"] } },
      hidden: { groups: [], panels: ["billing"], tabs: ["chat/sandbox"] },
      moves: { panels: { billing: "AI" }, tabs: { "design/sketch": "security" } },
    };
    saveLayoutPrefs(prefs);
    expect(loadLayoutPrefs()).toEqual(prefs);
  });

  it("reads preferences written before moves existed as nothing moved", () => {
    localStorage.setItem(
      "vibecoder-layout-prefs:v1",
      JSON.stringify({ hidden: { panels: ["billing"] } }),
    );
    // Not a crash and not a reset: a v1 payload keeps working and simply has
    // no moves in it.
    const got = loadLayoutPrefs();
    expect(got.hidden.panels).toEqual(["billing"]);
    expect(got.moves).toEqual({ panels: {}, tabs: {} });
  });

  it("drops a move whose destination is not a string", () => {
    localStorage.setItem(
      "vibecoder-layout-prefs:v1",
      JSON.stringify({ moves: { tabs: { "design/sketch": 7, "design/hub": "security" } } }),
    );
    expect(loadLayoutPrefs().moves.tabs).toEqual({ "design/hub": "security" });
  });

  it("returns the shipped layout when nothing is stored", () => {
    expect(loadLayoutPrefs()).toEqual(EMPTY_PREFS);
  });

  it("survives a corrupt stored value", () => {
    localStorage.setItem("vibecoder-layout-prefs:v1", "{not json");
    // Refusing to render the app because a preference will not parse is a
    // worse outcome than ignoring the preference.
    expect(loadLayoutPrefs()).toEqual(EMPTY_PREFS);
  });

  it("fills in branches an older build never wrote", () => {
    localStorage.setItem("vibecoder-layout-prefs:v1", JSON.stringify({ hidden: { panels: ["x"] } }));
    const got = loadLayoutPrefs();
    expect(got.hidden.panels).toEqual(["x"]);
    expect(got.hidden.tabs).toEqual([]);
    expect(got.order.groups).toEqual([]);
  });

  it("drops non-string junk rather than trusting it", () => {
    localStorage.setItem(
      "vibecoder-layout-prefs:v1",
      JSON.stringify({ hidden: { panels: ["ok", 42, null] } }),
    );
    expect(loadLayoutPrefs().hidden.panels).toEqual(["ok"]);
  });

  it("notifies subscribers on save and on reset", () => {
    // The nav and the open panels mount before Settings does, and Settings is a
    // modal on top of them. Without this they keep their mounted layout and the
    // change looks ignored until restart.
    const seen: unknown[] = [];
    const stop = subscribeLayoutPrefs((p) => seen.push(p));
    saveLayoutPrefs({ ...EMPTY_PREFS, hidden: { groups: [], panels: ["billing"], tabs: [] } });
    resetLayoutPrefs();
    stop();
    saveLayoutPrefs(EMPTY_PREFS);

    expect(seen).toHaveLength(2);
    expect((seen[0] as typeof EMPTY_PREFS).hidden.panels).toEqual(["billing"]);
    expect(seen[1]).toEqual(EMPTY_PREFS);
  });
});

// ── Moves ───────────────────────────────────────────────────────────────────

const GROUPS = [
  { label: "AI", tabs: ["chat", "agent-os"] },
  { label: "Code Quality", tabs: ["security", "testing"] },
];

const prefsWith = (moves: Partial<LayoutPrefs["moves"]>): LayoutPrefs => ({
  ...EMPTY_PREFS,
  moves: { panels: {}, tabs: {}, ...moves },
});

describe("applyPanelMoves", () => {
  it("leaves the shipped grouping alone when nothing moved", () => {
    expect(applyPanelMoves(GROUPS, {})).toEqual([
      { label: "AI", tabs: ["chat", "agent-os"] },
      { label: "Code Quality", tabs: ["security", "testing"] },
    ]);
  });

  it("re-homes a panel into another group", () => {
    expect(applyPanelMoves(GROUPS, { security: "AI" })).toEqual([
      { label: "AI", tabs: ["chat", "agent-os", "security"] },
      { label: "Code Quality", tabs: ["testing"] },
    ]);
  });

  it("ignores a destination group that no longer exists", () => {
    // A group renamed or dropped in a later release must leave the panel where
    // it ships. A panel nobody can reach is worse than a move that stopped
    // applying.
    expect(applyPanelMoves(GROUPS, { security: "Retired Group" })).toEqual([
      { label: "AI", tabs: ["chat", "agent-os"] },
      { label: "Code Quality", tabs: ["security", "testing"] },
    ]);
  });
});

describe("movePanelToGroup", () => {
  it("records the destination", () => {
    const next = movePanelToGroup(EMPTY_PREFS, "security", "AI", "Code Quality");
    expect(next.moves.panels).toEqual({ security: "AI" });
  });

  it("clears the entry when the panel goes back where it ships", () => {
    // Preferences are a diff. An entry saying a panel is where it already
    // belongs would pin it there if a later release regrouped it.
    const moved = movePanelToGroup(EMPTY_PREFS, "security", "AI", "Code Quality");
    const back = movePanelToGroup(moved, "security", "Code Quality", "Code Quality");
    expect(back.moves.panels).toEqual({});
  });
});

describe("moveTabToPanel", () => {
  it("keys the move by where the tab ships, not where it lands", () => {
    const next = moveTabToPanel(EMPTY_PREFS, "design", "sketch", "security");
    expect(next.moves.tabs).toEqual({ "design/sketch": "security" });
  });

  it("moves the same tab on again without losing track of it", () => {
    const once = moveTabToPanel(EMPTY_PREFS, "design", "sketch", "security");
    const twice = moveTabToPanel(once, "design", "sketch", "testing");
    expect(twice.moves.tabs).toEqual({ "design/sketch": "testing" });
  });

  it("clears the entry when the tab goes home", () => {
    const moved = moveTabToPanel(EMPTY_PREFS, "design", "sketch", "security");
    expect(moveTabToPanel(moved, "design", "sketch", "design").moves.tabs).toEqual({});
  });

  it("keeps the rank the tab held, so moving it away and back restores it", () => {
    // The stale entry is inert — `applyLayout` ranks only the items it is
    // handed — and it is what puts the tab back where it was on the return
    // trip instead of at the end of the list.
    const start: LayoutPrefs = {
      ...EMPTY_PREFS,
      order: { ...EMPTY_PREFS.order, tabs: { design: ["sketch", "hub"] } },
    };
    const away = moveTabToPanel(start, "design", "sketch", "security");
    expect(away.order.tabs.design).toEqual(["sketch", "hub"]);

    const back = moveTabToPanel(away, "design", "sketch", "design");
    expect(back.moves.tabs).toEqual({});
    expect(
      applyLayout(
        [{ key: "design/hub" }, { key: "design/sketch" }],
        (t) => t.key,
        back.order.tabs.design.map((id) => `design/${id}`),
        [],
      ).map((t) => t.key),
    ).toEqual(["design/sketch", "design/hub"]);
  });
});

describe("tabHost / tabsMovedInto", () => {
  it("answers with the shipping panel when nothing moved", () => {
    expect(tabHost("design/sketch", {})).toBe("design");
  });

  it("answers with the destination once moved", () => {
    expect(tabHost("design/sketch", { "design/sketch": "security" })).toBe("security");
  });

  it("lists only tabs that came from somewhere else", () => {
    const moves = { "design/sketch": "security", "security/scan": "security" };
    // `security/scan` names its own panel as the destination, which is not a
    // move — counting it would render the tab twice.
    expect(tabsMovedInto("security", moves)).toEqual(["design/sketch"]);
  });
});

describe("splitTabKey", () => {
  it("splits at the first slash", () => {
    expect(splitTabKey("design/sketch")).toEqual({ panelId: "design", tabId: "sketch" });
  });

  it("treats a bare id as having no panel", () => {
    // Order lists written before moves existed hold bare tab ids.
    expect(splitTabKey("sketch")).toEqual({ panelId: "", tabId: "sketch" });
  });
});

describe("resolveGroups", () => {
  it("applies moves, then order, then hiding", () => {
    const prefs: LayoutPrefs = {
      ...prefsWith({ panels: { security: "AI" } }),
      order: { groups: ["Code Quality"], panels: { AI: ["security"] }, tabs: {} },
      hidden: { groups: [], panels: ["agent-os"], tabs: [] },
    };
    expect(resolveGroups(GROUPS, prefs)).toEqual([
      { label: "Code Quality", tabs: ["testing"] },
      { label: "AI", tabs: ["security", "chat"] },
    ]);
  });

  it("keeps hidden entries listed for the settings screen", () => {
    const prefs: LayoutPrefs = {
      ...EMPTY_PREFS,
      hidden: { groups: ["AI"], panels: ["testing"], tabs: [] },
    };
    // Settings is where you go to switch something back on, so it cannot be
    // the one screen that stops showing it.
    expect(resolveGroups(GROUPS, prefs, { includeHidden: true })).toEqual([
      { label: "AI", tabs: ["chat", "agent-os"] },
      { label: "Code Quality", tabs: ["security", "testing"] },
    ]);
  });
});
