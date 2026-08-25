import { render, act, waitFor, screen } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Tauri ───────────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

type EventCallback = (event: { payload: unknown }) => void;
const eventListeners: Record<string, EventCallback[]> = {};

vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, cb: EventCallback) => {
    (eventListeners[event] ??= []).push(cb);
    return Promise.resolve(() => {
      const idx = eventListeners[event]?.indexOf(cb) ?? -1;
      if (idx >= 0) eventListeners[event].splice(idx, 1);
    });
  },
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

vi.mock('lucide-react', () => ({
  Mic: () => <span />,
  User: () => <span />,
  Paperclip: () => <span />,
  X: () => <span />,
  FileText: () => <span />,
  Loader2: () => <span />,
  Download: () => <span />,
  ZoomIn: () => <span />,
}));

vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({ toast: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), success: vi.fn() } }),
}));

vi.mock('../ContextPicker', () => ({ ContextPicker: () => <div /> }));
vi.mock('../../utils/FlowContext', () => ({ flowContext: { add: vi.fn() } }));

// The renderer whose cost this test measures. Counting its renders is a proxy
// for "how much of the transcript did that chunk re-render" — it is the most
// expensive thing in a bubble and there is one per prose run.
let markdownRenders = 0;
vi.mock('@vibe/shared/markdown/Markdown', () => ({
  Markdown: ({ text }: { text: string }) => {
    markdownRenders += 1;
    return <div data-testid="md">{text}</div>;
  },
}));

import { AIChat } from '../AIChat';
import type { Message } from '../AIChat';

const TRANSCRIPT = 30;

function transcript(): Message[] {
  return Array.from({ length: TRANSCRIPT }, (_, i) => ({
    role: (i % 2 === 0 ? 'user' : 'assistant') as Message['role'],
    content: `Turn ${i}: some prose with a \`span\` in it.\n\nAnd a second paragraph.`,
    timestamp: 1_700_000_000_000 + i,
  }));
}

function emit(event: string, payload: unknown) {
  for (const cb of eventListeners[event] ?? []) cb({ payload });
}

beforeEach(() => {
  vi.clearAllMocks();
  for (const key of Object.keys(eventListeners)) eventListeners[key] = [];
  mockInvoke.mockResolvedValue(null);
  Element.prototype.scrollIntoView = vi.fn();
  markdownRenders = 0;
});

describe('AIChat streaming cost', () => {
  /**
   * The regression this pins: `chat:chunk` fires a state update per streamed
   * chunk, and the transcript used to be re-rendered — re-parsed, re-markdown'd
   * — in full on every one of them. A response therefore cost
   * O(transcript x chunks), so every answer in a session was slower than the
   * last. Bubbles that did not change must not re-render at all.
   */
  it('does not re-render settled bubbles for every streamed chunk', async () => {
    render(
      <AIChat provider="ollama" messages={transcript()} onMessagesChange={vi.fn()} />,
    );
    await waitFor(() => expect(screen.getAllByTestId('md').length).toBeGreaterThan(0));

    const afterMount = markdownRenders;
    expect(afterMount).toBeGreaterThan(0);

    // One `act` per chunk. Batching them into one would merge 20 updates into
    // a single render pass and understate the cost by 20x — the real stream
    // arrives as separate events.
    const CHUNKS = 20;
    for (let i = 0; i < CHUNKS; i += 1) {
      // eslint-disable-next-line no-await-in-loop
      await act(async () => emit('chat:chunk', `token ${i} `));
    }

    // Zero is the honest expectation: none of the 30 settled messages changed,
    // so none of them should have been re-rendered. Measured on the code this
    // replaced: 300 extra renders here, i.e. all 15 assistant bubbles rebuilt
    // once per chunk.
    expect(markdownRenders - afterMount).toBe(0);
  });

  /** The memo must not freeze content: a real edit still re-renders. */
  it('still re-renders a bubble whose content changed', async () => {
    const msgs = transcript();
    const { rerender } = render(
      <AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />,
    );
    await waitFor(() => expect(screen.getAllByTestId('md').length).toBeGreaterThan(0));
    const afterMount = markdownRenders;

    const edited = msgs.map((m, i) => (i === 1 ? { ...m, content: 'rewritten body' } : m));
    await act(async () => {
      rerender(<AIChat provider="ollama" messages={edited} onMessagesChange={vi.fn()} />);
    });

    expect(markdownRenders).toBeGreaterThan(afterMount);
    expect(screen.getByText('rewritten body')).toBeInTheDocument();
  });
});
