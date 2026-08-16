// Derives panelId -> [{id,label}] by reading LazyPanels.tsx + the composite files.
// Output is checked in as constants/panelCatalog.ts and guarded by a test.
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

const out = {};
for (const [panelId, comp] of panelToComp) {
  const file = compToFile.get(comp);
  if (!file) { out[panelId] = []; continue; }
  let text;
  try {
    text = readFileSync(join(SRC, "components", file + ".tsx"), "utf8");
  } catch {
    out[panelId] = [];
    continue;
  }
  const tabs = [];
  // { id: "x", label: "Y", importFn: ... }
  for (const m of text.matchAll(/\{\s*id:\s*"([^"]+)",\s*label:\s*"([^"]+)"/g)) {
    tabs.push({ id: m[1], label: m[2] });
  }
  out[panelId] = tabs;
}

const total = Object.values(out).reduce((n, t) => n + t.length, 0);
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
`,
);
console.log(`panels=${Object.keys(out).length} tabs=${total}`);
