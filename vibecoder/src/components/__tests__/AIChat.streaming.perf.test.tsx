import { render, act, waitFor, screen, fireEvent } from '@testing-library/react';
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

import { useState } from 'react';
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

async function flushAll() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

/** Start an agent run so the panel owns the events that follow. */
async function startAgentRun() {
  function ControlledAgentChat() {
    const [messages, setMessages] = useState<Message[]>([]);
    return (
      <AIChat
        provider="test-provider"
        messages={messages}
        onMessagesChange={setMessages}
        useAgentLoop
        onUseAgentLoopChange={() => {}}
      />
    );
  }
  render(<ControlledAgentChat />);
  await flushAll();
  const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
  fireEvent.change(textarea, { target: { value: 'build the thing', selectionStart: 15 } });
  fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });
  await flushAll();
}

describe('AIChat circuit-breaker notices', () => {
  /**
   * `AgentEvent::CircuitBreak` is a notice, not the end of a run. The loop
   * emits it when it compacts context and when it retires a degrading agent
   * in favour of a fresh successor — and then keeps going. Only `BLOCKED` is
   * terminal, and the Rust side reports that as a separate `agent:error`.
   *
   * Reporting "Agent halted" here made the two mechanisms that keep a long
   * task alive look like a crash, and dropped the rest of the run on the
   * floor with it.
   */
  it('keeps the run going when the harness recovers', async () => {
    await startAgentRun();

    await act(async () => {
      emit('agent:circuit_break', {
        state: 'PROGRESS',
        reason: 'Handing off to a fresh agent (hand-off 1/2).',
      });
    });
    await flushAll();

    expect(screen.getByText(/Handing off to a fresh agent/)).toBeInTheDocument();
    // Not an error bubble, and no "halted" claim about a run that is running.
    expect(document.querySelector('.message-error')).toBeNull();
    expect(screen.queryByText(/halted/i)).toBeNull();
  });

  it('reports a health warning without claiming the run ended', async () => {
    await startAgentRun();

    await act(async () => {
      emit('agent:circuit_break', { state: 'DEGRADED', reason: 'Responses are shortening.' });
    });
    await flushAll();

    expect(screen.getByText(/Agent health: DEGRADED/)).toBeInTheDocument();
    expect(screen.queryByText(/halted/i)).toBeNull();
  });
});

describe('AIChat compaction budget', () => {
  /**
   * The panel used to compact at one constant for every model. On a small
   * local model that let the conversation grow far past what the server could
   * hold — and Ollama's answer to an oversized prompt is to drop the *front*
   * of it, system prompt and tool contract first, silently.
   */
  it('asks the backend what the selected model can hold', async () => {
    render(<AIChat provider="ollama" messages={[]} onMessagesChange={vi.fn()} />);
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('model_context_budget', {
        provider: 'ollama',
        model: null,
      }),
    );
  });

  function bulkTranscript(): Message[] {
    const long = 'x'.repeat(4_000);
    return Array.from({ length: 30 }, (_, i) => ({
      role: (i % 2 === 0 ? 'user' : 'assistant') as Message['role'],
      content: long,
      timestamp: 1_700_000_000_000 + i,
    }));
  }

  /** A provider whose vendor does not publish the number must still compact. */
  it('falls back to the default when the budget is unknown', async () => {
    mockInvoke.mockResolvedValue(null); // model_context_budget -> unknown
    render(<AIChat provider="ollama" messages={bulkTranscript()} onMessagesChange={vi.fn()} />);
    // 30 x 4k = 120k chars, over the 80k default.
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('summarise_messages', expect.anything()),
    );
  });

  /** A model that reports a large budget must not be compacted at the default. */
  it('does not compact a conversation that fits the reported budget', async () => {
    mockInvoke.mockImplementation((cmd: string) =>
      // 1M tokens ~ 4M chars, so 120k of transcript is nowhere near it.
      Promise.resolve(cmd === 'model_context_budget' ? 1_000_000 : null),
    );
    render(<AIChat provider="gemini" messages={bulkTranscript()} onMessagesChange={vi.fn()} />);
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith('model_context_budget', expect.anything()),
    );
    await act(async () => { await new Promise((r) => setTimeout(r, 0)); });
    expect(mockInvoke).not.toHaveBeenCalledWith('summarise_messages', expect.anything());
  });
});
