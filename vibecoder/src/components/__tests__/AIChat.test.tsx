import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock Tauri invoke ────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Capture listen callbacks so tests can emit Tauri events.
type EventCallback = (event: { payload: unknown }) => void;
const eventListeners: Record<string, EventCallback[]> = {};

const mockListen = vi.fn();
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, cb: EventCallback) => {
    mockListen(event, cb);
    if (!eventListeners[event]) eventListeners[event] = [];
    eventListeners[event].push(cb);
    const unlisten = () => {
      const idx = eventListeners[event]?.indexOf(cb) ?? -1;
      if (idx >= 0) eventListeners[event].splice(idx, 1);
    };
    return Promise.resolve(unlisten);
  },
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

// ── Mock lucide-react icons as simple spans ─────────────────────────────────

vi.mock('lucide-react', () => ({
  Mic: (props: Record<string, unknown>) => <span data-testid="icon-mic" {...props} />,
  User: (props: Record<string, unknown>) => <span data-testid="icon-user" {...props} />,
  Paperclip: (props: Record<string, unknown>) => <span data-testid="icon-paperclip" {...props} />,
  X: (props: Record<string, unknown>) => <span data-testid="icon-x" {...props} />,
  FileText: (props: Record<string, unknown>) => <span data-testid="icon-filetext" {...props} />,
  Loader2: (props: Record<string, unknown>) => <span data-testid="icon-loader2" {...props} />,
  Download: (props: Record<string, unknown>) => <span data-testid="icon-download" {...props} />,
  ZoomIn: (props: Record<string, unknown>) => <span data-testid="icon-zoomin" {...props} />,
  AtSign: (props: Record<string, unknown>) => <span data-testid="icon-atsign" {...props} />,
  AudioLines: (props: Record<string, unknown>) => <span data-testid="icon-audiolines" {...props} />,
  Plus: (props: Record<string, unknown>) => <span data-testid="icon-plus" {...props} />,
}));

// ── Mock internal dependencies ──────────────────────────────────────────────

vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toast: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), success: vi.fn() },
  }),
}));

vi.mock('../ContextPicker', () => ({
  ContextPicker: ({ query, onSelect, onClose }: { query: string; onSelect: (s: string) => void; onClose: () => void }) => (
    <div data-testid="context-picker" data-query={query}>
      <button onClick={() => onSelect('@file:test.ts')}>select</button>
      <button onClick={onClose}>close</button>
    </div>
  ),
}));

vi.mock('../../utils/FlowContext', () => ({
  flowContext: { add: vi.fn() },
}));

// ── Import component under test (after mocks) ──────────────────────────────

import { AIChat } from '../AIChat';
import type { Message } from '../AIChat';

// ── Setup ───────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks();
  // Clear captured event listeners
  for (const key of Object.keys(eventListeners)) {
    eventListeners[key] = [];
  }
  // Default invoke returns nothing
  mockInvoke.mockResolvedValue(null);
  // Reset SpeechRecognition to avoid voice-input side effects
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (window as any).SpeechRecognition = undefined;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (window as any).webkitSpeechRecognition = undefined;
  // jsdom does not implement scrollIntoView
  Element.prototype.scrollIntoView = vi.fn();
});

// ── Tests ───────────────────────────────────────────────────────────────────

describe('AIChat', () => {
  // ── Rendering ───────────────────────────────────────────────────────────

  it('renders without crashing with minimal props', async () => {
    render(<AIChat provider="ollama" />);
    await waitFor(() => {
      expect(screen.getByText('AI Assistant')).toBeInTheDocument();
    });
  });

  it('shows provider name in the header', async () => {
    render(<AIChat provider="ollama" />);
    await waitFor(() => {
      expect(screen.getByText('ollama')).toBeInTheDocument();
    });
  });

  it('shows message input textarea', () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/);
    expect(textarea).toBeInTheDocument();
    expect(textarea.tagName).toBe('TEXTAREA');
  });

  it('shows send button', () => {
    render(<AIChat provider="ollama" />);
    const sendBtn = screen.getByRole('button', { name: /Send message/i });
    expect(sendBtn).toBeInTheDocument();
  });

  it('shows empty state when no messages', () => {
    render(<AIChat provider="ollama" />);
    expect(screen.getByText('AI Coding Assistant')).toBeInTheDocument();
    expect(screen.getByText('/fix')).toBeInTheDocument();
    expect(screen.getByText('/explain')).toBeInTheDocument();
  });

  // ── Input ─────────────────────────────────────────────────────────────

  it('typing in textarea updates input state', () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'hello world', selectionStart: 11 } });
    expect(textarea.value).toBe('hello world');
  });

  it('pressing Enter calls sendMessage (invokes stream_chat_message)', async () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'Fix the bug', selectionStart: 11 } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'stream_chat_message',
        expect.objectContaining({
          request: expect.objectContaining({
            provider: 'ollama',
          }),
        }),
      );
    });
  });

  it('empty input does not submit', async () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    // Input is empty by default
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });

    // Wait a tick to make sure no invoke happened
    await new Promise((r) => setTimeout(r, 50));
    expect(mockInvoke).not.toHaveBeenCalledWith('stream_chat_message', expect.anything());
  });

  it('Shift+Enter does not submit', () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'some text', selectionStart: 9 } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true });
    // Should not have invoked
    expect(mockInvoke).not.toHaveBeenCalledWith('stream_chat_message', expect.anything());
  });

  // ── Messages ──────────────────────────────────────────────────────────

  it('user messages appear in the chat', () => {
    const msgs: Message[] = [
      { role: 'user', content: 'What does this function do?', timestamp: Date.now() },
    ];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);
    expect(screen.getByText('What does this function do?')).toBeInTheDocument();
  });

  it('renders markdown-like content in assistant messages (code blocks)', () => {
    const msgs: Message[] = [
      { role: 'assistant', content: 'Here is the fix:\n```rust\nfn main() {}\n```', timestamp: Date.now() },
    ];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);
    expect(screen.getByText('fn main() {}')).toBeInTheDocument();
    expect(screen.getByText('rust')).toBeInTheDocument();
  });

  // Messages restored from a saved session (or pushed in by the watch bridge)
  // arrive as raw model output — never parsed by the `chat:complete` path.
  // Before this was normalised at render, the tags showed up verbatim on screen.
  it('collapses a reasoning block that arrives unparsed in message content', () => {
    const msgs: Message[] = [
      {
        role: 'assistant',
        content: '<thinking>The user just says "hi". Respond briefly.</thinking> Hello! How can I help?',
      },
    ];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);

    expect(screen.getByText(/Hello! How can I help\?/)).toBeInTheDocument();
    // The reasoning is behind the collapsed disclosure, not inline.
    expect(screen.queryByText(/The user just says/)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Work · thinking/ })).toBeInTheDocument();
  });

  // A reasoning model that is cut off mid-block leaves the opening tag with no
  // closing one. When the reply also contains an inline code span — a line
  // number, an identifier, a type — the unclosed-tag rule used to look only at
  // the *last* prose run, and the tag sat in the first one. Result: a message
  // that opened with a literal "<thinking>" on screen.
  it('collapses an unclosed reasoning block that opens before a code span', () => {
    const msgs: Message[] = [
      {
        role: 'assistant',
        content:
          '<thinking>The changes were applied successfully. Let me verify the key changes:\n\n' +
          '1. Line 637: `next_oid: Arc<AtomicI32>,` - field added to struct\n' +
          '2. Line 751: `next_oid:` initialised',
        timestamp: Date.now(),
      },
    ];
    const { container } = render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);

    expect(container.textContent).not.toContain('<thinking>');
    expect(screen.getByRole('button', { name: /Work · thinking/ })).toBeInTheDocument();
    // The reasoning is behind the disclosure, code span and all.
    expect(screen.queryByText(/field added to struct/)).not.toBeInTheDocument();
  });

  it('leaves tags inside a code block alone', () => {
    const msgs: Message[] = [
      {
        role: 'assistant',
        content: 'Models emit this:\n```xml\n<thinking>reasoning goes here</thinking>\n```',
        timestamp: Date.now(),
      },
    ];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);

    // The sample is the answer — stripping it would leave an empty fence.
    expect(screen.getByText(/reasoning goes here/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Work · thinking/ })).not.toBeInTheDocument();
  });

  it('error messages render with error styling', () => {
    const msgs: Message[] = [
      { role: 'assistant', content: 'Connection failed', timestamp: Date.now(), isError: true },
    ];
    const { container } = render(
      <AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />,
    );
    const errorMsg = container.querySelector('.message-error');
    expect(errorMsg).not.toBeNull();
    expect(screen.getByText('Connection failed')).toBeInTheDocument();
  });

  it('error messages show retry button for the last message', () => {
    const msgs: Message[] = [
      { role: 'user', content: 'Help me', timestamp: Date.now() },
      { role: 'assistant', content: 'Error occurred', timestamp: Date.now(), isError: true },
    ];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);
    expect(screen.getByText('Retry')).toBeInTheDocument();
  });

  // ── Streaming / Loading ───────────────────────────────────────────────

  it('shows typing indicator when loading', async () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'Hello', selectionStart: 5 } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });

    await waitFor(() => {
      const indicator = document.querySelector('.typing-indicator');
      expect(indicator).not.toBeNull();
    });
  });

  it('shows stop button while loading', async () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'Hello', selectionStart: 5 } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });

    await waitFor(() => {
      expect(screen.getByText('Stop')).toBeInTheDocument();
    });
  });

  // ── Slash commands ────────────────────────────────────────────────────

  it('typing / opens slash command menu', async () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: '/', selectionStart: 1 } });

    await waitFor(() => {
      // The slash palette should show commands
      expect(screen.getByText('Fix errors in the current file')).toBeInTheDocument();
      expect(screen.getByText('Explain selected code')).toBeInTheDocument();
    });
  });

  it('selecting a slash command populates input with prefix', async () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: '/fix', selectionStart: 4 } });

    await waitFor(() => {
      expect(screen.getByText('Fix errors in the current file')).toBeInTheDocument();
    });

    // Click the /fix item
    const fixItem = screen.getByText('Fix errors in the current file').closest('.slash-item');
    if (fixItem) fireEvent.click(fixItem);

    await waitFor(() => {
      expect(textarea.value).toContain('Fix the following errors');
    });
  });

  // ── Attachments ───────────────────────────────────────────────────────

  it('shows attachment count badge when attachments present via controlled messages', () => {
    const msgs: Message[] = [
      {
        role: 'user',
        content: 'Check this file',
        timestamp: Date.now(),
        attachments: [
          { name: 'test.rs', mime_type: 'text/plain', data: '', size: 100, text_content: 'fn main(){}' },
        ],
      },
    ];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);
    expect(screen.getByText(/1 file attached/)).toBeInTheDocument();
  });

  // ── Mode selector ─────────────────────────────────────────────────────

  it('can switch between chat modes (fast/balanced/thorough)', () => {
    render(<AIChat provider="ollama" />);
    // Default is "Balanced" (chat mode)
    const balancedBtn = screen.getByText('Balanced');
    expect(balancedBtn.closest('.mode-btn-active')).not.toBeNull();

    // Click "Fast"
    const fastBtn = screen.getByText('Fast');
    fireEvent.click(fastBtn);
    expect(fastBtn.closest('.mode-btn-active') || fastBtn.classList.contains('mode-btn-active') || fastBtn.closest('button')?.classList.contains('mode-btn-active')).toBeTruthy();

    // Click "Thorough" (planning mode)
    const thoroughBtn = screen.getByText('Thorough');
    fireEvent.click(thoroughBtn);
    expect(thoroughBtn.closest('button')?.classList.contains('mode-btn-active')).toBeTruthy();
  });

  // ── Event listeners ───────────────────────────────────────────────────

  it('registers Tauri event listeners on mount', async () => {
    render(<AIChat provider="ollama" />);
    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith('chat:chunk', expect.any(Function));
      expect(mockListen).toHaveBeenCalledWith('chat:complete', expect.any(Function));
      expect(mockListen).toHaveBeenCalledWith('chat:error', expect.any(Function));
      expect(mockListen).toHaveBeenCalledWith('chat:status', expect.any(Function));
      expect(mockListen).toHaveBeenCalledWith('chat:metrics', expect.any(Function));
    });
  });

  // ── Clear chat ────────────────────────────────────────────────────────

  it('clear button removes all messages', async () => {
    const msgs: Message[] = [
      { role: 'user', content: 'Hello', timestamp: Date.now() },
      { role: 'assistant', content: 'Hi there', timestamp: Date.now() },
    ];
    const onMessagesChange = vi.fn();
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={onMessagesChange} />);

    const clearBtn = screen.getByTitle('Clear chat history');
    fireEvent.click(clearBtn);

    expect(onMessagesChange).toHaveBeenCalledWith([]);
  });

  // ── Provider display ──────────────────────────────────────────────────

  it('displays provider label in the chat header', () => {
    render(<AIChat provider="gemini" />);
    expect(screen.getByText('gemini')).toBeInTheDocument();
  });

  // ── Send button disabled state ────────────────────────────────────────

  it('send button is disabled when input is empty', () => {
    render(<AIChat provider="ollama" />);
    const sendBtn = screen.getByRole('button', { name: /Send message/i });
    expect(sendBtn).toBeDisabled();
  });

  it('send button is enabled when input has text', () => {
    render(<AIChat provider="ollama" />);
    const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'hello', selectionStart: 5 } });
    const sendBtn = screen.getByRole('button', { name: /Send message/i });
    expect(sendBtn).not.toBeDisabled();
  });
});

// ── Response persistence tests (controlled mode) ────────────────────────────
//
// Regression tests for the bug where chat responses would visually disappear
// for one frame in controlled mode (ChatTabManager). The root cause was that
// chat:complete cleared streaming state synchronously while the finalized
// message prop hadn't propagated from the parent yet.

/** Emit a Tauri event to all registered listeners. */
function emitTauriEvent(event: string, payload: unknown) {
  for (const cb of eventListeners[event] ?? []) {
    cb({ payload });
  }
}

/** Flush pending microtasks, promises, and timers. */
async function flushAll() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

import { useState } from 'react';

/**
 * Wrapper that mimics ChatTabManager's controlled-message pattern.
 * Messages live in the parent's state and are passed to AIChat as a prop.
 * This is the scenario where the disappear bug occurred.
 */
function ControlledAIChat() {
  const [messages, setMessages] = useState<Message[]>([]);
  return (
    <AIChat
      provider="test-provider"
      messages={messages}
      onMessagesChange={setMessages}
    />
  );
}

/**
 * Helper: type a message, press Enter to send, then wait for isLoading=true.
 * This simulates the user sending a question before the AI responds.
 */
async function sendUserMessage(text: string) {
  const textarea = screen.getByPlaceholderText(/Ask anything/) as HTMLTextAreaElement;
  fireEvent.change(textarea, { target: { value: text, selectionStart: text.length } });
  fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });
  // Wait for the loading state to settle (invoke is mocked to resolve immediately)
  await flushAll();
}

describe('AIChat — response does not disappear (controlled mode)', () => {
  it('streaming text is visible during streaming', async () => {
    render(<ControlledAIChat />);
    await flushAll();

    // User sends a message → isLoading becomes true
    await sendUserMessage('What is the answer?');

    // Simulate streaming chunks arriving
    act(() => { emitTauriEvent('chat:chunk', 'Hello '); });
    await flushAll();
    expect(screen.getByText(/Hello/)).toBeInTheDocument();

    // A chunk arriving inside STREAM_FLUSH_MS of the previous one is coalesced
    // into the next flush rather than costing its own render, so the assertion
    // is that it *becomes* visible — which is the actual contract — not that it
    // is visible in the same tick.
    act(() => { emitTauriEvent('chat:chunk', 'world!'); });
    await waitFor(() => {
      expect(screen.getByText(/Hello world!/)).toBeInTheDocument();
    });
  });

  it('response remains visible after chat:complete — no disappearing frame', async () => {
    render(<ControlledAIChat />);
    await flushAll();
    await sendUserMessage('What is the answer?');

    // Stream some content
    act(() => { emitTauriEvent('chat:chunk', 'The answer is 42'); });
    await flushAll();
    expect(screen.getByText(/The answer is 42/)).toBeInTheDocument();

    // Fire chat:complete — this is the critical moment.
    // Before the fix, the streaming text would be cleared immediately while
    // the finalized message hadn't arrived from the parent yet, causing
    // the response to vanish for one render frame.
    act(() => {
      emitTauriEvent('chat:complete', {
        message: 'The answer is 42',
        tool_output: '',
      });
    });

    // Immediately after chat:complete: the streaming text must still be
    // showing (deferred clear) and/or the finalized message has arrived.
    // The text must NEVER be absent.
    await flushAll();
    const matchesAfterComplete = screen.queryAllByText(/The answer is 42/);
    expect(matchesAfterComplete.length).toBeGreaterThanOrEqual(1);

    // After the useEffect on `messages` fires and cleanup runs,
    // exactly the finalized message should be in the DOM.
    await flushAll();
    expect(screen.queryAllByText(/The answer is 42/).length).toBeGreaterThanOrEqual(1);
  });

  // The finalized message and the live streaming bubble rendered the same text
  // at once: one committed bubble with a timestamp, one streaming bubble
  // without. Any chunk arriving after chat:complete re-populated streamingText
  // after the deferred clear had already run, and nothing clears it again.
  // Reproduces the reported screen state directly: the finalized reply is in
  // `messages` (rendered with a timestamp) while the live streaming bubble is
  // still showing the same prose (rendered without one). Whatever leaves the
  // stream un-cleared — a deferred clear that never lands, a late chunk, a
  // second listener — the user must never be shown the same answer twice.
  it('does not show the streaming bubble once the same text is already committed', async () => {
    const text = 'Vibe Agent at your service! How can I assist you today?';
    render(
      <AIChat
        provider="test-provider"
        messages={[{ role: 'assistant', content: text, timestamp: 1 }]}
        onMessagesChange={vi.fn()}
      />,
    );
    await flushAll();

    // Put the component into a streaming state carrying that same text.
    await sendUserMessage('hi');
    act(() => { emitTauriEvent('chat:chunk', text); });
    await flushAll();

    const shown = screen.getAllByText(new RegExp('Vibe Agent at your service'));
    expect(
      shown.length,
      `expected the response once, found ${shown.length} copies on screen`,
    ).toBe(1);
  });

  it('a chunk arriving after chat:complete does not duplicate the response', async () => {
    render(<ControlledAIChat />);
    await flushAll();
    await sendUserMessage('hi');

    act(() => { emitTauriEvent('chat:chunk', 'Vibe Agent at your service!'); });
    await flushAll();

    act(() => {
      emitTauriEvent('chat:complete', { message: 'Vibe Agent at your service!' });
    });
    await flushAll();

    // A late chunk from the same turn — the backend has already said it is done.
    act(() => { emitTauriEvent('chat:chunk', 'Vibe Agent at your service!'); });
    await flushAll();

    const shown = screen.getAllByText(/Vibe Agent at your service!/);
    expect(
      shown.length,
      `expected the response once, found ${shown.length} copies on screen`,
    ).toBe(1);
  });

  it('error response remains visible after chat:error', async () => {
    render(<ControlledAIChat />);
    await flushAll();
    await sendUserMessage('Help me');

    // Stream some content then error
    act(() => { emitTauriEvent('chat:chunk', 'Partial output'); });
    await flushAll();
    expect(screen.getByText(/Partial output/)).toBeInTheDocument();

    act(() => {
      emitTauriEvent('chat:error', 'Provider connection failed');
    });

    // The error or the streaming text must be visible — never blank
    await flushAll();
    const visible =
      screen.queryByText(/Provider connection failed/) ||
      screen.queryByText(/Partial output/);
    expect(visible).toBeInTheDocument();

    // After the useEffect clears streaming, error message is the final state
    await flushAll();
    expect(screen.getByText(/Provider connection failed/)).toBeInTheDocument();
  });

  it('response is visible in uncontrolled mode (immediate cleanup)', async () => {
    // Uncontrolled: no messages/onMessagesChange props
    render(<AIChat provider="test-provider" />);
    await flushAll();
    await sendUserMessage('Question');

    act(() => { emitTauriEvent('chat:chunk', 'Direct response'); });
    await flushAll();
    expect(screen.getByText(/Direct response/)).toBeInTheDocument();

    act(() => {
      emitTauriEvent('chat:complete', {
        message: 'Direct response',
        tool_output: '',
      });
    });
    await flushAll();

    // In uncontrolled mode, the message and streaming cleanup are in the
    // same component, so no gap is possible.
    expect(screen.getByText(/Direct response/)).toBeInTheDocument();
  });

  it('multiple sequential responses remain visible', async () => {
    render(<ControlledAIChat />);
    await flushAll();

    // First response cycle
    await sendUserMessage('First question');
    act(() => { emitTauriEvent('chat:chunk', 'First reply'); });
    await flushAll();
    act(() => {
      emitTauriEvent('chat:complete', { message: 'First reply', tool_output: '' });
    });
    await flushAll();
    expect(screen.queryAllByText(/First reply/).length).toBeGreaterThanOrEqual(1);

    // Second response cycle
    await sendUserMessage('Second question');
    act(() => { emitTauriEvent('chat:chunk', 'Second reply'); });
    await flushAll();
    act(() => {
      emitTauriEvent('chat:complete', { message: 'Second reply', tool_output: '' });
    });
    await flushAll();

    // Both responses must be in the DOM
    expect(screen.queryAllByText(/First reply/).length).toBeGreaterThanOrEqual(1);
    expect(screen.queryAllByText(/Second reply/).length).toBeGreaterThanOrEqual(1);
  });

  it('workspace change does not clear messages or streaming', async () => {
    render(<ControlledAIChat />);
    await flushAll();

    // User sends a message and gets a finalized response
    await sendUserMessage('Explain this code');
    act(() => { emitTauriEvent('chat:chunk', 'This function does X'); });
    await flushAll();
    act(() => {
      emitTauriEvent('chat:complete', { message: 'This function does X', tool_output: '' });
    });
    await flushAll();
    expect(screen.queryAllByText(/This function does X/).length).toBeGreaterThanOrEqual(1);

    // User opens a new folder → workspace-changed event fires
    act(() => {
      window.dispatchEvent(new CustomEvent('vibecoder:workspace-changed', { detail: '/new/folder' }));
    });
    await flushAll();

    // Messages must still be visible after workspace change
    expect(screen.queryAllByText(/This function does X/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/Explain this code/)).toBeInTheDocument();
  });

  it('agent loop completes a multi-step task (useAgentLoop=true)', async () => {
    function ControlledAgentChat() {
      const [messages, setMessages] = useState<Message[]>([]);
      return (
        <AIChat
          provider="test-provider"
          messages={messages}
          onMessagesChange={setMessages}
          useAgentLoop={true}
          onUseAgentLoopChange={() => {}}
        />
      );
    }
    render(<ControlledAgentChat />);
    await flushAll();

    // Send the initial task
    await sendUserMessage('list the files in src/');

    // sendMessage must have called start_agent_task (NOT stream_chat_message)
    expect(mockInvoke).toHaveBeenCalledWith(
      'start_agent_task',
      expect.objectContaining({ task: 'list the files in src/', approvalPolicy: 'suggest' }),
    );
    expect(mockInvoke).not.toHaveBeenCalledWith('stream_chat_message', expect.anything());

    // Agent streams a planning chunk
    act(() => { emitTauriEvent('agent:chunk', 'Planning: I will list the directory.'); });
    await flushAll();
    expect(screen.getByText(/Planning: I will list/)).toBeInTheDocument();

    // Agent completes a tool step
    act(() => {
      emitTauriEvent('agent:step', {
        step_num: 1,
        tool_name: 'list_directory',
        tool_summary: "list 'src/'",
        output: 'a.ts\nb.ts',
        success: true,
        approved: true,
      });
    });
    await flushAll();
    // Step cards live inside the "Work" section, which is collapsed by default.
    fireEvent.click(screen.getByRole('button', { name: /Work · 1 step/ }));
    expect(screen.getByText('list_directory')).toBeInTheDocument();
    expect(screen.getByText(/a\.ts/)).toBeInTheDocument();

    // Agent completes the run with a summary; the markdown renderer may split
    // text across nodes, so match on document.body.textContent rather than
    // requiring a single matching text node.
    act(() => { emitTauriEvent('agent:complete', 'AGENT_COMPLETE_TOKEN: src contains files'); });
    await flushAll();
    await waitFor(() => {
      expect(document.body.textContent ?? '').toContain('AGENT_COMPLETE_TOKEN');
    });
  });

  it('approval rejection halts the tool call (useAgentLoop=true)', async () => {
    function ControlledAgentChat() {
      const [messages, setMessages] = useState<Message[]>([]);
      return (
        <AIChat
          provider="test-provider"
          messages={messages}
          onMessagesChange={setMessages}
          useAgentLoop={true}
          onUseAgentLoopChange={() => {}}
        />
      );
    }
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('delete temp/');

    // Backend asks for approval on a destructive tool
    act(() => {
      emitTauriEvent('agent:pending', {
        name: 'delete_file',
        summary: "delete 'temp/' recursively",
        is_destructive: true,
      });
    });
    await flushAll();

    // Banner must be visible with both buttons
    expect(screen.getByText(/Destructive tool/)).toBeInTheDocument();
    const rejectBtn = screen.getByRole('button', { name: /Reject/i });
    expect(rejectBtn).toBeInTheDocument();
    // Exact match — the banner also carries "Approve all for this run".
    expect(screen.getByRole('button', { name: 'Approve' })).toBeInTheDocument();

    // User rejects → respond_to_agent_approval(false)
    fireEvent.click(rejectBtn);
    await flushAll();

    expect(mockInvoke).toHaveBeenCalledWith(
      'respond_to_agent_approval',
      { approved: false },
    );
    // Banner clears after rejection
    expect(screen.queryByText(/Destructive tool/)).not.toBeInTheDocument();
  });

  it('verifier card renders PASS / NITS / FAIL (useAgentLoop=true)', async () => {
    function ControlledAgentChat() {
      const [messages, setMessages] = useState<Message[]>([]);
      return (
        <AIChat
          provider="test-provider"
          messages={messages}
          onMessagesChange={setMessages}
          useAgentLoop={true}
          onUseAgentLoopChange={() => {}}
        />
      );
    }
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('do the thing');

    // PASS — green card with no message body
    act(() => {
      emitTauriEvent('agent:verifier', { status: 'pass', message: '' });
    });
    await flushAll();
    const passCard = await screen.findByTestId('verifier-card');
    expect(passCard.textContent).toContain('PASS');

    // NITS — yellow card with the nit text
    act(() => {
      emitTauriEvent('agent:verifier', { status: 'nits', message: 'commit message could be tighter' });
    });
    await flushAll();
    const nitsCard = await screen.findByTestId('verifier-card');
    expect(nitsCard.textContent).toContain('NITS');
    expect(nitsCard.textContent).toContain('commit message could be tighter');

    // FAIL — red card with the failure reason
    act(() => {
      emitTauriEvent('agent:verifier', { status: 'fail', message: 'tests are still failing' });
    });
    await flushAll();
    const failCard = await screen.findByTestId('verifier-card');
    expect(failCard.textContent).toContain('FAIL');
    expect(failCard.textContent).toContain('tests are still failing');
  });

  it('stop button aborts mid-run (useAgentLoop=true)', async () => {
    function ControlledAgentChat() {
      const [messages, setMessages] = useState<Message[]>([]);
      return (
        <AIChat
          provider="test-provider"
          messages={messages}
          onMessagesChange={setMessages}
          useAgentLoop={true}
          onUseAgentLoopChange={() => {}}
        />
      );
    }
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('do a long task');

    // Agent emits a chunk so we know it's mid-run
    act(() => { emitTauriEvent('agent:chunk', 'Working...'); });
    await flushAll();

    // Stop button is visible while loading
    const stopBtn = await screen.findByText('Stop');
    fireEvent.click(stopBtn);
    await flushAll();

    // stop_agent_task is invoked (NOT stop_chat_stream — this tab owns an agent)
    expect(mockInvoke).toHaveBeenCalledWith('stop_agent_task');
    expect(mockInvoke).not.toHaveBeenCalledWith('stop_chat_stream');
  });

  it('per-tab agent events: only the matching sessionId tab handles them', async () => {
    // Two AIChat instances mounted simultaneously, each with its own sessionId.
    // A scoped `agent:tab-A:complete` event must land on tab-A only; tab-B must
    // not surface the assistant message.
    function TwoTabs() {
      const [msgsA, setMsgsA] = useState<Message[]>([]);
      const [msgsB, setMsgsB] = useState<Message[]>([]);
      return (
        <>
          <div data-testid="tab-A-wrap">
            <AIChat
              provider="test-provider"
              messages={msgsA}
              onMessagesChange={setMsgsA}
              useAgentLoop={true}
              onUseAgentLoopChange={() => {}}
              sessionId="tab-A"
            />
          </div>
          <div data-testid="tab-B-wrap">
            <AIChat
              provider="test-provider"
              messages={msgsB}
              onMessagesChange={setMsgsB}
              useAgentLoop={true}
              onUseAgentLoopChange={() => {}}
              sessionId="tab-B"
            />
          </div>
        </>
      );
    }
    render(<TwoTabs />);
    await flushAll();

    // Mark both tabs as agent-run owners (the sendMessage path normally does
    // this; we skip the user input here and trigger the events directly).
    // Emit a scoped event meant for tab-A only.
    act(() => {
      emitTauriEvent('agent:tab-A:complete', 'Tab A finished its task');
      emitTauriEvent('agent:tab-B:complete', 'Tab B finished its task');
    });
    await flushAll();

    // Each tab should only see its own message — no cross-tab contamination.
    const tabA = screen.getByTestId('tab-A-wrap');
    const tabB = screen.getByTestId('tab-B-wrap');
    expect(tabA.textContent).toContain('Tab A finished');
    expect(tabA.textContent).not.toContain('Tab B finished');
    expect(tabB.textContent).toContain('Tab B finished');
    expect(tabB.textContent).not.toContain('Tab A finished');
  });

  it('streaming response survives workspace change mid-stream', async () => {
    render(<ControlledAIChat />);
    await flushAll();

    // User sends a message, AI starts streaming
    await sendUserMessage('Help me');
    act(() => { emitTauriEvent('chat:chunk', 'Working on it'); });
    await flushAll();
    expect(screen.getByText(/Working on it/)).toBeInTheDocument();

    // Workspace changes mid-stream
    act(() => {
      window.dispatchEvent(new CustomEvent('vibecoder:workspace-changed', { detail: '/other/folder' }));
    });
    await flushAll();

    // Streaming text must still be visible
    expect(screen.getByText(/Working on it/)).toBeInTheDocument();

    // Stream completes successfully after workspace change
    act(() => {
      emitTauriEvent('chat:complete', { message: 'Working on it — done!', tool_output: '' });
    });
    await flushAll();
    expect(screen.queryAllByText(/Working on it/).length).toBeGreaterThanOrEqual(1);
  });
});

// ── Tool-result auto-continue loop ───────────────────────────────────────────
// The loop feeds executed tool output back so the model can act on it. Tool
// results must be replayed as a *user* turn: a request ending on an assistant
// message makes providers (Ollama/GLM) return an empty completion, which
// silently stopped the run after the first tool round.

/** Last `stream_chat_message` request payload seen by the mocked invoke. */
function lastChatRequest(): {
  messages: Array<{ role: string; content: string }>;
} {
  const calls = mockInvoke.mock.calls.filter(c => c[0] === 'stream_chat_message');
  return calls[calls.length - 1][1].request;
}

function chatRequestCount(): number {
  return mockInvoke.mock.calls.filter(c => c[0] === 'stream_chat_message').length;
}

describe('AIChat — tool-result continuation', () => {
  it('auto-continues after tool output and replays the result as a user turn', async () => {
    render(<ControlledAIChat />);
    await flushAll();
    await sendUserMessage('Review the codebase');

    act(() => {
      emitTauriEvent('chat:complete', {
        message: 'Let me look around.\n<list_dir path="src" />',
        tool_output: "Directory 'src':\n- lib.rs (file)",
      });
    });
    await flushAll();

    expect(chatRequestCount()).toBe(2);
    const { messages } = lastChatRequest();
    const last = messages[messages.length - 1];
    expect(last.role).toBe('user');
    expect(last.content).toContain('[Tool results]');
    expect(last.content).toContain('lib.rs');
  });

  it('replays the assistant turn with its tool tags intact', async () => {
    render(<ControlledAIChat />);
    await flushAll();
    await sendUserMessage('Review the codebase');

    act(() => {
      emitTauriEvent('chat:complete', {
        message: 'Looking around.\n<list_dir path="src" />',
        tool_output: "Directory 'src':\n- lib.rs (file)",
      });
    });
    await flushAll();

    const { messages } = lastChatRequest();
    const assistantTurn = messages.find(m => m.role === 'assistant');
    expect(assistantTurn?.content).toContain('<list_dir path="src" />');
  });

  it('never sends a request whose last message is from the assistant', async () => {
    render(<ControlledAIChat />);
    await flushAll();
    await sendUserMessage('Review the codebase');

    act(() => {
      emitTauriEvent('chat:complete', {
        message: '<list_dir path="src" />',
        tool_output: "Directory 'src':\n- lib.rs (file)",
      });
    });
    await flushAll();

    for (const call of mockInvoke.mock.calls.filter(c => c[0] === 'stream_chat_message')) {
      const msgs = call[1].request.messages as Array<{ role: string }>;
      expect(msgs[msgs.length - 1].role).toBe('user');
    }
  });

  it('stops without auto-continuing when no tools ran', async () => {
    render(<ControlledAIChat />);
    await flushAll();
    await sendUserMessage('Just answer');

    act(() => {
      emitTauriEvent('chat:complete', { message: 'Here is the answer.', tool_output: '' });
    });
    await flushAll();

    expect(chatRequestCount()).toBe(1);
  });

  it('reports an empty continuation instead of going quiet', async () => {
    render(<ControlledAIChat />);
    await flushAll();
    await sendUserMessage('Review the codebase');

    act(() => {
      emitTauriEvent('chat:complete', {
        message: '<list_dir path="src" />',
        tool_output: "Directory 'src':\n- lib.rs (file)",
      });
    });
    await flushAll();

    // Continuation turn comes back empty — the provider gave up.
    act(() => {
      emitTauriEvent('chat:complete', { message: '', tool_output: '' });
    });
    await flushAll();

    expect(screen.getByText(/empty response/i)).toBeInTheDocument();
  });
});

// ── Agent approval flow ──────────────────────────────────────────────────────
// A long agent run prompts for every tool call under the "suggest" policy. The
// backend keeps ONE pending-approval slot and ONE abort handle, so starting a
// second run while the first waits orphans it — the first run blocks forever on
// an approval channel that no longer exists.

function ControlledAgentChat() {
  const [messages, setMessages] = useState<Message[]>([]);
  return (
    <AIChat
      provider="test-provider"
      messages={messages}
      onMessagesChange={setMessages}
      useAgentLoop={true}
      onUseAgentLoopChange={() => {}}
    />
  );
}

function emitPending(name: string, summary: string, destructive: boolean) {
  act(() => {
    emitTauriEvent('agent:pending', { name, summary, is_destructive: destructive });
  });
}

function approvalCalls() {
  return mockInvoke.mock.calls.filter(c => c[0] === 'respond_to_agent_approval');
}

describe('AIChat — agent approvals', () => {
  it('shows the approval banner for a pending tool call', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('review the codebase');

    emitPending('bash', 'bash(find crates -name "*.rs")', false);
    await flushAll();

    expect(screen.getByText(/Tool approval required/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Approve all for this run/ })).toBeInTheDocument();
  });

  it('"Approve all" auto-approves every later call in the run', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('review the codebase');

    emitPending('bash', 'bash(ls)', false);
    await flushAll();
    fireEvent.click(screen.getByRole('button', { name: /Approve all for this run/ }));
    await flushAll();
    expect(approvalCalls()).toHaveLength(1);

    // A later call must not stop to ask again.
    emitPending('read_file', 'read_file(src/lib.rs)', false);
    await flushAll();
    expect(screen.queryByText(/approval required/i)).not.toBeInTheDocument();
    expect(approvalCalls()).toHaveLength(2);
    expect(approvalCalls()[1][1]).toEqual({ approved: true });
  });

  it('auto-approval does not carry into the next run', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('first task');
    emitPending('bash', 'bash(ls)', false);
    await flushAll();
    fireEvent.click(screen.getByRole('button', { name: /Approve all for this run/ }));
    await flushAll();

    act(() => { emitTauriEvent('agent:complete', 'done'); });
    await flushAll();
    await sendUserMessage('second task');

    emitPending('bash', 'bash(ls)', false);
    await flushAll();
    expect(screen.getByText(/approval required/i)).toBeInTheDocument();
  });

  it('refuses to start a second run while an approval is pending', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('review the codebase');
    const runsBefore = mockInvoke.mock.calls.filter(c => c[0] === 'start_agent_task').length;

    emitPending('bash', 'bash(ls)', false);
    await flushAll();

    await sendUserMessage('approve all');
    const runsAfter = mockInvoke.mock.calls.filter(c => c[0] === 'start_agent_task').length;
    expect(runsAfter).toBe(runsBefore);
    // …and the pending call is still there to act on.
    expect(screen.getByText(/approval required/i)).toBeInTheDocument();
  });

  it('labels a destructive call differently from a read-only one', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('review the codebase');

    emitPending('bash', 'bash(rm -rf target)', true);
    await flushAll();
    expect(screen.getByText(/Destructive tool — approval required/)).toBeInTheDocument();
  });
});

// ── Approval mode selector ───────────────────────────────────────────────────
// The policy is fixed when the run starts (the backend reads it once in
// start_agent_task), and it only governs agent runs.

describe('AIChat — approval mode selector', () => {
  it('is hidden until agent mode is on', async () => {
    render(<AIChat provider="test-provider" />);
    await flushAll();
    expect(screen.queryByLabelText('Agent approval mode')).not.toBeInTheDocument();
  });

  it('is shown when agent mode is on, defaulting to asking every time', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    const select = screen.getByLabelText('Agent approval mode') as HTMLSelectElement;
    expect(select.value).toBe('suggest');
  });

  it('offers every backend policy that makes sense with the agent loop', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    const select = screen.getByLabelText('Agent approval mode') as HTMLSelectElement;
    const values = Array.from(select.options).map(o => o.value);
    expect(values).toEqual(['suggest', 'read-only', 'auto-edit', 'full-auto']);
  });

  it('sends the chosen mode as the run policy', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    fireEvent.change(screen.getByLabelText('Agent approval mode'), {
      target: { value: 'read-only' },
    });
    await sendUserMessage('review the codebase');

    expect(mockInvoke).toHaveBeenCalledWith(
      'start_agent_task',
      expect.objectContaining({ approvalPolicy: 'read-only' }),
    );
  });

  it('is locked while a run is in flight', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('go');
    expect(screen.getByLabelText('Agent approval mode')).toBeDisabled();
  });

  it('reports changes to the tab manager when controlled', async () => {
    const onApprovalModeChange = vi.fn();
    function ControlledMode() {
      const [messages, setMessages] = useState<Message[]>([]);
      const [mode, setMode] = useState<'suggest' | 'read-only' | 'auto-edit' | 'full-auto'>('suggest');
      return (
        <AIChat
          provider="test-provider"
          messages={messages}
          onMessagesChange={setMessages}
          useAgentLoop={true}
          onUseAgentLoopChange={() => {}}
          approvalMode={mode}
          onApprovalModeChange={(m) => { onApprovalModeChange(m); setMode(m); }}
        />
      );
    }
    render(<ControlledMode />);
    await flushAll();
    fireEvent.change(screen.getByLabelText('Agent approval mode'), {
      target: { value: 'full-auto' },
    });
    await flushAll();

    expect(onApprovalModeChange).toHaveBeenCalledWith('full-auto');
    expect((screen.getByLabelText('Agent approval mode') as HTMLSelectElement).value).toBe('full-auto');
  });
});

// ── Agent terminal events ────────────────────────────────────────────────────

describe('AIChat — agent terminal events', () => {
  it('renders the circuit-breaker reason, not [object Object]', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('go');

    act(() => {
      emitTauriEvent('agent:circuit_break', {
        state: 'Blocked',
        reason: 'no progress after 3 rotations',
      });
    });
    await flushAll();

    expect(screen.getByText(/no progress after 3 rotations/)).toBeInTheDocument();
    expect(screen.queryByText(/\[object Object\]/)).not.toBeInTheDocument();
  });

  it('falls back gracefully when the circuit-break payload is a bare string', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('go');

    act(() => { emitTauriEvent('agent:circuit_break', 'stalled'); });
    await flushAll();
    expect(screen.getByText(/stalled/)).toBeInTheDocument();
  });

  it('shows a reasoning-only completion instead of a content-free placeholder', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('go');

    act(() => {
      emitTauriEvent('agent:complete', '<thinking>Let me read key files first.</thinking>');
    });
    await flushAll();

    expect(screen.getByText(/Let me read key files first/)).toBeInTheDocument();
    expect(screen.queryByText('Agent task complete.')).not.toBeInTheDocument();
  });

  it('keeps reasoning collapsed when there is a real summary', async () => {
    render(<ControlledAgentChat />);
    await flushAll();
    await sendUserMessage('go');

    act(() => {
      emitTauriEvent('agent:complete', '<thinking>weighing options</thinking>Reviewed 3 crates.');
    });
    await flushAll();

    expect(screen.getByText(/Reviewed 3 crates/)).toBeInTheDocument();
    expect(screen.queryByText(/weighing options/)).not.toBeInTheDocument();
  });
});

// ── Watch sync echo ─────────────────────────────────────────────────────────
//
// A session-tracked tab writes its own turns to sessions.db, which is the same
// table useWatchSync polls for Watch/mobile messages. Both guards — the cursor
// advance on chat:complete, and the normalized content match — must keep that
// reply from coming back as a second bubble.

describe('AIChat — watch sync does not echo the tab\'s own reply', () => {
  const RAW = '<thinking>weighing options</thinking>Use a HashMap for constant-time lookups.';

  function mockSessionRow(rowId: number) {
    mockInvoke.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'watch_get_session_messages') {
        const after = (args?.afterId as number | null) ?? 0;
        return Promise.resolve({
          session_id: 's1',
          messages: after < rowId
            ? [{ id: rowId, role: 'assistant', content: RAW, created_at: 1700000000000 }]
            : [],
        });
      }
      return Promise.resolve(null);
    });
  }

  function countReplies() {
    return screen.getAllByText((_t, el) =>
      el?.className === 'message-content' &&
      (el.textContent ?? '').includes('constant-time lookups'),
    ).length;
  }

  it('skips rows the backend reports it wrote for this turn', async () => {
    vi.useFakeTimers();
    try {
      // Seed poll happens before the turn, so the row is not there yet.
      mockInvoke.mockResolvedValue({ session_id: 's1', messages: [] });
      render(<AIChat provider="ollama" sessionId="s1" />);
      await act(async () => {});

      mockSessionRow(7);
      act(() => {
        eventListeners['chat:complete']?.forEach((cb) =>
          cb({ payload: { message: RAW, tool_output: '', session_msg_id: 7 } }));
      });
      await act(async () => { vi.advanceTimersByTime(2200); });
      await act(async () => {});

      expect(countReplies()).toBe(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it('drops a raw echo that arrives before the cursor advances', async () => {
    vi.useFakeTimers();
    try {
      mockInvoke.mockResolvedValue({ session_id: 's1', messages: [] });
      render(<AIChat provider="ollama" sessionId="s1" />);
      await act(async () => {});

      // No session_msg_id — the poll wins the race, so only the content match
      // stands between the raw DB row and a duplicate bubble.
      mockSessionRow(7);
      act(() => {
        eventListeners['chat:complete']?.forEach((cb) =>
          cb({ payload: { message: RAW, tool_output: '' } }));
      });
      await act(async () => { vi.advanceTimersByTime(2200); });
      await act(async () => {});

      expect(countReplies()).toBe(1);
    } finally {
      vi.useRealTimers();
    }
  });
});


// ── Canonical tool-call markup ──────────────────────────────────────────────
//
// Local models emit native tool calls, which the provider transcribes into
// `<tool_call name="…">` markup. The panel has to render those as tool cards —
// before this, they reached the bubble as literal text.

describe('AIChat — canonical tool_call blocks render as cards', () => {
  /** Open the collapsed "Work" disclosure so the tool cards are visible. */
  function expandWork() {
    fireEvent.click(screen.getByRole('button', { name: /Work/ }));
  }

  it('shows a write_file card and no raw markup', () => {
    const msgs: Message[] = [{
      role: 'assistant',
      content: 'Adding it now.\n<tool_call name="write_file"><path>src/main.rs</path>'
        + '<content>fn main() {}</content></tool_call>',
      timestamp: Date.now(),
    }];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);

    expect(document.body.textContent).not.toContain('<tool_call');
    expect(screen.getByText('Adding it now.')).toBeInTheDocument();
    expect(screen.getByText('Work · 1 tool')).toBeInTheDocument();
    expandWork();
    expect(screen.getByText('src/main.rs')).toBeInTheDocument();
  });

  it('decodes escaped content in the card', () => {
    const msgs: Message[] = [{
      role: 'assistant',
      content: '<tool_call name="read_file"><path>a &amp; b.txt</path></tool_call>',
      timestamp: Date.now(),
    }];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);
    expandWork();
    expect(screen.getByText('a & b.txt')).toBeInTheDocument();
  });

  // A tool the panel has no card for still must not leak as markup — the user
  // sees what the model reached for instead.
  it('shows an unknown tool as a card rather than raw text', () => {
    const msgs: Message[] = [{
      role: 'assistant',
      content: '<tool_call name="container.exec"><cmd>ls</cmd></tool_call>',
      timestamp: Date.now(),
    }];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);
    expect(document.body.textContent).not.toContain('<tool_call');
    expandWork();
    expect(screen.getByText(/container\.exec/)).toBeInTheDocument();
  });

  // The legacy dialect keeps working: cloud models mid-conversation, and every
  // session already on disk, speak it.
  it('still renders the tag dialect', () => {
    const msgs: Message[] = [{
      role: 'assistant',
      content: 'Done.\n<write_file path="a.txt">hello</write_file>',
      timestamp: Date.now(),
    }];
    render(<AIChat provider="ollama" messages={msgs} onMessagesChange={vi.fn()} />);
    expect(document.body.textContent).not.toContain('<write_file');
    expandWork();
    expect(screen.getByText('a.txt')).toBeInTheDocument();
  });
});
