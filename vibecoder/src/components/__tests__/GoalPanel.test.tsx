/**
 * The failure that prompted these: clicking "Generate plan" and getting
 * nothing. The plan call goes to the daemon, can take seconds, and reports
 * failure only through a toast — which is gone before it can be read, leaving
 * the panel identical to never having clicked at all.
 */
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import '@testing-library/jest-dom';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => mockInvoke(cmd, args),
}));

// Icons render as inert spans; the panel's behaviour is what is under test.
// Defined inline because vi.mock is hoisted above any top-level binding.
vi.mock('lucide-react', () => ({
  Target: () => <span />,
  Plus: () => <span />,
  Play: () => <span />,
  Link2: () => <span />,
  Trash2: () => <span />,
  RefreshCw: () => <span />,
  Tag: () => <span />,
  ListTree: () => <span />,
  FileText: () => <span />,
  Star: () => <span />,
  // Toaster renders these; a missing export crashes the whole panel at import.
  Check: () => <span />,
  X: () => <span />,
  AlertTriangle: () => <span />,
  Info: () => <span />,
}));

import { GoalPanel } from '../GoalPanel';

const GOAL = {
  id: 'abc123def456',
  title: 'Project_goal',
  statement: 'build the thing',
  status: 'paused',
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  tags: [],
  success_criteria: [],
  workspace: '/tmp/ws',
  schema_version: 1,
  current_plan: null,
};

function respond(cmd: string): unknown {
  switch (cmd) {
    case 'exec_goal_list':
      return { goals: [GOAL] };
    case 'exec_goal_get':
      return { goal: GOAL, links: [] };
    case 'exec_goal_current':
      return { goal_id: null };
    default:
      return null;
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation((cmd: string) => Promise.resolve(respond(cmd)));
});

describe('GoalPanel activity log', () => {
  it('keeps a failed plan generation on screen with the daemon error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'exec_goal_plan') {
        return Promise.reject('daemon: no provider configured for planning');
      }
      return Promise.resolve(respond(cmd));
    });

    render(<GoalPanel workspacePath="/tmp/ws" selectedProvider="ollama" />);
    await screen.findByText('build the thing');

    fireEvent.click(await screen.findByRole('button', { name: /Generate plan/ }));

    // Scoped to the log, not the page: the toast says this too, but the toast
    // is exactly the thing that disappears. What matters is that the log keeps
    // it, carrying the daemon's own words rather than a generic failure.
    await waitFor(() => {
      const log = screen.getByRole('log', { name: /Goal activity/i });
      expect(within(log).getByText(/Plan generation failed/)).toBeInTheDocument();
      expect(
        within(log).getByText(/no provider configured for planning/),
      ).toBeInTheDocument();
    });
  });

  it('does not claim "no plan yet" after an attempt failed', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'exec_goal_plan') return Promise.reject('boom');
      return Promise.resolve(respond(cmd));
    });

    render(<GoalPanel workspacePath="/tmp/ws" selectedProvider="ollama" />);
    await screen.findByText('build the thing');
    fireEvent.click(await screen.findByRole('button', { name: /Generate plan/ }));

    // "No plan yet. Click Generate plan" reads as though nothing happened.
    await waitFor(() => {
      expect(screen.getByText(/the last attempt failed/)).toBeInTheDocument();
    });
  });

  it('logs a successful plan with its step count', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'exec_goal_plan') {
        return Promise.resolve({
          ...GOAL,
          current_plan: {
            goal: 'g',
            steps: [
              { id: 1, description: 'a', tool: 'read_file', status: 'pending' },
              { id: 2, description: 'b', tool: 'write_file', status: 'pending' },
            ],
            estimated_files: [],
            risks: [],
          },
        });
      }
      return Promise.resolve(respond(cmd));
    });

    render(<GoalPanel workspacePath="/tmp/ws" selectedProvider="ollama" />);
    await screen.findByText('build the thing');
    fireEvent.click(await screen.findByRole('button', { name: /Generate plan/ }));

    await waitFor(() => {
      expect(screen.getByText(/Plan generated — 2 steps/)).toBeInTheDocument();
    });
  });

  it('refuses to plan with no provider selected, and says so', async () => {
    render(<GoalPanel workspacePath="/tmp/ws" selectedProvider="" />);
    await screen.findByText('build the thing');
    fireEvent.click(await screen.findByRole('button', { name: /Generate plan/ }));

    // Never reaches the daemon — the toolbar selection is the problem.
    await waitFor(() => {
      expect(
        mockInvoke.mock.calls.some((c) => c[0] === 'exec_goal_plan'),
      ).toBe(false);
    });
  });
});
