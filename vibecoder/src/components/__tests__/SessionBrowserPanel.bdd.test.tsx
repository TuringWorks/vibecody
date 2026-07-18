/**
 * BDD tests for SessionBrowserPanel — session list / search / delete /
 * fork / replay flow.
 *
 * Scenarios:
 *  1. Empty workspace renders the empty-state copy
 *  2. list_sessions error surfaces an error block
 *  3. Search filters the list by id (case-insensitive)
 *  4. Delete requires a second click — first click only arms it
 *  5. Confirmed delete invokes the daemon and removes the row
 *  6. Delete failure announces via role="alert"
 *  7. Fork invokes the daemon and shows a status banner
 *  8. Replay loads a session's messages and supports prev/next
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import SessionBrowserPanel from '../SessionBrowserPanel';

const SAMPLE = [
  { id: 'sess-alpha-100', timestamp: 100, message_count: 3, file_size: 1024, has_messages: true, has_context: false },
  { id: 'sess-beta-200', timestamp: 200, message_count: 7, file_size: 2048, has_messages: true, has_context: true },
];

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'list_sessions') return SAMPLE;
    return null;
  });
});

describe('Given the SessionBrowserPanel mounts with no sessions', () => {
  it('When list_sessions returns [], Then the empty-state copy renders', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_sessions') return [];
      return null;
    });
    render(<SessionBrowserPanel />);
    await waitFor(() => {
      expect(screen.getByText(/No sessions found/i)).toBeInTheDocument();
    });
  });
});

describe('Given list_sessions throws', () => {
  it('When the panel mounts, Then the error block displays the message', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_sessions') throw new Error('cannot read traces dir');
      return null;
    });
    render(<SessionBrowserPanel />);
    await waitFor(() => {
      expect(screen.getByText(/cannot read traces dir/i)).toBeInTheDocument();
    });
  });
});

describe('Given two sessions are listed', () => {
  it('When the user types "alpha" in the search box, Then only the matching session shows', async () => {
    render(<SessionBrowserPanel />);
    await waitFor(() => expect(screen.getByText('sess-alpha-100')).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText(/Search sessions/i), { target: { value: 'alpha' } });
    expect(screen.getByText('sess-alpha-100')).toBeInTheDocument();
    expect(screen.queryByText('sess-beta-200')).not.toBeInTheDocument();
  });

  it('When the search has no match, Then the no-matches empty-state appears', async () => {
    render(<SessionBrowserPanel />);
    await waitFor(() => expect(screen.getByText('sess-alpha-100')).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText(/Search sessions/i), { target: { value: 'nomatch' } });
    expect(screen.getByText(/No sessions match your search/i)).toBeInTheDocument();
  });
});

describe('Given the user clicks Delete on a session', () => {
  it('When they click once, Then delete_session is NOT yet invoked and the button arms', async () => {
    render(<SessionBrowserPanel />);
    await waitFor(() => expect(screen.getByText('sess-alpha-100')).toBeInTheDocument());

    const deleteBtns = screen.getAllByRole('button', { name: /Delete session sess-alpha-100/i });
    fireEvent.click(deleteBtns[0]);
    // delete_session should NOT have been invoked yet
    const calls = mockInvoke.mock.calls.filter(c => c[0] === 'delete_session');
    expect(calls).toHaveLength(0);
    // Button label switches to confirm
    expect(screen.getByRole('button', { name: /Confirm delete session sess-alpha-100/i })).toBeInTheDocument();
  });

  it('When they click twice, Then delete_session is invoked and the row is removed', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_sessions') return SAMPLE;
      if (cmd === 'delete_session') return null;
      return null;
    });
    render(<SessionBrowserPanel />);
    await waitFor(() => expect(screen.getByText('sess-alpha-100')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: /Delete session sess-alpha-100/i }));
    fireEvent.click(screen.getByRole('button', { name: /Confirm delete session sess-alpha-100/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('delete_session', expect.objectContaining({ sessionId: 'sess-alpha-100' }));
    });
    await waitFor(() => {
      expect(screen.queryByText('sess-alpha-100')).not.toBeInTheDocument();
    });
    // Status banner uses role="status" for the success path
    expect(screen.getByRole('status').textContent).toMatch(/Session deleted/i);
  });

  it('When delete_session rejects, Then a role="alert" surfaces the failure', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_sessions') return SAMPLE;
      if (cmd === 'delete_session') throw new Error('permission denied');
      return null;
    });
    render(<SessionBrowserPanel />);
    await waitFor(() => expect(screen.getByText('sess-alpha-100')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: /Delete session sess-alpha-100/i }));
    fireEvent.click(screen.getByRole('button', { name: /Confirm delete session sess-alpha-100/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert').textContent).toMatch(/permission denied/i);
    });
  });
});

describe('Given the user clicks Fork', () => {
  it('When fork_session returns a new id, Then a status banner shows it', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_sessions') return SAMPLE;
      if (cmd === 'fork_session') return 'fork-sess-alpha-100-9999';
      return null;
    });
    render(<SessionBrowserPanel />);
    await waitFor(() => expect(screen.getByText('sess-alpha-100')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: /Fork session sess-alpha-100/i }));

    await waitFor(() => {
      expect(screen.getByRole('status').textContent).toMatch(/fork-sess-alpha-100-9999/);
    });
  });
});

describe('Given the user clicks a session ID to replay', () => {
  it('When messages load, Then the Replay tab shows them and prev/next paginates', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_sessions') return SAMPLE;
      if (cmd === 'get_session_detail') {
        return [
          { role: 'user', content: 'first' },
          { role: 'assistant', content: 'second' },
          { role: 'user', content: 'third' },
        ];
      }
      return null;
    });
    render(<SessionBrowserPanel />);
    await waitFor(() => expect(screen.getByText('sess-alpha-100')).toBeInTheDocument());

    // Click the title text to switch to replay
    fireEvent.click(screen.getByText('sess-alpha-100'));
    await waitFor(() => expect(screen.getByText('first')).toBeInTheDocument());
    expect(screen.getByText(/Step 1 \/ 3/)).toBeInTheDocument();

    // Step forward
    fireEvent.click(screen.getByRole('button', { name: /Next/i }));
    expect(screen.getByText(/Step 2 \/ 3/)).toBeInTheDocument();
    expect(screen.getByText('second')).toBeInTheDocument();
  });
});
