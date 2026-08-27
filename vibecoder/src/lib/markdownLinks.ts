/**
 * Where a markdown link points.
 *
 * A rendered document is full of hrefs that mean three different things, and
 * the preview used to treat only one of them — `http` — as real. A link to
 * `docs/README.md` sitting next to one to `https://example.com` is not "not a
 * URL", it is a different kind of destination: another file in the workspace,
 * addressed the way a browser addresses one, relative to the document that
 * names it.
 */
export type MarkdownLinkTarget =
  /** Hand to the OS browser. */
  | { kind: "external"; url: string }
  /** Stays inside the rendered document. */
  | { kind: "anchor"; fragment: string }
  /** Another file in the workspace, already resolved to a full path. */
  | { kind: "file"; path: string; fragment: string | null };

/**
 * A scheme needs two characters before the colon, so a Windows path (`C:/…`)
 * is a path rather than a protocol nobody has heard of.
 */
const SCHEME = /^[a-zA-Z][a-zA-Z0-9+.-]+:/;

/**
 * Only a bare domain whose suffix is a web TLD is upgraded to `https://`.
 * Without the list, `CHANGELOG.md` reads as a host in the `.md` domain and the
 * link leaves the app instead of opening the file next door.
 */
const WEB_TLDS = new Set([
  "com", "org", "net", "io", "co", "dev", "ai", "app", "xyz", "tech", "gov", "edu",
]);

/** Split an href into its path and fragment parts. */
export function splitFragment(href: string): { path: string; fragment: string | null } {
  const hash = href.indexOf("#");
  if (hash === -1) return { path: href, fragment: null };
  return { path: href.slice(0, hash), fragment: href.slice(hash + 1) || null };
}

function decodeSafely(href: string): string {
  try {
    return decodeURIComponent(href);
  } catch {
    return href;
  }
}

/** The part of a path that `..` can never climb past: `/`, `C:/`, or nothing. */
function rootOf(path: string): string {
  if (/^[a-zA-Z]:\//.test(path)) return path.slice(0, 3);
  if (path.startsWith("/")) return "/";
  return "";
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

/**
 * Resolve a link target against the document that contains it, the way a
 * browser resolves a relative URL: against the document's *directory*, not the
 * document itself.
 *
 * `basePath` absent means the caller does not know where the document lives,
 * so a relative target stays relative — a guessed workspace root would be a
 * plausible answer to a question nobody asked.
 */
export function resolveRelativePath(
  basePath: string | null | undefined,
  target: string,
): string {
  const wanted = target.replace(/\\/g, "/");
  const base = (basePath ?? "").replace(/\\/g, "/");

  const targetRoot = rootOf(wanted);
  if (targetRoot) return targetRoot + normalizeSegments(wanted.slice(targetRoot.length));

  const baseRoot = rootOf(base);
  const baseBody = base.slice(baseRoot.length);
  const dir = baseBody.includes("/") ? baseBody.slice(0, baseBody.lastIndexOf("/") + 1) : "";
  return baseRoot + normalizeSegments(`${dir}${wanted}`);
}

/** A schema-less host that is worth upgrading to `https://` — `github.com/x`. */
function looksLikeBareDomain(href: string): boolean {
  const firstSlash = href.indexOf("/");
  const firstDot = href.indexOf(".");
  if (firstDot === -1) return false;
  if (firstSlash !== -1 && firstSlash < firstDot) return false;
  const end = firstSlash === -1 ? href.length : firstSlash;
  const rest = href.slice(firstDot + 1, end).toLowerCase();
  // `a.b.com/x` — the TLD is the last label of the host, not the first.
  const tld = rest.includes(".") ? rest.slice(rest.lastIndexOf(".") + 1) : rest;
  return WEB_TLDS.has(tld);
}

/**
 * Classify a markdown href. Returns `null` for an href with no destination —
 * an empty string, or a bare `#` — so a caller never opens "nothing".
 */
export function classifyMarkdownLink(
  href: string,
  basePath?: string | null,
): MarkdownLinkTarget | null {
  const raw = href.trim();
  if (!raw) return null;

  if (raw.startsWith("#")) {
    const fragment = raw.slice(1);
    return fragment ? { kind: "anchor", fragment } : null;
  }

  // Protocol-relative: the webview has no origin worth borrowing, so pick the
  // scheme rather than resolving against `tauri://localhost`.
  if (raw.startsWith("//")) return { kind: "external", url: `https:${raw}` };

  if (SCHEME.test(raw)) {
    // `file:` names a path on this machine; opening it in a browser is a
    // detour through the OS back to a file the editor can already show.
    if (/^file:/i.test(raw)) {
      const withoutScheme = raw.replace(/^file:\/\/(localhost)?/i, "").replace(/^file:/i, "");
      const { path, fragment } = splitFragment(decodeSafely(withoutScheme));
      return path ? { kind: "file", path: resolveRelativePath(basePath, path), fragment } : null;
    }
    return { kind: "external", url: raw };
  }

  if (looksLikeBareDomain(raw)) return { kind: "external", url: `https://${raw}` };

  const { path, fragment } = splitFragment(decodeSafely(raw));
  if (!path) return null;
  return { kind: "file", path: resolveRelativePath(basePath, path), fragment };
}

/**
 * The id GitHub gives a heading, so `#answer-style` in a document written for
 * GitHub finds the same heading here.
 */
export function slugifyHeading(text: string): string {
  return text
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, "")
    // One hyphen per space, not per run: GitHub strips the punctuation first,
    // so `Style — dense` leaves two spaces behind and its id has two hyphens.
    .replace(/\s/g, "-");
}
