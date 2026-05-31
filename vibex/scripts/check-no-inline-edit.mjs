#!/usr/bin/env node
/**
 * VX-013 — CI grep-gate enforcing the locked product decision:
 * VibeX must NOT contain Cmd+K inline-completion / InlineChat / FIM / ghost-text
 * editing. AI edits go through conversation+Review or the ⌘. DiffCompleteModal
 * surface only (see pdm/08 §1). This script fails the build on any violation.
 *
 * Usage: node scripts/check-no-inline-edit.mjs   (run from vibex/)
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, extname } from "node:path";

// Patterns that indicate the banned inline-completion / Cmd+K edit surface.
// Each entry: [regex, human reason]. Kept narrow to avoid false positives —
// e.g. we ban the *edit trigger*, not every mention of the Cmd key.
const BANNED = [
  [/registerInlineCompletionsProvider/, "inline-completion provider (banned)"],
  [/\bInlineChat\b/, "InlineChat component (deleted, banned)"],
  [/\bregisterInlineCompletions\b/, "inline completions (banned)"],
  [/fill[-_]?in[-_]?the[-_]?middle|\bFIM\b/i, "fill-in-the-middle completion (banned)"],
  [/ghost[-_ ]?text/i, "ghost-text completion (banned)"],
  [/(cmd|⌘|ctrl|meta)\s*\+?\s*k\b[^.]{0,40}(edit|inline|complete)/i, "Cmd+K inline-edit trigger (banned)"],
];

// Allow this guard file itself (it necessarily names the patterns) and the docs.
const SELF = "check-no-inline-edit.mjs";

const SRC_DIRS = ["src", "src-tauri/src"];
const EXTS = new Set([".ts", ".tsx", ".js", ".jsx", ".rs", ".css"]);

function walk(dir, out = []) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return out;
  }
  for (const name of entries) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) {
      if (name === "node_modules" || name === "target" || name === "dist") continue;
      walk(p, out);
    } else if (EXTS.has(extname(name)) && name !== SELF) {
      out.push(p);
    }
  }
  return out;
}

const violations = [];
for (const dir of SRC_DIRS) {
  for (const file of walk(dir)) {
    const text = readFileSync(file, "utf8");
    const lines = text.split("\n");
    lines.forEach((line, i) => {
      // Skip lines that are explicit negations / comments about the ban.
      if (/\bNO Cmd\+K\b|banned|do NOT|not adopt|never reintroduce/i.test(line)) return;
      for (const [re, reason] of BANNED) {
        if (re.test(line)) {
          violations.push(`${file}:${i + 1}  ${reason}\n    ${line.trim()}`);
        }
      }
    });
  }
}

if (violations.length > 0) {
  console.error("❌ VX-013: banned inline-edit / Cmd+K patterns found in vibex/:\n");
  console.error(violations.join("\n\n"));
  console.error(
    "\nVibeX uses conversation+Review and the ⌘. DiffCompleteModal surface only. See pdm/08 §1."
  );
  process.exit(1);
}

console.log("✓ VX-013: no banned inline-edit / Cmd+K patterns in vibex/");
