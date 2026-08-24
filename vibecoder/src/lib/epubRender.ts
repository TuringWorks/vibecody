/**
 * epubRender — turning a chapter's markup and stylesheets into something safe
 * to put on screen without losing what makes a book look like a book.
 *
 * EPUB content is T5 (attacker-controlled) per docs/security/threat-model.md:
 * the user opened a file that came from somewhere else. Two sinks matter here,
 * and each is closed on purpose:
 *
 *   • **Markup** — sanitised by DOMPurify at the point of insertion (see
 *     `sanitizeEpubHtml` in DocumentViewer.tsx). This module only rewrites
 *     references *after* that, so it can never re-introduce a tag.
 *   • **Styles** — the book's own CSS is what carries its typography, so it is
 *     applied rather than dropped. It is scoped to the chapter container, and
 *     the constructs that let CSS reach outside that container — `@import`,
 *     remote `url()`, `position: fixed`, `expression()`, script URLs — are
 *     removed here, with tests naming each one.
 */

/** Resolve a reference (container path or authored href) to an object URL. */
export type ResolveUrl = (reference: string) => string | undefined;

// ── Stylesheets ──────────────────────────────────────────────────────

/** Declarations that let a rule escape the chapter it belongs to. */
const ESCAPING_POSITIONS = ["fixed", "sticky"];

/**
 * Scope a book's stylesheet to one container and strip what must not run.
 *
 * `scope` is a selector for the element the chapter is rendered into; `resolve`
 * maps a `url(...)` target to a usable URL. A target that does not resolve is
 * dropped rather than left pointing at the app's own origin.
 */
export function scopeEpubCss(css: string, scope: string, resolve: ResolveUrl): string {
  const source = stripComments(css);
  return scopeBlock(source, scope, resolve);
}

function stripComments(css: string): string {
  let out = "";
  let i = 0;
  while (i < css.length) {
    const start = css.indexOf("/*", i);
    if (start === -1) {
      out += css.slice(i);
      break;
    }
    out += css.slice(i, start);
    const end = css.indexOf("*/", start + 2);
    if (end === -1) break;
    i = end + 2;
  }
  return out;
}

function scopeBlock(css: string, scope: string, resolve: ResolveUrl): string {
  const out: string[] = [];
  let i = 0;

  while (i < css.length) {
    while (i < css.length && /[\s;]/.test(css[i])) i++;
    if (i >= css.length) break;

    const braceIndex = css.indexOf("{", i);
    if (braceIndex === -1) break;

    // A statement at-rule ends at its semicolon and has no body — `@import`,
    // `@charset`, `@namespace`. Skipping to the semicolon matters: reading up
    // to the next `{` instead would treat the *following* rule's body as this
    // statement's, and drop it along with the statement.
    const semicolonIndex = css.indexOf(";", i);
    if (semicolonIndex !== -1 && semicolonIndex < braceIndex) {
      i = semicolonIndex + 1;
      continue;
    }

    const prelude = css.slice(i, braceIndex).trim();
    const bodyEnd = matchBrace(css, braceIndex);
    if (bodyEnd === -1) break;
    const body = css.slice(braceIndex + 1, bodyEnd);
    i = bodyEnd + 1;

    if (prelude.startsWith("@")) {
      const name = prelude.slice(1).split(/[\s(]/, 1)[0].toLowerCase();
      switch (name) {
        // Conditional groups contain ordinary rules: recurse so their
        // selectors get scoped too.
        case "media":
        case "supports":
        case "layer":
          out.push(`${prelude} { ${scopeBlock(body, scope, resolve)} }`);
          break;
        // Keyframe selectors are percentages, not elements — scoping them
        // would produce `.chapter 0%`, which matches nothing.
        case "keyframes":
        case "-webkit-keyframes":
          out.push(`${prelude} { ${body} }`);
          break;
        case "font-face":
          out.push(`${prelude} { ${scopeDeclarations(body, resolve)} }`);
          break;
        // `@import` fetches a stylesheet the reader never saw, from a URL the
        // book chooses. `@page` styles the printed document, not this element.
        default:
          break;
      }
      continue;
    }

    if (!prelude) continue;
    out.push(`${scopeSelectors(prelude, scope)} { ${scopeDeclarations(body, resolve)} }`);
  }

  return out.join("\n");
}

function matchBrace(css: string, open: number): number {
  let depth = 0;
  for (let i = open; i < css.length; i++) {
    if (css[i] === "{") depth++;
    else if (css[i] === "}") {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

export function scopeSelectors(selectorList: string, scope: string): string {
  return selectorList
    .split(",")
    .map((selector) => selector.trim())
    .filter(Boolean)
    .map((selector) => {
      // The book's `body` *is* the container it renders into.
      const rooted = selector.replace(/^(html|body|:root)\b/i, "").trim();
      if (rooted === "") return scope;
      if (rooted.startsWith(">") || rooted.startsWith("+") || rooted.startsWith("~")) {
        return `${scope} ${rooted}`;
      }
      return selector === rooted ? `${scope} ${selector}` : `${scope} ${rooted}`;
    })
    .join(", ");
}

function scopeDeclarations(body: string, resolve: ResolveUrl): string {
  return body
    .split(";")
    .map((declaration) => declaration.trim())
    .filter(Boolean)
    .filter((declaration) => !isUnsafeDeclaration(declaration))
    .map((declaration) => rewriteUrls(declaration, resolve))
    .filter((declaration) => declaration !== null)
    .join("; ");
}

function isUnsafeDeclaration(declaration: string): boolean {
  const lower = declaration.toLowerCase();
  if (lower.includes("expression(")) return true;
  if (lower.includes("javascript:") || lower.includes("vbscript:")) return true;
  if (lower.startsWith("position")) {
    const value = lower.split(":").slice(1).join(":");
    return ESCAPING_POSITIONS.some((escape) => value.includes(escape));
  }
  return false;
}

/** Rewrite `url(...)` targets; drop the declaration if any target is unknown. */
function rewriteUrls(declaration: string, resolve: ResolveUrl): string | null {
  if (!declaration.toLowerCase().includes("url(")) return declaration;
  let dropped = false;
  const rewritten = declaration.replace(/url\(\s*(['"]?)([^'")]*)\1\s*\)/gi, (_all, _q, target) => {
    const reference = String(target).trim();
    if (reference.startsWith("data:")) return `url("${reference}")`;
    const url = resolve(reference);
    if (!url) {
      dropped = true;
      return "none";
    }
    return `url("${url}")`;
  });
  // A background that cannot be resolved is better absent than pointing at the
  // application's own origin, where the book chose the path.
  return dropped && /^(background|src)\b/i.test(declaration) ? null : rewritten;
}

// ── Markup ───────────────────────────────────────────────────────────

export interface RewriteResult {
  html: string;
  /** References the chapter used that no resource matched. */
  unresolved: string[];
}

/**
 * Point a chapter's media at the resources that came with it, and mark its
 * internal links so the viewer can navigate them.
 *
 * Runs on already-sanitised markup: it rewrites attributes and never adds
 * elements, so it cannot widen what the sanitiser allowed.
 */
export function rewriteChapterHtml(sanitizedHtml: string, resolve: ResolveUrl): RewriteResult {
  const doc = new DOMParser().parseFromString(
    `<div id="vibedoc-root">${sanitizedHtml}</div>`,
    "text/html",
  );
  const root = doc.getElementById("vibedoc-root");
  if (!root) return { html: sanitizedHtml, unresolved: [] };

  const unresolved: string[] = [];

  for (const element of Array.from(root.querySelectorAll("img, image, source, video, audio"))) {
    for (const attribute of ["src", "href", "xlink:href", "poster"]) {
      const value = element.getAttribute(attribute);
      if (!value || value.startsWith("data:")) continue;
      if (/^[a-z]+:\/\//i.test(value)) {
        // A remote reference in an offline book is a tracking pixel far more
        // often than a picture; drop it rather than fetch it.
        element.removeAttribute(attribute);
        unresolved.push(value);
        continue;
      }
      const url = resolve(value);
      if (url) element.setAttribute(attribute, url);
      else {
        element.removeAttribute(attribute);
        unresolved.push(value);
      }
    }
  }

  for (const anchor of Array.from(root.querySelectorAll("a[href]"))) {
    const href = anchor.getAttribute("href") ?? "";
    if (/^[a-z][a-z0-9+.-]*:/i.test(href)) {
      // An absolute URL is handed to the browser — but only for the three
      // schemes a book has any business using. DOMPurify has already dropped
      // script URLs; this is the second gate, because the value ends up in an
      // `openUrl()` call rather than in the DOM.
      if (/^(https?|mailto):/i.test(href)) {
        anchor.setAttribute("data-epub-external", href);
      } else {
        anchor.removeAttribute("href");
        unresolved.push(href);
      }
      continue;
    }
    anchor.setAttribute("data-epub-link", href);
    // Neutralise the href itself: an unresolved relative URL inside the app
    // would navigate the whole webview away from the editor.
    anchor.setAttribute("href", "#");
  }

  return { html: root.innerHTML, unresolved };
}

/** Split an EPUB href into its path and fragment parts. */
export function splitHref(href: string): { path: string; fragment: string | null } {
  const [path, fragment] = href.split("#");
  return { path, fragment: fragment || null };
}

/**
 * Resolve a chapter-relative href to a container path, the way a browser would
 * against the chapter's own directory.
 */
export function resolveAgainst(basePath: string, href: string): string {
  const target = splitHref(decodeSafely(href)).path;
  if (!target) return basePath;
  if (target.startsWith("/")) return normalizeSegments(target.slice(1));
  const dir = basePath.includes("/") ? basePath.slice(0, basePath.lastIndexOf("/") + 1) : "";
  return normalizeSegments(`${dir}${target}`);
}

function decodeSafely(href: string): string {
  try {
    return decodeURIComponent(href);
  } catch {
    return href;
  }
}

function normalizeSegments(path: string): string {
  const parts: string[] = [];
  for (const segment of path.split("/")) {
    if (segment === "" || segment === ".") continue;
    if (segment === "..") parts.pop();
    else parts.push(segment);
  }
  return parts.join("/");
}
