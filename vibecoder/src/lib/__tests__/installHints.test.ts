/**
 * Validates the install-hint parser against the **real** hint table, read out
 * of `manager.rs`.
 *
 * Sample-based tests only prove the parser handles the samples. The hints are
 * free text written per language, and the failure mode is quiet: a truncated
 * command like `cargo install --git` looks plausible on a button and does
 * nothing when pasted. Checking all 79 catches the next hint that is phrased
 * unusually, at the time it is added rather than when a user tries it.
 */

import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseInstallHint } from "../lsp";

const MANAGER_RS = join(
  __dirname,
  "..",
  "..",
  "..",
  "crates",
  "vibe-lsp",
  "src",
  "manager.rs",
);

/** `h.insert("lang", "hint")` pairs from the Rust install-hint table. */
function installHints(): Map<string, string> {
  const source = readFileSync(MANAGER_RS, "utf8").split("#[cfg(test)]")[0];
  const hints = new Map<string, string>();
  for (const match of source.matchAll(
    /h\.insert\(\s*"([^"]+)",\s*"((?:[^"\\]|\\.)*)"/gs,
  )) {
    hints.set(match[1], match[2].replace(/\\"/g, '"'));
  }
  return hints;
}

describe("install hints", () => {
  const hints = installHints();

  it("finds the whole table", () => {
    // Guards the parsing: an empty map would make every assertion vacuous.
    expect(hints.size).toBeGreaterThan(60);
    expect(hints.get("rust")).toContain("rustup");
  });

  it("never yields a command that cannot be run as-is", () => {
    const broken: string[] = [];
    for (const [language, hint] of hints) {
      const { command } = parseInstallHint(hint);
      if (command === undefined) continue;
      const lastToken = command.split(/\s+/).at(-1) ?? "";
      if (lastToken.startsWith("-")) {
        broken.push(`${language}: ends on a flag → ${command}`);
      }
      if (command.includes("|")) {
        broken.push(`${language}: two commands in one → ${command}`);
      }
      if (command.includes("(") !== command.includes(")")) {
        broken.push(`${language}: unbalanced parenthesis → ${command}`);
      }
    }
    expect(broken).toEqual([]);
  });

  it("offers a command or a link for the great majority of languages", () => {
    let actionable = 0;
    for (const hint of hints.values()) {
      const action = parseInstallHint(hint);
      if (action.command !== undefined || action.url !== undefined) actionable++;
    }
    // The remainder are genuinely prose — "Included with Xcode", "Included with
    // the Gleam toolchain" — where there is no command to give.
    expect(actionable / hints.size).toBeGreaterThan(0.9);
  });

  it("extracts the command for the servers most people will need", () => {
    const expected: Record<string, string> = {
      rust: "rustup component add rust-analyzer",
      typescript: "npm i -g typescript-language-server typescript",
      go: "go install golang.org/x/tools/gopls@latest",
      asm: "cargo install asm-lsp",
      cmake: "pip install cmake-language-server",
      svelte: "npm i -g svelte-language-server",
      vue: "npm i -g @vue/language-server",
      nix: "nix profile install nixpkgs#nil",
    };
    for (const [language, command] of Object.entries(expected)) {
      const hint = hints.get(language);
      expect(hint, `${language} has no hint`).toBeDefined();
      expect(parseInstallHint(hint as string).command, language).toBe(command);
    }
  });

  it("picks the first platform when a hint lists several", () => {
    // "brew install llvm (macOS) | apt install clangd (Linux)"
    expect(parseInstallHint(hints.get("c") as string).command).toBe(
      "brew install llvm",
    );
  });
});
