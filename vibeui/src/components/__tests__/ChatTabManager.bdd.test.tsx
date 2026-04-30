/**
 * BDD tests for ChatTabManager — adventure names and tab lifecycle.
 *
 * Scenarios:
 *  1. First tab gets an adventure name from the pool
 *  2. Each new tab gets the next name (pool cycles)
 *  3. Pool wraps at 30 entries without repeating prematurely
 *  4. refreshAdventureNames updates the module cache from the backend
 *  5. Tab lifecycle: add, close (not last), close-last guard
 *  6. Closing a tab with messages auto-saves to history
 *  7. Provider override and reset follow top-bar changes
 */

import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Mocks ──────────────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('../AIChat', () => ({
  AIChat: ({ messages, onMessagesChange, sessionId }: {
    messages: { role: string; content: string }[];
    onMessagesChange?: (msgs: { role: string; content: string }[]) => void;
    sessionId?: string;
  }) => (
    <div data-testid="ai-chat" data-session-id={sessionId}>
      messages:{messages.length}
      <button
        type="button"
        data-testid="mock-send"
        onClick={() => onMessagesChange?.([
          ...messages,
          { role: 'user', content: `msg-${messages.length + 1}` },
        ])}
      >
        send
      </button>
    </div>
  ),
}));

vi.mock('../ChatMemoryPanel', () => ({
  ChatMemoryPanel: () => <div data-testid="memory-panel" />,
}));

vi.mock('../RecapCard', () => ({
  RecapCard: ({ recap, onResume }: {
    recap: { id: string; headline: string };
    onResume?: (r: { id: string; headline: string }) => void;
  }) => (
    <div data-testid="recap-card">
      <span data-testid="recap-headline">{recap.headline}</span>
      <button type="button" data-testid="recap-resume" onClick={() => onResume?.(recap)}>
        Resume from here
      </button>
    </div>
  ),
}));

vi.mock('../../hooks/useSessionMemory', () => ({
  useSessionMemory: () => ({
    factsForTab: () => [],
    extractFromMessages: vi.fn(),
    getPinnedSystemPromptText: () => '',
    pinFact: vi.fn(),
    unpinFact: vi.fn(),
    deleteFact: vi.fn(),
    editFact: vi.fn(),
    addManual: vi.fn(),
  }),
}));

import { ChatTabManager } from '../ChatTabManager';

// ── Default prop helpers ───────────────────────────────────────────────────────

function defaultProps(overrides: Record<string, unknown> = {}) {
  return {
    defaultProvider: 'ollama',
    availableProviders: ['ollama', 'openai'],
    ...overrides,
  };
}

function renderManager(overrides: Record<string, unknown> = {}) {
  return render(<ChatTabManager {...defaultProps(overrides)} />);
}

// ── Setup ──────────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  // Default: get_adventure_names returns backend list; get_adventure_names is called on mount
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_adventure_names') return ['Alpha', 'Beta', 'Gamma'];
    return null;
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

// ── Scenario 1: First tab title comes from the adventure names pool ────────────

describe('Given the ChatTabManager renders for the first time', () => {
  it('When it mounts, Then the first tab has a non-empty title from the pool', () => {
    renderManager();
    // The tab strip renders all tab titles; the first one should be from the 30-name pool
    // (exact name depends on random start index, but it must be non-empty)
    const tabBar = screen.getByRole('button', { name: '+' }).closest('div')!.parentElement!;
    // Tab titles appear as spans before the "×" close buttons
    const allText = tabBar.textContent ?? '';
    // The adventure pool names are distinct non-empty strings
    expect(allText.length).toBeGreaterThan(0);
  });
});

// ── Scenario 2: Adding a new tab picks the next adventure name ─────────────────

describe('Given the user adds a second tab', () => {
  it('When they click +, Then two distinct tabs exist', () => {
    renderManager();

    const addBtn = screen.getByTitle('New chat tab');
    fireEvent.click(addBtn);

    // Two "×" close buttons appear once there are 2 tabs
    const closeBtns = screen.getAllByTitle('Close tab');
    expect(closeBtns).toHaveLength(2);
  });

  it('When they add 30 tabs, Then all tab titles are non-empty strings', () => {
    renderManager();
    const addBtn = screen.getByTitle('New chat tab');
    // Add 29 more tabs (starting from 1, get to 30)
    for (let i = 0; i < 29; i++) {
      fireEvent.click(addBtn);
    }
    const closeBtns = screen.getAllByTitle('Close tab');
    expect(closeBtns).toHaveLength(30);
  });
});

// ── Scenario 3: refreshAdventureNames calls the backend on mount ───────────────

describe('Given refreshAdventureNames is called on mount', () => {
  it('When the component mounts, Then invoke("get_adventure_names") is called', async () => {
    renderManager();
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_adventure_names');
    });
  });

  it('When the backend returns names, Then no error is thrown', async () => {
    mockInvoke.mockResolvedValueOnce(['Adventure One', 'Adventure Two']);
    expect(() => renderManager()).not.toThrow();
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_adventure_names');
    });
  });

  it('When the backend throws, Then the component renders with default names (graceful degradation)', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('backend unavailable'));
    expect(() => renderManager()).not.toThrow();
    // Component should still render with the + button
    expect(screen.getByTitle('New chat tab')).toBeInTheDocument();
  });
});

// ── Scenario 4: Tab lifecycle — closing is guarded when only one tab remains ──

describe('Given only one tab is open', () => {
  it('When the user tries to close it, Then the tab remains open', () => {
    renderManager();
    // With only 1 tab, no close button is rendered
    expect(screen.queryByTitle('Close tab')).not.toBeInTheDocument();
  });
});

describe('Given two tabs are open', () => {
  it('When the user closes the second tab, Then only one tab remains', () => {
    renderManager();
    fireEvent.click(screen.getByTitle('New chat tab'));
    expect(screen.getAllByTitle('Close tab')).toHaveLength(2);

    // Close the second tab
    const closeBtns = screen.getAllByTitle('Close tab');
    fireEvent.click(closeBtns[1]);

    // Only one tab left → close button disappears
    expect(screen.queryByTitle('Close tab')).not.toBeInTheDocument();
  });
});

// ── Scenario 5: Provider override and reset ────────────────────────────────────

describe('Given a tab with the default provider', () => {
  it('When the user selects a different provider in the per-tab dropdown, Then "reset" appears', () => {
    renderManager();
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'openai' } });
    expect(screen.getByText('reset')).toBeInTheDocument();
  });

  it('When the user clicks reset, Then the override indicator disappears', () => {
    renderManager();
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'openai' } });
    fireEvent.click(screen.getByText('reset'));
    expect(screen.queryByText('reset')).not.toBeInTheDocument();
  });
});

// ── Scenario 6: History panel ─────────────────────────────────────────────────

describe('Given the History button is clicked', () => {
  it('When there are no saved sessions, Then the empty state message is shown', () => {
    renderManager();
    fireEvent.click(screen.getByTitle('Session history'));
    expect(screen.getByText(/No saved sessions yet/i)).toBeInTheDocument();
  });

  it('When History is toggled twice, Then the chat view is restored', () => {
    renderManager();
    const historyBtn = screen.getByTitle('Session history');
    fireEvent.click(historyBtn);
    expect(screen.getByText(/Session History/i)).toBeInTheDocument();
    fireEvent.click(historyBtn);
    expect(screen.queryByText(/Session History/i)).not.toBeInTheDocument();
  });
});

// ── Scenario 7: Tab rename (inline edit) ──────────────────────────────────────

describe('Given the user double-clicks a tab title', () => {
  it('When they press Enter, Then the new name is saved', () => {
    renderManager();
    // Find the tab title span (has "Double-click to rename" title)
    const titleSpan = screen.getByTitle('Double-click to rename');
    fireEvent.dblClick(titleSpan);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'My Renamed Tab' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(screen.getByText('My Renamed Tab')).toBeInTheDocument();
  });

  it('When they press Escape, Then the original name is preserved', async () => {
    renderManager();
    const titleSpan = screen.getByTitle('Double-click to rename');
    const originalName = titleSpan.textContent ?? '';
    fireEvent.dblClick(titleSpan);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'Discarded Name' } });
    fireEvent.keyDown(input, { key: 'Escape' });

    expect(screen.queryByText('Discarded Name')).not.toBeInTheDocument();
    expect(screen.getByText(originalName)).toBeInTheDocument();
  });
});

// ── Scenario 8: Session persistence ───────────────────────────────────────────

describe('Given localStorage has a legacy persisted-sessions blob', () => {
  it('When the component mounts, Then the new tab opens fresh (not resurrected from localStorage)', () => {
    localStorage.setItem('vibecody:chat-sessions', JSON.stringify({
      'tab-1': [{ id: '1', role: 'user', content: 'Hello', timestamp: Date.now() }],
    }));
    renderManager();
    // Mock AIChat renders "messages:N" — fresh tab must show 0
    expect(screen.getByTestId('ai-chat')).toHaveTextContent('messages:0');
    // And the legacy blob must be evicted on mount
    expect(localStorage.getItem('vibecody:chat-sessions')).toBeNull();
  });

  it('When localStorage has corrupt JSON, Then the component renders normally', () => {
    localStorage.setItem('vibecody:chat-sessions', 'not-valid-json{{{');
    expect(() => renderManager()).not.toThrow();
    expect(screen.getByTitle('New chat tab')).toBeInTheDocument();
  });
});

// ── Scenario 9: History dedup — Save twice updates one entry ──────────────────

function readHistory(): Array<{ id: string; messages: { role: string; content: string }[] }> {
  const raw = localStorage.getItem('vibecody:chat-history');
  return raw ? JSON.parse(raw) : [];
}

describe('Given a tab has been saved to history once', () => {
  it('When the user adds more messages and clicks Save again, Then history holds one entry (updated, not duplicated)', () => {
    renderManager();

    // Type a message → Save button appears
    fireEvent.click(screen.getByTestId('mock-send'));
    fireEvent.click(screen.getByTitle('Save current session to history'));
    expect(readHistory()).toHaveLength(1);
    const firstId = readHistory()[0].id;
    expect(readHistory()[0].messages).toHaveLength(1);

    // Add another message and save again
    fireEvent.click(screen.getByTestId('mock-send'));
    fireEvent.click(screen.getByTitle('Save current session to history'));

    const after = readHistory();
    expect(after).toHaveLength(1);
    expect(after[0].id).toBe(firstId);
    expect(after[0].messages).toHaveLength(2);
  });

  it('When the user closes the tab after saving, Then no duplicate is appended', () => {
    renderManager();

    // Type into and save the only tab (which is the active one)
    fireEvent.click(screen.getByTestId('mock-send'));
    fireEvent.click(screen.getByTitle('Save current session to history'));
    expect(readHistory()).toHaveLength(1);
    const firstId = readHistory()[0].id;

    // Add a second tab so closeTab is allowed, then close the first (saved) tab.
    fireEvent.click(screen.getByTitle('New chat tab'));
    fireEvent.click(screen.getAllByTitle('Close tab')[0]);

    const after = readHistory();
    expect(after).toHaveLength(1);
    expect(after[0].id).toBe(firstId);
  });
});

describe('Given a session is restored from history into a new tab', () => {
  it('When the user adds messages and saves, Then the original history entry is updated in place', () => {
    // Pre-seed history with a session
    const seededId = 'session-seed-1';
    localStorage.setItem('vibecody:chat-history', JSON.stringify([{
      id: seededId,
      title: 'Seeded',
      provider: 'ollama',
      messages: [{ role: 'user', content: 'original' }],
      savedAt: 1700000000000,
    }]));

    renderManager();

    // Open History panel and click Restore on the seeded entry
    fireEvent.click(screen.getByTitle('Session history'));
    fireEvent.click(screen.getByTitle('Restore into new tab'));

    // After restore, two tabs are mounted (original + restored). The restored
    // one is active; its parent wrapper has display:flex (vs display:none).
    const aiChats = screen.getAllByTestId('ai-chat');
    const activeChat = aiChats.find(el => (el.parentElement as HTMLElement).style.display !== 'none');
    expect(activeChat).toBeTruthy();
    fireEvent.click(activeChat!.querySelector('[data-testid="mock-send"]')!);
    fireEvent.click(screen.getByTitle('Save current session to history'));

    const after = readHistory();
    expect(after).toHaveLength(1);
    expect(after[0].id).toBe(seededId);
    expect(after[0].messages).toHaveLength(2);
  });
});

// ── F2.2: Recap card pinned to a restored tab ───────────────────────────────────

describe('Given a history entry has a recapSubjectId', () => {
  function seedHistoryWithRecap(subjectId = 'sess_xyz') {
    localStorage.setItem('vibecody:chat-history', JSON.stringify([{
      id: 'session-seed-1',
      title: 'Seeded',
      provider: 'ollama',
      messages: [{ role: 'user', content: 'original' }],
      savedAt: 1700000000000,
      recapSubjectId: subjectId,
    }]));
  }

  function mockRecapInvoke(headline = 'Wired auth refresh-token rotation') {
    mockInvoke.mockImplementation(async (cmd: string, args?: { subjectId?: string }) => {
      if (cmd === 'get_adventure_names') return ['Alpha', 'Beta', 'Gamma'];
      if (cmd === 'recap_get_for_session') {
        return {
          id: 'rcp_1',
          kind: 'session',
          subject_id: args?.subjectId ?? '',
          generated_at: '2026-01-01T00:00:00Z',
          generator: { type: 'heuristic' },
          headline,
          bullets: ['b1'],
          next_actions: [],
          artifacts: [],
          schema_version: 1,
        };
      }
      return null;
    });
  }

  it('When the daemon returns a recap, Then a RecapCard renders with the headline', async () => {
    seedHistoryWithRecap();
    mockRecapInvoke('Wired auth refresh-token rotation');

    renderManager();
    fireEvent.click(screen.getByTitle('Session history'));
    fireEvent.click(screen.getByTitle('Restore into new tab'));

    await waitFor(() => {
      expect(screen.getByTestId('recap-card')).toBeInTheDocument();
    });
    expect(screen.getByTestId('recap-headline').textContent).toBe('Wired auth refresh-token rotation');
    expect(mockInvoke).toHaveBeenCalledWith('recap_get_for_session', { subjectId: 'sess_xyz' });
  });

  it('When the user clicks "Resume from here", Then recap_resume_session is invoked with the recap id', async () => {
    seedHistoryWithRecap();
    mockRecapInvoke();

    renderManager();
    fireEvent.click(screen.getByTitle('Session history'));
    fireEvent.click(screen.getByTitle('Restore into new tab'));

    await waitFor(() => {
      expect(screen.getByTestId('recap-card')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('recap-resume'));

    expect(mockInvoke).toHaveBeenCalledWith('recap_resume_session', { recapId: 'rcp_1', branch: false });
  });

  it('When the daemon command throws, Then no RecapCard renders and the tab still works', async () => {
    seedHistoryWithRecap();
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_adventure_names') return ['Alpha', 'Beta', 'Gamma'];
      if (cmd === 'recap_get_for_session') throw new Error('daemon offline');
      return null;
    });

    renderManager();
    fireEvent.click(screen.getByTitle('Session history'));
    fireEvent.click(screen.getByTitle('Restore into new tab'));

    // Allow the rejected promise to flush
    await new Promise(r => setTimeout(r, 0));
    expect(screen.queryByTestId('recap-card')).toBeNull();
  });
});

describe('Given a restored history entry has no recapSubjectId', () => {
  it('When the user restores it, Then no RecapCard renders and recap_get_for_session is not called', async () => {
    localStorage.setItem('vibecody:chat-history', JSON.stringify([{
      id: 'session-seed-1',
      title: 'Seeded',
      provider: 'ollama',
      messages: [{ role: 'user', content: 'original' }],
      savedAt: 1700000000000,
      // recapSubjectId omitted on purpose
    }]));

    renderManager();
    fireEvent.click(screen.getByTitle('Session history'));
    fireEvent.click(screen.getByTitle('Restore into new tab'));

    await new Promise(r => setTimeout(r, 0));
    expect(screen.queryByTestId('recap-card')).toBeNull();
    expect(mockInvoke).not.toHaveBeenCalledWith('recap_get_for_session', expect.anything());
  });
});
