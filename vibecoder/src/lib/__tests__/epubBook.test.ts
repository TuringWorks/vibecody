/**
 * The EPUB command boundary. Same rule as the rest of the document surface:
 * parse the payload, never cast it — and never turn a resource the viewer
 * cannot identify into something it will render anyway.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  dataUrl,
  parseEpubBook,
  parseEpubChapter,
  readEpubBook,
  readEpubChapter,
  resourceUrls,
  type EpubResource,
} from '../epubBook';

const resource = (over: Partial<EpubResource> = {}): EpubResource => ({
  path: 'OEBPS/images/fig1.png',
  href: '../images/fig1.png',
  mime: 'image/png',
  base64: 'AAAA',
  ...over,
});

const bookPayload = (over: Record<string, unknown> = {}) => ({
  title: 'A Test Book',
  authors: ['Ada Lovelace'],
  language: 'en',
  publisher: null,
  chapters: [{ path: 'OEBPS/text/ch1.xhtml', title: 'One' }],
  toc: [{ label: 'One', path: 'OEBPS/text/ch1.xhtml', fragment: null, level: 0 }],
  cover: resource({ mime: 'image/jpeg' }),
  warnings: [],
  ...over,
});

const chapterPayload = (over: Record<string, unknown> = {}) => ({
  path: 'OEBPS/text/ch1.xhtml',
  title: 'One',
  html: '<h1>One</h1>',
  css: 'p { color: red }',
  resources: [resource()],
  warnings: [{ code: 'epub.missing_resource', message: 'gone.png is not in the book' }],
  ...over,
});

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('parseEpubBook', () => {
  it('parses a well-formed book', () => {
    const book = parseEpubBook(bookPayload());
    expect(book.title).toBe('A Test Book');
    expect(book.authors).toEqual(['Ada Lovelace']);
    expect(book.publisher).toBeNull();
    expect(book.chapters[0].path).toBe('OEBPS/text/ch1.xhtml');
    expect(book.toc[0].level).toBe(0);
    expect(book.cover?.mime).toBe('image/jpeg');
  });

  it('accepts a book with no cover and no contents list', () => {
    const book = parseEpubBook(bookPayload({ cover: null, toc: [] }));
    expect(book.cover).toBeNull();
    expect(book.toc).toEqual([]);
  });

  it('names the field when the payload is wrong', () => {
    expect(() => parseEpubBook(bookPayload({ chapters: 'one' }))).toThrow(/"chapters"/);
    expect(() => parseEpubBook(bookPayload({ authors: [1] }))).toThrow(/authors\[0\]/);
    expect(() => parseEpubBook(bookPayload({ toc: [{ label: 'x' }] }))).toThrow(/"path"/);
    expect(() => parseEpubBook(null)).toThrow(/"book"/);
  });
});

describe('parseEpubChapter', () => {
  it('parses markup, styles, resources and warnings', () => {
    const chapter = parseEpubChapter(chapterPayload());
    expect(chapter.html).toBe('<h1>One</h1>');
    expect(chapter.css).toBe('p { color: red }');
    expect(chapter.resources[0].path).toBe('OEBPS/images/fig1.png');
    expect(chapter.warnings[0].code).toBe('epub.missing_resource');
  });

  it('rejects a chapter with no markup field rather than rendering blank', () => {
    const { html: _dropped, ...withoutHtml } = chapterPayload();
    expect(() => parseEpubChapter(withoutHtml)).toThrow(/"html"/);
  });
});

describe('resourceUrls', () => {
  const createObjectURL = vi.fn(() => 'blob:vibecoder/1');
  const revokeObjectURL = vi.fn();

  beforeEach(() => {
    createObjectURL.mockClear();
    revokeObjectURL.mockClear();
    Object.assign(URL, { createObjectURL, revokeObjectURL });
  });

  it('keys each resource by both the container path and the authored href', () => {
    // Markup says `../images/fig1.png`; a stylesheet says `OEBPS/images/fig1.png`.
    // Both must find the same blob.
    const { urls } = resourceUrls([resource()]);
    expect(urls.get('OEBPS/images/fig1.png')).toBe('blob:vibecoder/1');
    expect(urls.get('../images/fig1.png')).toBe('blob:vibecoder/1');
  });

  it('refuses to make a URL for a resource whose type is unknown', () => {
    // The backend uses application/octet-stream to mean "I will not guess."
    // Rendering it anyway would let a book decide how its bytes are treated.
    const { urls } = resourceUrls([resource({ mime: 'application/octet-stream' })]);
    expect(urls.size).toBe(0);
    expect(createObjectURL).not.toHaveBeenCalled();
  });

  it('revokes every URL it created', () => {
    const { revoke } = resourceUrls([resource(), resource({ path: 'b.png', href: 'b.png' })]);
    revoke();
    expect(revokeObjectURL).toHaveBeenCalledTimes(2);
  });
});

describe('dataUrl', () => {
  it('builds a data URL for a resource that outlives one chapter', () => {
    expect(dataUrl(resource({ mime: 'image/jpeg', base64: 'QUJD' }))).toBe(
      'data:image/jpeg;base64,QUJD',
    );
  });
});

describe('commands', () => {
  it('reads a book by path', async () => {
    mockInvoke.mockResolvedValueOnce(bookPayload());
    await readEpubBook('/books/x.epub');
    expect(mockInvoke).toHaveBeenCalledWith('read_epub_book', { path: '/books/x.epub' });
  });

  it('reads a chapter by its container path', async () => {
    mockInvoke.mockResolvedValueOnce(chapterPayload());
    await readEpubChapter('/books/x.epub', 'OEBPS/text/ch1.xhtml');
    expect(mockInvoke).toHaveBeenCalledWith('read_epub_chapter', {
      path: '/books/x.epub',
      chapter: 'OEBPS/text/ch1.xhtml',
    });
  });
});
