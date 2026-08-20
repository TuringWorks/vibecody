// Derives panelId -> [{id,label}] by reading LazyPanels.tsx + the composite files.
// Writes two files, both checked in and both guarded by a test:
//
//   constants/panelCatalog.ts  — what tabs exist, for the settings list
//   constants/tabRegistry.ts   — how to load one, for hosting it in another panel
//
// The registry is generated rather than assembled at import time on purpose: a
// module that imported all 41 composites to collect their tab definitions would
// pull every one of them into the main bundle and undo the code splitting in
// LazyPanels. Emitting the `import()` thunks as literals keeps each panel in its
// own chunk, loaded only when something renders it.
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const SRC = process.argv[2] ?? new URL("../src/", import.meta.url).pathname;
const lazy = readFileSync(join(SRC, "components/LazyPanels.tsx"), "utf8");

// const FooComposite = lazy(() => import("./composite/FooComposite")...
const compToFile = new Map();
for (const m of lazy.matchAll(/const (\w+) = lazy\(\(\) => import\("\.\/([^"]+)"\)/g)) {
  compToFile.set(m[1], m[2]);
}

// {panel("id", <LazyPanel Component={FooComposite}
const panelToComp = new Map();
for (const m of lazy.matchAll(/panel\("([^"]+)",\s*<LazyPanel Component=\{(\w+)\}/g)) {
  panelToComp.set(m[1], m[2]);
}

// Panels rendered outside that switch — shared with the contract test so the
// generator and its guard cannot disagree about which panels exist.
const external = JSON.parse(
  readFileSync(new URL("./external-panels.json", import.meta.url), "utf8"),
);
const panelToFile = new Map([
  ...[...panelToComp].map(([panelId, comp]) => [panelId, compToFile.get(comp)]),
  ...Object.entries(external).filter(([panelId]) => !panelId.startsWith("_")),
]);

const out = {};
for (const [panelId, file] of panelToFile) {
  if (!file) { out[panelId] = []; continue; }
  let text;
  try {
    text = readFileSync(join(SRC, "components", file + ".tsx"), "utf8");
  } catch {
    out[panelId] = [];
    continue;
  }
  const tabs = [];
  // { id: "x", label: "Y", importFn: () => import("../Z"), exportName: "Z" }
  //
  // `importFn` is optional: the two hand-built composites (Chat, Diagnostics)
  // inline their content instead, and their tabs take bespoke props no generic
  // host could supply. Those are catalogued but not registered, which is what
  // makes them un-movable in Settings — stated there, not silently dropped.
  const re =
    /\{\s*id:\s*"([^"]+)",\s*label:\s*"([^"]+)"(?:,\s*importFn:\s*\(\)\s*=>\s*import\("([^"]+)"\))?(?:,\s*exportName:\s*"([^"]+)")?/g;
  for (const m of text.matchAll(re)) {
    tabs.push({ id: m[1], label: m[2], from: m[3], exportName: m[4] });
  }
  out[panelId] = tabs;
}

/**
 * Rewrite a composite's import specifier so it resolves from `src/constants/`.
 *
 * Unknown shapes throw rather than being skipped: a tab quietly missing from
 * the registry is a tab Settings offers to move and then cannot render.
 */
function rebase(spec, panelId, tabId) {
  if (spec.startsWith("../")) return "../components/" + spec.slice(3);
  if (spec.startsWith("./")) return "../components/composite/" + spec.slice(2);
  throw new Error(
    `${panelId}/${tabId}: cannot rebase import ${JSON.stringify(spec)} — ` +
      `teach gen-panel-catalog.mjs this form before using it in a composite.`,
  );
}

const total = Object.values(out).reduce((n, t) => n + t.length, 0);
const movableKeys = Object.entries(out).flatMap(([panel, tabs]) =>
  tabs.filter((t) => t.from).map((t) => `${panel}/${t.id}`),
);
const movable = movableKeys.length;

// Built before the catalog is written: the catalog's doc comment cites its size.
const registry = Object.entries(out)
  .flatMap(([panel, tabs]) =>
    tabs
      .filter((t) => t.from)
      .map((t) => {
        const key = `${panel}/${t.id}`;
        const spec = rebase(t.from, panel, t.id);
        const named = t.exportName ? `, exportName: ${JSON.stringify(t.exportName)}` : "";
        return `  ${JSON.stringify(key)}: { panelId: ${JSON.stringify(panel)}, tabId: ${JSON.stringify(t.id)}, label: ${JSON.stringify(t.label)}, load: () => import(${JSON.stringify(spec)})${named} },`;
      }),
  )
  .join("\n");
const body = Object.entries(out)
  .map(([panel, tabs]) => {
    const entries = tabs.map(t => `    { id: ${JSON.stringify(t.id)}, label: ${JSON.stringify(t.label)} },`).join("\n");
    return `  ${JSON.stringify(panel)}: [\n${entries}\n  ],`;
  })
  .join("\n");

writeFileSync(
  join(SRC, "constants/panelCatalog.ts"),
  `/**
 * Every panel and the subfeature tabs it contains.
 *
 * Pure data on purpose. Settings needs to list all ${total} subfeatures across
 * ${Object.keys(out).length} panels without pulling in the panels themselves —
 * importing the composites to read their tab lists would defeat the code
 * splitting that keeps startup cheap, loading every panel in the app the
 * moment someone opens Settings.
 *
 * The cost of a hand-maintained copy is drift, so it is not hand-maintained:
 * \`panelCatalog.contract.test.ts\` reads the composite sources and fails if
 * this file disagrees with them.
 *
 * GENERATED — do not edit. Run \`npm run gen:panel-catalog\` after adding,
 * removing or renaming a tab.
 */

export interface SubfeatureMeta {
  id: string;
  label: string;
}

/** Panel id -> its tabs, in the order the composite declares them. */
export const PANEL_CATALOG: Record<string, SubfeatureMeta[]> = {
${body}
};

/** Panels that render a single view and have no subfeature tabs. */
export const SINGLE_VIEW_PANELS: string[] = Object.entries(PANEL_CATALOG)
  .filter(([, tabs]) => tabs.length === 0)
  .map(([panel]) => panel);

/**
 * Tabs Settings may offer to re-home in another panel — the ones \`tabRegistry\`
 * knows how to load on their own.
 *
 * Here rather than read from \`tabRegistry\` because Settings needs the answer
 * for every row and the registry is ${Math.round(registry.length / 1024)} KB of import thunks that only a panel
 * actually hosting a moved tab should have to load.
 */
export const MOVABLE_TABS: readonly string[] = [
${movableKeys.map((k) => `  ${JSON.stringify(k)},`).join("\n")}
];
`,
);

// ── constants/tabRegistry.ts ────────────────────────────────────────────────

writeFileSync(
  join(SRC, "constants/tabRegistry.ts"),
  `/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * How to render any movable tab, from any panel.
 *
 * Settings lets a tab be hosted by a panel other than the one it ships in. The
 * hosting panel has never imported it, so it needs a way to load it by key —
 * that is this file.
 *
 * Only tabs declared through \`createComposite\` are here. The two hand-built
 * composites (Chat, Diagnostics) build their content inline and pass bespoke
 * props — a chat tab needs its provider list, its pending-write callback and
 * its goal handler — and no generic host can supply those. Settings reads
 * absence from this map as "not movable" and says so on the row.
 *
 * Loading is by \`import()\` thunk, so importing this module costs the map and
 * nothing else: each panel still arrives in its own chunk when first rendered.
 *
 * GENERATED — do not edit. Run \`npm run gen:panel-catalog\`.
 */

import type { ComponentType } from "react";

export interface RegisteredTab {
  /** The panel this tab ships in — its identity, which a move does not change. */
  panelId: string;
  tabId: string;
  label: string;
  load: () => Promise<{ default: ComponentType<any> } | Record<string, ComponentType<any>>>;
  /** Named export to pick out of the module, when it has no default. */
  exportName?: string;
}

/** \`panelId/tabId\` -> how to render it. ${movable} of ${total} tabs. */
export const TAB_REGISTRY: Record<string, RegisteredTab> = {
${registry}
};

/** Whether Settings may offer to move this tab to another panel. */
export function isMovableTab(key: string): boolean {
  return key in TAB_REGISTRY;
}
`,
);

console.log(`panels=${Object.keys(out).length} tabs=${total} movable=${movable}`);
