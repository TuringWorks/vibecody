import { describe, it, expect } from 'vitest';
import {
  archiveContainer,
  archiveDisplayPath,
  isArchiveFile,
  isArchiveMemberPath,
  isArchivePath,
  joinArchivePath,
  splitArchivePath,
  stripArchiveExtension,
} from '../archive';

describe('isArchiveFile', () => {
  it('recognizes the zip family under all its names', () => {
    expect(isArchiveFile('dist.zip')).toBe(true);
    expect(isArchiveFile('plugin.VSIX')).toBe(true);
    expect(isArchiveFile('lib.jar')).toBe(true);
    expect(isArchiveFile('/abs/path/app.apk')).toBe(true);
  });

  it('recognizes tarballs, compressed or not', () => {
    for (const name of ['a.tar', 'a.tar.gz', 'a.tgz', 'a.tar.bz2', 'a.tar.zst', 'a.txz']) {
      expect(isArchiveFile(name), name).toBe(true);
    }
  });

  it('recognizes a single compressed file', () => {
    expect(isArchiveFile('server.log.gz')).toBe(true);
    expect(isArchiveFile('dump.sql.zst')).toBe(true);
  });

  it('leaves documents to the document viewer', () => {
    // These are zips, but the editor renders them; expanding a .docx into a
    // tree of XML parts would be a regression, not a feature.
    for (const name of ['report.docx', 'sheet.xlsx', 'book.epub', 'notes.odt']) {
      expect(isArchiveFile(name), name).toBe(false);
    }
  });

  it('is not fooled by ordinary files or a bare suffix', () => {
    expect(isArchiveFile('main.rs')).toBe(false);
    expect(isArchiveFile('Makefile')).toBe(false);
    expect(isArchiveFile('.gz')).toBe(false);
  });
});

describe('splitArchivePath', () => {
  it('splits a member reference', () => {
    expect(splitArchivePath('/w/dist.zip!/src/main.rs')).toEqual({
      archive: '/w/dist.zip',
      inner: 'src/main.rs',
    });
  });

  it('leaves an ordinary path alone even when it contains the separator', () => {
    // A real directory can be named `we!`; the left half has to name an
    // archive before this is a member reference.
    expect(splitArchivePath('/w/we!/there/main.rs')).toBeNull();
    expect(splitArchivePath('/w/main.rs')).toBeNull();
  });

  it('splits at the outermost archive for a nested reference', () => {
    expect(splitArchivePath('/w/a.zip!/b.jar!/C.class')).toEqual({
      archive: '/w/a.zip',
      inner: 'b.jar!/C.class',
    });
  });

  it('round-trips through joinArchivePath', () => {
    const joined = joinArchivePath('/w/a.tar.gz', '/docs/README.md');
    expect(joined).toBe('/w/a.tar.gz!/docs/README.md');
    expect(splitArchivePath(joined)).toEqual({ archive: '/w/a.tar.gz', inner: 'docs/README.md' });
  });
});

describe('path predicates', () => {
  it('separates a member from its container', () => {
    expect(isArchiveMemberPath('/w/a.zip!/x')).toBe(true);
    expect(isArchiveMemberPath('/w/a.zip')).toBe(false);
    expect(archiveContainer('/w/a.zip!/x')).toBe('/w/a.zip');
    expect(archiveContainer('/w/main.rs')).toBe('/w/main.rs');
  });

  it('routes both the archive and its inner folders to the archive lister', () => {
    expect(isArchivePath('/w/a.zip')).toBe(true);
    expect(isArchivePath('/w/a.zip!/src')).toBe(true);
    expect(isArchivePath('/w/src')).toBe(false);
  });
});

describe('stripArchiveExtension', () => {
  it('drops the whole compound suffix', () => {
    expect(stripArchiveExtension('dist.tar.gz')).toBe('dist');
    expect(stripArchiveExtension('plugin.vsix')).toBe('plugin');
    expect(stripArchiveExtension('/tmp/a/b.zip')).toBe('b');
  });

  it('keeps the inner extension of a single compressed file', () => {
    // `server.log.gz` extracts to a folder holding `server.log`.
    expect(stripArchiveExtension('server.log.gz')).toBe('server.log');
  });

  it('returns a non-archive name unchanged', () => {
    expect(stripArchiveExtension('main.rs')).toBe('main.rs');
  });
});

describe('archiveDisplayPath', () => {
  it('names the archive a member came from', () => {
    expect(archiveDisplayPath('/w/proj/dist.zip!/src/main.rs')).toBe('dist.zip → src/main.rs');
  });

  it('passes an ordinary path straight through', () => {
    expect(archiveDisplayPath('/w/proj/src/main.rs')).toBe('/w/proj/src/main.rs');
  });
});
