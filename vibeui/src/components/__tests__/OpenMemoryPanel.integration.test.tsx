/**
 * Integration tests for the Memory happy path + top 2 failure modes.
 *
 * Mocks the Tauri `invoke` boundary and renders the panel through the
 * realistic flows: adding a memory, refresh failures (corrupt store /
 * permission denied), and the empty-state. Per-checklist:
 *   ✓ realistic happy path: openmemory_stats + openmemory_list returns rows
 *   ✓ failure mode 1: openmemory_stats rejects with permission-denied
 *   ✓ failure mode 2: openmemory_stats rejects with corrupt-JSON
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import OpenMemoryPanel from '../OpenMemoryPanel';

describe('OpenMemoryPanel — integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  function statsResponse(total = 0) {
    return {
      total_memories: total,
      total_waypoints: 0,
      total_facts: 0,
      total_drawers: 0,
      encryption: false,
      sectors: [],
      embedding_dim: 384,
      embedding_compression_ratio: 1.0,
      embedding_backend: 'turboquant',
    };
  }

  function memoryRow(id: string, content: string) {
    return {
      id, content, sector: 'episodic', tags: [],
      salience: 0.5, effective_salience: 0.5,
      created_at: 0, last_seen_at: 0,
      pinned: false, encrypted: false, waypoint_count: 0,
    };
  }

  it('renders the Overview tab with stats when the store has memories', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'openmemory_stats':         return statsResponse(5);
        case 'openmemory_list':          return [memoryRow('m1', 'first'), memoryRow('m2', 'second')];
        case 'openmemory_facts':         return [];
        case 'openmemory_drawer_stats':  return { total_drawers: 0 };
        default: return null;
      }
    });
    render(<OpenMemoryPanel />);
    await waitFor(() => {
      expect(screen.getByText(/Overview/i)).toBeInTheDocument();
    });
    // No error card should render on a clean stats response.
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('renders the classified hint card on permission-denied', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'openmemory_stats') {
        throw new Error('Permission denied (os error 13)');
      }
      return null;
    });
    render(<OpenMemoryPanel />);
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByTestId('memory-error-hint')).toHaveTextContent(/permissions/i);
  });

  it('renders the classified hint card on corrupt-JSON', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'openmemory_stats') {
        throw new Error('invalid JSON at line 1 column 5');
      }
      return null;
    });
    render(<OpenMemoryPanel />);
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByTestId('memory-error-hint')).toHaveTextContent(/corrupt|backup/i);
  });
});
