/**
 * BDD tests for AgentPanel — agent task submission and lifecycle.
 *
 * Scenarios:
 *  1. Run button is disabled when task is empty or provider is empty
 *  2. Clicking Run calls invoke("start_agent_task") with task + policy + provider
 *  3. During run: textarea and Run button are disabled, Stop button appears
 *  4. Stop calls invoke("stop_agent_task") and resets to idle
 *  5. Turbo Mode toggle sets approval policy to "full-auto"
 *  6. Disabling Turbo Mode reverts approval policy to "auto-edit"
 *  7. Changing approval dropdown to full-auto activates Turbo Mode
 *  8. "agent:complete" event sets status to complete, shows Reset button
 *  9. "agent:error" event sets status to error, shows Retry button
 * 10. Approval prompt shows Approve/Reject buttons when pending call arrives
 * 11. Approve calls invoke("respond_to_agent_approval", { approved: true })
 * 12. Reject calls invoke("respond_to_agent_approval", { approved: false })
 * 13. Reset clears task, steps, and sets status to idle
 * 14. Missing provider shows a warning message
 * 15. ⌘Enter / Ctrl+Enter triggers Run
 */

import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Mocks ──────────────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// Colletable listen callbacks so tests can fire Tauri events
const eventHandlers: Record<string, (payload: unknown) => void> = {};
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, handler: (e: { payload: unknown }) => void) => {
    eventHandlers[event] = (payload) => handler({ payload });
    return () => { delete eventHandlers[event]; };
  }),
}));

vi.mock('lucide-react', () => {
  const icon = (name: string) => () => <span data-testid={`icon-${name}`} />;
  return { Bot: icon('bot'), Loader2: icon('loader'), Square: icon('square'), Zap: icon('zap') };
});

vi.mock('../../utils/FlowContext', () => ({
  flowContext: { add: vi.fn() },
}));

vi.mock('../../utils/LinterIntegration', () => ({
  runLinter: vi.fn().mockResolvedValue({ errors: [], warnings: [] }),
  formatLintForAgent: vi.fn().mockReturnValue(''),
}));

vi.mock('../Toaster', () => ({ Toaster: () => null }));

vi.mock('../AgentUIRenderer', () => ({
  AgentUIRenderer: () => null,
  parseVibeUIBlocks: vi.fn().mockReturnValue([]),
  stripVibeUIBlocks: vi.fn((s: string) => s),
}));

// Replace useToast with a real-ish inline version so toast.error works
vi.mock('../../hooks/useToast', () => ({
  useToast: () => ({
    toasts: [],
    toast: { success: vi.fn(), error: vi.fn(), info: vi.fn(), warn: vi.fn() },
    dismiss: vi.fn(),
  }),
}));

import { AgentPanel } from '../AgentPanel';

// ── Helpers ────────────────────────────────────────────────────────────────────

function renderPanel(provider = 'ollama', workspacePath = '/workspace') {
  return render(<AgentPanel provider={provider} workspacePath={workspacePath} />);
}

function fillTask(text: string) {
  fireEvent.change(screen.getByPlaceholderText(/Add a \/health endpoint/i), {
    target: { value: text },
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  Object.keys(eventHandlers).forEach(k => delete eventHandlers[k]);
  mockInvoke.mockResolvedValue(undefined);
  // jsdom doesn't implement scrollIntoView
  window.HTMLElement.prototype.scrollIntoView = vi.fn();
});

afterEach(() => vi.restoreAllMocks());

// ── Scenario 1: Run disabled when task/provider missing ───────────────────────

describe('Given the task textarea is empty', () => {
  it('When the component mounts, Then the Run button is disabled', () => {
    renderPanel();
    expect(screen.getByRole('button', { name: /Run/i })).toBeDisabled();
  });
});

describe('Given no provider is set', () => {
  it('When a task is typed, Then Run is still disabled', () => {
    renderPanel('');
    fillTask('Write a test');
    expect(screen.getByRole('button', { name: /Run/i })).toBeDisabled();
  });

  it('When no provider is set, Then a warning message appears', () => {
    renderPanel('');
    expect(screen.getByText(/Select an AI provider/i)).toBeInTheDocument();
  });
});

// ── Scenario 2: Run calls invoke("start_agent_task") ─────────────────────────

describe('Given a task is entered and a provider is set', () => {
  it('When Run is clicked, Then invoke("start_agent_task") is called with the task', async () => {
    renderPanel('ollama');
    fillTask('Add unit tests to auth.ts');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('start_agent_task', expect.objectContaining({
        task: 'Add unit tests to auth.ts',
        provider: 'ollama',
      }));
    });
  });

  it('When Run is clicked, Then the approval policy is included in the invoke call', async () => {
    renderPanel('ollama');
    fillTask('Refactor main.rs');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('start_agent_task', expect.objectContaining({
        approvalPolicy: 'auto-edit',
      }));
    });
  });
});

// ── Scenario 3: Running state ─────────────────────────────────────────────────

describe('Given the agent is running', () => {
  it('When running, Then the task textarea is disabled', async () => {
    renderPanel('ollama');
    fillTask('Do something');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => {
      expect(screen.getByRole('textbox')).toBeDisabled();
    });
  });

  it('When running, Then the Stop button appears', async () => {
    renderPanel('ollama');
    fillTask('Do something');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => {
      expect(screen.getByTitle('Stop the agent')).toBeInTheDocument();
    });
  });
});

// ── Scenario 4: Stop ──────────────────────────────────────────────────────────

describe('Given the agent is running', () => {
  it('When Stop is clicked, Then invoke("stop_agent_task") is called', async () => {
    renderPanel('ollama');
    fillTask('Long task');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => screen.getByTitle('Stop the agent'));
    fireEvent.click(screen.getByTitle('Stop the agent'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('stop_agent_task');
    });
  });

  it('When Stop completes, Then the Stop button disappears', async () => {
    renderPanel('ollama');
    fillTask('Long task');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => screen.getByTitle('Stop the agent'));
    fireEvent.click(screen.getByTitle('Stop the agent'));
    await waitFor(() => {
      expect(screen.queryByTitle('Stop the agent')).not.toBeInTheDocument();
    });
  });
});

// ── Scenario 5 & 6: Turbo Mode ───────────────────────────────────────────────

describe('Given Turbo Mode is off', () => {
  it('When the Turbo button is clicked, Then the approval select shows "full-auto"', () => {
    renderPanel('ollama');
    const turboBtn = screen.getByTitle(/Turbo Mode OFF/i);
    fireEvent.click(turboBtn);
    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select.value).toBe('full-auto');
  });
});

describe('Given Turbo Mode is on', () => {
  it('When the Turbo button is clicked again, Then approval reverts to "auto-edit"', () => {
    renderPanel('ollama');
    const turboBtn = screen.getByTitle(/Turbo Mode/i);
    fireEvent.click(turboBtn); // turn on
    fireEvent.click(screen.getByTitle(/Turbo Mode ON/i)); // turn off
    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select.value).toBe('auto-edit');
  });
});

// ── Scenario 7: Approval dropdown syncs Turbo ────────────────────────────────

describe('Given the user changes the approval dropdown to "full-auto"', () => {
  it('Then the Turbo button switches to ON state', () => {
    renderPanel('ollama');
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'full-auto' } });
    expect(screen.getByTitle(/Turbo Mode ON/i)).toBeInTheDocument();
  });
});

// ── Scenario 8: agent:complete event ─────────────────────────────────────────

describe('Given the agent completes successfully', () => {
  it('When "agent:complete" fires, Then the Reset button appears', async () => {
    renderPanel('ollama');
    await waitFor(() => expect(eventHandlers['agent:complete']).toBeDefined());
    fillTask('Build something');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => screen.getByTitle('Stop the agent'));
    act(() => { eventHandlers['agent:complete']('Task done!'); });
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Reset/i })).toBeInTheDocument();
    });
  });
});

// ── Scenario 9: agent:error event ────────────────────────────────────────────

describe('Given the agent encounters an error', () => {
  it('When "agent:error" fires, Then the Retry button appears', async () => {
    renderPanel('ollama');
    await waitFor(() => expect(eventHandlers['agent:error']).toBeDefined());
    fillTask('Broken task');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => screen.getByTitle('Stop the agent'));
    act(() => { eventHandlers['agent:error']('provider timeout'); });
    await waitFor(() => {
      expect(screen.getByTitle(/Retry/i)).toBeInTheDocument();
    });
  });
});

// ── Scenario 10-12: Approval prompt ──────────────────────────────────────────

describe('Given a pending approval request arrives', () => {
  async function setupPendingState() {
    renderPanel('ollama');
    // Wait for all 6 Tauri event listeners to register before proceeding
    await waitFor(() => {
      expect(eventHandlers['agent:pending']).toBeDefined();
    });
    fillTask('Pending task');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => screen.getByTitle('Stop the agent'));
    act(() => {
      eventHandlers['agent:pending']({
        name: 'bash',
        summary: 'rm -rf /tmp/old',
        is_destructive: true,
      });
    });
    await waitFor(() => screen.getByRole('button', { name: /Approve/i }));
  }

  it('When an approval request arrives, Then Approve and Reject buttons appear', async () => {
    await setupPendingState();
    expect(screen.getByRole('button', { name: /Approve/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Reject/i })).toBeInTheDocument();
  });

  it('When Approve is clicked, Then invoke("respond_to_agent_approval", { approved: true }) is called', async () => {
    await setupPendingState();
    fireEvent.click(screen.getByRole('button', { name: /Approve/i }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('respond_to_agent_approval', { approved: true });
    });
  });

  it('When Reject is clicked, Then invoke("respond_to_agent_approval", { approved: false }) is called', async () => {
    await setupPendingState();
    fireEvent.click(screen.getByRole('button', { name: /Reject/i }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('respond_to_agent_approval', { approved: false });
    });
  });

  it('When the pending call is destructive, Then the label says "Destructive action"', async () => {
    await setupPendingState();
    expect(screen.getByText(/Destructive action/i)).toBeInTheDocument();
  });
});

// ── Scenario 13: Reset ────────────────────────────────────────────────────────

describe('Given the agent has completed and Reset is clicked', () => {
  it('When Reset is clicked, Then the task textarea is cleared', async () => {
    renderPanel('ollama');
    await waitFor(() => expect(eventHandlers['agent:complete']).toBeDefined());
    fillTask('Some task');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => screen.getByTitle('Stop the agent'));
    act(() => { eventHandlers['agent:complete']('done'); });
    await waitFor(() => screen.getByRole('button', { name: /Reset/i }));
    fireEvent.click(screen.getByRole('button', { name: /Reset/i }));
    expect((screen.getByRole('textbox') as HTMLTextAreaElement).value).toBe('');
  });

  it('When Reset is clicked, Then the step count disappears', async () => {
    renderPanel('ollama');
    await waitFor(() => expect(eventHandlers['agent:step']).toBeDefined());
    fillTask('Some task');
    fireEvent.click(screen.getByRole('button', { name: /Run/i }));
    await waitFor(() => screen.getByTitle('Stop the agent'));
    act(() => {
      eventHandlers['agent:step']({ step_num: 1, tool_name: 'write_file', tool_summary: 'wrote out.rs', output: '', success: true, approved: true });
    });
    act(() => { eventHandlers['agent:complete']('done'); });
    await waitFor(() => screen.getByRole('button', { name: /Reset/i }));
    fireEvent.click(screen.getByRole('button', { name: /Reset/i }));
    expect(screen.queryByText(/step.*completed/i)).not.toBeInTheDocument();
  });
});

// ── Scenario 14: Keyboard shortcut ───────────────────────────────────────────

describe('Given a task is entered', () => {
  it('When Ctrl+Enter is pressed in the textarea, Then the agent starts', async () => {
    renderPanel('ollama');
    await waitFor(() => expect(eventHandlers['agent:complete']).toBeDefined());
    const textarea = screen.getByRole('textbox');
    fireEvent.change(textarea, { target: { value: 'Keyboard shortcut task' } });
    fireEvent.keyDown(textarea, { key: 'Enter', ctrlKey: true });
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('start_agent_task', expect.objectContaining({
        task: 'Keyboard shortcut task',
      }));
    });
  });
});
