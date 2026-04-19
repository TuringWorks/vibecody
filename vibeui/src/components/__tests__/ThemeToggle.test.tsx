/**
 * BDD tests for ThemeToggle component.
 *
 * Scenarios:
 *  - Renders a button
 *  - Shows Moon icon when in dark mode
 *  - Shows Sun icon when in light mode
 *  - Respects stored mode from localStorage
 *  - Respects system preference when no stored mode
 *  - Clicking toggles to paired theme via applyThemeById
 *  - Falls back to dark/light defaults when no pair found
 *  - Aria label reflects current mode
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';

// ── Mocks ─────────────────────────────────────────────────────────────────────

// Mock SettingsPanel's getPairedTheme and applyThemeById
const mockApplyThemeById = vi.fn();
const mockGetPairedTheme = vi.fn();

vi.mock('../SettingsPanel', () => ({
  getPairedTheme: (...args: unknown[]) => mockGetPairedTheme(...args),
  applyThemeById: (...args: unknown[]) => mockApplyThemeById(...args),
}));

// Mock lucide-react — we only need Sun and Moon
vi.mock('lucide-react', () => ({
  Sun:  (props: Record<string, unknown>) => <span data-testid="sun-icon" {...props} />,
  Moon: (props: Record<string, unknown>) => <span data-testid="moon-icon" {...props} />,
}));

import { ThemeToggle } from '../ThemeToggle';

// ── Helpers ───────────────────────────────────────────────────────────────────

function mockStorage(mode: 'dark' | 'light' | null, id: string | null = null) {
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => {
      if (key === 'vibeui-theme') return mode;
      if (key === 'vibeui-theme-id') return id;
      return null;
    },
    setItem: vi.fn(),
    removeItem: vi.fn(),
  });
}

function mockMatchMedia(prefersDark: boolean) {
  vi.stubGlobal('window', {
    ...window,
    matchMedia: (query: string) => ({
      matches: query.includes('dark') ? prefersDark : !prefersDark,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  });
}

// ── beforeEach / afterEach ────────────────────────────────────────────────────

beforeEach(() => {
  mockApplyThemeById.mockReset();
  mockGetPairedTheme.mockReset();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

// ── Rendering ─────────────────────────────────────────────────────────────────

describe('ThemeToggle — rendering', () => {
  it('renders a button', () => {
    mockStorage('dark');
    render(<ThemeToggle />);
    expect(screen.getByRole('button')).toBeDefined();
  });

  it('shows Moon icon in dark mode', async () => {
    mockStorage('dark', 'dark-default');
    await act(async () => { render(<ThemeToggle />); });
    expect(screen.getByTestId('moon-icon')).toBeDefined();
    expect(screen.queryByTestId('sun-icon')).toBeNull();
  });

  it('shows Sun icon in light mode', async () => {
    mockStorage('light', 'light-default');
    await act(async () => { render(<ThemeToggle />); });
    expect(screen.getByTestId('sun-icon')).toBeDefined();
    expect(screen.queryByTestId('moon-icon')).toBeNull();
  });
});

// ── Aria label ────────────────────────────────────────────────────────────────

describe('ThemeToggle — aria label', () => {
  it('says "Switch to light mode" when in dark mode', async () => {
    mockStorage('dark', 'dark-default');
    await act(async () => { render(<ThemeToggle />); });
    expect(screen.getByRole('button').getAttribute('aria-label')).toMatch(/light/i);
  });

  it('says "Switch to dark mode" when in light mode', async () => {
    mockStorage('light', 'light-default');
    await act(async () => { render(<ThemeToggle />); });
    expect(screen.getByRole('button').getAttribute('aria-label')).toMatch(/dark/i);
  });
});

// ── System preference ─────────────────────────────────────────────────────────

describe('ThemeToggle — system preference', () => {
  it('uses system light preference when no stored theme', async () => {
    mockStorage(null, null);
    mockMatchMedia(false); // system prefers light
    await act(async () => { render(<ThemeToggle />); });
    expect(screen.getByTestId('sun-icon')).toBeDefined();
  });

  it('uses system dark preference when no stored theme', async () => {
    mockStorage(null, null);
    mockMatchMedia(true); // system prefers dark
    await act(async () => { render(<ThemeToggle />); });
    expect(screen.getByTestId('moon-icon')).toBeDefined();
  });
});

// ── Toggle behaviour ──────────────────────────────────────────────────────────

describe('ThemeToggle — toggle behaviour', () => {
  it('calls applyThemeById with the paired theme on click', async () => {
    mockStorage('dark', 'dark-default');
    mockGetPairedTheme.mockReturnValue({ id: 'light-default', mode: 'light' });

    await act(async () => { render(<ThemeToggle />); });
    fireEvent.click(screen.getByRole('button'));

    expect(mockApplyThemeById).toHaveBeenCalledWith('light-default');
  });

  it('toggles state to light after clicking from dark (with pair)', async () => {
    mockStorage('dark', 'dark-default');
    mockGetPairedTheme.mockReturnValue({ id: 'light-default', mode: 'light' });

    await act(async () => { render(<ThemeToggle />); });
    fireEvent.click(screen.getByRole('button'));

    expect(screen.getByTestId('sun-icon')).toBeDefined();
  });

  it('falls back to default dark theme when no paired theme is found from dark', async () => {
    mockStorage('dark', 'custom-dark');
    mockGetPairedTheme.mockReturnValue(undefined); // no pair

    await act(async () => { render(<ThemeToggle />); });
    fireEvent.click(screen.getByRole('button'));

    // Should fall back to light toggle using default id
    expect(mockApplyThemeById).toHaveBeenCalledWith(expect.stringContaining('light'));
  });

  it('falls back to default light theme when no paired theme is found from light', async () => {
    mockStorage('light', 'custom-light');
    mockGetPairedTheme.mockReturnValue(undefined);

    await act(async () => { render(<ThemeToggle />); });
    fireEvent.click(screen.getByRole('button'));

    expect(mockApplyThemeById).toHaveBeenCalledWith(expect.stringContaining('dark'));
  });

  it('calls applyThemeById on mount to restore the stored theme', async () => {
    mockStorage('dark', 'dark-ocean');
    await act(async () => { render(<ThemeToggle />); });
    // Should have applied the stored theme on mount
    expect(mockApplyThemeById).toHaveBeenCalledWith('dark-ocean');
  });
});
