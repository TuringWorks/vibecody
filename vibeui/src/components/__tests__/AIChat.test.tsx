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

    act(() => { emitTauriEvent('chat:chunk', 'world!'); });
    await flushAll();
    expect(screen.getByText(/Hello world!/)).toBeInTheDocument();
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
      window.dispatchEvent(new CustomEvent('vibeui:workspace-changed', { detail: '/new/folder' }));
    });
    await flushAll();

    // Messages must still be visible after workspace change
    expect(screen.queryAllByText(/This function does X/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/Explain this code/)).toBeInTheDocument();
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
      window.dispatchEvent(new CustomEvent('vibeui:workspace-changed', { detail: '/other/folder' }));
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
