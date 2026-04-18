/**
 * BDD tests for DesignImportPanel.
 *
 * Scenarios:
 *  1. Drop zone calls preventDefault on dragover (so drop is allowed)
 *  2. Dropping an image file invokes create_design_import with framework + Image source
 *  3. Drop zone is keyboard-focusable (tabIndex 0) and has aria-label
 *  4. Failed create_design_import surfaces a toast.error (not a swallowed catch)
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const toastError = vi.fn();
const toastSuccess = vi.fn();
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    toast: { error: toastError, success: toastSuccess, info: vi.fn(), warn: vi.fn() },
    dismiss: vi.fn(),
  }),
}));

import DesignImportPanel from '../DesignImportPanel';

beforeEach(() => {
  mockInvoke.mockReset();
  toastError.mockReset();
  toastSuccess.mockReset();
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === 'list_design_imports') return Promise.resolve([]);
    if (cmd === 'create_design_import') return Promise.resolve({
      id: 1, name: 'x', framework: 'React', source: 'Image', date: 'now', components: 0,
    });
    return Promise.resolve(null);
  });
});

function renderPanel() {
  return render(<DesignImportPanel />);
}

describe('DesignImportPanel — drop zone', () => {
  it('1. dragover preventDefault is called so drop is allowed', () => {
    renderPanel();
    const drop = screen.getByLabelText(/Drop zone for design files/i);

    const evt = new Event('dragover', { bubbles: true, cancelable: true });
    Object.defineProperty(evt, 'dataTransfer', { value: { files: [], types: ['Files'] } });
    drop.dispatchEvent(evt);
    expect(evt.defaultPrevented).toBe(true);
  });

  it('2. dropping an image file triggers create_design_import with Image source', async () => {
    renderPanel();
    const drop = screen.getByLabelText(/Drop zone for design files/i);
    const file = new File(['data'], 'screenshot.png', { type: 'image/png' });

    const dropEvt = new Event('drop', { bubbles: true, cancelable: true });
    Object.defineProperty(dropEvt, 'dataTransfer', { value: { files: [file], types: ['Files'] } });
    drop.dispatchEvent(dropEvt);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'create_design_import',
        expect.objectContaining({ source: 'Image', framework: 'React' }),
      );
    });
    expect(dropEvt.defaultPrevented).toBe(true);
  });

  it('3. drop zone is keyboard-focusable with aria-label', () => {
    renderPanel();
    const drop = screen.getByLabelText(/Drop zone for design files/i);
    expect(drop.getAttribute('tabIndex') ?? drop.getAttribute('tabindex')).toBe('0');
  });

  it('4. failed create_design_import surfaces toast.error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'list_design_imports') return Promise.resolve([]);
      if (cmd === 'create_design_import') return Promise.reject(new Error('nope'));
      return Promise.resolve(null);
    });

    renderPanel();
    const urlInput = screen.getByLabelText(/Figma URL input/i);
    fireEvent.change(urlInput, { target: { value: 'https://www.figma.com/file/xyz' } });
    fireEvent.click(screen.getByRole('button', { name: /Import design/i }));

    await waitFor(() => expect(toastError).toHaveBeenCalled());
  });
});
