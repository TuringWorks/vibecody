import { render, screen, waitFor, fireEvent, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('lucide-react', () => ({
  FolderOpen: () => <span />,
  AlertTriangle: () => <span />,
  X: () => <span />,
  ChevronDown: () => <span />,
}));

// The review body is a panel of its own; this test is about which tab shows it.
vi.mock('../ReviewPanel', () => ({
  ReviewPanel: () => <div data-testid="review-panel">review findings</div>,
}));

vi.mock('../Toaster', () => ({ Toaster: () => null }));
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    toast: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), success: vi.fn() },
    dismiss: vi.fn(),
  }),
}));

import { GitPanel } from '../GitPanel';

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'get_git_status':
        return Promise.resolve({ branch: 'main', file_statuses: { 'src/a.ts': 'M' } });
      case 'git_list_branches':
        return Promise.resolve(['main']);
      case 'get_github_sync_status':
        return Promise.resolve({ has_remote: false });
      default:
        return Promise.resolve(null);
    }
  });
});

async function renderPanel() {
  render(<GitPanel workspacePath="/repo" selectedProvider="ollama" />);
  await waitFor(() => expect(screen.getByRole('tab', { name: 'Changes' })).toBeInTheDocument());
}

const openTab = async (name: string) => {
  await act(async () => { fireEvent.click(screen.getByRole('tab', { name })); });
};

describe('GitPanel tabs', () => {
  it('offers Review, Changelog and Settings as tabs', async () => {
    await renderPanel();
    for (const name of ['Changes', 'Review', 'Changelog', 'Settings', 'GitHub']) {
      expect(screen.getByRole('tab', { name })).toBeInTheDocument();
    }
  });

  /**
   * The point of the change: these three used to be collapsible sections
   * stacked under the changes list, sharing its one scroll region. Each now
   * owns the panel, so none of them may render while Changes is showing.
   */
  it('does not render the three bodies inside the Changes view', async () => {
    await renderPanel();
    // The heading, not the tab — both say "Changes".
    expect(screen.getByRole('heading', { name: 'Changes' })).toBeInTheDocument();
    expect(screen.queryByTestId('review-panel')).toBeNull();
    expect(screen.queryByPlaceholderText(/since \(e\.g\. HEAD~10/)).toBeNull();
    expect(screen.queryByPlaceholderText('User name')).toBeNull();
  });

  it('shows the review on its own tab', async () => {
    await renderPanel();
    await openTab('Review');
    expect(screen.getByTestId('review-panel')).toBeInTheDocument();
    // …and the changes list is no longer competing for the height.
    expect(screen.queryByText('No changes')).toBeNull();
  });

  it('shows the changelog generator on its own tab', async () => {
    await renderPanel();
    await openTab('Changelog');
    expect(screen.getByPlaceholderText(/since \(e\.g\. HEAD~10/)).toBeInTheDocument();
    expect(screen.queryByTestId('review-panel')).toBeNull();
  });

  it('shows git settings on its own tab', async () => {
    await renderPanel();
    await openTab('Settings');
    expect(screen.getByPlaceholderText('User name')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Email')).toBeInTheDocument();
    expect(screen.queryByTestId('review-panel')).toBeNull();
  });

  it('marks exactly one tab selected at a time', async () => {
    await renderPanel();
    await openTab('Settings');
    const selected = screen.getAllByRole('tab').filter(
      (t) => t.getAttribute('aria-selected') === 'true',
    );
    expect(selected).toHaveLength(1);
    expect(selected[0]).toHaveTextContent('Settings');
  });

  /** Switching away and back must not lose what was typed. */
  it('keeps changelog input across a tab switch', async () => {
    await renderPanel();
    await openTab('Changelog');
    const input = screen.getByPlaceholderText(/since \(e\.g\. HEAD~10/) as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'v1.2.0' } });
    await openTab('Changes');
    await openTab('Changelog');
    expect((screen.getByPlaceholderText(/since \(e\.g\. HEAD~10/) as HTMLInputElement).value)
      .toBe('v1.2.0');
  });
});
