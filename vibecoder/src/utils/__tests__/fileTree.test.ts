import { describe, it, expect } from 'vitest';
import {
  visibleDirs,
  missingDirs,
  isUnderAny,
  mergeListings,
  pruneExpanded,
  type Listing,
} from '../fileTree';

interface E {
  path: string;
  name: string;
}

const entry = (path: string): E => ({ path, name: path.split('/').pop() ?? path });
const listing = (dir: string, paths: string[] | null): Listing<E> =>
  [dir, paths === null ? null : paths.map(entry)] as const;

describe('visibleDirs', () => {
  it('is the root plus every expanded folder, without duplicates', () => {
    const dirs = visibleDirs('/ws', new Set(['/ws', '/ws/src', '/ws/src/utils']));
    expect(dirs.sort()).toEqual(['/ws', '/ws/src', '/ws/src/utils']);
  });

  it('includes the root even when nothing is expanded', () => {
    expect(visibleDirs('/ws', new Set())).toEqual(['/ws']);
  });
});

describe('mergeListings', () => {
  it('replaces a stale listing with the fresh one', () => {
    // The bug this whole module exists for: a file created inside an expanded
    // subfolder was invisible because its cached listing was never re-read.
    const cache = new Map<string, E[]>([['/ws/src', [entry('/ws/src/old.ts')]]]);
    const next = mergeListings(
      cache,
      [listing('/ws/src', ['/ws/src/old.ts', '/ws/src/new.ts'])],
      [],
      '/',
    );
    expect(next.get('/ws/src')?.map(e => e.path)).toEqual(['/ws/src/old.ts', '/ws/src/new.ts']);
  });

  it('leaves directories that were not re-listed untouched', () => {
    const cache = new Map<string, E[]>([
      ['/ws', [entry('/ws/a')]],
      ['/ws/other', [entry('/ws/other/b')]],
    ]);
    const next = mergeListings(cache, [listing('/ws', ['/ws/a', '/ws/c'])], [], '/');
    expect(next.get('/ws/other')?.map(e => e.path)).toEqual(['/ws/other/b']);
  });

  it('drops a vanished directory and everything cached beneath it', () => {
    const cache = new Map<string, E[]>([
      ['/ws', [entry('/ws/gone')]],
      ['/ws/gone', [entry('/ws/gone/deep')]],
      ['/ws/gone/deep', [entry('/ws/gone/deep/f.ts')]],
      ['/ws/kept', [entry('/ws/kept/f.ts')]],
    ]);
    const listed = [listing('/ws', ['/ws/kept']), listing('/ws/gone', null)];
    const next = mergeListings(cache, listed, missingDirs(listed), '/');

    expect(next.has('/ws/gone')).toBe(false);
    expect(next.has('/ws/gone/deep')).toBe(false);
    expect(next.has('/ws/kept')).toBe(true);
  });

  it('does not treat a sibling with a shared prefix as a descendant', () => {
    // "/ws/src2" starts with "/ws/src" as a string but is not inside it.
    const cache = new Map<string, E[]>([['/ws/src2', [entry('/ws/src2/f.ts')]]]);
    const next = mergeListings(cache, [listing('/ws/src', null)], ['/ws/src'], '/');
    expect(next.has('/ws/src2')).toBe(true);
  });

  it('does not mutate the cache it was given', () => {
    const cache = new Map<string, E[]>([['/ws', [entry('/ws/a')]]]);
    mergeListings(cache, [listing('/ws', ['/ws/a', '/ws/b'])], [], '/');
    expect(cache.get('/ws')?.map(e => e.path)).toEqual(['/ws/a']);
  });
});

describe('pruneExpanded', () => {
  it('forgets a deleted folder and its expanded children', () => {
    const expanded = new Set(['/ws', '/ws/gone', '/ws/gone/deep', '/ws/kept']);
    expect([...pruneExpanded(expanded, ['/ws/gone'], '/')].sort()).toEqual(['/ws', '/ws/kept']);
  });

  it('is a copy, not the same set, when nothing is missing', () => {
    const expanded = new Set(['/ws']);
    const next = pruneExpanded(expanded, [], '/');
    expect(next).not.toBe(expanded);
    expect([...next]).toEqual(['/ws']);
  });
});

describe('isUnderAny', () => {
  it('handles Windows separators', () => {
    expect(isUnderAny('C:\\ws\\src\\a.ts', ['C:\\ws\\src'], '\\')).toBe(true);
    expect(isUnderAny('C:\\ws\\src2\\a.ts', ['C:\\ws\\src'], '\\')).toBe(false);
  });
});

describe('archive nodes in the tree cache', () => {
  it('drops a vanished archive\'s member listings along with it', () => {
    // `dist.zip!/src` is beneath `dist.zip` even though no filesystem
    // separator joins them.
    expect(isUnderAny('/w/dist.zip!/src', ['/w/dist.zip'], '/')).toBe(true);
    expect(isUnderAny('/w/dist.zip', ['/w/dist.zip'], '/')).toBe(true);
    expect(isUnderAny('/w/other.zip!/src', ['/w/dist.zip'], '/')).toBe(false);
  });

  it('prunes both the cache and the expanded set for a deleted archive', () => {
    const cache = new Map([
      ['/w', [{ path: '/w/dist.zip' }]],
      ['/w/dist.zip', [{ path: '/w/dist.zip!/src' }]],
      ['/w/dist.zip!/src', [{ path: '/w/dist.zip!/src/main.rs' }]],
    ]);
    const merged = mergeListings(cache, [['/w/dist.zip', null]], ['/w/dist.zip'], '/');
    expect([...merged.keys()]).toEqual(['/w']);

    const expanded = new Set(['/w', '/w/dist.zip', '/w/dist.zip!/src']);
    expect([...pruneExpanded(expanded, ['/w/dist.zip'], '/')]).toEqual(['/w']);
  });
});
