/**
 * Source-scan regression test — `.panel-btn` must be self-sufficient.
 *
 * A `<button>` with no `background` / `color` does not look unstyled; it looks
 * like a *native OS button* — `buttonface` light grey with `buttontext` black.
 * On a dark theme that reads as "this page doesn't match the app", and nothing
 * catches it: no lint rule, no type error, no failing render test.
 *
 * `.panel-btn` shipped without either property, so every
 * `<button className="panel-btn">` that carried no modifier class and no
 * inline background rendered as native chrome — 43 of them across 15 files,
 * the visible one being Configuration → Keys (Reveal / Edit / Delete / Add).
 *
 * The fix is that the *base* class is themed, so a bare `.panel-btn` is always
 * a valid button and modifiers are opt-in emphasis rather than a requirement.
 * These tests pin that, plus the box-metric consistency that came with it.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(__dirname, "..", "..", "App.css"), "utf8");

/**
 * Body of the *last* `.panel-btn { … }` rule that declares `background`.
 * App.css defines `.panel-btn` more than once (the second only adds
 * `min-height`), so picking the first match blindly would test the wrong rule.
 */
function baseRule(): string {
  const bodies = [...css.matchAll(/(^|\})\s*\.panel-btn\s*\{([^}]*)\}/g)].map((m) => m[2]);
  expect(bodies.length, "expected at least one `.panel-btn` rule in App.css").toBeGreaterThan(0);
  return bodies.filter((b) => /background\s*:/.test(b)).at(-1) ?? "";
}

function ruleBody(selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const m = css.match(new RegExp(`(^|\\})\\s*${escaped}\\s*\\{([^}]*)\\}`));
  return m?.[2] ?? "";
}

describe(".panel-btn theming contract", () => {
  it("declares its own background — a bare .panel-btn must never fall through to buttonface", () => {
    expect(baseRule(), "add `background: var(--…)` to `.panel-btn` in App.css").toMatch(
      /background\s*:\s*var\(--/,
    );
  });

  it("declares its own text colour — otherwise it inherits buttontext (black)", () => {
    expect(baseRule(), "add `color: var(--…)` to `.panel-btn` in App.css").toMatch(
      /(^|[;{\s])color\s*:\s*var\(--/,
    );
  });

  it("gives every variant the same border box so buttons in a row line up", () => {
    // Base carries the 1px border; `-secondary` only recolours it. If the base
    // reverted to `border: none`, `-secondary` would be 2px taller than
    // `-primary` sitting beside it — the state this replaced.
    expect(baseRule()).toMatch(/border\s*:\s*1px\s+solid/);
    expect(ruleBody(".panel-btn-secondary")).toMatch(/border-color\s*:\s*var\(--/);
    expect(
      ruleBody(".panel-btn-secondary"),
      "`-secondary` should only recolour the base border, not redeclare the box",
    ).not.toMatch(/border\s*:\s*1px/);
  });

  it("keeps the emphasis modifiers themed", () => {
    for (const variant of [".panel-btn-primary", ".panel-btn-danger", ".panel-btn-secondary"]) {
      const body = ruleBody(variant);
      expect(body, `${variant} is missing from App.css`).not.toBe("");
      expect(body, `${variant} must set a token background`).toMatch(/background\s*:\s*var\(--/);
      expect(body, `${variant} must set a token colour`).toMatch(/color\s*:\s*var\(--/);
    }
  });
});
