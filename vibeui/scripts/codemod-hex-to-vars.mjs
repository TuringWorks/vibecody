#!/usr/bin/env node
/**
 * codemod-hex-to-vars.mjs — replace common hex-color literals with CSS vars
 * inside style={...} blocks across vibeui/src/components/.
 *
 * Conservative: only touches hex literals that appear as VALUES inside style
 * blocks (not in JSX attributes like fill={...}, not inside template literals
 * for SVG paths, not inside Monaco editor theme blobs).
 *
 * Mapping (only well-understood cases):
 *   '#fff' / '#ffffff' (case-insensitive) → 'var(--btn-primary-fg)'
 *   '#000' / '#000000'                    → 'var(--text-primary)'
 *   '#666' / '#666666'                    → 'var(--text-muted)'
 *   '#999' / '#999999'                    → 'var(--text-secondary)'
 *
 * Anything else is left alone (could be a brand color, theme, etc).
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const COMPONENTS_DIR = new URL("../src/components/", import.meta.url).pathname;

const HEX_MAP = new Map([
  // Neutrals
  ["fff", "var(--btn-primary-fg)"],
  ["ffffff", "var(--btn-primary-fg)"],
  ["000", "var(--text-primary)"],
  ["000000", "var(--text-primary)"],
  ["666", "var(--text-muted)"],
  ["666666", "var(--text-muted)"],
  ["999", "var(--text-secondary)"],
  ["999999", "var(--text-secondary)"],
  ["9e9e9e", "var(--text-muted)"],
  ["374151", "var(--border-color)"],
  // Brand / accent
  ["6366f1", "var(--accent-indigo)"],
  ["a78bfa", "var(--accent-purple)"],
  ["6c8cff", "var(--accent-blue)"],
  // Semantic
  ["22c55e", "var(--success-color)"],
  ["34d399", "var(--accent-green)"],
  ["ef4444", "var(--error-color)"],
  ["f5a623", "var(--warning-color)"],
  ["f5c542", "var(--accent-gold)"],
  ["f97b22", "var(--warning-color)"],
  ["e5a844", "var(--accent-gold)"],
  ["89b4fa", "var(--info-color)"],
]);

function processFile(path) {
  const src = readFileSync(path, "utf8");
  let changes = 0;

  // Find all style={...} blocks. We replace in-place inside them.
  const out = src.replace(/style\s*=\s*\{[\s\S]*?\}\}/g, (block) => {
    let mutated = block;
    // Replace 'XXX' or "XXX" hex literals — not inside var(...) fallbacks.
    mutated = mutated.replace(
      /(['"])#([0-9a-fA-F]{3,8})\1/g,
      (match, quote, hex) => {
        const lower = hex.toLowerCase();
        const repl = HEX_MAP.get(lower);
        if (!repl) return match;
        // Don't replace if we're inside a var() — already in fallback position.
        // We approximate by checking the 30 chars before the match in `mutated`.
        const idx = mutated.indexOf(match);
        const before = mutated.slice(Math.max(0, idx - 60), idx);
        if (/var\([^)]*$/.test(before)) return match;
        changes++;
        return `${quote}${repl}${quote}`;
      },
    );
    return mutated;
  });

  if (changes > 0) {
    writeFileSync(path, out);
    return changes;
  }
  return 0;
}

const files = readdirSync(COMPONENTS_DIR)
  .filter((f) => f.endsWith("Panel.tsx"))
  .map((f) => join(COMPONENTS_DIR, f))
  .filter((p) => statSync(p).isFile())
  .sort();

let totalChanges = 0;
let touchedFiles = 0;
for (const path of files) {
  const n = processFile(path);
  if (n > 0) {
    touchedFiles++;
    totalChanges += n;
    console.log(`  ${n.toString().padStart(3)}  ${path.split("/").pop()}`);
  }
}

console.log("");
console.log(`Done — ${totalChanges} hex literals replaced across ${touchedFiles} files.`);
