#!/usr/bin/env node
/**
 * Contract: every command VibeDesk invokes is registered in Tauri.
 *
 * `invoke("some_command")` joins the frontend to Rust through a bare string.
 * Nothing checks it: `tsc` sees a string literal and `rustc` sees a handler
 * list nobody imports, so a panel can call a command that was never registered
 * and the mistake surfaces only when a user clicks the button. That has
 * happened here — a whole panel's worth of commands were invoked and never
 * registered, and the panel simply did nothing.
 *
 * VibeCoder covers this with `src/__tests__/invokeHandlerParity.test.ts`.
 * VibeDesk has no test runner, so the same check ships as a script in the shape
 * this project already uses for `check-no-inline-edit.mjs`.
 *
 * Usage: node scripts/check-invoke-parity.mjs   (run from vibedesk/)
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve, relative } from "node:path";

const SRC = resolve(import.meta.dirname, "../src");
const TAURI_LIB = resolve(import.meta.dirname, "../src-tauri/src/lib.rs");

/** Strip `//` and block comments so prose cannot pose as a registered name. */
function stripComments(rust) {
  return rust.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");
}

/** Command names inside `generate_handler![...]`. */
function registeredCommands() {
  const rust = stripComments(readFileSync(TAURI_LIB, "utf8"));
  const start = rust.indexOf("generate_handler!");
  if (start === -1) throw new Error("generate_handler! not found in lib.rs");

  // Walk to the matching bracket. Regexing to the first `]` would stop at a
  // nested one and silently truncate the list — which reads as "unregistered".
  const open = rust.indexOf("[", start);
  let depth = 0;
  let end = open;
  for (let i = open; i < rust.length; i++) {
    if (rust[i] === "[") depth++;
    else if (rust[i] === "]" && --depth === 0) {
      end = i;
      break;
    }
  }

  const names = new Set();
  for (const entry of rust.slice(open + 1, end).split(",")) {
    const leaf = entry.trim().split("::").pop() ?? "";
    if (/^[a-z][a-z0-9_]*$/.test(leaf)) names.add(leaf);
  }
  return names;
}

function sourceFiles(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    if (entry === "node_modules") continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) sourceFiles(full, out);
    else if (/\.(ts|tsx)$/.test(entry) && !/\.test\./.test(entry)) out.push(full);
  }
  return out;
}

/** Literal `invoke("name")` calls, plus a count of the ones we cannot read. */
function collectInvocations() {
  const literal = [];
  let dynamic = 0;
  for (const file of sourceFiles(SRC)) {
    const text = readFileSync(file, "utf8");
    // Whole-text scan, not line by line: `invoke(` is regularly followed by a
    // newline, and a line-oriented regex misses every one of those.
    const re = /\binvoke\s*(?:<[^>]*>)?\s*\(\s*(["'`]?)([A-Za-z0-9_$.]*)\1/g;
    let m;
    while ((m = re.exec(text)) !== null) {
      if (m[1] === "") {
        dynamic++;
        continue;
      }
      if (/^[a-z][a-z0-9_]*$/.test(m[2])) {
        literal.push({ command: m[2], file: relative(SRC, file) });
      }
    }
  }
  return { literal, dynamic };
}

const registered = registeredCommands();
const { literal, dynamic } = collectInvocations();

// Floors, so a broken parser cannot pass vacuously. An empty `registered` set
// with an empty `literal` list would otherwise agree perfectly.
if (registered.size < 10) {
  console.error(`✖ parsed only ${registered.size} registered commands — the parser is broken`);
  process.exit(1);
}
if (literal.length < 10) {
  console.error(`✖ found only ${literal.length} invoke() calls — the scan is broken`);
  process.exit(1);
}

const missing = literal
  // Tauri's own plugin commands are namespaced — `invoke("plugin:dialog|open")`
  // — and the scan captures those as the bare word `plugin`. Exactly that word,
  // never a prefix match: `startsWith("plugin")` would also skip every app
  // command named `plugin_*`, which is precisely where this bug has lived.
  .filter((i) => i.command !== "plugin")
  .filter((i) => !registered.has(i.command));

if (missing.length > 0) {
  const seen = new Set();
  console.error(
    "✖ These commands are invoked but not in generate_handler!; clicking the\n" +
      "  control that calls them fails at runtime:\n"
  );
  for (const i of missing.sort((a, b) => a.command.localeCompare(b.command))) {
    const key = `${i.command}@${i.file}`;
    if (seen.has(key)) continue;
    seen.add(key);
    console.error(`    ${i.command}  (${i.file})`);
  }
  process.exit(1);
}

console.log(
  `✓ ${literal.length} invoke() call(s) all registered among ${registered.size} handlers.`
);
// Reported, not asserted: a blind spot named is a blind spot a reader can weigh.
if (dynamic > 0) {
  console.log(`  note: ${dynamic} dynamic invoke(<variable>) call(s) were not checkable.`);
}
