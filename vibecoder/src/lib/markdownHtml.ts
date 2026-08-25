/**
 * Raw HTML inside a markdown document, made readable.
 *
 * `react-markdown` renders a raw HTML node as its literal source, and nothing
 * in this app adds `rehype-raw` — deliberately: the documents rendered here
 * come from a model or from a workspace file, and neither is trusted with
 * script-bearing markup inside the app webview. So a generated study guide
 * that hides its answers behind `<details><summary><b>Answer</b></summary>`
 * shows the reader the tags instead of the disclosure they describe.
 *
 * This rewrites the tags a reader cannot use into their markdown equivalent —
 * `<b>` → `**`, `<a href>` → a link — and drops the rest while keeping the
 * text they wrapped. Fenced blocks and inline code spans are passed through
 * untouched, so an HTML *example* still reads as an HTML example. That split
 * has a cost worth stating: a tag named in prose without backticks ("wrap it
 * in <div>") disappears along with the real markup. Backticks keep it.
 *
 * `<details>` collapses to its summary line rather than becoming a working
 * disclosure — a real one needs a raw-HTML pipeline, which is the thing this
 * module exists instead of.
 *
 * Only names that are actually HTML elements are touched, so `Vec<String>`,
 * `<T>` and `<https://example.com>` survive.
 */

/** Elements whose tags carry no meaning once the HTML is not rendered. */
const STRUCTURAL = [
  "a", "abbr", "address", "article", "aside", "audio", "bdi", "bdo", "big",
  "blockquote", "body", "button", "caption", "center", "cite", "col",
  "colgroup", "datalist", "dd", "details", "dfn", "dialog", "div", "dl", "dt",
  "fieldset", "figcaption", "figure", "font", "footer", "form", "h1", "h2",
  "h3", "h4", "h5", "h6", "head", "header", "hgroup", "html", "iframe",
  "input", "ins", "label", "legend", "main", "map", "mark", "menu", "meta",
  "nav", "noscript", "object", "ol", "optgroup", "option", "output", "p",
  "param", "picture", "pre", "progress", "q", "samp", "script", "section",
  "select", "small", "source", "span", "style", "sub", "summary", "sup",
  "svg", "table", "tbody", "td", "textarea", "tfoot", "th", "thead", "time",
  "tr", "track", "u", "ul", "var", "video", "wbr",
];

/** `</?name …>` for any of `names`, tolerating attributes and self-closing. */
const tag = (names: readonly string[]): RegExp =>
  new RegExp(`</?(?:${names.join("|")})(?:\\s[^<>]*)?/?>`, "gi");

const attr = (name: string, source: string): string | undefined =>
  new RegExp(`\\b${name}\\s*=\\s*("([^"]*)"|'([^']*)'|([^\\s"'<>]+))`, "i")
    .exec(source)
    ?.slice(2)
    .find((v) => v !== undefined);

const HTML_COMMENT = /<!--[\s\S]*?-->/g;
const ANCHOR = /<a\s[^<>]*>([\s\S]*?)<\/a\s*>/gi;
const IMAGE = /<img(\s[^<>]*)?\/?>/gi;

/** Applied in order; each entry is behaviour-preserving on non-HTML text. */
const REWRITES: ReadonlyArray<readonly [RegExp, string]> = [
  [/<br(?:\s[^<>]*)?\/?>/gi, "  \n"],
  [/<hr(?:\s[^<>]*)?\/?>/gi, "\n\n---\n\n"],
  [/<li(?:\s[^<>]*)?>/gi, "\n- "],
  [/<\/li\s*>/gi, ""],
  [tag(["b", "strong"]), "**"],
  [tag(["i", "em"]), "*"],
  [tag(["del", "s", "strike"]), "~~"],
  [tag(["code", "kbd"]), "`"],
  [tag(STRUCTURAL), ""],
];

const rewriteProse = (text: string): string =>
  REWRITES.reduce(
    (acc, [pattern, replacement]) => acc.replace(pattern, replacement),
    text
      .replace(HTML_COMMENT, "")
      .replace(ANCHOR, (whole, label: string) => {
        const href = attr("href", whole);
        return href ? `[${label.trim() || href}](${href})` : label;
      })
      .replace(IMAGE, (whole) => {
        const src = attr("src", whole);
        return src ? `![${attr("alt", whole) ?? ""}](${src})` : "";
      }),
  );

/**
 * Inline code spans are left alone. The split keeps the delimiters in the odd
 * positions, so the even ones are exactly the prose.
 */
const rewriteLine = (line: string): string =>
  line
    .split(/(`+[^`]*`+)/g)
    .map((part, i) => (i % 2 === 1 ? part : rewriteProse(part)))
    .join("");

const FENCE = /^ {0,3}(`{3,}|~{3,})/;

/** Rewrite the raw HTML in `source`, leaving fenced code exactly as written. */
export function htmlToMarkdown(source: string): string {
  // Local mutation only: the fence marker is line-to-line state, and building
  // the output with spread inside a reduce would make a long document O(n²).
  let fence: string | null = null;
  return source
    .split("\n")
    .map((line) => {
      const marker = FENCE.exec(line)?.[1];
      if (fence === null) {
        if (marker !== undefined) {
          fence = marker;
          return line;
        }
        return rewriteLine(line);
      }
      const closes =
        marker !== undefined &&
        marker[0] === fence[0] &&
        marker.length >= fence.length &&
        line.slice(line.indexOf(marker) + marker.length).trim() === "";
      if (closes) fence = null;
      return line;
    })
    .join("\n");
}
