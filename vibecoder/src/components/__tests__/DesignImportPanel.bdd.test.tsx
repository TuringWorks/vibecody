/**
 * BDD tests for DesignImportPanel.
 *
 * The panel's contract is that it never reports an import it did not perform:
 *  1. Drop zone allows a drop (preventDefault on dragover) and is keyboard-reachable
 *  2. Dropping an image loads it — it does NOT record an import by itself
 *  3. Generating without a provider selected refuses instead of calling the model
 *  4. A failing generator surfaces the error rather than an empty success
 *  5. Figma import without a saved token says so instead of calling the backend
 *  6. History that cannot be read is reported as unreadable, not as empty
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const toastError = vi.fn();
const toastSuccess = vi.fn();
const toastWarn = vi.fn();
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    toast: { error: toastError, success: toastSuccess, info: vi.fn(), warn: toastWarn },
    dismiss: vi.fn(),
  }),
}));

const mockLoadFigmaToken = vi.fn();
vi.mock('../../lib/figmaToken', () => ({
  loadFigmaToken: () => mockLoadFigmaToken(),
  saveFigmaToken: vi.fn(),
  deleteFigmaToken: vi.fn(),
}));

import DesignImportPanel from '../DesignImportPanel';

beforeEach(() => {
  mockInvoke.mockReset();
  toastError.mockReset();
  toastSuccess.mockReset();
  toastWarn.mockReset();
  mockLoadFigmaToken.mockReset();
  mockLoadFigmaToken.mockResolvedValue('figd_test');
  mockInvoke.mockImplementation((cmd: string) => {
    if (cmd === 'design_import_history') return Promise.resolve([]);
    return Promise.resolve(null);
  });
});

function renderPanel(provider = 'Claude (claude-opus-5)') {
  return render(<DesignImportPanel workspacePath="/tmp/ws" provider={provider} />);
}

describe('DesignImportPanel — drop zone', () => {
  it('1. dragover preventDefault is called so drop is allowed, and the zone is focusable', () => {
    renderPanel();
    const drop = screen.getByLabelText(/Drop zone for design files/i);
    expect(drop.getAttribute('tabIndex') ?? drop.getAttribute('tabindex')).toBe('0');

    const evt = new Event('dragover', { bubbles: true, cancelable: true });
    Object.defineProperty(evt, 'dataTransfer', { value: { files: [], types: ['Files'] } });
    drop.dispatchEvent(evt);
    expect(evt.defaultPrevented).toBe(true);
  });

  it('2. dropping an image loads it and records nothing until a generator runs', async () => {
    renderPanel();
    const drop = screen.getByLabelText(/Drop zone for design files/i);
    const file = new File(['data'], 'screenshot.png', { type: 'image/png' });

    const dropEvt = new Event('drop', { bubbles: true, cancelable: true });
    Object.defineProperty(dropEvt, 'dataTransfer', { value: { files: [file], types: ['Files'] } });
    drop.dispatchEvent(dropEvt);

    expect(dropEvt.defaultPrevented).toBe(true);
    await waitFor(() => expect(screen.getByText('screenshot.png')).toBeTruthy());
    expect(mockInvoke).not.toHaveBeenCalledWith('design_import_record', expect.anything());
  });
});

describe('DesignImportPanel — generation', () => {
  it('3. with no provider selected it refuses instead of calling the generator', async () => {
    renderPanel('');
    const drop = screen.getByLabelText(/Drop zone for design files/i);
    const file = new File(['data'], 'shot.png', { type: 'image/png' });
    const dropEvt = new Event('drop', { bubbles: true, cancelable: true });
    Object.defineProperty(dropEvt, 'dataTransfer', { value: { files: [file], types: ['Files'] } });
    drop.dispatchEvent(dropEvt);

    await waitFor(() => expect(screen.getByText('shot.png')).toBeTruthy());
    const button = screen.getByLabelText(/Generate from image/i) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(mockInvoke).not.toHaveBeenCalledWith('generate_app_from_image', expect.anything());
  });

  it('4. a failing generator shows the error rather than an empty success', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'design_import_history') return Promise.resolve([]);
      if (cmd === 'import_figma') return Promise.reject(new Error('Figma API error: 403'));
      return Promise.resolve(null);
    });
    renderPanel();
    fireEvent.change(screen.getByLabelText(/Figma URL input/i), {
      target: { value: 'https://www.figma.com/file/xyz' },
    });
    fireEvent.click(screen.getByLabelText(/Import design/i));

    await waitFor(() => expect(screen.getByRole('alert').textContent).toMatch(/403/));
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it('5. a Figma import with no saved token says so and does not call the backend', async () => {
    mockLoadFigmaToken.mockResolvedValue('');
    renderPanel();
    fireEvent.change(screen.getByLabelText(/Figma URL input/i), {
      target: { value: 'https://www.figma.com/file/xyz' },
    });
    fireEvent.click(screen.getByLabelText(/Import design/i));

    await waitFor(() => expect(screen.getByRole('alert').textContent).toMatch(/No Figma token saved/i));
    expect(mockInvoke).not.toHaveBeenCalledWith('import_figma', expect.anything());
  });
});

describe('DesignImportPanel — history', () => {
  it('6. an unreadable history is reported as unreadable, not as empty', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'design_import_history') return Promise.reject(new Error('permission denied'));
      return Promise.resolve(null);
    });
    renderPanel();
    fireEvent.click(screen.getByRole('tab', { name: /History/i }));

    await waitFor(() =>
      expect(screen.getByRole('alert').textContent).toMatch(/Could not read import history/i),
    );
    expect(screen.queryByText(/No imports yet/i)).toBeNull();
  });
});
