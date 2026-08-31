/**
 * The two-page spread for the formats that are text rather than sheets.
 *
 * A DOCX or an EPUB chapter has no pages of its own, so a spread is two columns
 * of one screen — laid out by CSS, paged by scrolling the pane by its own
 * width. jsdom has no layout, so what is pinned here is the wiring: the toggle
 * puts the pane and its content into the state the stylesheet acts on, the
 * pager appears with it, and the choice survives leaving the tab and coming
 * back. The columns themselves were checked in a real browser.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('../../lib/pdfDocument', () => ({
  openPdf: vi.fn(async () => ({
    pageCount: 1,
    naturalSize: vi.fn(async () => ({ width: 612, height: 792 })),
    renderPage: vi.fn(async () => ({ width: 1, height: 1 })),
    close: vi.fn(),
  })),
}));

import { DocumentViewer } from '../DocumentViewer';
import { forgetDocument } from '../../lib/documentDrafts';

const DOCX = '/docs/report.docx';

const documentText = {
  format: 'docx',
  language: 'markdown',
  text: '# Title\n\nbody text\n',
  sections: 1,
  warnings: [] as Array<{ code: string; message: string }>,
  writable: true,
};

beforeEach(() => {
  forgetDocument(DOCX);
  mockInvoke.mockReset();
  mockInvoke.mockImplementation((command: string) => {
    if (command === 'read_document_text') return Promise.resolve(documentText);
    if (command === 'read_document_preview') return Promise.resolve(null);
    return Promise.resolve(null);
  });
});

/** The pane the stylesheet pages, and the flow inside it. */
function pane(container: HTMLElement) {
  return container.querySelector('.docx-page');
}

describe('reading spread', () => {
  it('opens one up, with no pager', async () => {
    const { container } = render(<DocumentViewer filePath={DOCX} base64Data="" />);
    await screen.findByText('body text');

    expect(pane(container)).not.toHaveClass('reading-paged');
    expect(container.querySelector('.reading-columns')).toBeNull();
    expect(screen.queryByLabelText('Next screen')).not.toBeInTheDocument();
  });

  it('lays the document out in two columns and offers a pager', async () => {
    const { container } = render(<DocumentViewer filePath={DOCX} base64Data="" />);
    await screen.findByText('body text');

    fireEvent.click(screen.getByRole('button', { name: /two up/i }));

    expect(pane(container)).toHaveClass('reading-paged');
    expect(container.querySelector('.reading-columns')).not.toBeNull();
    expect(screen.getByLabelText('Next screen')).toBeInTheDocument();
    expect(screen.getByText(/^Screen \d+ of \d+$/)).toBeInTheDocument();
  });

  it('goes back to one page, and says which state it is in', async () => {
    const { container } = render(<DocumentViewer filePath={DOCX} base64Data="" />);
    await screen.findByText('body text');

    const toggle = () => screen.getByRole('button', { name: /(two|one) up/i });
    expect(toggle()).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(toggle());
    expect(toggle()).toHaveAttribute('aria-pressed', 'true');

    fireEvent.click(toggle());
    expect(pane(container)).not.toHaveClass('reading-paged');
    expect(screen.queryByLabelText('Next screen')).not.toBeInTheDocument();
  });

  it('remembers the layout across a tab switch', async () => {
    const first = render(<DocumentViewer filePath={DOCX} base64Data="" />);
    await screen.findByText('body text');
    fireEvent.click(screen.getByRole('button', { name: /two up/i }));
    first.unmount();

    const { container } = render(<DocumentViewer filePath={DOCX} base64Data="" />);
    await screen.findByText('body text');
    await waitFor(() => expect(pane(container)).toHaveClass('reading-paged'));
  });

  it("does not carry one document's layout onto another", async () => {
    const first = render(<DocumentViewer filePath={DOCX} base64Data="" />);
    await screen.findByText('body text');
    fireEvent.click(screen.getByRole('button', { name: /two up/i }));
    first.unmount();

    const { container } = render(
      <DocumentViewer filePath="/docs/other.docx" base64Data="" />,
    );
    await screen.findByText('body text');
    expect(pane(container)).not.toHaveClass('reading-paged');
    forgetDocument('/docs/other.docx');
  });
});
