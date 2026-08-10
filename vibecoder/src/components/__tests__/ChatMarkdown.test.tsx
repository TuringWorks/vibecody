/**
 * Chat replies must render as markdown, not as raw text.
 *
 * Assistant output is markdown — headings, lists, emphasis, and above all
 * tables. VibeCoder's chat used to drop all of it into a `<pre>`, so a reply
 * containing a table arrived as a wall of `| --- |` rows and every emphasis as
 * literal asterisks, while VibeDesk and VibeAIChat rendered the same reply
 * properly through the shared component.
 *
 * Two things could silently break this again, neither of which a type-check
 * catches:
 *
 *  1. **`remark-gfm` going missing.** It was absent from VibeCoder's
 *     dependencies entirely, so tables could not have rendered even once the
 *     renderer was wired up. Core markdown still works without it — only the
 *     GFM extensions (tables, task lists, strikethrough) quietly stop.
 *  2. **The alias table losing an entry.** `packages/vibe-ui-shared` has no
 *     `node_modules` of its own; its bare imports resolve through the alias
 *     list in `vite.config.ts`, which `vitest.config.ts` deliberately reuses.
 *     A missing alias fails at bundle time, not at type-check time.
 *
 * This renders through the same alias table the dev server uses, so it
 * exercises the wiring rather than just the component.
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Markdown } from "@vibe/shared/markdown/Markdown";

describe("chat markdown rendering", () => {
  it("renders a GFM table as a real table, not raw pipes", () => {
    const reply = [
      "| # | Story | Acceptance |",
      "|---|-------|------------|",
      "| NF-1 | **Client-Side Validation Only** | Form submit aborts early. |",
      "| NF-2 | **Rate-Limiting UI Feedback** | Throttling message shown. |",
    ].join("\n");

    const { container } = render(<Markdown text={reply} />);

    const table = container.querySelector("table");
    expect(table, "a markdown table must render as <table>").not.toBeNull();

    // Header cells come through as <th>, body rows as <td> — the structure is
    // the point, not just that the characters survived.
    const headers = [...container.querySelectorAll("th")].map((h) => h.textContent);
    expect(headers).toEqual(["#", "Story", "Acceptance"]);
    expect(container.querySelectorAll("tbody tr")).toHaveLength(2);

    // And the raw delimiter row must be gone from the output.
    expect(container.textContent).not.toContain("|---|");
    expect(container.textContent).not.toContain("| NF-1 |");
  });

  it("renders emphasis as markup rather than literal asterisks", () => {
    const { container } = render(<Markdown text="**Client-Side Validation Only** matters" />);
    expect(container.querySelector("strong")?.textContent).toBe(
      "Client-Side Validation Only",
    );
    expect(container.textContent).not.toContain("**");
  });

  it("renders headings and lists", () => {
    const { container } = render(<Markdown text={"## Acceptance\n\n- one\n- two\n"} />);
    expect(container.querySelector("h2")?.textContent).toBe("Acceptance");
    expect(container.querySelectorAll("li")).toHaveLength(2);
  });

  it("keeps inline code readable — how models usually write file paths", () => {
    render(<Markdown text="See `src/components/AuthForm.test.tsx` for coverage." />);
    expect(screen.getByText("src/components/AuthForm.test.tsx").tagName).toBe("CODE");
  });

  /** Fenced blocks are extracted by `renderContent` before reaching Markdown,
   *  but an indented or stray fence must still not leak backticks as text. */
  it("renders a fenced block as a code element, not backticks", () => {
    const { container } = render(<Markdown text={"```ts\nconst a = 1;\n```"} />);
    expect(container.querySelector("pre code")).not.toBeNull();
    expect(container.textContent).not.toContain("```");
  });
});
