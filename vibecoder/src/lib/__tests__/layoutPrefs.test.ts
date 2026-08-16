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
  moveWithin,
  toggleHidden,
  tabKey,
  loadLayoutPrefs,
  saveLayoutPrefs,
  resetLayoutPrefs,
  subscribeLayoutPrefs,
  EMPTY_PREFS,
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
    const prefs = {
      order: { groups: ["AI"], panels: { AI: ["chat"] }, tabs: { chat: ["sandbox"] } },
      hidden: { groups: [], panels: ["billing"], tabs: ["chat/sandbox"] },
    };
    saveLayoutPrefs(prefs);
    expect(loadLayoutPrefs()).toEqual(prefs);
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
