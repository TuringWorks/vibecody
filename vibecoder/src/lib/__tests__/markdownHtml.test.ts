/**
 * Generated documents arrive with raw HTML in them — `<details>` around an
 * answer, `<br>` between lines, `<b>` where `**` was meant. None of it renders
 * (there is no rehype-raw in the pipeline), so before this transform the reader
 * saw the tags themselves. What must not break: a document *about* HTML, where
 * the tags are the subject and live inside code.
 */
import { describe, expect, it } from "vitest";
import { htmlToMarkdown } from "../markdownHtml";

describe("htmlToMarkdown", () => {
  it("collapses a details/summary answer to its summary line", () => {
    const out = htmlToMarkdown(
      "<details><summary><b>Answer</b></summary>\n\nThree components.\n\n</details>",
    );
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
