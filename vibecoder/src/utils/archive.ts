/**
 * Archive paths, front-end half.
 *
 * The explorer addresses a file inside an archive with a *virtual path*: the
 * archive's real path, the separator `!/`, then the member's path inside the
 * archive.
 *
 *     /home/me/proj/dist.zip!/dist/index.js
 *
 * `vibe_core::archive` (Rust) is the authority — it owns the same extension
 * table, the same split rule, and every bound. This module exists so the tree
 * can decide "is this row expandable?" and "is this tab read-only?" without a
 * round trip per row. Keep the two extension lists in step: a format only the
 * backend knows about will never get a chevron, and one only the front end
 * knows about gets a chevron that errors when clicked.
 */

export const ARCHIVE_SEPARATOR = "!/";

/** Zip containers under another name. Deliberately excludes DOCX/XLSX/PPTX/
 *  ODT/EPUB — they are zips, but the editor renders them as documents. */
const ZIP_EXTENSIONS = [
  "zip", "jar", "war", "ear", "apk", "aab", "ipa", "whl", "egg", "vsix",
  "xpi", "nupkg", "zipx", "maff", "sketch",
];

/** Longest suffix first: `.tar.gz` must beat `.gz`. */
const ARCHIVE_SUFFIXES = [
  ".tar.gz", ".tar.bz2", ".tar.zst", ".tar.zstd", ".tar.xz",
  ".tgz", ".tbz", ".tbz2", ".tzst", ".txz", ".tar",
  ".gz", ".bz2", ".zst", ".zstd", ".xz",
];

const baseName = (path: string): string =>
  path.split(/[\\/]/).pop() ?? path;

/** Whether the explorer should render this file as an expandable node. */
export function isArchiveFile(name: string): boolean {
  const lower = baseName(name).toLowerCase();
  if (ARCHIVE_SUFFIXES.some(s => lower.endsWith(s) && lower.length > s.length)) return true;
  const dot = lower.lastIndexOf(".");
  if (dot <= 0) return false;
  return ZIP_EXTENSIONS.includes(lower.slice(dot + 1));
}

/**
 * Split `archive!/inner`, or `null` for an ordinary path.
 *
 * The left half has to name an archive, so a real directory containing `!`
 * (`~/we!/there/main.rs`) stays an ordinary path. For a nested reference the
 * first archive-looking prefix wins — the backend refines that to "the first
 * one that exists on disk", which it can check and this cannot.
 */
export function splitArchivePath(path: string): { archive: string; inner: string } | null {
  let offset = 0;
  for (;;) {
    const at = path.indexOf(ARCHIVE_SEPARATOR, offset);
    if (at === -1) return null;
    const archive = path.slice(0, at);
    if (isArchiveFile(archive)) {
      return { archive, inner: path.slice(at + ARCHIVE_SEPARATOR.length) };
    }
    offset = at + ARCHIVE_SEPARATOR.length;
  }
}

/** True for a path that points *inside* an archive. */
export function isArchiveMemberPath(path: string): boolean {
  return splitArchivePath(path) !== null;
}

/** The on-disk file behind a path that may be virtual. */
export function archiveContainer(path: string): string {
  return splitArchivePath(path)?.archive ?? path;
}

/** Build the virtual path for `inner` inside `archive`. */
export function joinArchivePath(archive: string, inner: string): string {
  return `${archive}${ARCHIVE_SEPARATOR}${inner.replace(/^\/+/, "")}`;
}

/**
 * Whether this path should be listed with `list_archive` rather than
 * `list_directory`: the archive file itself, or a folder inside one.
 */
export function isArchivePath(path: string): boolean {
  return isArchiveMemberPath(path) || isArchiveFile(path);
}

/** The archive's name with its archive extension removed — the folder an
 *  extraction creates. `dist.tar.gz` → `dist`, `plugin.vsix` → `plugin`. */
export function stripArchiveExtension(name: string): string {
  const base = baseName(name);
  const lower = base.toLowerCase();
  const suffix = ARCHIVE_SUFFIXES.find(s => lower.endsWith(s) && lower.length > s.length);
  if (suffix) return base.slice(0, base.length - suffix.length);
  const dot = base.lastIndexOf(".");
  if (dot > 0 && ZIP_EXTENSIONS.includes(lower.slice(dot + 1))) return base.slice(0, dot);
  return base;
}

/** Display form for a member: `dist.zip → src/main.rs`. Used in tab titles and
 *  the read-only banner, where the bare file name would not say which archive
 *  it came from. */
export function archiveDisplayPath(path: string): string {
  const split = splitArchivePath(path);
  if (!split) return path;
  return `${baseName(split.archive)} → ${split.inner}`;
}
