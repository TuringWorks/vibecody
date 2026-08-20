/**
 * A routing failure describes one token count, on one tab.
 *
 * "No configured model has a context window large enough for 1,010,000 tokens"
 * was set into a panel-wide error and never cleared: it followed the reader
 * onto the models tab, where it described nothing on screen, and it survived
 * both moving the slider and a later successful route. A banner that outlives
 * the question it answered is a claim nobody measured.
 */
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const mockOpen = vi.fn();
vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: (...args: unknown[]) => mockOpen(...args),
}));

import { LongContextPanel } from '../LongContextPanel';

const MODELS = [
  {
    model_id: 'gemini-2.5-pro',
    name: 'gemini-2.5-pro',
    provider: 'gemini',
    max_tokens: 1_000_000,
    cost_per_1k_input: 0.0013,
    cost_per_1k_output: 0.005,
    supports_long_context: true,
  },
];

const CHOSEN = {
  input_tokens: 32_000,
  chosen_model: 'gemini-2.5-pro',
  provider: 'gemini',
  cost_estimate_usd: 0.04,
  reason: 'fits the window',
};

const TOO_BIG = 'No configured model has a context window large enough for 1010000 tokens';

// Braced, not an expression body: `mockReset()` returns the mock, and a hook
// that returns a function hands vitest a teardown — it then *calls the mock*
// after every test. Harmless for `invoke`; for a mock whose implementation
// throws, the teardown call fails the test that set it.
beforeEach(() => { mockInvoke.mockReset(); });

/** Render the panel with a model list, and wait for it to land. */
async function panel(route: (tokenCount: number) => Promise<unknown> = async () => CHOSEN) {
  mockInvoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
    if (cmd === 'long_context_models') return MODELS;
    if (cmd === 'long_context_route') return route(args.tokenCount as number);
    return null;
  });
  render(<LongContextPanel />);
  await waitFor(() => expect(screen.getByRole('button', { name: 'Find Best Model' })).toBeTruthy());
}

/** Ask the router for a verdict on whatever the slider currently reads. */
async function findBestModel() {
  fireEvent.click(screen.getByRole('button', { name: 'Find Best Model' }));
}

/** Move the slider to `tokens`. */
function setTokens(tokens: number) {
  fireEvent.change(screen.getByRole('slider'), { target: { value: String(tokens) } });
}

/** A router that refuses anything above a million tokens. */
const refusesHugeCounts = async (tokenCount: number) => {
  if (tokenCount > 1_000_000) throw new Error(TOO_BIG);
  return CHOSEN;
};

describe('LongContextPanel — a routing failure', () => {
  it('is shown on the routing tab when the router refuses the count', async () => {
    await panel(refusesHugeCounts);
    setTokens(1_010_000);
    await findBestModel();

    await waitFor(() => expect(screen.getByText(new RegExp(TOO_BIG))).toBeTruthy());
  });

  it('disappears when the slider moves off the count it was about', async () => {
    await panel(refusesHugeCounts);
    setTokens(1_010_000);
    await findBestModel();
    await waitFor(() => expect(screen.getByText(new RegExp(TOO_BIG))).toBeTruthy());

    // Nobody has routed 900k — saying it has no model would be inventing a result.
    setTokens(900_000);
    expect(screen.queryByText(new RegExp(TOO_BIG))).toBeNull();
  });

  it('is replaced by the model a later successful route chose', async () => {
    await panel(refusesHugeCounts);
    setTokens(1_010_000);
    await findBestModel();
    await waitFor(() => expect(screen.getByText(new RegExp(TOO_BIG))).toBeTruthy());

    setTokens(500_000);
    await findBestModel();

    await waitFor(() => expect(screen.getByText('gemini-2.5-pro')).toBeTruthy());
    expect(screen.queryByText(new RegExp(TOO_BIG))).toBeNull();
  });

  it('does not follow the reader onto the models tab', async () => {
    await panel(refusesHugeCounts);
    setTokens(1_010_000);
    await findBestModel();
    await waitFor(() => expect(screen.getByText(new RegExp(TOO_BIG))).toBeTruthy());

    fireEvent.click(screen.getByRole('button', { name: 'models' }));

    // The models table is what this tab is about; the routing verdict is not.
    expect(screen.getByText('Max Tokens')).toBeTruthy();
    expect(screen.queryByText(new RegExp(TOO_BIG))).toBeNull();
  });
});

describe('LongContextPanel — the model list', () => {
  it('reports its own failure to the whole panel, because nothing else has data', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'long_context_models') throw new Error('daemon unreachable');
      return null;
    });
    render(<LongContextPanel />);

    await waitFor(() => expect(screen.getByText(/daemon unreachable/)).toBeTruthy());
  });
});

/**
 * The ingest tab took an absolute path typed by hand and nothing else, so the
 * only way to find out a path was wrong was to run the ingest and read the
 * failure. Browse… hands the path to the field from the OS dialog.
 */
describe('LongContextPanel — choosing the file to ingest', () => {
  /** Open the panel on its ingest tab. */
  async function ingestTab() {
    await panel();
    fireEvent.click(screen.getByRole('button', { name: 'ingest' }));
    return screen.getByLabelText('File Path') as HTMLInputElement;
  }

  beforeEach(() => { mockOpen.mockReset(); });

  it('puts the picked file into the path field', async () => {
    mockOpen.mockResolvedValue('/Users/me/corpus/huge.txt');
    const input = await ingestTab();

    fireEvent.click(screen.getByRole('button', { name: /browse/i }));

    await waitFor(() => expect(input.value).toBe('/Users/me/corpus/huge.txt'));
    expect(screen.getByRole('button', { name: 'Start Ingest' })).not.toHaveProperty('disabled', true);
  });

  it('asks for a single file, not a directory', async () => {
    mockOpen.mockResolvedValue(null);
    await ingestTab();

    fireEvent.click(screen.getByRole('button', { name: /browse/i }));

    await waitFor(() => expect(mockOpen).toHaveBeenCalled());
    const opts = mockOpen.mock.calls[0][0] as { multiple: boolean; directory: boolean };
    expect(opts.multiple).toBe(false);
    expect(opts.directory).toBe(false);
  });

  it('leaves a typed path alone when the dialog is cancelled', async () => {
    mockOpen.mockResolvedValue(null);
    const input = await ingestTab();
    fireEvent.change(input, { target: { value: '/typed/by/hand.txt' } });

    fireEvent.click(screen.getByRole('button', { name: /browse/i }));

    await waitFor(() => expect(mockOpen).toHaveBeenCalled());
    expect(input.value).toBe('/typed/by/hand.txt');
  });

  it('says so when the dialog itself fails, rather than doing nothing', async () => {
    mockOpen.mockImplementation(async () => { throw new Error('dialog plugin not registered'); });
    await ingestTab();

    fireEvent.click(screen.getByRole('button', { name: /browse/i }));

    await waitFor(() =>
      expect(screen.getByRole('alert').textContent).toContain('dialog plugin not registered'),
    );
  });
});
