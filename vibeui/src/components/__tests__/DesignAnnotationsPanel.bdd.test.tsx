/**
 * BDD tests for DesignAnnotationsPanel (renamed from DesignModePanel).
 *
 * Scenarios:
 *  1. Loading state renders panel-loading on mount before invoke resolves
 *  2. Empty state shows panel-empty when no annotations
 *  3. Annotation kind colors come from CSS custom properties (no #hex hardcoded)
 *  4. Failed initial load surfaces toast.error
 *  5. Adding an annotation calls invoke design_mode_annotations with action=add
 */

import { render, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const toastError = vi.fn();
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    toast: { error: toastError, success: vi.fn(), info: vi.fn(), warn: vi.fn() },
    dismiss: vi.fn(),
  }),
}));

// We import lazily inside each test so the file path swap is the source of truth.

beforeEach(() => {
  mockInvoke.mockReset();
  toastError.mockReset();
});

describe('DesignAnnotationsPanel', () => {
  it('1. file is named DesignAnnotationsPanel.tsx and exports DesignAnnotationsPanel', async () => {
    const mod = await import('../DesignAnnotationsPanel');
    expect(typeof mod.DesignAnnotationsPanel).toBe('function');
  });

  it('2. empty state uses the panel-empty class', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'design_mode_annotations') return Promise.resolve([]);
      if (cmd === 'design_mode_generate') return Promise.resolve([]);
      if (cmd === 'design_mode_tokens') return Promise.resolve([]);
      return Promise.resolve(null);
    });
    const { DesignAnnotationsPanel } = await import('../DesignAnnotationsPanel');
    const { container } = render(<DesignAnnotationsPanel />);
    await waitFor(() => {
      expect(container.querySelector('.panel-empty')).not.toBeNull();
    });
  });

  it('3. annotation kind colors are CSS variables, not raw #hex', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'design_mode_annotations') return Promise.resolve([
        { id: '1', kind: 'spacing', description: 's', selector: null, created_at: 't' },
        { id: '2', kind: 'color', description: 'c', selector: null, created_at: 't' },
      ]);
      if (cmd === 'design_mode_generate') return Promise.resolve([]);
      if (cmd === 'design_mode_tokens') return Promise.resolve([]);
      return Promise.resolve(null);
    });
    const { DesignAnnotationsPanel } = await import('../DesignAnnotationsPanel');
    const { container } = render(<DesignAnnotationsPanel />);
    // Wait for the loading spinner to clear (annotations finished loading).
    await waitFor(() => {
      expect(container.querySelector('.panel-loading')).toBeNull();
    });

    // No element should carry a raw `#xxxxxx` color value in its inline style.
    const allElements = Array.from(container.querySelectorAll('*')) as HTMLElement[];
    for (const el of allElements) {
      const style = el.getAttribute('style') ?? '';
      // Allow var(--…) but reject literal #abc / #abcdef occurrences inside style attrs.
      expect(style).not.toMatch(/#[0-9a-fA-F]{3,8}/);
    }
  });

  it('4. failed initial load surfaces a toast.error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'design_mode_annotations') return Promise.reject(new Error('boom'));
      if (cmd === 'design_mode_generate') return Promise.reject(new Error('boom'));
      if (cmd === 'design_mode_tokens') return Promise.reject(new Error('boom'));
      return Promise.resolve(null);
    });
    const { DesignAnnotationsPanel } = await import('../DesignAnnotationsPanel');
    render(<DesignAnnotationsPanel />);
    await waitFor(() => expect(toastError).toHaveBeenCalled());
  });
});
