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
 * `<details>` is the exception: a balanced block is lifted out by
 * `splitDetails` and rendered as a real disclosure, so the answer it hides
 * stays hidden until the reader clicks. Only an *unbalanced* `<details>` falls
 * through to the stripping above — a document should not lose its remaining
 * text behind a toggle because a closing tag went missing.
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

/**
 * Walk `source` line by line, applying `rewrite` to the lines outside a fenced
 * code block and `rewriteFenced` (identity by default) to the fence lines and
 * everything between them. Both the HTML rewrite and the code mask need
 * exactly this walk, and a second copy of fence bookkeeping is a second place
 * to get the closing rule wrong.
 */
function mapLinesOutsideFences(
  source: string,
  rewrite: (line: string) => string,
  rewriteFenced: (line: string) => string = (line) => line,
): string {
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
          return rewriteFenced(line);
        }
        return rewrite(line);
      }
      const closes =
        marker !== undefined &&
        marker[0] === fence[0] &&
        marker.length >= fence.length &&
        line.slice(line.indexOf(marker) + marker.length).trim() === "";
      if (closes) fence = null;
      return rewriteFenced(line);
    })
    .join("\n");
}

/** Rewrite the raw HTML in `source`, leaving fenced code exactly as written. */
export const htmlToMarkdown = (source: string): string =>
  mapLinesOutsideFences(source, rewriteLine);

/**
 * A copy of `source` with every code region blanked to spaces, same length and
 * same line structure. Scanning this instead of the source is what keeps a
 * `<details>` written *inside* an example from being taken for real markup.
 */
const blank = (text: string): string => " ".repeat(text.length);

const maskCode = (source: string): string =>
  mapLinesOutsideFences(
    source,
    (line) =>
      line
        .split(/(`+[^`]*`+)/g)
        .map((part, i) => (i % 2 === 1 ? blank(part) : part))
        .join(""),
    blank,
  );

/** A document is a sequence of markdown runs and the disclosures between them. */
export type DocSegment =
  | { readonly kind: "markdown"; readonly text: string }
  | {
      readonly kind: "details";
      readonly summary: string;
      readonly body: string;
      readonly open: boolean;
    };

/** `<details …>` and `</details>`, with the slash captured to count depth. */
const DETAILS_TAG = "<(/?)details(?:\\s[^<>]*)?>";
const SUMMARY_OPEN = /^\s*<summary(?:\s[^<>]*)?>/i;
const SUMMARY_CLOSE = /<\/summary\s*>/i;

/** Index just past the `</details>` that closes the block opened before `from`. */
function findClose(mask: string, from: number): { start: number; end: number } | null {
  // A fresh regex per scan: the /g lastIndex is state, and this function is
  // called from a loop that also scans with its own cursor.
  const tag = new RegExp(DETAILS_TAG, "gi");
  tag.lastIndex = from;
  let depth = 1;
  for (let m = tag.exec(mask); m !== null; m = tag.exec(mask)) {
    depth += m[1] === "/" ? -1 : 1;
    if (depth === 0) return { start: m.index, end: m.index + m[0].length };
  }
  return null;
}

/** Split a `<details>` body into its `<summary>` label and the rest. */
function takeSummary(body: string, mask: string): { summary: string; body: string } {
  const open = SUMMARY_OPEN.exec(mask);
  if (open === null) return { summary: "", body };
  const rest = mask.slice(open[0].length);
  const close = SUMMARY_CLOSE.exec(rest);
  if (close === null) return { summary: "", body };
  const labelStart = open[0].length;
  const labelEnd = labelStart + close.index;
  return {
    summary: body.slice(labelStart, labelEnd),
    body: body.slice(labelEnd + close[0].length),
  };
}

/**
 * Lift every balanced `<details>` block out of `source` so it can be rendered
 * as a real disclosure. Blocks written inside code are left where they are,
 * and so is an opening tag with no closing one — hiding the whole remainder of
 * a document behind a toggle is a worse failure than showing a stray tag.
 *
 * Nesting is handled by the depth count here; the body comes back unparsed, so
 * a nested block is found by splitting the body again.
 */
export function splitDetails(source: string): DocSegment[] {
  const mask = maskCode(source);
  const opener = new RegExp(DETAILS_TAG, "gi");
  const segments: DocSegment[] = [];
  let cursor = 0;

  for (let m = opener.exec(mask); m !== null; m = opener.exec(mask)) {
    if (m[1] === "/" || m.index < cursor) continue;
    const bodyStart = m.index + m[0].length;
    const close = findClose(mask, bodyStart);
    if (close === null) break;
    const raw = takeSummary(source.slice(bodyStart, close.start), mask.slice(bodyStart, close.start));
    segments.push({ kind: "markdown", text: source.slice(cursor, m.index) });
    segments.push({
      kind: "details",
      summary: raw.summary,
      body: raw.body,
      open: /\sopen(\s|=|$)/i.test(m[0].slice(0, -1)),
    });
    cursor = close.end;
    opener.lastIndex = cursor;
  }

  segments.push({ kind: "markdown", text: source.slice(cursor) });
  return segments.filter((s) => s.kind !== "markdown" || s.text.trim() !== "");
}
