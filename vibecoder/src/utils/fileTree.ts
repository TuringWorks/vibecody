/**
 * Pure state transitions behind the explorer's Refresh.
 *
 * Refresh used to re-list the workspace root and nothing else, so a file
 * created inside an expanded subfolder never appeared however many times the
 * button was clicked. Refreshing every *visible* directory means handling the
 * directories that vanished between listings too — a cache entry for a deleted
 * folder renders children that no longer exist, and an expanded set that still
 * names it keeps trying to re-list it forever.
 *
 * Kept out of App.tsx so the pruning rules are testable without mounting the
 * editor.
 */

import { ARCHIVE_SEPARATOR } from "./archive";

/** Minimal shape the tree cache needs — the real `FileEntry` satisfies it. */
export interface TreeEntry {
  path: string;
}

/**
 * One listing attempt. `null` means the directory could not be listed, which
 * for a directory the tree was showing a moment ago means it is gone.
 */
export type Listing<E extends TreeEntry> = readonly [string, E[] | null];

/** The directories the tree is showing: the root plus everything expanded. */
export function visibleDirs(root: string, expanded: ReadonlySet<string>): string[] {
  return [...new Set<string>([root, ...expanded])];
}

/** Directories that could not be listed. */
export function missingDirs<E extends TreeEntry>(listed: ReadonlyArray<Listing<E>>): string[] {
  return listed.filter(([, entries]) => entries === null).map(([dir]) => dir);
}

/**
 * Is `path` one of the missing directories, or inside one?
 *
 * "Inside" covers both separators the tree uses: the filesystem's, and the
 * archive separator. A cached listing for `dist.zip!/src` is beneath
 * `dist.zip`, so deleting the archive has to drop it — matching on the
 * filesystem separator alone left those entries behind, and a new archive of
 * the same name would have shown the old contents.
 */
export function isUnderAny(path: string, dirs: readonly string[], sep: string): boolean {
  return dirs.some(
    dir => path === dir || path.startsWith(dir + sep) || path.startsWith(dir + ARCHIVE_SEPARATOR),
  );
}

/**
 * Fold fresh listings into the cache, dropping the vanished directories and
 * everything cached beneath them — a deleted folder's descendants are
 * unreachable but would survive in the cache and reappear the moment a
 * same-named folder was created.
 */
export function mergeListings<E extends TreeEntry>(
  cache: ReadonlyMap<string, E[]>,
  listed: ReadonlyArray<Listing<E>>,
  missing: readonly string[],
  sep: string,
): Map<string, E[]> {
  const next = new Map(cache);
  for (const [dir, entries] of listed) {
    if (entries) next.set(dir, entries);
  }
  if (missing.length > 0) {
    for (const key of [...next.keys()]) {
      if (isUnderAny(key, missing, sep)) next.delete(key);
    }
  }
  return next;
}

/** Forget the expansion state of directories that no longer exist. */
export function pruneExpanded(
  expanded: ReadonlySet<string>,
  missing: readonly string[],
  sep: string,
): Set<string> {
  if (missing.length === 0) return new Set(expanded);
  return new Set([...expanded].filter(dir => !isUnderAny(dir, missing, sep)));
}
