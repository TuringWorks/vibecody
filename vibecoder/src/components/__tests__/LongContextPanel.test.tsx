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

beforeEach(() => mockInvoke.mockReset());

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
