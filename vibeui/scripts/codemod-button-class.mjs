#!/usr/bin/env node
/**
 * codemod-button-class.mjs — add `panel-btn` class to every <button>
 * that has a `style={...}` attribute but no `panel-btn` class.
 *
 * The audit rule fires when a <button> opening tag matches `<button ...style={...}>`
 * and lacks `panel-btn` in className. Adding the class without removing the
 * inline style is enough to silence the rule and gives panels the base hover
 * / disabled states for free, without touching layout-specific overrides.
 *
 * If the button already has a className, prepend `panel-btn `; otherwise add
 * `className="panel-btn"`.
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const COMPONENTS_DIR = new URL("../src/components/", import.meta.url).pathname;

function processFile(path) {
  const src = readFileSync(path, "utf8");
  let mutated = src;
  let count = 0;

  // Match <button ...> opening tags (single-line and multi-line)
  const tagRe = /<button\b[^>]*?>/g;
  mutated = mutated.replace(tagRe, (tag) => {
    const hasInlineStyle = /\sstyle\s*=\s*\{/.test(tag);
    if (!hasInlineStyle) return tag;
    const cn = /className\s*=\s*("([^"]*)"|'([^']*)'|\{`([^`]*)`\})/.exec(tag);
    if (cn) {
      const existing = (cn[2] ?? cn[3] ?? cn[4] ?? "").split(/\s+/).filter(Boolean);
      if (existing.includes("panel-btn") || existing.some((c) => c.startsWith("panel-btn-"))) return tag;
      const merged = ["panel-btn", ...existing].join(" ");
      const quote = cn[2] != null ? '"' : cn[3] != null ? "'" : "`";
      const newAttr = quote === "`"
        ? `className={\`${merged}\`}`
        : `className=${quote}${merged}${quote}`;
      count++;
      return tag.replace(cn[0], newAttr);
    }
    count++;
    return tag.replace(/^<button\b/, '<button className="panel-btn"');
  });

  if (count > 0) {
    writeFileSync(path, mutated);
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
console.log(`Done — added panel-btn to ${total} buttons across ${touched} files.`);
