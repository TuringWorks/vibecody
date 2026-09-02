/**
 * BDD: the shared skill browser, driven through the shell that has shipped it
 * longest.
 *
 * The view now backs all three Tauri shells, and the thing they have to agree
 * on is not the layout — it is the sentence a picked skill turns into. If
 * VibeDesk seeds "Use the foo skill" and VibeCoder seeds "Load foo", the same
 * pick means two different prompts, which is the drift that made this shared
 * in the first place. So the scenarios assert on the text handed to `onUse`,
 * not on which button was clicked.
 *
 * The other claim worth pinning is the empty state: "the daemon reports no
 * skills" and "nothing matches your filter" are different facts, and a browser
 * that says the first when it means the second sends the user to look at a
 * daemon that is working.
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { SkillsView } from "@vibe/shared/skills/SkillsView";
import { skillPromptSeed, type SkillCatalog } from "@vibe/shared/skills/catalog";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const rows = [
  { name: "dora-metrics-program", category: "devex", summary: "The four keys.", source: "builtin" },
  { name: "test-first", category: "engineering", summary: "Pin behaviour first.", source: "builtin" },
  { name: "vendor-thing", category: "engineering", summary: "From a plugin.", source: "acme" },
];

/** A catalogue with a stable identity — the view refetches when it changes. */
function catalogOf(list: typeof rows): SkillCatalog {
  return {
    list: async () => list,
    get: async (name: string) => ({ name, body: `# ${name}\n\nBody.`, triggers: ["a trigger"] }),
  };
}

// ── Scenarios ────────────────────────────────────────────────────────────────

describe("skill catalogue browser", () => {
  it("groups the catalogue by category and marks non-builtin provenance", async () => {
    render(<SkillsView catalog={catalogOf(rows)} />);

    await screen.findByText("dora-metrics-program");
    expect(screen.getByText("devex")).toBeTruthy();
    expect(screen.getByText("engineering")).toBeTruthy();
    // A plugin's skill says whose it is; a builtin says nothing, because
    // "builtin" on every row is noise rather than provenance.
    expect(screen.getByText("acme")).toBeTruthy();
    expect(screen.queryByText("builtin")).toBeNull();
  });

  it("says which kind of empty it is", async () => {
    const { unmount } = render(<SkillsView catalog={catalogOf([])} />);
    await screen.findByText("The daemon reports no skills.");
    unmount();

    render(<SkillsView catalog={catalogOf(rows)} />);
    await screen.findByText("test-first");
    fireEvent.change(screen.getByPlaceholderText("Filter skills…"), {
      target: { value: "nothing matches this" },
    });
    expect(screen.getByText(/No skill matches/)).toBeTruthy();
    expect(screen.queryByText("The daemon reports no skills.")).toBeNull();
  });

  it("hands the composer one sentence naming every picked skill", async () => {
    const onUse = vi.fn();
    render(<SkillsView catalog={catalogOf(rows)} onUse={onUse} />);
    await screen.findByText("test-first");

    fireEvent.click(screen.getByLabelText("Select test-first"));
    fireEvent.click(screen.getByLabelText("Select dora-metrics-program"));
    expect(screen.getByText("2 selected")).toBeTruthy();

    fireEvent.click(screen.getByText("Use 2 skills"));

    const [text, names] = onUse.mock.calls[0];
    expect(names).toEqual(["test-first", "dora-metrics-program"]);
    expect(text).toBe(skillPromptSeed(["test-first", "dora-metrics-program"]));
    // Both names reach the prompt — a seed that silently dropped one would
    // look right in the tray and be wrong in the composer.
    expect(text).toContain("test-first");
    expect(text).toContain("dora-metrics-program");
  });

  it("clears the selection once it has been used, so the next pick starts empty", async () => {
    const onUse = vi.fn();
    render(<SkillsView catalog={catalogOf(rows)} onUse={onUse} />);
    await screen.findByText("test-first");

    fireEvent.click(screen.getByLabelText("Select test-first"));
    fireEvent.click(screen.getByText("Use skill"));

    await waitFor(() => expect(screen.queryByText("1 selected")).toBeNull());
  });

  it("offers the skill's own text as a separate action from the reference", async () => {
    const onUse = vi.fn();
    render(<SkillsView catalog={catalogOf(rows)} onUse={onUse} />);
    fireEvent.click(await screen.findByText("test-first"));

    // "Insert full text" only appears once the body has actually arrived —
    // an always-present button would paste an empty string mid-fetch.
    fireEvent.click(await screen.findByText("Insert full text"));
    expect(onUse.mock.calls[0][0]).toContain("# test-first");
  });

  it("renders no selection affordances when the host cannot take one", async () => {
    render(<SkillsView catalog={catalogOf(rows)} />);
    await screen.findByText("test-first");
    expect(screen.queryByLabelText("Select test-first")).toBeNull();
  });
});

describe("skillPromptSeed", () => {
  it("is empty for an empty pick, so a host cannot append a bare instruction", () => {
    expect(skillPromptSeed([])).toBe("");
  });

  it("agrees in number with what was picked", () => {
    expect(skillPromptSeed(["one"])).toContain("the skill `one`");
    const two = skillPromptSeed(["one", "two"]);
    expect(two).toContain("the skills");
    expect(two).toContain("`one` and `two`");
    expect(skillPromptSeed(["a", "b", "c"])).toContain("`a`, `b` and `c`");
  });
});
