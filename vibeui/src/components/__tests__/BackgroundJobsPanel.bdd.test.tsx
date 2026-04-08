/**
 * BDD tests for BackgroundJobsPanel — daemon integration patterns.
 *
 * Scenarios:
 *  1. Shows offline indicator when daemon is unreachable
 *  2. Shows online indicator when daemon responds
 *  3. Job list is rendered when daemon returns jobs
 *  4. Submit button disabled while daemon offline or task is empty
 *  5. Submitting a task calls POST /agent with correct body
 *  6. Cancel calls POST /jobs/{id}/cancel
 *  7. vibeui:daemon-status custom event updates online state
 *  8. Refreshes job list when vibeui:daemon-status fires with online=true
 *  9. No jobs message shown when list is empty and daemon is online
 * 10. Raw output start text in error panel references vibecli serve command
 */

import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Mocks ──────────────────────────────────────────────────────────────────────

vi.mock('lucide-react', () => {
  const icon = (name: string) => () => <span data-testid={`icon-${name}`} />;
  return {
    Clock: icon('clock'),
    CircleCheck: icon('circlecheck'),
    CircleX: icon('circlex'),
    Square: icon('square'),
    Loader2: icon('loader'),
    Play: icon('play'),
  };
});

vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    toast: { success: vi.fn(), error: vi.fn(), warn: vi.fn(), info: vi.fn() },
    dismiss: vi.fn(),
  }),
}));

vi.mock('../Toaster', () => ({
  Toaster: () => null,
}));

vi.mock('../../hooks/useModelRegistry', () => ({
  useModelRegistry: () => ({
    providers: ['ollama', 'openai', 'anthropic'],
    models: [],
  }),
}));

import { BackgroundJobsPanel } from '../BackgroundJobsPanel';

// ── Test fixtures ──────────────────────────────────────────────────────────────

const runningJob = {
  session_id: 'job-1',
  task: 'Refactor auth module',
  status: 'running' as const,
  provider: 'ollama',
  started_at: Date.now() - 5000,
};

const completedJob = {
  session_id: 'job-2',
  task: 'Write unit tests for utils',
  status: 'complete' as const,
  provider: 'openai',
  started_at: Date.now() - 60000,
  finished_at: Date.now() - 1000,
  summary: 'Added 12 tests with 100% pass rate.',
};

// ── fetch mock helpers ─────────────────────────────────────────────────────────

function mockFetchOffline() {
  vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('ECONNREFUSED')));
}

function mockFetchOnline(jobs: unknown[] = []) {
  vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string, opts?: RequestInit) => {
    const method = opts?.method ?? 'GET';
    if (method === 'GET' && String(url).endsWith('/jobs')) {
      return { ok: true, json: async () => jobs };
    }
    if (method === 'POST' && String(url).includes('/agent')) {
      return { ok: true, json: async () => ({ session_id: 'new-job' }) };
    }
    if (method === 'POST' && String(url).includes('/cancel')) {
      return { ok: true, json: async () => ({}) };
    }
    return { ok: true, json: async () => ({}) };
  }));
}

// ── Setup ──────────────────────────────────────────────────────────────────────

beforeEach(() => {
  vi.clearAllMocks();
  mockFetchOffline();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

// ── Scenario 1: Offline indicator ─────────────────────────────────────────────

describe('Given the vibecli daemon is not running', () => {
  it('When the panel mounts, Then the offline indicator is shown', async () => {
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => {
      expect(screen.getByText(/offline/i)).toBeInTheDocument();
    });
  });

  it('When the panel mounts, Then the "Daemon not running" error message appears', async () => {
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => {
      expect(screen.getByText(/Daemon not running/i)).toBeInTheDocument();
    });
  });

  it('When the panel mounts, Then the vibecli serve command is mentioned', async () => {
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => {
      expect(screen.getByText(/vibecli/i)).toBeInTheDocument();
    });
  });
});

// ── Scenario 2: Online indicator ──────────────────────────────────────────────

describe('Given the vibecli daemon is online', () => {
  beforeEach(() => mockFetchOnline());

  it('When the panel mounts, Then the online indicator is shown', async () => {
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => {
      expect(screen.getByText(/online/i)).toBeInTheDocument();
    });
  });
});

// ── Scenario 3: Job list rendering ────────────────────────────────────────────

describe('Given the daemon returns a list of jobs', () => {
  beforeEach(() => mockFetchOnline([runningJob, completedJob]));

  it('When the panel mounts, Then the running job task is visible', async () => {
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => {
      expect(screen.getByText('Refactor auth module')).toBeInTheDocument();
    });
  });

  it('When the panel mounts, Then the completed job task is visible', async () => {
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => {
      expect(screen.getByText('Write unit tests for utils')).toBeInTheDocument();
    });
  });

  it('When the panel mounts, Then a Cancel button is shown for the running job', async () => {
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => {
      expect(screen.getByText('Cancel')).toBeInTheDocument();
    });
  });
});

// ── Scenario 4: Submit button disabled states ─────────────────────────────────

describe('Given the task input is empty', () => {
  it('When the daemon is online, Then Submit is disabled', async () => {
    mockFetchOnline();
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => screen.getByRole('button', { name: /Submit/i }));
    const btn = screen.getByRole('button', { name: /Submit/i }).closest('button')!;
    expect(btn).toBeDisabled();
  });
});

describe('Given the daemon is offline', () => {
  it('When the user types a task, Then Submit is still disabled', async () => {
    mockFetchOffline();
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    const textarea = screen.getByPlaceholderText(/background agent task/i);
    fireEvent.change(textarea, { target: { value: 'do something' } });
    await waitFor(() => screen.getByRole('button', { name: /Submit/i }));
    const btn = screen.getByRole('button', { name: /Submit/i }).closest('button')!;
    expect(btn).toBeDisabled();
  });
});

// ── Scenario 5: Submit calls POST /agent ──────────────────────────────────────

describe('Given the daemon is online and the user enters a task', () => {
  it('When Submit is clicked, Then POST /agent is called with the task', async () => {
    const fetchMock = vi.fn().mockImplementation(async (_url: string, opts?: RequestInit) => {
      const method = opts?.method ?? 'GET';
      if (method === 'GET') return { ok: true, json: async () => [] };
      return { ok: true, json: async () => ({}) };
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => screen.getByText(/online/i));

    const textarea = screen.getByPlaceholderText(/background agent task/i);
    fireEvent.change(textarea, { target: { value: 'Implement feature X' } });

    await waitFor(() => {
      const btn = screen.getByRole('button', { name: /Submit/i }).closest('button')!;
      expect(btn).not.toBeDisabled();
    });

    fireEvent.click(screen.getByRole('button', { name: /Submit/i }));

    await waitFor(() => {
      const calls: unknown[][] = fetchMock.mock.calls;
      const postCall = calls.find((c: unknown[]) =>
        String(c[0]).includes('/agent') && (c[1] as RequestInit)?.method === 'POST'
      );
      expect(postCall).toBeDefined();
      const body = JSON.parse((postCall![1] as RequestInit).body as string);
      expect(body.task).toBe('Implement feature X');
    });
  });

  it('When Submit is clicked, Then the task textarea is cleared on success', async () => {
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (_url: string, opts?: RequestInit) => {
      const method = opts?.method ?? 'GET';
      if (method === 'GET') return { ok: true, json: async () => [] };
      return { ok: true, json: async () => ({}) };
    }));

    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => screen.getByText(/online/i));

    const textarea = screen.getByPlaceholderText(/background agent task/i);
    fireEvent.change(textarea, { target: { value: 'My task' } });

    await waitFor(() => {
      const btn = screen.getByRole('button', { name: /Submit/i }).closest('button')!;
      expect(btn).not.toBeDisabled();
    });
    fireEvent.click(screen.getByRole('button', { name: /Submit/i }));

    await waitFor(() => {
      expect((textarea as HTMLTextAreaElement).value).toBe('');
    });
  });
});

// ── Scenario 6: Cancel calls POST /jobs/{id}/cancel ───────────────────────────

describe('Given a running job is visible', () => {
  it('When Cancel is clicked, Then POST /jobs/{id}/cancel is called', async () => {
    const fetchMock = vi.fn().mockImplementation(async (url: string, opts?: RequestInit) => {
      const method = opts?.method ?? 'GET';
      if (method === 'GET' && String(url).endsWith('/jobs')) {
        return { ok: true, json: async () => [runningJob] };
      }
      return { ok: true, json: async () => ({}) };
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => screen.getByText('Cancel'));
    fireEvent.click(screen.getByText('Cancel'));

    await waitFor(() => {
      const cancelCall = fetchMock.mock.calls.find((c: unknown[]) =>
        String(c[0]).includes('/cancel') && (c[1] as RequestInit)?.method === 'POST'
      );
      expect(cancelCall).toBeDefined();
      expect(cancelCall![0]).toContain('job-1');
    });
  });
});

// ── Scenario 7: vibeui:daemon-status custom event ─────────────────────────────

describe('Given the vibeui:daemon-status event fires', () => {
  it('When online:true is dispatched, Then the online indicator appears', async () => {
    mockFetchOnline();
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);

    act(() => {
      window.dispatchEvent(new CustomEvent('vibeui:daemon-status', {
        detail: { online: true, checkedAt: Date.now() },
      }));
    });

    await waitFor(() => {
      expect(screen.getByText(/online/i)).toBeInTheDocument();
    });
  });

  it('When online:false is dispatched, Then the offline indicator appears', async () => {
    mockFetchOnline();
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => screen.getByText(/online/i));

    act(() => {
      window.dispatchEvent(new CustomEvent('vibeui:daemon-status', {
        detail: { online: false, checkedAt: Date.now() },
      }));
    });

    await waitFor(() => {
      expect(screen.getByText(/offline/i)).toBeInTheDocument();
    });
  });

  it('When online:true is dispatched, Then GET /jobs is called to refresh the list', async () => {
    const fetchMock = vi.fn().mockImplementation(async (url: string) => {
      if (String(url).endsWith('/jobs')) return { ok: true, json: async () => [] };
      return { ok: true, json: async () => ({}) };
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);

    const callsBefore = fetchMock.mock.calls.length;
    act(() => {
      window.dispatchEvent(new CustomEvent('vibeui:daemon-status', {
        detail: { online: true, checkedAt: Date.now() },
      }));
    });

    await waitFor(() => {
      expect(fetchMock.mock.calls.length).toBeGreaterThan(callsBefore);
    });
  });
});

// ── Scenario 8: Empty job list message ───────────────────────────────────────

describe('Given the daemon is online but no jobs exist', () => {
  it('When the panel renders, Then the empty-state message is shown', async () => {
    mockFetchOnline([]);
    render(<BackgroundJobsPanel daemonUrl="http://localhost:7878" />);
    await waitFor(() => {
      expect(screen.getByText(/No jobs yet/i)).toBeInTheDocument();
    });
  });
});

// ── Scenario 9: Custom daemonUrl prop ─────────────────────────────────────────

describe('Given a custom daemonUrl is provided', () => {
  it('When the panel mounts, Then it fetches from the custom URL', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => [] });
    vi.stubGlobal('fetch', fetchMock);

    render(<BackgroundJobsPanel daemonUrl="http://localhost:9000" />);
    await waitFor(() => {
      const urls: string[] = fetchMock.mock.calls.map((c: unknown[]) => String(c[0]));
      expect(urls.some(u => u.includes('localhost:9000'))).toBe(true);
    });
  });
});
