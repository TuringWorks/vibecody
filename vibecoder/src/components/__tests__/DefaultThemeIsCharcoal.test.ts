/**
 * The default theme is Charcoal — in the registry, in every fallback, and in
 * the stylesheet that paints before any of them run.
 *
 * Three separate things had to agree and didn't: `ThemeToggle` booted
 * `dark-sherwood`, `useEditorTheme` fell back to `dark-default`, and
 * `tokens.css` shipped the midnight-blue palette as `:root`. A user with no
 * stored choice therefore got one palette for the app, another for the code
 * pane, and a flash of a third before React mounted. Naming the default once
 * fixed the first two; this file keeps the third in step, because a CSS file
 * and a TS object cannot import each other.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { THEMES, DEFAULT_DARK_THEME_ID, DEFAULT_LIGHT_THEME_ID } from "../../theme/themes";

const TOKENS = readFileSync(
  resolve(__dirname, "..", "..", "..", "design-system", "tokens.css"),
  "utf8"
);

/** The declarations of one top-level block, as a token → value map. */
function block(selector: string): Record<string, string> {
  const start = TOKENS.indexOf(`${selector} {`);
  expect(start, `no ${selector} block — the scan is not reading tokens.css`).toBeGreaterThan(-1);
  const body = TOKENS.slice(start, TOKENS.indexOf("\n}", start)).replace(/\/\*[\s\S]*?\*\//g, "");
  return Object.fromEntries(
    [...body.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)].map((m) => [
      m[1],
      m[2].trim().replace(/\s+/g, " "),
    ])
  );
}

const theme = (id: string) => {
  const t = THEMES.find((x) => x.id === id);
  expect(t, `${id} is not in the registry`).toBeTruthy();
  return t!;
};

describe("the default theme", () => {
  it("is Charcoal, and both halves of the pair exist", () => {
    expect(DEFAULT_DARK_THEME_ID).toBe("dark-charcoal");
    expect(DEFAULT_LIGHT_THEME_ID).toBe("light-charcoal");
    expect(theme(DEFAULT_DARK_THEME_ID).mode).toBe("dark");
    expect(theme(DEFAULT_LIGHT_THEME_ID).mode).toBe("light");
    expect(theme(DEFAULT_DARK_THEME_ID).pairId).toBe(theme(DEFAULT_LIGHT_THEME_ID).pairId);
  });

  it("is what :root paints before any JS runs", () => {
    const root = block(":root");
    for (const [key, value] of Object.entries(theme(DEFAULT_DARK_THEME_ID).vars)) {
      expect(root[key], `${key} in tokens.css :root differs from dark-charcoal`).toBe(
        value.replace(/\s+/g, " ")
      );
    }
  });

  it("is what [data-theme=light] paints, for the light half", () => {
    const light = block('[data-theme="light"]');
    for (const [key, value] of Object.entries(theme(DEFAULT_LIGHT_THEME_ID).vars)) {
      expect(light[key], `${key} in the light block differs from light-charcoal`).toBe(
        value.replace(/\s+/g, " ")
      );
    }
  });

  it("is the same default the other shells use", () => {
    const shared = readFileSync(
      resolve(__dirname, "..", "..", "..", "..", "packages", "vibe-ui-shared", "src", "theme", "themes.ts"),
      "utf8"
    );
    expect(shared).toMatch(
      new RegExp(`DEFAULT_DARK_THEME_ID\\s*=\\s*"${DEFAULT_DARK_THEME_ID}"`)
    );
    expect(shared).toMatch(
      new RegExp(`DEFAULT_LIGHT_THEME_ID\\s*=\\s*"${DEFAULT_LIGHT_THEME_ID}"`)
    );
  });
});
