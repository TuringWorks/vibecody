import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Import after mocks ────────────────────────────────────────────────────

import { TurboQuantPanel } from '../TurboQuantPanel';

// ── Test data ──────────────────────────────────────────────────────────────

const mockStats = {
  num_vectors: 500,
  dimension: 384,
  compressed_bytes: 76000,
  uncompressed_bytes: 768000,
  compression_ratio: 10.1,
  bits_per_dimension: 3.17,
};

const emptyStats = {
  num_vectors: 0,
  dimension: 384,
  compressed_bytes: 0,
  uncompressed_bytes: 0,
  compression_ratio: 0,
  bits_per_dimension: 0,
};

const mockSearchResults = [
  { id: "chunk_0", score: 0.95, metadata: { file: "main.rs" } },
  { id: "chunk_3", score: 0.82, metadata: { file: "lib.rs" } },
  { id: "chunk_7", score: 0.71, metadata: { file: "utils.rs" } },
];

const mockBenchmark = {
  num_vectors: 500,
  dimension: 128,
  compressed_bytes: 19000,
  uncompressed_bytes: 256000,
  compression_ratio: 13.5,
  recall_at_10: 0.72,
  avg_query_ms: 1.3,
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
      case "turboquant_stats": return mockStats;
      case "turboquant_insert": return null;
      case "turboquant_search": return mockSearchResults;
      case "turboquant_benchmark": return mockBenchmark;
      case "turboquant_clear": return null;
      default: return null;
    }
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  setupMocks();
});

// ── Tests ──────────────────────────────────────────────────────────────────

describe('TurboQuantPanel', () => {
  // ── Rendering ─────────────────────────────────────────────────────────

  it('renders without crashing', async () => {
    render(<TurboQuantPanel />);
    await waitFor(() => {
      expect(screen.getByText('TurboQuant')).toBeInTheDocument();
    });
  });

  it('shows header with technique description', async () => {
    render(<TurboQuantPanel />);
    await waitFor(() => {
      expect(screen.getByText(/PolarQuant \+ QJL/)).toBeInTheDocument();
    });
  });

  it('renders all four tab buttons', () => {
    render(<TurboQuantPanel />);
    expect(screen.getByText('Overview')).toBeInTheDocument();
    expect(screen.getByText('Compress')).toBeInTheDocument();
    expect(screen.getByText('Search')).toBeInTheDocument();
    expect(screen.getByText('Benchmark')).toBeInTheDocument();
  });

  // ── Overview tab ──────────────────────────────────────────────────────

  it('loads stats on mount and displays them', async () => {
    render(<TurboQuantPanel />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('turboquant_stats');
    });
    await waitFor(() => {
      expect(screen.getByText('500')).toBeInTheDocument();
      expect(screen.getByText('384')).toBeInTheDocument();
      expect(screen.getByText('10.1\u00d7')).toBeInTheDocument(); // 10.1×
    });
  });

  it('shows compression ratio bar when vectors exist', async () => {
    render(<TurboQuantPanel />);
    await waitFor(() => {
      expect(screen.getByText('TQ')).toBeInTheDocument();
      expect(screen.getByText('f32')).toBeInTheDocument();
    });
  });

  it('handles empty index stats', async () => {
    setupMocks({ turboquant_stats: emptyStats });
    render(<TurboQuantPanel />);
    await waitFor(() => {
      expect(screen.getByText('0')).toBeInTheDocument();
    });
  });

  it('calls turboquant_clear on clear button', async () => {
    render(<TurboQuantPanel />);
    await waitFor(() => {
      expect(screen.getByText('Clear Index')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText('Clear Index'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('turboquant_clear');
    });
  });

  it('refreshes stats on Refresh button', async () => {
    render(<TurboQuantPanel />);
    await waitFor(() => {
      expect(screen.getByText('Refresh')).toBeInTheDocument();
    });
    mockInvoke.mockClear();
    fireEvent.click(screen.getByText('Refresh'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('turboquant_stats');
    });
  });

  // ── Compress tab ──────────────────────────────────────────────────────

  it('shows compress form when tab is clicked', async () => {
    render(<TurboQuantPanel />);
    fireEvent.click(screen.getByText('Compress'));
    await waitFor(() => {
      expect(screen.getByText('Compress & Insert')).toBeInTheDocument();
      expect(screen.getByPlaceholderText(/auto-generated/)).toBeInTheDocument();
    });
  });

  it('inserts vector via turboquant_insert', async () => {
    render(<TurboQuantPanel />);
    fireEvent.click(screen.getByText('Compress'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('0.12, -0.45, 0.78, ...')).toBeInTheDocument();
    });

    const textarea = screen.getByPlaceholderText('0.12, -0.45, 0.78, ...');
    fireEvent.change(textarea, { target: { value: '0.1, 0.2, 0.3' } });

    const idInput = screen.getByPlaceholderText(/auto-generated/);
    fireEvent.change(idInput, { target: { value: 'test_vec' } });

    fireEvent.click(screen.getByText('Compress & Insert'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('turboquant_insert', {
        id: 'test_vec',
        vector: [0.1, 0.2, 0.3],
      });
    });
  });

  it('shows error for invalid vector input', async () => {
    render(<TurboQuantPanel />);
    fireEvent.click(screen.getByText('Compress'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('0.12, -0.45, 0.78, ...')).toBeInTheDocument();
    });

    const textarea = screen.getByPlaceholderText('0.12, -0.45, 0.78, ...');
    fireEvent.change(textarea, { target: { value: 'not, a, number' } });
    fireEvent.click(screen.getByText('Compress & Insert'));

    await waitFor(() => {
      expect(screen.getByText(/Invalid vector/)).toBeInTheDocument();
    });
  });

  // ── Search tab ────────────────────────────────────────────────────────

  it('shows search form when tab is clicked', async () => {
    render(<TurboQuantPanel />);
    fireEvent.click(screen.getByText('Search'));
    await waitFor(() => {
      expect(screen.getByText('Query vector (comma-separated)')).toBeInTheDocument();
    });
  });

  it('performs search and displays results', async () => {
    render(<TurboQuantPanel />);
    fireEvent.click(screen.getByText('Search'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/query:/)).toBeInTheDocument();
    });

    const textarea = screen.getByPlaceholderText(/query:/);
    fireEvent.change(textarea, { target: { value: '0.5, 0.5, 0.5' } });

    // The accent-colored Search button (not the tab) — find by role
    const searchButtons = screen.getAllByRole('button', { name: 'Search' });
    // The last one is the action button (the first is the tab)
    fireEvent.click(searchButtons[searchButtons.length - 1]);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('turboquant_search', {
        vector: [0.5, 0.5, 0.5],
        topK: 10,
      });
    });

    await waitFor(() => {
      expect(screen.getByText('chunk_0')).toBeInTheDocument();
      expect(screen.getByText('chunk_3')).toBeInTheDocument();
      expect(screen.getByText('0.9500')).toBeInTheDocument();
    });
  });

  // ── Benchmark tab ─────────────────────────────────────────────────────

  it('shows benchmark form when tab is clicked', async () => {
    render(<TurboQuantPanel />);
    fireEvent.click(screen.getByText('Benchmark'));
    await waitFor(() => {
      expect(screen.getByText('Run Benchmark')).toBeInTheDocument();
    });
  });

  it('runs benchmark and displays results', async () => {
    render(<TurboQuantPanel />);
    fireEvent.click(screen.getByText('Benchmark'));

    await waitFor(() => {
      expect(screen.getByText('Run Benchmark')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Run Benchmark'));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('turboquant_benchmark', {
        numVectors: 500,
        dimension: 128,
      });
    });

    await waitFor(() => {
      expect(screen.getByText('13.5\u00d7')).toBeInTheDocument(); // 13.5×
      expect(screen.getByText('72%')).toBeInTheDocument(); // recall
    });
  });

  // ── Error handling ────────────────────────────────────────────────────

  it('handles stats loading error gracefully', async () => {
    setupMocks({ turboquant_stats: new Error('connection failed') });
    render(<TurboQuantPanel />);
    // Should not crash
    await waitFor(() => {
      expect(screen.getByText('TurboQuant')).toBeInTheDocument();
    });
  });

  it('handles insert error gracefully', async () => {
    setupMocks({ turboquant_insert: new Error('dimension mismatch') });
    render(<TurboQuantPanel />);
    fireEvent.click(screen.getByText('Compress'));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('0.12, -0.45, 0.78, ...')).toBeInTheDocument();
    });

    const textarea = screen.getByPlaceholderText('0.12, -0.45, 0.78, ...');
    fireEvent.change(textarea, { target: { value: '0.1, 0.2' } });
    fireEvent.click(screen.getByText('Compress & Insert'));

    await waitFor(() => {
      expect(screen.getByText(/Error:/)).toBeInTheDocument();
    });
  });

  // ── Tab switching ─────────────────────────────────────────────────────

  it('switches between tabs correctly', async () => {
    render(<TurboQuantPanel />);

    // Default is Overview
    await waitFor(() => {
      expect(screen.getByText('Refresh')).toBeInTheDocument();
    });

    // Switch to Compress
    fireEvent.click(screen.getByText('Compress'));
    await waitFor(() => {
      expect(screen.getByText('Compress & Insert')).toBeInTheDocument();
    });

    // Switch to Benchmark
    fireEvent.click(screen.getByText('Benchmark'));
    await waitFor(() => {
      expect(screen.getByText('Run Benchmark')).toBeInTheDocument();
    });

    // Switch back to Overview
    fireEvent.click(screen.getByText('Overview'));
    await waitFor(() => {
      expect(screen.getByText('Refresh')).toBeInTheDocument();
    });
  });
});
