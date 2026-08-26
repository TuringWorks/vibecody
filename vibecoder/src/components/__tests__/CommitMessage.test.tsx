import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => mockInvoke(...a) }));
vi.mock('lucide-react', () => ({ FolderOpen: () => <span />, AlertTriangle: () => <span />, X: () => <span /> }));
vi.mock('../ReviewPanel', () => ({
  ReviewPanel: () => <div />,
  ReviewControls: () => <div />,
  useCodeReview: () => ({
    baseRef: '', setBaseRef: vi.fn(), isLoading: false,
    report: null, error: null, runId: 0, runReview: vi.fn(),
  }),
}));
vi.mock('../Toaster', () => ({ Toaster: () => null }));
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({ toasts: [], toast: { error: vi.fn(), success: vi.fn(), info: vi.fn(), warn: vi.fn() }, dismiss: vi.fn() }),
}));

import { GitPanel } from '../GitPanel';

/** A real commit message: a subject line, a blank line, then a long body. */
const SUBJECT = 'feat(cli): register the MCP server during setup';
const BODY = Array.from({ length: 12 }, (_, i) => `body line ${i} with some detail`).join('\n');
const LONG = `${SUBJECT}\n\n${BODY}`;

beforeEach(() => {
  vi.clearAllMocks();
  Element.prototype.scrollIntoView = vi.fn();
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'get_git_status': return Promise.resolve({ branch: 'main', file_statuses: {} });
      case 'git_list_branches': return Promise.resolve(['main']);
      case 'get_github_sync_status': return Promise.resolve({ has_remote: false });
      case 'git_get_history':
        return Promise.resolve([
          { hash: 'abcdef1234567890', author: 'me', timestamp: 1_700_000_000, message: LONG },
        ]);
      default: return Promise.resolve(null);
    }
  });
});

async function openHistory() {
  render(<GitPanel workspacePath="/repo" />);
  await screen.findByRole('heading', { name: 'Changes' });
  fireEvent.click(screen.getByRole('button', { name: 'History' }));
  await screen.findByText(/Commit History/);
}

describe('commit messages in history', () => {
  /**
   * The defect: the full message rendered, so one commit with a real body
   * pushed "Files Changed" and the file list below it off the visible area.
   */
  it('shows only the subject line, not the body', async () => {
    await openHistory();
    expect(screen.getByText(/register the MCP server/)).toBeInTheDocument();
    expect(screen.queryByText(/body line 7/)).toBeNull();
  });

  it('marks the clipped message with an ellipsis and a way to expand', async () => {
    await openHistory();
    expect(document.body.textContent ?? '').toContain('…');
    expect(screen.getByRole('button', { name: 'more' })).toBeInTheDocument();
  });

  it('reveals the whole message on demand, and hides it again', async () => {
    await openHistory();
    fireEvent.click(screen.getByRole('button', { name: 'more' }));
    expect(screen.getByText(/body line 7/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'less' }));
    expect(screen.queryByText(/body line 7/)).toBeNull();
  });

  /** A short message must not carry a control that does nothing. */
  it('offers no toggle when nothing is hidden', async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      cmd === 'git_get_history'
        ? Promise.resolve([{ hash: 'aaaaaaa1', author: 'me', timestamp: 1, message: 'fix typo' }])
        : cmd === 'get_git_status'
          ? Promise.resolve({ branch: 'main', file_statuses: {} })
          : cmd === 'git_list_branches'
            ? Promise.resolve(['main'])
            : Promise.resolve(null),
    );
    await openHistory();
    expect(screen.getByText('fix typo')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'more' })).toBeNull();
    expect(document.body.textContent ?? '').not.toContain('…');
  });
});
