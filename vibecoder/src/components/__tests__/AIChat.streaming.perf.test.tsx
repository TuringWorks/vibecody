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

/** Render a controlled chat and put a request in flight. */
async function startChatStream() {
  function ControlledChat() {
    const [messages, setMessages] = useState<Message[]>([]);
    return <AIChat provider="ollama" messages={messages} onMessagesChange={setMessages} />;
  }
  render(<ControlledChat />);
  await flushAll();
  const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
  fireEvent.change(textarea, { target: { value: 'hello', selectionStart: 5 } });
  fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });
  await flushAll();
}

describe('AIChat streaming render frequency', () => {
  /**
   * `chat:chunk` arrives as its own Tauri event, so it used to cost its own
   * React render — and every one of those renders re-ran `extractThinking`
   * and a full markdown parse over the whole reply so far. The cost of a
   * response was therefore quadratic in its own length: a long answer visibly
   * slowed down as it was written.
   *
   * A burst faster than STREAM_FLUSH_MS must collapse into one render. How
   * much that saves scales with the provider's token rate — a model slower
   * than the interval renders on every chunk exactly as before, by design.
   */
  it('coalesces a burst of chunks into a single render', async () => {
    await startChatStream();
    markdownRenders = 0;

    // One `act` per chunk. Batching them into one would let React collapse the
    // renders by itself and the assertion below would hold with or without the
    // throttle — the real stream arrives as 40 separate Tauri events.
    for (let i = 0; i < 40; i += 1) {
      // eslint-disable-next-line no-await-in-loop
      await act(async () => emit('chat:chunk', `tok${i} `));
    }

    // One leading-edge publish. Before this, 40 chunks meant 40 renders of the
    // live bubble, each re-parsing everything received so far.
    // Measured: 40 before (one full re-parse of the reply-so-far per chunk),
    // 1-2 after.
    expect(markdownRenders).toBeGreaterThan(0);
    expect(markdownRenders).toBeLessThanOrEqual(3);

    // …and nothing is lost: the trailing flush carries the whole burst.
    await waitFor(() => {
      expect(document.body.textContent ?? '').toContain('tok39');
    });
  });

  /** The tail must survive a stop, which reads the accumulator, not the view. */
  it('keeps chunks that had not been published when the user stops', async () => {
    await startChatStream();

    await act(async () => emit('chat:chunk', 'first '));
    await act(async () => emit('chat:chunk', 'and the unpublished tail'));
    const stop = screen.getByTitle(/Stop/i);
    await act(async () => { fireEvent.click(stop); });

    await waitFor(() => {
      expect(document.body.textContent ?? '').toContain('and the unpublished tail');
    });
  });
});

describe('AIChat rejected tool calls', () => {
  /**
   * The backend refuses a tool tag whose `path` cannot be a filename — the
   * shape a model emits when its markup is malformed. Before this the file
   * simply never appeared and nothing anywhere said why, which is the failure
   * the rejection was added to replace.
   */
  it('tells the user which tool calls were ignored, once, at the end of the turn', async () => {
    await startChatStream();

    await act(async () => {
      emit('chat:status', {
        type: 'tool_call_rejected',
        tool: 'write_file',
        reason: 'path contains the control character \'\\n\'',
      });
      // The same defect repeated must not be reported twice.
      emit('chat:status', {
        type: 'tool_call_rejected',
        tool: 'write_file',
        reason: 'path contains the control character \'\\n\'',
      });
    });

    // Nothing is claimed until the turn ends.
    expect(document.body.textContent ?? '').not.toContain('were ignored');

    await act(async () => {
      emit('chat:complete', { message: 'Done.', session_msg_id: null });
    });

    await waitFor(() => {
      expect(document.body.textContent ?? '').toContain('could not be acted on');
    });
    const body = document.body.textContent ?? '';
    expect(body).toContain('control character');
    expect(body.split('control character').length - 1).toBe(1);
  });

  /** A rejection from a turn that never completed must not be blamed on the next. */
  it('does not carry a rejection into the following turn', async () => {
    await startChatStream();
    await act(async () => {
      emit('chat:status', { type: 'tool_call_rejected', tool: 'write_file', reason: 'stale' });
    });

    // Second turn starts without the first ever completing.
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'again', selectionStart: 5 } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });
    await flushAll();

    await act(async () => {
      emit('chat:complete', { message: 'Done.', session_msg_id: null });
    });
    await flushAll();
    expect(document.body.textContent ?? '').not.toContain('stale');
  });
});
