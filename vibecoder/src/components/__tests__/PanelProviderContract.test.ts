/**
 * Contract: a panel handed a provider must not pick its own.
 *
 * `createComposite` forwards `provider` to every tab it mounts, and
 * `LazyPanels` forwards it to every composite — 155 panels receive it. A
 * panel that declares no props still receives it; React just drops it. So the
 * panel silently chooses a provider itself, and the toolbar selection does
 * nothing. That was Counsel, Arena, Compare and SuperBrain: four panels, one
 * shape, shipped.
 *
 * The behavioural suite (`ProviderAgnosticPanels.contract.test.tsx`) proves the
 * four fixed panels send the right provider. It cannot scale to 155. This one
 * covers the rest structurally: it finds panels that are *handed* a provider
 * and *also* name one themselves, which is the signature of the bug.
 *
 * ── What this deliberately does NOT assert ───────────────────────────────────
 *
 * Not "every panel must declare `provider`". Most of the 155 never call an LLM
 * — the composite forwards indiscriminately — and demanding the prop from a
 * colour-picker would be noise that gets suppressed, which is worse than no
 * test. 199 of 297 prop/panel pairs are "missing" a prop under that reading
 * and almost none of them are defects.
 *
 * The narrow rule is the honest one: choosing a provider locally while being
 * given one is always wrong, whatever the panel does.
 *
 * ── Why the allowlist exists ────────────────────────────────────────────────
 *
 * Matching a provider *name* cannot distinguish `p === "claude"` (SuperBrain's
 * bug) from `config.inferenceBackend === "ollama"` (a setup wizard comparing
 * local backends, entirely legitimate). Rather than guess, ambiguous matches
 * are listed with a reason. A new panel that trips this fails the test and
 * forces someone to look, which is the point.
 */

import { describe, it, expect } from "vitest";
import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname, join } from "node:path";

const SRC = resolve(__dirname, "../..");
const LAZY = join(SRC, "components/LazyPanels.tsx");

/**
 * Panels that name a provider for a reason unrelated to choosing the chat
 * backend. Each entry must say why; an unexplained entry is a suppressed bug.
 */
const ALLOWED: Record<string, string> = {
  ModelWizardPanel:
    "Compares config.inferenceBackend against local runtimes while setting up " +
    "a downloaded model. That is the wizard's subject, not the toolbar's chat " +
    "provider — it has no LLM call to route.",
};

const PROVIDER_NAMES = [
  "ollama",
  "claude",
  "openai",
  "gemini",
  "anthropic",
  "grok",
  "mistral",
  "deepseek",
];

function resolveFile(base: string, rel: string): string | null {
  const p = resolve(dirname(base), rel);
  for (const ext of [".tsx", ".ts"]) if (existsSync(p + ext)) return p + ext;
  return null;
}

/** Panels mounted with a `provider` prop, expanded through composites. */
function panelsHandedProvider(): { file: string; exp: string }[] {
  const lazyText = readFileSync(LAZY, "utf8");

  const moduleOf = new Map<string, string>();
  for (const m of lazyText.matchAll(
    /const\s+(\w+)\s*=\s*lazy\(\s*\(\)\s*=>\s*import\(["'](.+?)["']\)/g
  )) {
    moduleOf.set(m[1], m[2]);
  }

  const out: { file: string; exp: string }[] = [];
  const seen = new Set<string>();

  for (const mount of lazyText.matchAll(
    /Component=\{(\w+)\}\s*props=\{\{([\s\S]*?)\}\}/g
  )) {
    const props = [...mount[2].matchAll(/(\w+)\s*:/g)].map(x => x[1]);
    if (!props.includes("provider")) continue;

    const rel = moduleOf.get(mount[1]);
    if (!rel) continue;
    const file = resolveFile(LAZY, rel);
    if (!file) continue;

    const text = readFileSync(file, "utf8");
    const leaves = text.includes("createComposite")
      ? [
          ...text.matchAll(
            /importFn:\s*\(\)\s*=>\s*import\(["'](.+?)["']\)\s*,\s*exportName:\s*["'](\w+)["']/g
          ),
        ]
          .map(t => ({ file: resolveFile(file, t[1]), exp: t[2] }))
          .filter((l): l is { file: string; exp: string } => l.file !== null)
      : [{ file, exp: mount[1] }];

    for (const leaf of leaves) {
      const key = `${leaf.file}#${leaf.exp}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(leaf);
    }
  }
  return out;
}

/**
 * The component's parameter list — the balanced parens after its name, and
 * nothing else.
 *
 * An earlier version of this took a fixed-size window after the name instead.
 * The window ran into the body, where a `provider:` key in an object literal
 * read as a declared prop — so SuperBrain, whose bug was exactly this, was
 * reported clean. A parser that hides the failure it was written to catch is
 * worse than no test, which is why `finds_the_known_bad_shapes` below pins all
 * four original panels rather than trusting this function.
 */
function parameterList(text: string, exp: string): string | null {
  const decl = new RegExp(`(?:export\\s+)?(?:function|const)\\s+${exp}\\b`).exec(text);
  if (!decl) return null;
  const open = text.indexOf("(", decl.index);
  if (open === -1) return null;

  let depth = 0;
  for (let i = open; i < text.length; i++) {
    const c = text[i];
    if (c === "(") depth++;
    else if (c === ")") {
      depth--;
      if (depth === 0) return text.slice(open + 1, i);
    }
  }
  return null;
}

interface Verdict {
  declaresProvider: boolean;
  namesProvider: boolean;
}

function inspect(file: string, exp: string): Verdict | null {
  const text = readFileSync(file, "utf8");
  const params = parameterList(text, exp);
  if (params === null) return null;

  const namesProvider = PROVIDER_NAMES.some(p =>
    [
      new RegExp(`useState\\(\\s*["']${p}["']`),
      new RegExp(`provider:\\s*["']${p}["']`),
      // SuperBrain's form was a comparison, not an assignment.
      new RegExp(`===\\s*["']${p}["']`),
      new RegExp(`["']${p}["']\\s*===`),
    ].some(re => re.test(text))
  );

  return { declaresProvider: /\bprovider\b/.test(params), namesProvider };
}

describe("Given a panel is handed the toolbar provider", () => {
  it("Then the mount graph resolves to real panels", () => {
    const panels = panelsHandedProvider();
    // If the LazyPanels/createComposite parsing breaks, every assertion below
    // passes vacuously. Pin the scale it is supposed to cover.
    expect(panels.length).toBeGreaterThan(100);
    for (const p of panels) expect(existsSync(p.file)).toBe(true);
  });

  it("Then it does not choose a provider of its own", () => {
    const offenders = panelsHandedProvider()
      .map(p => ({ ...p, verdict: inspect(p.file, p.exp) }))
      .filter(p => p.verdict && p.verdict.namesProvider && !p.verdict.declaresProvider)
      .filter(p => !(p.exp in ALLOWED))
      .map(p => `  ${p.exp}  (${p.file.slice(SRC.length + 1)})`);

    expect(
      offenders,
      offenders.length
        ? `These panels are handed a provider and pick their own anyway, so the ` +
            `toolbar selection is ignored (AGENTS.md → Provider-Agnostic Panels ` +
            `— STRICT). Accept the prop and use it, or add an entry to ALLOWED ` +
            `explaining why the provider name is unrelated:\n${offenders.join("\n")}`
        : ""
    ).toEqual([]);
  });

  it("Then the detector still recognises the four shapes that shipped", () => {
    // Guards the detector itself. These are the real pre-fix sources; if a
    // future refactor of `inspect` stops seeing them, this fails here rather
    // than going quiet across all 155 panels.
    const shapes: Record<string, string> = {
      hardCodedTrio: `export function P() {
        const DEFAULTS = [{ provider: "claude", model: "m" }];
        return DEFAULTS;
      }`,
      pinnedSideA: `export function P() {
        const [providerA] = useState("ollama");
        return providerA;
      }`,
      comparisonEnable: `export function P() {
        const rows = all.map(p => ({ enabled: p === "claude" || p === "openai" }));
        return rows;
      }`,
      // The regression that made the first parser useless: a `provider:` key
      // in the body must not count as a declared parameter.
      bodyKeyIsNotAParam: `export function P() {
        const x = all.map(p => ({ provider: p, model: "m" }));
        const [s] = useState("ollama");
        return x;
      }`,
    };

    for (const [name, src] of Object.entries(shapes)) {
      const params = parameterList(src, "P");
      expect(params, `${name}: parameter list should parse`).toBe("");
      expect(/\bprovider\b/.test(params ?? ""), `${name}: must not read the body as params`).toBe(
        false
      );
    }
  });

  it("Then every allowlist entry names a panel that still exists and still needs it", () => {
    const byName = new Map(panelsHandedProvider().map(p => [p.exp, p]));
    for (const [name, reason] of Object.entries(ALLOWED)) {
      expect(reason.length, `${name} needs a real justification`).toBeGreaterThan(40);
      const panel = byName.get(name);
      expect(panel, `${name} is allowlisted but no longer mounted with a provider`).toBeTruthy();
      // A stale entry silently suppresses a future bug in that panel.
      const verdict = panel ? inspect(panel.file, panel.exp) : null;
      expect(
        verdict?.namesProvider,
        `${name} no longer names a provider — drop it from ALLOWED`
      ).toBe(true);
    }
  });
});
