/**
 * "Export to HTML" wrote the markdown *source* into a `<body>` — the exported
 * file showed `# Title` as text and every list as hyphens, while a comment in
 * the file claimed it snapshotted the rendered pane. Nothing type-checked that
 * claim, and nothing rendered the export, so it stayed wrong.
 */
import { describe, expect, it } from "vitest";
import { renderDocumentHtml } from "../markdownDocument";

describe("renderDocumentHtml", () => {
  it("exports rendered HTML, not the markdown source", () => {
    const html = renderDocumentHtml("# Title\n\n- one\n- two\n", "notes");

    expect(html).toContain("<h1");
    expect(html).toContain(">Title</h1>");
    expect(html).toContain("<li");
    expect(html).not.toContain("# Title");
    expect(html).not.toContain("- one");
  });

  it("carries the disclosures into the exported file", () => {
    const html = renderDocumentHtml(
      "<details><summary>Answer</summary>\n\nhidden\n\n</details>",
      "quiz",
    );

    expect(html).toContain("<details");
    expect(html).toContain("md-details__summary");
    expect(html).toContain("hidden");
    expect(html).not.toContain("&lt;details&gt;");
  });

  it("defines the theme variables its inline styles use", () => {
    // The panel's components style themselves with app theme vars; a standalone
    // file has none, so an undefined var would silently drop every border.
    const html = renderDocumentHtml(
      "# H\n\n> quote\n\n`inline` and [link](x.md)\n\n```js\nconst a = 1;\n```\n",
      "t",
    );
    const used = [...html.matchAll(/var\((--[\w-]+)/g)].map((m) => m[1]);

    expect(used.length).toBeGreaterThan(0);
    for (const name of new Set(used)) {
      expect(html).toContain(`${name}:`);
    }
  });

  it("escapes the title", () => {
    expect(renderDocumentHtml("x", '<script>"')).toContain(
      "<title>&lt;script&gt;&quot;</title>",
    );
  });
});
