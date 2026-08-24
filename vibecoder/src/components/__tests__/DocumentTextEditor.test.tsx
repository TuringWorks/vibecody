/**
 * DocumentTextEditor / DocumentViewer — the honesty of the save path.
 *
 * The panel's job is to never let a document look saved when it is not. These
 * pin the three ways that could go wrong: a failed write reported as success,
 * a warning from the backend swallowed, and the buffer's dirty state surviving
 * a save that errored.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// A textarea stands in for Monaco so the buffer can be driven with fireEvent.
vi.mock('@monaco-editor/react', () => ({
  __esModule: true,
  default: ({
    value,
    onChange,
    language,
    options,
  }: {
    value: string;
    onChange: (next: string | undefined) => void;
    language?: string;
    options?: { readOnly?: boolean };
  }) => (
    <textarea
      data-testid="monaco-mock"
      data-language={language ?? ''}
      readOnly={options?.readOnly ?? false}
      value={value}
      onChange={(e) => onChange(e.target.value)}
    />
  ),
}));

import { DocumentTextEditor } from '../DocumentTextEditor';
import { DocumentViewer, isDocumentFile, needsRawBytes } from '../DocumentViewer';
import { clearDraft, hasDraft } from '../../lib/documentDrafts';

const docxText = {
  format: 'docx',
  language: 'markdown',
  text: '# Title\n\nbody\n',
  sections: 1,
  warnings: [] as Array<{ code: string; message: string }>,
  writable: true,
};

const writeOk = {
  format: 'docx',
  bytes_written: 4096,
  backup: null,
  warnings: [] as Array<{ code: string; message: string }>,
  verified: true,
};

/** Route each command to a canned response. */
function respondWith(handlers: Record<string, unknown | (() => unknown)>) {
  mockInvoke.mockImplementation((command: string) => {
    const handler = handlers[command];
    if (handler === undefined) return Promise.reject(`unexpected command ${command}`);
    const value = typeof handler === 'function' ? (handler as () => unknown)() : handler;
    return value instanceof Promise ? value : Promise.resolve(value);
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  // The draft store is module state shared by every test in this file.
  clearDraft('/d/report.docx');
  clearDraft('/d/memo.pages');
});

describe('file routing', () => {
  it('claims the four document formats and only reads bytes for the two that need them', () => {
    expect(isDocumentFile('a.pdf')).toBe(true);
    expect(isDocumentFile('a.epub')).toBe(true);
    expect(isDocumentFile('a.docx')).toBe(true);
    expect(isDocumentFile('a.pages')).toBe(true);
    expect(isDocumentFile('a.txt')).toBe(false);

    expect(needsRawBytes('a.pdf')).toBe(true);
    expect(needsRawBytes('a.epub')).toBe(true);
    // DOCX and Pages are parsed by the backend; base64-ing them would move the
    // whole file through a JS string for nothing.
    expect(needsRawBytes('a.docx')).toBe(false);
    expect(needsRawBytes('a.pages')).toBe(false);
  });
});

describe('DocumentTextEditor', () => {
  it('loads the buffer with the language the backend chose', async () => {
    respondWith({ read_document_text: { ...docxText, format: 'pages', language: 'plaintext' } });
    render(<DocumentTextEditor filePath="/d/memo.pages" format="pages" onClose={vi.fn()} />);

    const editor = await screen.findByTestId('monaco-mock');
    expect(editor).toHaveAttribute('data-language', 'plaintext');
  });

  it('reports a save only after the backend verified it', async () => {
    respondWith({ read_document_text: docxText, write_document_text: writeOk });
    render(<DocumentTextEditor filePath="/d/report.docx" format="docx" onClose={vi.fn()} />);

    const editor = await screen.findByTestId('monaco-mock');
    fireEvent.change(editor, { target: { value: '# Changed\n' } });
    expect(screen.getByText('unsaved')).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /save/i }));
    });

    await waitFor(() => expect(screen.getByText(/Saved —/)).toBeInTheDocument());
    expect(screen.getByText(/4,096 bytes, verified/)).toBeInTheDocument();
    expect(screen.queryByText('unsaved')).not.toBeInTheDocument();
  });

  it('shows a refused write as an error and keeps the buffer dirty', async () => {
    respondWith({
      read_document_text: docxText,
      write_document_text: () =>
        Promise.reject(
          'write verification failed: the rewritten document does not read back as the text you saved. Your file has not been changed.',
        ),
    });
    render(<DocumentTextEditor filePath="/d/report.docx" format="docx" onClose={vi.fn()} />);

    const editor = await screen.findByTestId('monaco-mock');
    fireEvent.change(editor, { target: { value: 'broken\n' } });
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /save/i }));
    });

    await waitFor(() =>
      expect(screen.getByText(/has not been changed/)).toBeInTheDocument(),
    );
    expect(screen.queryByText(/Saved —/)).not.toBeInTheDocument();
    // Still dirty: the edit is unsaved, and the tab must keep saying so.
    expect(screen.getByText('unsaved')).toBeInTheDocument();
  });

  it('shows every limitation the backend reported', async () => {
    respondWith({
      read_document_text: {
        ...docxText,
        warnings: [
          { code: 'pages.text_only', message: 'layout is not shown here' },
          { code: 'pages.text_only', message: 'layout is not shown here' },
        ],
      },
    });
    render(<DocumentTextEditor filePath="/d/memo.pages" format="pages" onClose={vi.fn()} />);

    await waitFor(() => expect(screen.getByText('layout is not shown here')).toBeInTheDocument());
    // Repeated codes collapse to one line rather than repeating.
    expect(screen.getAllByText('layout is not shown here')).toHaveLength(1);
  });

  it('cannot be edited or saved when the backend says it is not writable', async () => {
    respondWith({ read_document_text: { ...docxText, writable: false } });
    render(<DocumentTextEditor filePath="/d/report.docx" format="docx" onClose={vi.fn()} />);

    const editor = await screen.findByTestId('monaco-mock');
    expect(editor).toHaveAttribute('readonly');
    expect(screen.getByRole('button', { name: /save/i })).toBeDisabled();
  });

  it('surfaces a read failure instead of an empty editor', async () => {
    respondWith({ read_document_text: () => Promise.reject('parse error: word/document.xml is missing') });
    render(<DocumentTextEditor filePath="/d/broken.docx" format="docx" onClose={vi.fn()} />);

    await waitFor(() =>
      expect(screen.getByText(/word\/document.xml is missing/)).toBeInTheDocument(),
    );
  });
});

describe('unsaved buffers', () => {
  it('survive the tab switch that unmounts the editor', async () => {
    respondWith({ read_document_text: docxText });
    const first = render(
      <DocumentTextEditor filePath="/d/report.docx" format="docx" onClose={vi.fn()} />,
    );

    fireEvent.change(await screen.findByTestId('monaco-mock'), {
      target: { value: '# Half-finished edit\n' },
    });
    expect(hasDraft('/d/report.docx')).toBe(true);

    // Another tab is clicked: the editor unmounts with the edit unsaved.
    first.unmount();

    render(<DocumentTextEditor filePath="/d/report.docx" format="docx" onClose={vi.fn()} />);
    const reopened = await screen.findByTestId('monaco-mock');
    await waitFor(() => expect(reopened).toHaveValue('# Half-finished edit\n'));
    expect(screen.getByText('unsaved')).toBeInTheDocument();
  });

  it('are forgotten once the document has been written', async () => {
    respondWith({ read_document_text: docxText, write_document_text: writeOk });
    render(<DocumentTextEditor filePath="/d/report.docx" format="docx" onClose={vi.fn()} />);

    fireEvent.change(await screen.findByTestId('monaco-mock'), {
      target: { value: '# Saved edit\n' },
    });
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /save/i }));
    });

    await waitFor(() => expect(hasDraft('/d/report.docx')).toBe(false));
  });
});

describe('DocumentViewer', () => {
  it('renders a DOCX and switches to the text editor on request', async () => {
    respondWith({ read_document_text: docxText });
    render(<DocumentViewer filePath="/d/report.docx" base64Data="" />);

    await waitFor(() => expect(screen.getByText('DOCX')).toBeInTheDocument());
    expect(screen.queryByTestId('monaco-mock')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /edit text/i }));
    expect(await screen.findByTestId('monaco-mock')).toBeInTheDocument();
  });

  it('falls back to the recovered text when a Pages file embeds no preview', async () => {
    respondWith({
      read_document_text: {
        ...docxText,
        format: 'pages',
        language: 'plaintext',
        text: 'first line\nsecond line\n',
      },
      read_document_preview: null,
    });
    render(<DocumentViewer filePath="/d/memo.pages" base64Data="" />);

    await waitFor(() => expect(screen.getByText('first line')).toBeInTheDocument());
    expect(screen.getByRole('button', { name: 'Preview' })).toBeDisabled();
  });
});
