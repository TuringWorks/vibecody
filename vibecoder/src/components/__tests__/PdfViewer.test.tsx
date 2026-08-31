/**
 * The PDF viewer's paging and its two-page spread.
 *
 * The renderer is mocked: what is worth pinning here is which pages the viewer
 * asks for and what it claims to be showing, not whether a canvas got painted —
 * jsdom has no canvas, and a screenshot would not catch a viewer that says
 * "Pages 3–4" while drawing 1 and 2.
 */
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const renderPage = vi.fn(async () => ({ width: 400, height: 560 }));
const naturalSize = vi.fn(async () => ({ width: 612, height: 792 }));
const close = vi.fn();
const openPdf = vi.fn(async () => ({ pageCount: 5, naturalSize, renderPage, close }));

vi.mock('../../lib/pdfDocument', () => ({
  openPdf: (...args: unknown[]) => openPdf(...(args as [string])),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { DocumentViewer } from '../DocumentViewer';
import { forgetDocument } from '../../lib/documentDrafts';

const PATH = '/docs/paper.pdf';

function open() {
  return render(<DocumentViewer filePath={PATH} base64Data="cGRm" />);
}

/** Which pages the renderer was asked for, in order. */
function requested(): number[] {
  return renderPage.mock.calls.map((call) => (call as unknown as [number])[0]);
}

beforeEach(() => {
  forgetDocument(PATH);
  openPdf.mockClear();
  renderPage.mockClear();
  naturalSize.mockClear();
  close.mockClear();
});

describe('PdfViewer', () => {
  it('opens on one page and says which', async () => {
    open();
    expect(await screen.findByText('Page 1 of 5')).toBeInTheDocument();
    await waitFor(() => expect(requested()).toEqual([1]));
    expect(screen.getByRole('button', { name: 'Previous page' })).toBeDisabled();
  });

  it('shows two pages side by side, and draws both', async () => {
    open();
    await screen.findByText('Page 1 of 5');
    fireEvent.click(screen.getByRole('button', { name: /two up/i }));

    expect(await screen.findByText('Pages 1–2 of 5')).toBeInTheDocument();
    await waitFor(() => expect(requested()).toEqual([1, 1, 2]));
    expect(screen.getByLabelText('Page 1')).toBeInTheDocument();
    expect(screen.getByLabelText('Page 2')).toBeInTheDocument();
  });

  it('turns a whole spread at a time', async () => {
    open();
    await screen.findByText('Page 1 of 5');
    fireEvent.click(screen.getByRole('button', { name: /two up/i }));
    await screen.findByText('Pages 1–2 of 5');

    fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
    expect(await screen.findByText('Pages 3–4 of 5')).toBeInTheDocument();
  });

  it('does not offer a page the document does not have', async () => {
    open();
    await screen.findByText('Page 1 of 5');
    fireEvent.click(screen.getByRole('button', { name: /two up/i }));
    fireEvent.click(await screen.findByRole('button', { name: 'Next page' }));
    await screen.findByText('Pages 3–4 of 5');
    fireEvent.click(screen.getByRole('button', { name: 'Next page' }));

    // Five pages: the last spread is page five on its own.
    expect(await screen.findByText('Page 5 of 5')).toBeInTheDocument();
    expect(screen.queryByLabelText('Page 6')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
  });

  it('keeps you on the page you were reading when the layout changes', async () => {
    open();
    await screen.findByText('Page 1 of 5');
    fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
    fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
    await screen.findByText('Page 3 of 5');

    fireEvent.click(screen.getByRole('button', { name: /two up/i }));
    expect(await screen.findByText('Pages 3–4 of 5')).toBeInTheDocument();
  });

  it('pages with the arrow keys', async () => {
    open();
    await screen.findByText('Page 1 of 5');
    fireEvent.keyDown(window, { key: 'ArrowRight' });
    expect(await screen.findByText('Page 2 of 5')).toBeInTheDocument();
    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    expect(await screen.findByText('Page 1 of 5')).toBeInTheDocument();
  });

  it('remembers the spread across a tab switch', async () => {
    const first = open();
    await screen.findByText('Page 1 of 5');
    fireEvent.click(screen.getByRole('button', { name: /two up/i }));
    await screen.findByText('Pages 1–2 of 5');
    first.unmount();

    open();
    expect(await screen.findByText('Pages 1–2 of 5')).toBeInTheDocument();
  });

  it('reports a document it could not open instead of an empty pane', async () => {
    openPdf.mockRejectedValueOnce('this PDF is encrypted');
    open();
    expect(await screen.findByText('this PDF is encrypted')).toBeInTheDocument();
  });

  it('lets go of the document when the tab does', async () => {
    const view = open();
    await screen.findByText('Page 1 of 5');
    view.unmount();
    await waitFor(() => expect(close).toHaveBeenCalled());
  });
});
