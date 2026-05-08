/**
 * BDD tests for ArenaPanel — blind A/B comparison and voting flow.
 *
 * Scenarios:
 *  1. Empty state copy renders before first battle
 *  2. Battle button is disabled with empty prompt
 *  3. Successful battle renders both blind response cards
 *  4. Error from compare_models surfaces in an alert
 *  5. Vote buttons are exposed with explicit aria-labels
 *  6. After voting, reveal panel announces the winner via aria-live
 *  7. save_arena_vote failure surfaces an inline alert (does not block reveal)
 *  8. Leaderboard renders rows when stats are returned
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('../../hooks/useModelRegistry', () => ({
  useModelRegistry: () => ({
    providers: ['ollama', 'openai'],
    modelsForProvider: () => ['llama3', 'gpt-4'],
  }),
  PROVIDER_DEFAULT_MODEL: { ollama: 'llama3', openai: 'gpt-4' },
  getDefaultProvider: () => 'openai',
}));

import { ArenaPanel } from '../ArenaPanel';

beforeEach(() => {
  vi.clearAllMocks();
  // Default: get_arena_history returns empty (fresh install).
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_arena_history') return [[], []];
    return null;
  });
});

describe('Given the ArenaPanel renders for the first time', () => {
  it('When there is no battle yet, Then the empty-state copy is shown', async () => {
    render(<ArenaPanel />);
    expect(screen.getByText(/Enter a prompt and click Battle/i)).toBeInTheDocument();
  });

  it('When the prompt is empty, Then the Battle button is disabled', () => {
    render(<ArenaPanel />);
    const battleBtn = screen.getByRole('button', { name: /Battle/i });
    expect(battleBtn).toBeDisabled();
  });
});

describe('Given a successful battle', () => {
  function mockBattle() {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_arena_history') return [[], []];
      if (cmd === 'compare_models') {
        return {
          a: { provider: 'ollama', model: 'llama3', content: 'response A', duration_ms: 120, tokens: 12, error: null },
          b: { provider: 'openai', model: 'gpt-4', content: 'response B', duration_ms: 240, tokens: 18, error: null },
        };
      }
      if (cmd === 'save_arena_vote') return null;
      return null;
    });
  }

  it('When the user submits a prompt, Then both blind response cards render', async () => {
    mockBattle();
    render(<ArenaPanel />);
    const ta = screen.getByRole('textbox');
    fireEvent.change(ta, { target: { value: 'What is 2+2?' } });
    fireEvent.click(screen.getByRole('button', { name: /Battle/i }));

    await waitFor(() => {
      expect(screen.getByText(/Model A/)).toBeInTheDocument();
      expect(screen.getByText(/Model B/)).toBeInTheDocument();
    });
    // The mock returns "response A" / "response B" — assertions just on content.
    expect(screen.getByText(/response A/)).toBeInTheDocument();
    expect(screen.getByText(/response B/)).toBeInTheDocument();
  });

  it('When responses arrive, Then the vote group is exposed with the right aria-labels', async () => {
    mockBattle();
    render(<ArenaPanel />);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'q' } });
    fireEvent.click(screen.getByRole('button', { name: /Battle/i }));
    await waitFor(() => expect(screen.getByText(/Model A/)).toBeInTheDocument());

    const group = screen.getByRole('group', { name: /cast your vote/i });
    expect(group).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Vote: A is better' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Vote: B is better' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Vote: Tie' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Vote: Both bad' })).toBeInTheDocument();
  });

  it('When the user votes "A is better", Then the reveal region announces the winner', async () => {
    mockBattle();
    render(<ArenaPanel />);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'q' } });
    fireEvent.click(screen.getByRole('button', { name: /Battle/i }));
    await waitFor(() => expect(screen.getByText(/Model A/)).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'Vote: A is better' }));

    await waitFor(() => {
      expect(screen.getByRole('region', { name: /battle reveal/i })).toBeInTheDocument();
    });
    expect(screen.getByText(/Model A wins/)).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith('save_arena_vote', expect.objectContaining({
      vote: expect.objectContaining({ winner: 'a' }),
    }));
  });
});

describe('Given compare_models rejects', () => {
  it('When the user clicks Battle, Then an alert displays the error', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_arena_history') return [[], []];
      if (cmd === 'compare_models') throw new Error('provider not configured');
      return null;
    });
    render(<ArenaPanel />);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'q' } });
    fireEvent.click(screen.getByRole('button', { name: /Battle/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert').textContent).toMatch(/provider not configured/i);
  });
});

describe('Given save_arena_vote rejects', () => {
  it('When the user votes, Then the reveal still shows AND a save-error alert appears', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_arena_history') return [[], []];
      if (cmd === 'compare_models') {
        return {
          a: { provider: 'ollama', model: 'llama3', content: 'rA', duration_ms: 1, tokens: null, error: null },
          b: { provider: 'openai', model: 'gpt-4', content: 'rB', duration_ms: 1, tokens: null, error: null },
        };
      }
      if (cmd === 'save_arena_vote') throw new Error('disk full');
      return null;
    });
    render(<ArenaPanel />);
    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'q' } });
    fireEvent.click(screen.getByRole('button', { name: /Battle/i }));
    await waitFor(() => expect(screen.getByText(/Model A/)).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'Vote: A is better' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    // Reveal still rendered — vote save failure does not block the reveal.
    expect(screen.getByRole('region', { name: /battle reveal/i })).toBeInTheDocument();
    expect(screen.getByRole('alert').textContent).toMatch(/couldn['’]t save/i);
  });
});

describe('Given prior arena history exists', () => {
  it('When the panel mounts, Then the leaderboard renders one row per provider', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_arena_history') {
        return [
          [
            { timestamp: 't', prompt: 'p', provider_a: 'ollama', model_a: 'm1', provider_b: 'openai', model_b: 'm2', winner: 'a' },
          ],
          [
            { provider: 'ollama', wins: 1, losses: 0, ties: 0, total: 1, win_rate: 1.0 },
            { provider: 'openai', wins: 0, losses: 1, ties: 0, total: 1, win_rate: 0.0 },
          ],
        ];
      }
      return null;
    });
    render(<ArenaPanel />);
    await waitFor(() => {
      expect(screen.getByText(/Leaderboard/i)).toBeInTheDocument();
    });
    // Both providers appear as table rows
    expect(screen.getAllByRole('row').length).toBeGreaterThanOrEqual(3); // header + 2 data
  });
});
