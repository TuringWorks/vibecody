/**
 * Contract: every command the frontend invokes is registered in Tauri.
 *
 * `invoke("some_command")` joins the frontend to Rust through a bare string.
 * Nothing checks it. `tsc` sees a string literal; `rustc` sees a handler list
 * that nobody imports. A panel can call a command that was never registered
 * and the mistake surfaces only when a user clicks the button — which is how
 * it has surfaced here before.
 *
 * This test walks the join directly: collect the command names the frontend
 * actually calls, collect the ones `generate_handler!` actually registers, and
 * assert containment.
 *
 * Two parsing traps, both previously hit while auditing this by hand:
 *
 *   1. Comments inside the `generate_handler!` block read as command names if
 *      you take every identifier. Prose like "// list_plugins is handled by…"
 *      then makes an unregistered command look registered — the failure this
 *      test exists to catch, hidden by the test's own parser. Comments are
 *      stripped first, and names must look like commands.
 *   2. `invoke(...)` is frequently written across several lines, so a
 *      line-oriented regex silently misses those calls. This scans the whole
 *      file text.
 *
 * Dynamic calls — `invoke(cmd)` where `cmd` is a variable — are not checkable
 * from here and are counted and reported rather than silently skipped, so the
 * blind spot stays visible instead of reading as full coverage.
 */

import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

const SRC = resolve(__dirname, "..");
const TAURI_LIB = resolve(__dirname, "../../src-tauri/src/lib.rs");

/** Strip `//` line comments and `/* *​/` blocks so prose cannot pose as code. */
function stripComments(rust: string): string {
  return rust
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/\/\/[^\n]*/g, "");
}

function registeredCommands(): Set<string> {
  const rust = stripComments(readFileSync(TAURI_LIB, "utf8"));
  const start = rust.indexOf("generate_handler!");
  if (start === -1) throw new Error("generate_handler! not found in lib.rs");

  // Walk to the matching bracket rather than regexing to the first `]`, which
  // would stop at a nested one.
  const open = rust.indexOf("[", start);
  let depth = 0;
  let end = open;
  for (let i = open; i < rust.length; i++) {
    if (rust[i] === "[") depth++;
    else if (rust[i] === "]") {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }

  const body = rust.slice(open + 1, end);
  const names = new Set<string>();
  for (const entry of body.split(",")) {
    const trimmed = entry.trim();
    if (!trimmed) continue;
    // `commands::foo` / `crate::x::foo` → `foo`
    const leaf = trimmed.split("::").pop() ?? "";
    if (/^[a-z][a-z0-9_]*$/.test(leaf)) names.add(leaf);
  }
  return names;
}

function sourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    if (entry === "node_modules" || entry === "__tests__") continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) sourceFiles(full, out);
    else if (/\.(ts|tsx)$/.test(entry) && !/\.test\./.test(entry)) out.push(full);
  }
  return out;
}

interface Invocation {
  command: string;
  file: string;
}

function collectInvocations(): { literal: Invocation[]; dynamic: number } {
  const literal: Invocation[] = [];
  let dynamic = 0;

  for (const file of sourceFiles(SRC)) {
    const text = readFileSync(file, "utf8");
    // Whole-text scan: `invoke(` is regularly followed by a newline.
    const re = /\binvoke\s*(?:<[^>]*>)?\s*\(\s*(["'`]?)([A-Za-z0-9_$.]*)\1/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      const quoted = m[1] !== "";
      const name = m[2];
      if (!quoted) {
        dynamic++;
        continue;
      }
      if (/^[a-z][a-z0-9_]*$/.test(name)) {
        literal.push({ command: name, file: file.slice(SRC.length + 1) });
      }
    }
  }
  return { literal, dynamic };
}

describe("Given the frontend invokes Tauri commands", () => {
  it("Then generate_handler! parses to real command names, not comment prose", () => {
    const registered = registeredCommands();
    // A sanity floor: if the parser breaks and returns almost nothing, the
    // containment assertion below would pass vacuously for an empty frontend
    // and fail confusingly otherwise. Assert the parse itself looks sane.
    expect(registered.size).toBeGreaterThan(50);
    for (const name of registered) {
      expect(name).toMatch(/^[a-z][a-z0-9_]*$/);
    }
  });

  it("Then every invoked command is registered", () => {
    const registered = registeredCommands();
    const { literal, dynamic } = collectInvocations();

    expect(literal.length).toBeGreaterThan(50); // the scan actually found calls

    const missing = literal
      .filter(i => !registered.has(i.command))
      // Tauri's own plugin commands are namespaced and never appear in the
      // app's handler list.
      .filter(i => !i.command.startsWith("plugin"));

    const detail = missing
      .map(i => `  ${i.command}  (${i.file})`)
      .sort()
      .join("\n");

    expect(
      missing,
      missing.length
        ? `These commands are invoked but not in generate_handler!; ` +
            `clicking the control that calls them fails at runtime:\n${detail}`
        : ""
    ).toEqual([]);

    // Not an assertion — a visible record of what this test cannot see.
    if (dynamic > 0) {
      console.info(
        `invokeHandlerParity: ${dynamic} dynamic invoke(...) call(s) were not checkable.`
      );
    }
  });
});
