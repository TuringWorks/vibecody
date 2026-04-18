#!/usr/bin/env node
/**
 * codemod-grid-snap.mjs — round off-grid padding/margin/gap pixel values
 * to the nearest 4-px-grid value.
 *
 * Grid: 0,1,2,3,4,8,12,16,20,24,28,32,40,48,56,64 (4px steps to 32, 8px after).
 *
 * Snap rules:
 *   ≤ 4  → 4         (5 → 4 doesn't usually happen; 1/2/3 are allowed already)
 *   5..32 → nearest of [4,8,12,16,20,24,28,32]
 *   >32  → nearest of [40,48,56,64], capped at 64
 *
 * Only touches values inside style={...} blocks where the property name is
 * `padding*`, `margin*`, or `gap*`. Skips template-literal interpolations
 * like `${x}px` (we can't infer the value).
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const COMPONENTS_DIR = new URL("../src/components/", import.meta.url).pathname;

const ALLOWED = [0, 1, 2, 3, 4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64];

function snap(n) {
  if (ALLOWED.includes(n)) return n;
  // Find the closest allowed value (ties round up)
  let best = ALLOWED[0];
  let bestDist = Math.abs(n - best);
  for (const v of ALLOWED) {
    const d = Math.abs(n - v);
    if (d < bestDist || (d === bestDist && v > best)) {
      best = v;
      bestDist = d;
    }
  }
  return best;
}

function processFile(path) {
  const src = readFileSync(path, "utf8");
  let count = 0;

  // Walk all style={…} blocks
  const styleRe = /style\s*=\s*\{\{([\s\S]*?)\}\}/g;
  const out = src.replace(styleRe, (full, body) => {
    let mutated = body;
    // For each padding/margin/gap (with optional axis suffix), find numeric Npx values.
    const propRe = /(\b(?:padding|margin|gap)(?:Top|Right|Bottom|Left|Block|Inline)?\s*:\s*)((?:["'][^"']*["']|[^,}]+))/g;
    mutated = mutated.replace(propRe, (whole, prefix, value) => {
      // Skip template literals and JS expressions we can't parse safely
      if (value.includes("${") || value.includes("`")) return whole;
      // Replace all `\d+px` tokens in the value
      const newValue = value.replace(/\b(\d+)px\b/g, (m, digits) => {
        const n = parseInt(digits, 10);
        const snapped = snap(n);
        if (snapped === n) return m;
        count++;
        return `${snapped}px`;
      });
      return prefix + newValue;
    });
    if (mutated === body) return full;
    return `style={{${mutated}}}`;
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
    console.log(`  ${n.toString().padStart(4)}  ${path.split("/").pop()}`);
  }
}
console.log("");
console.log(`Done — snapped ${total} values to grid across ${touched} files.`);
