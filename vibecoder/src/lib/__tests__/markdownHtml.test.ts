/**
 * Generated documents arrive with raw HTML in them — `<details>` around an
 * answer, `<br>` between lines, `<b>` where `**` was meant. None of it renders
 * (there is no rehype-raw in the pipeline), so before this transform the reader
 * saw the tags themselves. What must not break: a document *about* HTML, where
 * the tags are the subject and live inside code.
 */
import { describe, expect, it } from "vitest";
import { htmlToMarkdown, splitDetails } from "../markdownHtml";

describe("htmlToMarkdown", () => {
  it("strips an unbalanced details tag rather than leaving it on screen", () => {
    // A balanced block is lifted out by splitDetails long before this runs;
    // what reaches here is the opening tag whose partner never arrived.
    const out = htmlToMarkdown("<details><summary><b>Answer</b></summary>\n\nThree components.");
    expect(out).not.toContain("<details>");
    expect(out).not.toContain("<summary>");
    expect(out).toContain("**Answer**");
    expect(out).toContain("Three components.");
  });

  it("rewrites inline emphasis tags into markdown", () => {
    expect(htmlToMarkdown("a <b>bold</b> and <i>slanted</i> and <code>x</code>")).toBe(
      "a **bold** and *slanted* and `x`",
    );
  });

  it("turns <br> into a hard line break", () => {
    expect(htmlToMarkdown("one<br>two")).toBe("one  \ntwo");
  });

  it("rewrites anchors and images into markdown links", () => {
    expect(htmlToMarkdown('see <a href="https://x.dev/a">the docs</a>')).toBe(
      "see [the docs](https://x.dev/a)",
    );
    expect(htmlToMarkdown('<img src="d.png" alt="a diagram">')).toBe("![a diagram](d.png)");
  });

  it("drops tags with no markdown equivalent but keeps their text", () => {
    expect(htmlToMarkdown('<div class="note"><span>kept</span></div>')).toBe("kept");
  });

  it("leaves fenced code exactly as written", () => {
    const src = ["```html", "<details><summary>Answer</summary>", "<b>x</b>", "```"].join("\n");
    expect(htmlToMarkdown(src)).toBe(src);
  });

  it("leaves a longer fence, and its inner fence, alone", () => {
    const src = ["````md", "```html", "<div>", "```", "````", "<div>after</div>"].join("\n");
    expect(htmlToMarkdown(src)).toBe(
      ["````md", "```html", "<div>", "```", "````", "after"].join("\n"),
    );
  });

  it("leaves inline code spans alone", () => {
    expect(htmlToMarkdown("wrap it in `<div>` — not <div>this</div>")).toBe(
      "wrap it in `<div>` — not this",
    );
  });

  it("leaves non-HTML angle brackets alone", () => {
    expect(htmlToMarkdown("Vec<String> and <T> and <https://example.com>")).toBe(
      "Vec<String> and <T> and <https://example.com>",
    );
  });

  it("strips HTML comments", () => {
    expect(htmlToMarkdown("before <!-- hidden --> after")).toBe("before  after");
  });

  it("turns an HTML list into a markdown list", () => {
    expect(htmlToMarkdown("<ul><li>one</li><li>two</li></ul>")).toBe("\n- one\n- two");
  });

  it("is a no-op on plain markdown", () => {
    const src = "# Title\n\nSome **bold** text and a [link](x.md).\n";
    expect(htmlToMarkdown(src)).toBe(src);
  });
});

describe("splitDetails", () => {
  const details = (segments: ReturnType<typeof splitDetails>) =>
    segments.filter((s) => s.kind === "details");

  it("lifts a details block out with its summary and body", () => {
    const segments = splitDetails(
      "Question?\n\n<details><summary><b>Answer</b></summary>\n\nThree components.\n\n</details>\n",
    );
    expect(segments[0]).toEqual({ kind: "markdown", text: "Question?\n\n" });
    expect(segments[1]).toMatchObject({
      kind: "details",
      summary: "<b>Answer</b>",
      open: false,
    });
    expect(segments[1].kind === "details" && segments[1].body.trim()).toBe("Three components.");
  });

  it("keeps the text on both sides of the block", () => {
    const segments = splitDetails("before\n<details>\nhidden\n</details>\nafter");
    expect(segments.map((s) => s.kind)).toEqual(["markdown", "details", "markdown"]);
    expect(segments[2]).toMatchObject({ kind: "markdown", text: "\nafter" });
  });

  it("nests: the body still contains the inner block", () => {
    const segments = splitDetails(
      "<details><summary>Outer</summary>\n<details><summary>Inner</summary>\nx\n</details>\n</details>",
    );
    expect(details(segments)).toHaveLength(1);
    const inner = splitDetails(segments[0].kind === "details" ? segments[0].body : "");
    expect(details(inner)[0]).toMatchObject({ summary: "Inner" });
  });

  it("honours the open attribute", () => {
    const segments = splitDetails("<details open><summary>Shown</summary>\nx\n</details>");
    expect(details(segments)[0]).toMatchObject({ open: true });
  });

  it("ignores a details block written inside fenced code", () => {
    const src = ["```html", "<details><summary>x</summary>", "</details>", "```"].join("\n");
    expect(splitDetails(src)).toEqual([{ kind: "markdown", text: src }]);
  });

  it("ignores a details tag inside an inline code span", () => {
    const src = "use `<details>` for this";
    expect(splitDetails(src)).toEqual([{ kind: "markdown", text: src }]);
  });

  it("leaves an unclosed details block as markdown, so no text is swallowed", () => {
    const src = "<details><summary>Answer</summary>\n\nthe rest of the document";
    expect(splitDetails(src)).toEqual([{ kind: "markdown", text: src }]);
  });

  it("returns one markdown segment for a document with no details", () => {
    expect(splitDetails("# Title\n\ntext")).toEqual([{ kind: "markdown", text: "# Title\n\ntext" }]);
  });
});
