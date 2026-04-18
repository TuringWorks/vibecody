/**
 * BDD tests for DesignHubPanel — the unified design hub.
 *
 * Scenarios:
 *  1. Figma token loads from ProfileStore on mount, NOT from localStorage
 *  2. Saving a Figma token writes via profile_api_key_set, NOT localStorage
 *  3. Unchecking "Remember token" deletes the saved token via profile_api_key_delete
 *  4. Failed token import surfaces an error toast (not a swallowed catch)
 *  5. Failed token load surfaces an error toast (not a swallowed catch)
 *  6. Filter input narrows the displayed token list (no 50-item cap)
 *  7. Provider toggle is rendered as a <button>, not a <div onClick>
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Mocks ──────────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const toastError = vi.fn();
const toastSuccess = vi.fn();
const toastInfo = vi.fn();
const toastWarn = vi.fn();
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    toast: { error: toastError, success: toastSuccess, info: toastInfo, warn: toastWarn },
    dismiss: vi.fn(),
  }),
}));

vi.mock('../Icon', () => ({
  Icon: ({ name }: { name: string }) => <span data-testid={`icon-${name}`} />,
}));

import { DesignHubPanel } from '../DesignHubPanel';

beforeEach(() => {
  mockInvoke.mockReset();
  toastError.mockReset();
  toastSuccess.mockReset();
  toastInfo.mockReset();
  toastWarn.mockReset();
  // Pollute localStorage to make sure the panel does NOT read from it.
  localStorage.setItem('figma_token', 'TAINTED-LOCALSTORAGE-VALUE');
});

afterEach(() => {
  localStorage.removeItem('figma_token');
});

function renderPanel() {
  return render(<DesignHubPanel workspacePath="/ws" provider="anthropic" />);
}

// ── Helpers ────────────────────────────────────────────────────────────────

function defaultInvokeImpl(cmd: string) {
  switch (cmd) {
    case 'profile_api_key_get':
      return Promise.resolve(null);
    case 'profile_api_key_set':
    case 'profile_api_key_delete':
      return Promise.resolve(null);
    case 'load_design_system_tokens':
      return Promise.resolve({ tokens: [] });
    default:
      return Promise.resolve(null);
  }
}

// ── Scenarios ──────────────────────────────────────────────────────────────

describe('DesignHubPanel — Figma token security', () => {
  it('1. loads Figma token from ProfileStore on mount, not localStorage', async () => {
    mockInvoke.mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === 'profile_api_key_get') {
        const a = args as { profile_id: string; provider: string };
        if (a.provider === 'figma') return Promise.resolve('PROFILE-STORE-TOKEN');
      }
      return defaultInvokeImpl(cmd);
    });

    renderPanel();
    fireEvent.click(screen.getByRole('tab', { name: /^Figma$/ }));

    await waitFor(() => {
      const tokenInput = screen.getByPlaceholderText('figd_…') as HTMLInputElement;
      expect(tokenInput.value).toBe('PROFILE-STORE-TOKEN');
      expect(tokenInput.value).not.toBe('TAINTED-LOCALSTORAGE-VALUE');
    });

    expect(mockInvoke).toHaveBeenCalledWith(
      'profile_api_key_get',
      expect.objectContaining({ profile_id: 'default', provider: 'figma' }),
    );
  });

  it('2. saves Figma token via profile_api_key_set, not localStorage', async () => {
    mockInvoke.mockImplementation(defaultInvokeImpl);

    renderPanel();
    fireEvent.click(screen.getByRole('tab', { name: /^Figma$/ }));

    const urlInput = await screen.findByPlaceholderText('https://www.figma.com/file/…');
    const tokenInput = screen.getByPlaceholderText('figd_…');
    fireEvent.change(urlInput, { target: { value: 'https://www.figma.com/file/abc' } });
    fireEvent.change(tokenInput, { target: { value: 'NEW-TOKEN' } });
    // User opts in to remembering the token — should persist via ProfileStore.
    fireEvent.click(screen.getByLabelText(/Remember token/i));

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'import_figma') return Promise.resolve([]);
      return defaultInvokeImpl(cmd);
    });

    fireEvent.click(screen.getByRole('button', { name: /Import & Generate/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'profile_api_key_set',
        expect.objectContaining({ profile_id: 'default', provider: 'figma', api_key: 'NEW-TOKEN' }),
      );
    });
    // localStorage must remain untouched (the original tainted value, never overwritten).
    expect(localStorage.getItem('figma_token')).toBe('TAINTED-LOCALSTORAGE-VALUE');
  });

  it('3. unchecking "Remember token" deletes via profile_api_key_delete', async () => {
    mockInvoke.mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === 'profile_api_key_get') {
        const a = args as { provider: string };
        if (a.provider === 'figma') return Promise.resolve('EXISTING-TOKEN');
      }
      return defaultInvokeImpl(cmd);
    });

    renderPanel();
    fireEvent.click(screen.getByRole('tab', { name: /^Figma$/ }));

    const remember = await screen.findByLabelText(/Remember token/i) as HTMLInputElement;
    await waitFor(() => expect(remember.checked).toBe(true));

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'import_figma') return Promise.resolve([]);
      return defaultInvokeImpl(cmd);
    });

    fireEvent.click(remember);
    const urlInput = screen.getByPlaceholderText('https://www.figma.com/file/…');
    fireEvent.change(urlInput, { target: { value: 'https://www.figma.com/file/abc' } });
    fireEvent.click(screen.getByRole('button', { name: /Import & Generate/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'profile_api_key_delete',
        expect.objectContaining({ profile_id: 'default', provider: 'figma' }),
      );
    });
  });
});

describe('DesignHubPanel — error surfaces', () => {
  it('4. failed Figma import surfaces toast.error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'import_figma') return Promise.reject(new Error('boom'));
      return defaultInvokeImpl(cmd);
    });

    renderPanel();
    fireEvent.click(screen.getByRole('tab', { name: /^Figma$/ }));
    const urlInput = await screen.findByPlaceholderText('https://www.figma.com/file/…');
    const tokenInput = screen.getByPlaceholderText('figd_…');
    fireEvent.change(urlInput, { target: { value: 'https://www.figma.com/file/abc' } });
    fireEvent.change(tokenInput, { target: { value: 'tok' } });

    fireEvent.click(screen.getByRole('button', { name: /Import & Generate/i }));

    await waitFor(() => expect(toastError).toHaveBeenCalled());
    const msg = toastError.mock.calls[0][0] as string;
    expect(msg.toLowerCase()).toMatch(/figma|import|boom/);
  });

  it('5. failed token load surfaces toast.error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'load_design_system_tokens') return Promise.reject(new Error('cant load'));
      return defaultInvokeImpl(cmd);
    });

    renderPanel();
    fireEvent.click(screen.getByRole('button', { name: /Load Design Tokens/i }));

    await waitFor(() => expect(toastError).toHaveBeenCalled());
  });
});

describe('DesignHubPanel — UX and a11y', () => {
  it('6. filter input narrows the token list (no 50-item cap)', async () => {
    const tokens = Array.from({ length: 75 }, (_, i) => ({
      name: i % 5 === 0 ? `accent-${i}` : `gray-${i}`,
      token_type: 'color',
      value: `#${(i * 17).toString(16).padStart(6, '0')}`,
      provider: 'inhouse',
    }));
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'load_design_system_tokens') return Promise.resolve({ tokens });
      return defaultInvokeImpl(cmd);
    });

    renderPanel();
    fireEvent.click(screen.getByRole('button', { name: /Load Design Tokens/i }));
    await waitFor(() => expect(screen.queryByText(/75 token/)).not.toBeNull());

    fireEvent.click(screen.getByRole('tab', { name: /^Tokens$/ }));

    // Without filtering, all 75 tokens render — and the legacy "+25 more" footer should be gone.
    await waitFor(() => {
      expect(screen.queryByText(/and \d+ more/i)).toBeNull();
    });

    // Filter narrows to the 15 tokens whose name starts with "accent-".
    const filter = screen.getByPlaceholderText(/Filter tokens/i);
    fireEvent.change(filter, { target: { value: 'accent-' } });

    await waitFor(() => {
      const matches = screen.getAllByText(/^accent-/);
      expect(matches.length).toBe(15);
      expect(screen.queryByText(/^gray-/)).toBeNull();
    });
  });

  it('7. provider toggle is rendered as a button (not div onClick)', async () => {
    mockInvoke.mockImplementation(defaultInvokeImpl);
    renderPanel();
    const penpotToggle = await screen.findByRole('button', { name: /Penpot/i });
    expect(penpotToggle.tagName).toBe('BUTTON');
    expect(penpotToggle.getAttribute('aria-pressed')).toMatch(/true|false/);
  });

  it('8. switching tabs persists active tab via panel_settings_set', async () => {
    mockInvoke.mockImplementation(defaultInvokeImpl);
    renderPanel();
    fireEvent.click(screen.getByRole('tab', { name: /^Tokens$/ }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'panel_settings_set',
        expect.objectContaining({ panel: 'design-hub', key: 'activeTab', value: 'tokens' }),
      );
    });
  });

  it('9. toggling a provider persists activeProviders via panel_settings_set', async () => {
    mockInvoke.mockImplementation(defaultInvokeImpl);
    renderPanel();
    const penpotToggle = await screen.findByRole('button', { name: /Penpot/i });
    fireEvent.click(penpotToggle);

    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter(([cmd, args]) =>
        cmd === 'panel_settings_set' &&
        (args as { key?: string }).key === 'activeProviders',
      );
      expect(calls.length).toBeGreaterThan(0);
      const lastArgs = calls[calls.length - 1][1] as { value: string[] };
      expect(lastArgs.value).toContain('penpot');
      expect(lastArgs.value).toContain('inhouse');
    });
  });

  it('10. hydrates active tab + providers from panel_settings_get_all on mount', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'panel_settings_get_all') {
        return Promise.resolve({ activeTab: 'audit', activeProviders: ['figma', 'inhouse'] });
      }
      if (cmd === 'panel_settings_get_default_profile') return Promise.resolve('default');
      return defaultInvokeImpl(cmd);
    });

    renderPanel();
    // The Audit tab should be active after hydration.
    await waitFor(() => {
      const auditTab = screen.getByRole('tab', { name: /^Audit$/ });
      expect(auditTab.getAttribute('aria-selected')).toBe('true');
    });
    // Switch back to Providers tab and confirm Figma is hydrated as enabled.
    fireEvent.click(screen.getByRole('tab', { name: /^Providers$/ }));
    await waitFor(() => {
      const figmaProv = screen.getByRole('button', { name: /^Figma provider/i });
      expect(figmaProv.getAttribute('aria-pressed')).toBe('true');
    });
  });
});
