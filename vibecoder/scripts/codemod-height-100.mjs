#!/usr/bin/env node
/**
 * codemod-height-100.mjs — replace `height: '100%'` with `flex: 1, minHeight: 0`
 * inside style blocks that look like layout containers, not progress-bar fills.
 *
 * Heuristic: skip the substitution if the same style block also contains a
 * `width:` with a percentage template literal (e.g. `width: ${pct}%` or
 * `width: "50%"`) — those are visual bars/fills, not layout containers.
 *
 * Otherwise, replace `height: "100%"` (and single-quote variant) with
 * `flex: 1, minHeight: 0`.
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const COMPONENTS_DIR = new URL("../src/components/", import.meta.url).pathname;

function processFile(path) {
  const src = readFileSync(path, "utf8");
  let count = 0;

  // Walk all style={...} blocks
  const styleRe = /style\s*=\s*\{\{([\s\S]*?)\}\}/g;
  const out = src.replace(styleRe, (full, body) => {
    if (!/height\s*:\s*['"]?100%/.test(body)) return full;
    // Skip progress-bar style blocks (width also present with % value)
    if (/width\s*:\s*[`'"]\$?\{?[^,]*%/.test(body) || /width\s*:\s*[`'"][^"'`]*%[^"'`]*[`'"]/.test(body)) {
      return full;
    }
    const newBody = body.replace(/height\s*:\s*['"]100%['"]/g, "flex: 1, minHeight: 0");
    if (newBody === body) return full;
    count++;
    return `style={{${newBody}}}`;
  });

  if (count > 0) {
    writeFileSync(path, out);
    return count;
  }
  return 0;
}

const files = readdirSync(COMPONENTS_DIR)
  .filter((f) => f.endsWith("Panel.tsx"))
  .map((f) => join(COMPONENTS_DIR, f))
  .filter((p) => statSync(p).isFile())
  .sort();

let total = 0;
let touched = 0;
for (const path of files) {
  const n = processFile(path);
  if (n > 0) {
    touched++;
    total += n;
    console.log(`  ${n.toString().padStart(3)}  ${path.split("/").pop()}`);
  }
}
console.log("");
console.log(`Done — replaced ${total} height: 100% in ${touched} files.`);
