/**
 * The full-duplex voice path resolves a real provider *and* model.
 *
 * `chat_provider_for` on the daemon builds an override only when both are
 * present, and otherwise uses whatever the daemon booted with — silently. So
 * a missing model does not fail loudly: the microphone works, transcripts
 * arrive and appear as user turns, and nothing ever answers.
 *
 * The toolbar hands this panel a *display name* (`"Ollama (gpt-oss:120b-cloud)"`),
 * not a provider id, so indexing PROVIDER_DEFAULT_MODEL with it returned
 * `undefined` and that is exactly what shipped.
 */
import { render, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => mockInvoke(...a) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: () => Promise.resolve(() => {}) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
// Named explicitly: a Proxy does not satisfy vitest's ESM named-export check.
vi.mock('lucide-react', () => ({
  Mic: () => <span />,
  User: () => <span />,
  Paperclip: () => <span />,
  X: () => <span />,
  FileText: () => <span />,
  Loader2: () => <span />,
  Download: () => <span />,
  ZoomIn: () => <span />,
  AtSign: () => <span />,
  AudioLines: () => <span />,
  Plus: () => <span />,
}));
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({ toast: { info: vi.fn(), warn: vi.fn(), error: vi.fn(), success: vi.fn() } }),
}));
vi.mock('../ContextPicker', () => ({ ContextPicker: () => <div /> }));
vi.mock('../../utils/FlowContext', () => ({ flowContext: { add: vi.fn() } }));

/** Capture what the panel asks the duplex hook for. */
const duplexOptions: Array<{ provider?: string; model?: string }> = [];
vi.mock('@vibe/shared/voice/useVoiceDuplex', () => ({
  useVoiceDuplex: (opts: { provider?: string; model?: string }) => {
    duplexOptions.push({ provider: opts.provider, model: opts.model });
    return { state: { status: 'idle' }, active: false, supported: true, start: vi.fn(), stop: vi.fn() };
  },
}));
vi.mock('@vibe/shared/voice/DuplexVoiceButton', () => ({ DuplexVoiceButton: () => <button /> }));

import { AIChat } from '../AIChat';

beforeEach(() => {
  vi.clearAllMocks();
  duplexOptions.length = 0;
  mockInvoke.mockResolvedValue(null);
  Element.prototype.scrollIntoView = vi.fn();
});

const latest = () => duplexOptions[duplexOptions.length - 1];

describe('full-duplex provider selection', () => {
  it('resolves the toolbar display name to a provider id and the chosen model', async () => {
    render(<AIChat provider="Ollama (gpt-oss:120b-cloud)" messages={[]} onMessagesChange={vi.fn()} />);
    await waitFor(() => expect(duplexOptions.length).toBeGreaterThan(0));
    expect(latest().provider).toBe('ollama');
    // The model the user picked, not the registry's default for ollama.
    expect(latest().model).toBe('gpt-oss:120b-cloud');
  });

  /** The half-specified case is the one the daemon swallows. */
  it('never sends a provider without a model', async () => {
    render(<AIChat provider="Ollama (gpt-oss:120b-cloud)" messages={[]} onMessagesChange={vi.fn()} />);
    await waitFor(() => expect(duplexOptions.length).toBeGreaterThan(0));
    expect(latest().model, 'a provider with no model makes the daemon use its own').toBeTruthy();
  });

  it('still works when given a bare provider id', async () => {
    render(<AIChat provider="ollama" messages={[]} onMessagesChange={vi.fn()} />);
    await waitFor(() => expect(duplexOptions.length).toBeGreaterThan(0));
    expect(latest().provider).toBe('ollama');
    expect(latest().model).toBeTruthy();
  });
});
