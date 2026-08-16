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
import { PANEL_CATALOG } from "../panelCatalog";
import { TAB_GROUPS } from "../tabGroups";

const SRC = resolve(__dirname, "../..");
const lazyText = readFileSync(join(SRC, "components/LazyPanels.tsx"), "utf8");

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
  const file = compToFile().get(panelToComp().get(panelId) ?? "");
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
    const rendered = [...panelToComp().keys()].sort();
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
