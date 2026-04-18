#!/usr/bin/env node
/**
 * codemod-add-panel-container.mjs — adds `panel-container` to the root <div>
 * of every *Panel.tsx that exports a *Panel component but doesn't currently
 * include the class.
 *
 * Strategy: find the FIRST `return (\n    <div ...>` after the file's panel
 * function/const declaration, then either:
 *   - add `className="panel-container"` if no className exists
 *   - prepend `panel-container ` inside the existing className value
 *
 * Skips files that already include `panel-container` anywhere.
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const COMPONENTS_DIR = new URL("../src/components/", import.meta.url).pathname;

function processFile(path) {
  const src = readFileSync(path, "utf8");

  // Skip non-panel files
  if (!/export\s+(default\s+)?(function|const)\s+\w*Panel\b/.test(src)) return false;
  if (src.includes("panel-container")) return false;

  // Locate the first `return (` after the panel declaration
  const declRe = /export\s+(?:default\s+)?(?:function|const)\s+(\w*Panel)\b/;
  const declMatch = declRe.exec(src);
  if (!declMatch) return false;
  const startIdx = declMatch.index;

  // Find the first `return (` at-or-after declIdx that's followed by a <div
  const returnRe = /return\s*\(\s*(<div\b[^>]*>)/g;
  returnRe.lastIndex = startIdx;
  const match = returnRe.exec(src);
  if (!match) return false;

  const fullDiv = match[1];
  const before = src.slice(0, match.index + match[0].length - fullDiv.length);
  const after = src.slice(match.index + match[0].length);

  // Decide how to inject the className
  let newDiv;
  const classNameMatch = /className\s*=\s*("([^"]*)"|'([^']*)'|\{`([^`]*)`\}|\{"([^"]*)"\}|\{'([^']*)'\})/.exec(fullDiv);
  if (classNameMatch) {
    // Merge: prepend `panel-container ` to existing string
    const existing = classNameMatch[2] ?? classNameMatch[3] ?? classNameMatch[4] ?? classNameMatch[5] ?? classNameMatch[6] ?? "";
    if (existing.split(/\s+/).includes("panel-container")) return false;
    const merged = `panel-container ${existing}`.trim();
    const newClassNameAttr = `className="${merged}"`;
    newDiv = fullDiv.replace(classNameMatch[0], newClassNameAttr);
  } else {
    // Inject before the closing `>`
    newDiv = fullDiv.replace(/^<div\b/, `<div className="panel-container"`);
  }

  const out = before + newDiv + after;
  writeFileSync(path, out);
  return true;
}

const files = readdirSync(COMPONENTS_DIR)
  .filter((f) => f.endsWith("Panel.tsx"))
  .map((f) => join(COMPONENTS_DIR, f))
  .filter((p) => statSync(p).isFile())
  .sort();

let touched = 0;
for (const path of files) {
  if (processFile(path)) {
    touched++;
    console.log(`  + ${path.split("/").pop()}`);
  }
}
console.log("");
console.log(`Done — added panel-container to ${touched} files.`);
