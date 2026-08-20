/**
 * Contract: the catalog Settings reads matches the panels that actually exist.
 *
 * `PANEL_CATALOG` is a hand-checked-in copy of every panel's tab list, kept
 * separate so opening Settings does not have to import all 45 panels just to
 * name their 234 tabs. A copy that nothing checks is a copy that goes stale,
 * and the failure is quiet in the worst way: a tab added last week is simply
 * absent from Settings, so it cannot be turned off and nobody finds out why.
 *
 * So this reads the sources — `LazyPanels.tsx` for which composite serves which
 * panel, then each composite for its tabs — and fails if the catalog disagrees.
 * Regenerate the catalog rather than editing it to match.
 *
 * It also enforces the wiring the preference depends on: a composite whose tabs
 * are in the catalog but which never passes `panelId` renders a tab strip that
 * silently ignores everything Settings was told.
 */
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve, join } from "node:path";
import { MOVABLE_TABS, PANEL_CATALOG } from "../panelCatalog";
import { TAB_REGISTRY } from "../tabRegistry";
import { TAB_GROUPS } from "../tabGroups";

const SRC = resolve(__dirname, "../..");
const lazyText = readFileSync(join(SRC, "components/LazyPanels.tsx"), "utf8");

/**
 * Panels rendered somewhere other than the AI panel's lazy switch. Settings
 * still lists their tabs, so the catalog still has to be true about them —
 * only the place the test looks for the composite changes.
 */
/* Read from the same file the generator reads, so the guard and the thing it
 * guards cannot drift into disagreeing about which panels exist. */
const EXTERNAL_PANELS: Record<string, string> = Object.fromEntries(
  Object.entries(
    JSON.parse(readFileSync(resolve(__dirname, "../../../scripts/external-panels.json"), "utf8")),
  ).filter(([panelId]) => !panelId.startsWith("_")),
) as Record<string, string>;

/** Composite variable name -> module path, from the lazy import lines. */
function compToFile(): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of lazyText.matchAll(/const (\w+) = lazy\(\(\) => import\("\.\/([^"]+)"\)/g)) {
    out.set(m[1], m[2]);
  }
  return out;
}

/** Panel id -> composite variable name, from the `panel("id", <LazyPanel …>)` calls. */
function panelToComp(): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of lazyText.matchAll(/panel\("([^"]+)",\s*<LazyPanel Component=\{(\w+)\}/g)) {
    out.set(m[1], m[2]);
  }
  return out;
}

function sourceOf(panelId: string): { path: string; text: string } | null {
  const file = EXTERNAL_PANELS[panelId] ?? compToFile().get(panelToComp().get(panelId) ?? "");
  if (!file) return null;
  const path = join(SRC, "components", file + ".tsx");
  try {
    return { path, text: readFileSync(path, "utf8") };
  } catch {
    return null;
  }
}

/** Tabs a composite declares, in source order. */
function declaredTabs(text: string): { id: string; label: string }[] {
  return [...text.matchAll(/\{\s*id:\s*"([^"]+)",\s*label:\s*"([^"]+)"/g)].map((m) => ({
    id: m[1],
    label: m[2],
  }));
}

describe("PANEL_CATALOG", () => {
  it("names every panel the app can render", () => {
    const rendered = [...panelToComp().keys(), ...Object.keys(EXTERNAL_PANELS)].sort();
    expect(Object.keys(PANEL_CATALOG).sort()).toEqual(rendered);
  });

  it("lists each panel's tabs exactly as its composite declares them", () => {
    const drift: string[] = [];
    for (const panelId of Object.keys(PANEL_CATALOG)) {
      const src = sourceOf(panelId);
      if (!src) continue;
      const actual = declaredTabs(src.text);
      const listed = PANEL_CATALOG[panelId];
      if (JSON.stringify(actual) !== JSON.stringify(listed)) {
        drift.push(
          `${panelId}\n    catalog: ${JSON.stringify(listed.map((t) => t.id))}\n    source:  ${JSON.stringify(actual.map((t) => t.id))}`,
        );
      }
    }
    expect(drift, `catalog is stale — regenerate it:\n  ${drift.join("\n  ")}`).toEqual([]);
  });

  it("wires panelId into every composite whose tabs it lists", () => {
    // Without `panelId` the tab strip renders its shipped order and ignores the
    // preference, so Settings would offer a control that does nothing.
    const unwired: string[] = [];
    for (const [panelId, tabs] of Object.entries(PANEL_CATALOG)) {
      if (tabs.length === 0) continue;
      const src = sourceOf(panelId);
      if (!src) continue;
      if (!src.text.includes(`panelId: "${panelId}"`) && !src.text.includes(`panelId="${panelId}"`)) {
        unwired.push(`${panelId} (${src.path})`);
      }
    }
    expect(unwired, `these composites cannot be reordered:\n  ${unwired.join("\n  ")}`).toEqual([]);
  });

  it("covers every panel reachable from the nav", () => {
    // A panel in a nav group but not in the catalog appears in Settings with no
    // subfeatures, which reads as "this panel has no tabs" rather than "nobody
    // recorded them".
    const missing = TAB_GROUPS.flatMap((g) => g.tabs).filter((p) => !(p in PANEL_CATALOG));
    expect(missing).toEqual([]);
  });
});

/**
 * The registry is the other half of the same generated pair: the catalog says
 * a tab exists, the registry says how to render it somewhere else. A tab added
 * to a composite without regenerating would be offered as movable — or not
 * offered at all — and only fail once someone actually moved it.
 */
describe("TAB_REGISTRY", () => {
  /** Tabs whose composite declares an `importFn`, so a generic host can load them. */
  function loadableTabs(text: string): string[] {
    return [
      ...text.matchAll(
        /\{\s*id:\s*"([^"]+)",\s*label:\s*"[^"]+",\s*importFn:\s*\(\)\s*=>\s*import\("([^"]+)"\)/g,
      ),
    ].map((m) => m[1]);
  }

  it("holds exactly the tabs a generic host can load", () => {
    const expected: string[] = [];
    for (const panelId of Object.keys(PANEL_CATALOG)) {
      const src = sourceOf(panelId);
      if (!src) continue;
      expected.push(...loadableTabs(src.text).map((id) => `${panelId}/${id}`));
    }
    expect(Object.keys(TAB_REGISTRY).sort()).toEqual(expected.sort());
  });

  it("agrees with the movable list Settings reads", () => {
    // Settings decides whether to enable the move control from MOVABLE_TABS and
    // the panel loads from TAB_REGISTRY. If those disagree, Settings offers a
    // move that renders an empty tab, or refuses one that would have worked.
    expect([...MOVABLE_TABS].sort()).toEqual(Object.keys(TAB_REGISTRY).sort());
  });

  it("names a tab the catalog also knows about", () => {
    const unknown = Object.values(TAB_REGISTRY).filter(
      (t) => !(PANEL_CATALOG[t.panelId] ?? []).some((c) => c.id === t.tabId),
    );
    expect(unknown.map((t) => `${t.panelId}/${t.tabId}`)).toEqual([]);
  });

  it("keys every entry by the panel and tab it names", () => {
    // The key is the tab's identity — hiding, ordering and moving all address
    // it. An entry filed under the wrong key would be un-hideable.
    const mismatched = Object.entries(TAB_REGISTRY)
      .filter(([key, t]) => key !== `${t.panelId}/${t.tabId}`)
      .map(([key]) => key);
    expect(mismatched).toEqual([]);
  });
});
