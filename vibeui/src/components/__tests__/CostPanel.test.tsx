import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Import after mocks ────────────────────────────────────────────────────

import { CostPanel } from '../CostPanel';

// ── Test data ──────────────────────────────────────────────────────────────

const mockMetrics = {
  entries: [
    { session_id: "s1", provider: "ollama", model: "llama3.2", prompt_tokens: 1500, completion_tokens: 500, cost_usd: 0.002, timestamp_ms: Date.now() - 60000, task_hint: "Fix bug" },
    { session_id: "s2", provider: "claude", model: "claude-3-5-sonnet", prompt_tokens: 3000, completion_tokens: 1000, cost_usd: 0.04, timestamp_ms: Date.now() - 30000, task_hint: null },
    { session_id: "s3", provider: "openai", model: "gpt-4o", prompt_tokens: 2000, completion_tokens: 800, cost_usd: 0.03, timestamp_ms: Date.now() - 10000, task_hint: "Write tests" },
  ],
  by_provider: [
    { provider: "ollama", total_cost_usd: 0.002, total_tokens: 2000, call_count: 1 },
    { provider: "claude", total_cost_usd: 0.04, total_tokens: 4000, call_count: 1 },
    { provider: "openai", total_cost_usd: 0.03, total_tokens: 2800, call_count: 1 },
  ],
  total_cost_usd: 0.072,
  total_tokens: 8800,
  budget_limit_usd: 10.0,
  budget_remaining_usd: 9.928,
};

const emptyMetrics = {
  entries: [],
  by_provider: [],
  total_cost_usd: 0,
  total_tokens: 0,
  budget_limit_usd: null,
  budget_remaining_usd: null,
};

// ── Setup ──────────────────────────────────────────────────────────────────

function setupMocks(overrides: Record<string, unknown> = {}) {
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (overrides[cmd] !== undefined) {
      const val = overrides[cmd];
      if (val instanceof Error) throw val;
      return val;
    }
    switch (cmd) {
      case "get_cost_metrics": return mockMetrics;
      case "set_cost_limit": return null;
      case "clear_cost_history": return null;
      default: return null;
    }
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  setupMocks();
});

// ── Tests ──────────────────────────────────────────────────────────────────

describe('CostPanel', () => {
  // ── Rendering ─────────────────────────────────────────────────────────

  it('renders without crashing', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('Cost & Performance Observatory')).toBeInTheDocument();
    });
  });

  it('loads cost metrics on mount', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_cost_metrics");
    });
  });

  it('shows loading state initially', () => {
    // Delay the mock so loading is visible
    mockInvoke.mockImplementation(() => new Promise(() => {}));
    render(<CostPanel />);
    expect(screen.getByText('Loading…')).toBeInTheDocument();
  });

  // ── Summary cards ─────────────────────────────────────────────────────

  it('displays total cost', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('$0.0720')).toBeInTheDocument();
    });
  });

  it('displays total tokens', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('8.8K')).toBeInTheDocument();
    });
  });

  it('displays AI call count', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('3')).toBeInTheDocument();
      expect(screen.getByText('AI Calls')).toBeInTheDocument();
    });
  });

  // ── Budget ────────────────────────────────────────────────────────────

  it('shows Monthly Budget Limit section', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('Monthly Budget Limit')).toBeInTheDocument();
    });
  });

  it('shows budget remaining when budget is set', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText(/remaining/)).toBeInTheDocument();
    });
  });

  it('shows Save button for budget', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('Save')).toBeInTheDocument();
    });
  });

  // ── Provider breakdown ────────────────────────────────────────────────

  it('shows Cost by Provider section', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('Cost by Provider')).toBeInTheDocument();
    });
  });

  it('lists individual providers', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      // Provider names appear in both "by_provider" section and "entries" section
      expect(screen.getAllByText('ollama').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('claude').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('openai').length).toBeGreaterThanOrEqual(1);
    });
  });

  // ── Recent calls ──────────────────────────────────────────────────────

  it('shows Recent Calls section', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('Recent Calls')).toBeInTheDocument();
    });
  });

  it('shows Clear history button', async () => {
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText('Clear history')).toBeInTheDocument();
    });
  });

  // ── Empty state ───────────────────────────────────────────────────────

  it('shows empty state when no cost records', async () => {
    setupMocks({ get_cost_metrics: emptyMetrics });
    render(<CostPanel />);
    await waitFor(() => {
      expect(screen.getByText(/No cost records yet/)).toBeInTheDocument();
    });
  });
});
