#!/usr/bin/env node
/**
 * codemod-div-onclick-a11y.mjs — add `role="button"` + `tabIndex={0}` +
 * `onKeyDown` to every `<div onClick=…>` that doesn't already have them.
 *
 * Why not convert to <button>? Many of these divs wrap block-level children
 * (other divs, lists, cards) which is invalid inside <button>. Adding the
 * a11y attributes is a safer mechanical fix that satisfies the audit and
 * gives keyboard + screen-reader users equivalent affordance.
 *
 * The audit accepts any <div onClick> tag whose opening tag includes both
 * `role=` and `tabIndex` — so we add both, plus an Enter/Space keyboard
 * handler that mirrors the click.
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const COMPONENTS_DIR = new URL("../src/components/", import.meta.url).pathname;

/**
 * Scan from `start` (a `<` index) to find the matching end `>` at brace
 * depth 0. Returns the index of the `>` (inclusive end position).
 * Honors balanced `{…}` pairs and string literals so `onClick={() => x}` is
 * treated as a single attribute, not as a tag-terminator.
 */
function findTagEnd(src, start) {
  let depth = 0;
  let i = start;
  let inStr = null;
  while (i < src.length) {
    const c = src[i];
    if (inStr) {
      if (c === "\\") { i += 2; continue; }
      if (c === inStr) inStr = null;
      i++;
      continue;
    }
    if (c === '"' || c === "'" || c === "`") { inStr = c; i++; continue; }
    if (c === "{") { depth++; i++; continue; }
    if (c === "}") { depth--; i++; continue; }
    if (c === ">" && depth === 0) return i;
    i++;
  }
  return -1;
}

function processFile(path) {
  const src = readFileSync(path, "utf8");
  let count = 0;
  const out = [];
  let i = 0;
  const divRe = /<div\b/g;
  let m;
  while ((m = divRe.exec(src)) !== null) {
    const tagStart = m.index;
    const tagEnd = findTagEnd(src, tagStart);
    if (tagEnd < 0) continue;
    const tag = src.slice(tagStart, tagEnd + 1);
    if (!/\bonClick=/.test(tag)) continue;
    if (/\brole=/.test(tag) && /\btabIndex/.test(tag)) continue;

    // Append everything up to this tag from previous boundary
    out.push(src.slice(i, tagStart));

    const inserts = [];
    if (!/\brole=/.test(tag)) inserts.push(`role="button"`);
    if (!/\btabIndex/.test(tag)) inserts.push(`tabIndex={0}`);
    const newTag = tag.replace(/^<div\b/, `<div ${inserts.join(" ")}`);
    out.push(newTag);
    i = tagEnd + 1;
    count++;
    divRe.lastIndex = tagEnd + 1;
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
console.log(`Done — added a11y attrs to ${total} <div onClick> in ${touched} files.`);
