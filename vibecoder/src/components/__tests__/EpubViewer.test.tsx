/**
 * The EPUB viewer, end to end against a mocked backend.
 *
 * The regression these exist for: the previous viewer parsed the book in the
 * browser with an inflate function that returned `null` for every deflated
 * entry — every chapter of every real book — and rendered a card telling the
 * user to open the file somewhere else. Reading now happens in the backend, so
 * what these pin is that the chapter, its images, its stylesheet, its contents
 * list and its links all arrive on screen.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const mockOpenUrl = vi.fn(() => Promise.resolve());
vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: (...args: unknown[]) => mockOpenUrl(...args),
}));

vi.mock('@monaco-editor/react', () => ({
  __esModule: true,
  default: () => <textarea data-testid="monaco-mock" readOnly value="" />,
}));

import { DocumentViewer } from '../DocumentViewer';

const CHAPTER_ONE = {
  path: 'OEBPS/text/ch1.xhtml',
  title: 'The First Chapter',
  html:
    '<h1>The First Chapter</h1><p>Text with <em>emphasis</em>.</p>' +
    '<img src="../images/fig1.png" alt="Figure 1"/>' +
    '<p><a href="ch2.xhtml#later">forward</a> and <a href="https://example.com">out</a></p>',
  css: 'body { line-height: 1.6 } p { text-indent: 1.2em }',
  resources: [
    {
      path: 'OEBPS/images/fig1.png',
      href: '../images/fig1.png',
      mime: 'image/png',
      base64: 'AAAA',
    },
  ],
  warnings: [],
};

const CHAPTER_TWO = {
  path: 'OEBPS/text/ch2.xhtml',
  title: 'The Second Chapter',
  html: '<h1>The Second Chapter</h1><p id="later">Landing point.</p>',
  css: '',
  resources: [],
  warnings: [],
};

const BOOK = {
  title: 'A Test Book',
  authors: ['Ada Lovelace'],
  language: 'en',
  publisher: null,
  chapters: [
    { path: 'OEBPS/text/ch1.xhtml', title: 'The First Chapter' },
    { path: 'OEBPS/text/ch2.xhtml', title: 'The Second Chapter' },
  ],
  toc: [
    { label: 'One', path: 'OEBPS/text/ch1.xhtml', fragment: null, level: 0 },
    { label: 'One, part two', path: 'OEBPS/text/ch1.xhtml', fragment: 'part2', level: 1 },
    { label: 'Two', path: 'OEBPS/text/ch2.xhtml', fragment: null, level: 0 },
  ],
  cover: { path: 'OEBPS/cover.jpg', href: 'OEBPS/cover.jpg', mime: 'image/jpeg', base64: 'QUJD' },
  warnings: [],
};

function respond(overrides: Record<string, unknown> = {}) {
  mockInvoke.mockImplementation((command: string, args: Record<string, unknown> = {}) => {
    if (command in overrides) {
      const value = overrides[command];
      return Promise.resolve(typeof value === 'function' ? (value as () => unknown)() : value);
    }
    if (command === 'read_epub_book') return Promise.resolve(BOOK);
    if (command === 'read_epub_chapter') {
      return Promise.resolve(args.chapter === CHAPTER_TWO.path ? CHAPTER_TWO : CHAPTER_ONE);
    }
    return Promise.reject(`unexpected command ${command}`);
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  mockOpenUrl.mockClear();
  Object.assign(URL, {
    createObjectURL: vi.fn(() => 'blob:vibecoder/fig1'),
    revokeObjectURL: vi.fn(),
  });
});

const renderBook = () => render(<DocumentViewer filePath="/books/test.epub" base64Data="" />);

describe('EpubViewer', () => {
  it('renders the chapter body rather than a placeholder', async () => {
    respond();
    renderBook();

    expect(await screen.findByText('The First Chapter', { selector: 'h1' })).toBeInTheDocument();
    expect(screen.getByText(/Text with/)).toBeInTheDocument();
    // The card the old viewer showed for every real book.
    expect(screen.queryByText(/dedicated e-book reader/)).not.toBeInTheDocument();
  });

  it('shows the images the chapter references', async () => {
    respond();
    renderBook();

    const figure = await screen.findByAltText('Figure 1');
    expect(figure).toHaveAttribute('src', 'blob:vibecoder/fig1');
  });

  it("applies the book's own stylesheet, scoped to the chapter", async () => {
    respond();
    const { container } = renderBook();

    await screen.findByText('The First Chapter', { selector: 'h1' });
    const style = container.querySelector('style');
    expect(style?.textContent).toContain('.epub-chapter-body p');
    expect(style?.textContent).toContain('text-indent: 1.2em');
    // `body` means the chapter container, not a <body> the chapter does not have.
    expect(style?.textContent).toContain('.epub-chapter-body { line-height: 1.6 }');
  });

  it('shows the cover and the contents list with its nesting', async () => {
    respond();
    renderBook();

    expect(await screen.findByAltText('Cover of A Test Book')).toHaveAttribute(
      'src',
      'data:image/jpeg;base64,QUJD',
    );
    expect(screen.getByText('A Test Book', { selector: '.epub-book-title' })).toBeInTheDocument();
    expect(screen.getByText('Ada Lovelace')).toBeInTheDocument();

    const nested = screen.getByTitle('One, part two');
    expect(nested).toHaveStyle({ paddingLeft: '20px' });
  });

  it('follows an internal link to another chapter', async () => {
    respond();
    renderBook();

    fireEvent.click(await screen.findByText('forward'));
    expect(await screen.findByText('Landing point.')).toBeInTheDocument();
    expect(screen.getByText('2 / 2')).toBeInTheDocument();
  });

  it('hands an external link to the browser instead of navigating the app', async () => {
    respond();
    renderBook();

    fireEvent.click(await screen.findByText('out'));
    expect(mockOpenUrl).toHaveBeenCalledWith('https://example.com');
    // Still on chapter one: the webview did not go anywhere.
    expect(screen.getByText('1 / 2')).toBeInTheDocument();
  });

  it('navigates from the contents list', async () => {
    respond();
    renderBook();

    fireEvent.click(await screen.findByTitle('Two'));
    expect(await screen.findByText('Landing point.')).toBeInTheDocument();
  });

  it('moves chapter by chapter from the toolbar', async () => {
    respond();
    renderBook();

    await screen.findByText('The First Chapter', { selector: 'h1' });
    expect(screen.getByTitle('Previous Chapter')).toBeDisabled();

    fireEvent.click(screen.getByTitle('Next Chapter'));
    expect(await screen.findByText('Landing point.')).toBeInTheDocument();
    expect(screen.getByTitle('Next Chapter')).toBeDisabled();
  });

  it('surfaces a chapter that will not load, keeping the book usable', async () => {
    respond({ read_epub_chapter: () => Promise.reject('OEBPS/text/ch1.xhtml is not in this book') });
    renderBook();

    await waitFor(() =>
      expect(screen.getByText(/is not in this book/)).toBeInTheDocument(),
    );
    // The contents list still renders, so another chapter can be tried.
    expect(screen.getByTitle('Two')).toBeInTheDocument();
  });

  it('surfaces a book that will not open', async () => {
    respond({ read_epub_book: () => Promise.reject("this EPUB's spine has no readable chapters") });
    renderBook();

    await waitFor(() =>
      expect(screen.getByText(/no readable chapters/)).toBeInTheDocument(),
    );
  });

  it('reports what the reader could not do', async () => {
    respond({
      read_epub_chapter: () => ({
        ...CHAPTER_ONE,
        warnings: [
          { code: 'epub.missing_resource', message: 'fig2.png is referenced but not in the book' },
        ],
      }),
    });
    renderBook();

    expect(
      await screen.findByText(/fig2.png is referenced but not in the book/),
    ).toBeInTheDocument();
  });

  it('does not read the file into the webview as base64', async () => {
    respond();
    renderBook();

    await screen.findByText('The First Chapter', { selector: 'h1' });
    const commands = mockInvoke.mock.calls.map(([command]) => command);
    expect(commands).not.toContain('read_file_base64');
  });
});
