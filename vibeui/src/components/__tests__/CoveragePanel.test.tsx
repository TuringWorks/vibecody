import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Mock @tauri-apps/api/event (used by CoveragePanel for coverage:log) ────

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// ── Mock lucide-react icons ────────────────────────────────────────────────

vi.mock('lucide-react', () => {
  const icon = (name: string) => (props: Record<string, unknown>) => <span data-testid={`icon-${name}`} {...props} />;
  return {
    CircleCheck: icon('circlecheck'),
    FlaskConical: icon('flask'),
    Loader2: icon('loader'),
    Play: icon('play'),
  };
});

// ── Import after mocks ────────────────────────────────────────────────────

import { CoveragePanel } from '../CoveragePanel';

// ── Test data ──────────────────────────────────────────────────────────────

const mockCoverageResult = {
  framework: "cargo-llvm-cov",
  total_pct: 72.5,
  files: [
    { path: "src/main.rs", covered: 80, total: 100, pct: 80.0, uncovered_lines: [15, 22, 45, 88] },
    { path: "src/lib.rs", covered: 50, total: 100, pct: 50.0, uncovered_lines: [10, 20, 30, 40, 50] },
    { path: "src/utils.rs", covered: 100, total: 100, pct: 100.0, uncovered_lines: [] },
    { path: "src/config.rs", covered: 0, total: 50, pct: 0.0, uncovered_lines: [1, 2, 3, 4, 5] },
  ],
  raw_output: "Coverage report generated successfully.\nTotal: 72.5%",
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
      case "detect_coverage_tool": return "cargo-llvm-cov";
      case "run_coverage": return mockCoverageResult;
      default: return null;
    }
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  setupMocks();
  // jsdom doesn't implement scrollIntoView — stub it so CoveragePanel's auto-scroll doesn't throw
  window.HTMLElement.prototype.scrollIntoView = vi.fn();
});

// ── Tests ──────────────────────────────────────────────────────────────────

describe('CoveragePanel', () => {
  // ── Rendering ─────────────────────────────────────────────────────────

  it('renders without crashing', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('Coverage')).toBeInTheDocument();
    });
  });

  it('detects coverage tool on mount', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("detect_coverage_tool", { workspace: "/workspace" });
    });
  });

  it('displays detected tool label', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('Cargo llvm-cov')).toBeInTheDocument();
    });
  });

  it('shows Run Coverage button', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('Run Coverage')).toBeInTheDocument();
    });
  });

  it('shows "No workspace open" when workspacePath is null', () => {
    setupMocks({ detect_coverage_tool: new Error("no workspace") });
    render(<CoveragePanel workspacePath={null} />);
    expect(screen.getByText('No workspace open')).toBeInTheDocument();
  });

  it('shows "No coverage tool detected" when tool detection fails', async () => {
    setupMocks({ detect_coverage_tool: new Error("not found") });
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('No coverage tool detected')).toBeInTheDocument();
    });
  });

  // ── Running coverage ──────────────────────────────────────────────────

  it('runs coverage and displays total percentage', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Run Coverage'));
    fireEvent.click(screen.getByText('Run Coverage'));
    await waitFor(() => {
      expect(screen.getByText('72.5%')).toBeInTheDocument();
    });
  });

  it('shows file count after running coverage', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Run Coverage'));
    fireEvent.click(screen.getByText('Run Coverage'));
    await waitFor(() => {
      expect(screen.getByText('4 files')).toBeInTheDocument();
    });
  });

  it('displays error when coverage run fails', async () => {
    setupMocks({ run_coverage: new Error("Build failed") });
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Run Coverage'));
    fireEvent.click(screen.getByText('Run Coverage'));
    await waitFor(() => {
      expect(screen.getByText(/Build failed/)).toBeInTheDocument();
    });
  });

  // ── Filter tabs ───────────────────────────────────────────────────────

  it('shows filter tabs after running coverage', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    // Wait for tool detection so the button becomes enabled before clicking
    await waitFor(() => {
      const btn = screen.getByText('Run Coverage').closest('button');
      expect(btn).not.toBeDisabled();
    });
    fireEvent.click(screen.getByText('Run Coverage'));
    await waitFor(() => {
      expect(screen.getByText(/All \(4\)/)).toBeInTheDocument();
      expect(screen.getByText(/Partial/)).toBeInTheDocument();
      expect(screen.getByText(/Uncovered/)).toBeInTheDocument();
    });
  });

  it('shows Raw button to toggle raw output', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Run Coverage'));
    fireEvent.click(screen.getByText('Run Coverage'));
    await waitFor(() => {
      expect(screen.getByText('Raw')).toBeInTheDocument();
    });
  });

  it('shows raw output when Raw button clicked', async () => {
    render(<CoveragePanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Run Coverage'));
    fireEvent.click(screen.getByText('Run Coverage'));
    await waitFor(() => screen.getByText('Raw'));
    fireEvent.click(screen.getByText('Raw'));
    await waitFor(() => {
      expect(screen.getByText(/Coverage report generated successfully/)).toBeInTheDocument();
    });
  });
});
