/**
 * Source-scan regression test — the shared voice controls use real tokens.
 *
 * `packages/vibe-ui-shared/src/voice/voice.css` styles the mic and full-duplex
 * buttons for all three desktop apps. It shipped using `--border`, `--surface`,
 * `--fg`, `--muted` and `--accent-red`, none of which this design system
 * defines, each with a GitHub-dark hex fallback — plus five bare hex literals
 * for the duplex state dots.
 *
 * `var(--x, #hex)` naming a token that is never defined is not a fallback. The
 * hex wins on every theme, forever. So the buttons rendered in one fixed
 * palette inside apps themed otherwise, and no amount of theme switching
 * touched them.
 *
 * `DefinedTokensOnly` does not cover this: it scans `vibecoder/src` and the
 * design system, not `packages/`, and it deliberately permits fallback uses on
 * the grounds that the token is then optional. That reasoning holds only while
 * the token exists somewhere — which is the gap this file closes, for the one
 * stylesheet that fell into it.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const SRC = resolve(__dirname, "..", "..");
const VOICE_CSS = resolve(SRC, "..", "..", "packages", "vibe-ui-shared", "src", "voice", "voice.css");
const TOKENS = resolve(SRC, "..", "design-system", "tokens.css");

/**
 * Strip comments before scanning.
 *
 * The header of `voice.css` documents the bug by quoting `var(--foo, #hex)`
 * and naming the dead tokens. Scanning raw text reports the explanation as a
 * violation — the same trap the Monaco guard hit with a `<Editor>` mention.
 */
const code = (text: string) => text.replace(/\/\*[\s\S]*?\*\//g, "");

const voiceCss = () => code(readFileSync(VOICE_CSS, "utf8"));

describe("shared voice control theming", () => {
  it("names only tokens the design system defines", () => {
    const tokens = readFileSync(TOKENS, "utf8");
    const used = [...voiceCss().matchAll(/var\((--[a-z0-9-]+)/g)].map((m) => m[1]);
    expect(used.length, "no var() uses found — the scan is not reading the file")
      .toBeGreaterThan(5);

    const undefinedTokens = [...new Set(used)].filter(
      (t) => !new RegExp(`^\\s*${t}\\s*:`, "m").test(tokens),
    );
    expect(
      undefinedTokens,
      `voice.css names tokens this design system never defines, so any fallback `
        + `beside them is permanent:\n${undefinedTokens.map((t) => `  - ${t}`).join("\n")}`,
    ).toEqual([]);
  });

  it("carries no literal colours", () => {
    const literals = [
      ...voiceCss().matchAll(/#[0-9a-fA-F]{3,8}\b|\brgba?\([^)]*\)/g),
    ].map((m) => m[0]);
    expect(
      literals,
      `A literal colour cannot follow the theme. Use a semantic token:\n`
        + literals.map((l) => `  - ${l}`).join("\n"),
    ).toEqual([]);
  });

  /**
   * A fallback is how the original defect hid: it looked like defensive coding
   * while guaranteeing the token was never consulted. In a file whose tokens
   * are all verified to exist, a fallback has nothing left to do.
   */
  it("uses no var() fallbacks", () => {
    const withFallback = [...voiceCss().matchAll(/var\(--[a-z0-9-]+\s*,[^)]*\)/g)]
      .map((m) => m[0]);
    expect(withFallback, `Fallbacks mask a missing token rather than surfacing it`)
      .toEqual([]);
  });

  /**
   * A perfectly themed stylesheet nobody loads is the same as no stylesheet.
   *
   * VibeCoder rendered every one of these controls as bare markup — an
   * unstyled button beside the styled toolbar, a status dot with no colour —
   * for the whole life of the feature, because it was the one shell that never
   * imported the file. VibeDesk imports it in `main.tsx`, VibeAIChat in
   * `App.tsx`; there is nothing to notice unless something checks.
   */
  it("is imported by every shell that renders the controls", () => {
    const ROOT = resolve(SRC, "..", "..");
    const importers = [
      ["VibeCoder", resolve(SRC, "components", "AIChat.tsx")],
      ["VibeDesk", resolve(ROOT, "vibedesk", "src", "main.tsx")],
      ["VibeAIChat", resolve(ROOT, "vibeaichat", "src", "App.tsx")],
    ] as const;
    const missing = importers
      .filter(([, file]) => !readFileSync(file, "utf8").includes("voice/voice.css"))
      .map(([app]) => app);
    expect(
      missing,
      `These shells render the voice controls without loading their stylesheet:\n`
        + missing.map((a) => `  - ${a}`).join("\n"),
    ).toEqual([]);
  });
});
