#!/usr/bin/env node
/**
 * codemod-tab-class.mjs — when a <button> exists whose `onClick` is a
 * `setTab(...)` / `setActiveTab(...)` style call, add `panel-tab` to its
 * className so the audit recognizes it as a tab.
 *
 * Conservative: we only touch buttons (not divs) and only those whose onClick
 * passes a value to a setter that looks like a tab state setter.
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const COMPONENTS_DIR = new URL("../src/components/", import.meta.url).pathname;

function processFile(path) {
  const src = readFileSync(path, "utf8");
  // If the file already uses panel-tab anywhere, skip.
  if (/className\s*=\s*(?:["'`{][^"'`}]*\bpanel-tab|\{`[^`]*\bpanel-tab|\{"[^"]*\bpanel-tab)/.test(src)) return 0;
  // If it doesn't look tabby, skip.
  if (!/(setActiveTab|setTab\(|setTab[A-Z])/.test(src)) return 0;

  // Match button tags. Some are multi-line; cap with a brace-balanced scan.
  let count = 0;
  const out = [];
  let i = 0;
  const btnRe = /<button\b/g;
  let m;
  while ((m = btnRe.exec(src)) !== null) {
    const start = m.index;
    // Find matching `>` at depth 0
    let depth = 0;
    let inStr = null;
    let end = -1;
    for (let j = start + 7; j < src.length; j++) {
      const c = src[j];
      if (inStr) {
        if (c === "\\") { j++; continue; }
        if (c === inStr) inStr = null;
        continue;
      }
      if (c === '"' || c === "'" || c === "`") { inStr = c; continue; }
      if (c === "{") depth++;
      else if (c === "}") depth--;
      else if (c === ">" && depth === 0) { end = j; break; }
    }
    if (end < 0) break;
    const tag = src.slice(start, end + 1);

    // Does this button's onClick call a tab setter?
    const isTabBtn = /onClick=\{[^}]*\b(setActiveTab|setTab[A-Z]?\w*)\s*\(/.test(tag);
    if (!isTabBtn) {
      btnRe.lastIndex = end + 1;
      continue;
    }

    out.push(src.slice(i, start));

    // Add panel-tab to className
    const cn = /className\s*=\s*(("([^"]*)")|('([^']*)')|(\{`([^`]*)`\})|(\{"([^"]*)"\}))/.exec(tag);
    let newTag;
    if (cn) {
      const existing = cn[3] ?? cn[5] ?? cn[7] ?? cn[9] ?? "";
      if (existing.split(/\s+/).includes("panel-tab")) {
        out.push(tag);
        i = end + 1;
        btnRe.lastIndex = end + 1;
        continue;
      }
      const merged = `panel-tab ${existing}`.trim();
      const replacementValue = cn[2] ? `"${merged}"` : cn[4] ? `'${merged}'` : cn[6] ? `{\`${merged}\`}` : `{"${merged}"}`;
      newTag = tag.replace(cn[0], `className=${replacementValue}`);
    } else {
      newTag = tag.replace(/^<button\b/, '<button className="panel-tab"');
    }
    out.push(newTag);
    i = end + 1;
    btnRe.lastIndex = end + 1;
    count++;
  }
  if (count === 0) return 0;
  out.push(src.slice(i));
  writeFileSync(path, out.join(""));
  return count;
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
console.log(`Done — added panel-tab to ${total} buttons across ${touched} files.`);
