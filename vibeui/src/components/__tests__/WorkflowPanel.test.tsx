import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Import after mocks ────────────────────────────────────────────────────

import { WorkflowPanel } from '../WorkflowPanel';

// ── Test data ──────────────────────────────────────────────────────────────

const mockWorkflow = {
  name: "my_todo_app",
  description: "A simple todo application with React and Rust",
  current_stage: 1,
  stages: [
    { stage: "requirements", label: "Requirements", status: "complete", checklist: [{ id: 1, description: "Define user stories", done: true }], body: "" },
    { stage: "design", label: "Design", status: "in-progress", checklist: [
      { id: 1, description: "Create wireframes", done: true },
      { id: 2, description: "Design database schema", done: false },
      { id: 3, description: "Plan API endpoints", done: false },
    ], body: "Design notes here" },
    { stage: "implementation", label: "Implementation", status: "not-started", checklist: [], body: "" },
    { stage: "testing", label: "Testing", status: "not-started", checklist: [], body: "" },
    { stage: "deployment", label: "Deployment", status: "not-started", checklist: [], body: "" },
    { stage: "monitoring", label: "Monitoring", status: "not-started", checklist: [], body: "" },
    { stage: "optimization", label: "Optimization", status: "not-started", checklist: [], body: "" },
    { stage: "maintenance", label: "Maintenance", status: "not-started", checklist: [], body: "" },
  ],
  created_at: "2024-01-01",
  overall_progress: 18.75,
};

const mockWorkflows = [mockWorkflow];

// ── Setup ──────────────────────────────────────────────────────────────────

function setupMocks(overrides: Record<string, unknown> = {}) {
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (overrides[cmd] !== undefined) {
      const val = overrides[cmd];
      if (val instanceof Error) throw val;
      return val;
    }
    switch (cmd) {
      case "list_workflows": return mockWorkflows;
      case "get_workflow": return mockWorkflow;
      case "create_workflow": return mockWorkflow;
      case "update_workflow_checklist_item": return mockWorkflow;
      case "advance_workflow_stage": return mockWorkflow;
      case "generate_stage_checklist": return mockWorkflow;
      default: return null;
    }
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  setupMocks();
});

// ── Tests ──────────────────────────────────────────────────────────────────

describe('WorkflowPanel', () => {
  // ── Rendering ─────────────────────────────────────────────────────────

  it('renders without crashing', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('Workflow')).toBeInTheDocument();
    });
  });

  it('shows + New Workflow button', () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    expect(screen.getByText('+ New Workflow')).toBeInTheDocument();
  });

  it('shows refresh button', () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    expect(screen.getByText('↺')).toBeInTheDocument();
  });

  it('loads workflows on mount', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("list_workflows", { workspacePath: "/workspace" });
    });
  });

  // ── Workflow list ─────────────────────────────────────────────────────

  it('displays workflow names in list', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('my todo app')).toBeInTheDocument();
    });
  });

  it('shows workflow description', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('A simple todo application with React and Rust')).toBeInTheDocument();
    });
  });

  it('shows workflow progress percentage', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('19%')).toBeInTheDocument();
    });
  });

  it('shows stage info (Stage X/8)', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('Stage 2/8')).toBeInTheDocument();
    });
  });

  // ── Empty state ───────────────────────────────────────────────────────

  it('shows empty state when no workflows', async () => {
    setupMocks({ list_workflows: [] });
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('No workflows yet.')).toBeInTheDocument();
    });
  });

  // ── New workflow form ─────────────────────────────────────────────────

  it('toggles new workflow form when + New Workflow clicked', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    fireEvent.click(screen.getByText('+ New Workflow'));
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/Workflow name/)).toBeInTheDocument();
      expect(screen.getByPlaceholderText(/Describe the application/)).toBeInTheDocument();
    });
  });

  it('shows Create Workflow and Cancel buttons in form', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    fireEvent.click(screen.getByText('+ New Workflow'));
    await waitFor(() => {
      expect(screen.getByText('Create Workflow')).toBeInTheDocument();
      expect(screen.getByText('Cancel')).toBeInTheDocument();
    });
  });

  it('hides form when Cancel is clicked', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    fireEvent.click(screen.getByText('+ New Workflow'));
    await waitFor(() => screen.getByText('Cancel'));
    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByPlaceholderText(/Workflow name/)).not.toBeInTheDocument();
    });
  });

  // ── Workflow detail ───────────────────────────────────────────────────

  it('opens workflow detail when clicking a workflow', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('my todo app'));
    fireEvent.click(screen.getByText('my todo app'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_workflow", { workspacePath: "/workspace", name: "my_todo_app" });
    });
  });

  it('shows back button in detail view', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('my todo app'));
    fireEvent.click(screen.getByText('my todo app'));
    await waitFor(() => {
      expect(screen.getByText('← Back')).toBeInTheDocument();
    });
  });

  it('shows Generate Checklist button in detail view', async () => {
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('my todo app'));
    fireEvent.click(screen.getByText('my todo app'));
    await waitFor(() => {
      expect(screen.getByText('Generate Checklist')).toBeInTheDocument();
    });
  });

  // ── Error handling ────────────────────────────────────────────────────

  it('displays error when workflow loading fails', async () => {
    setupMocks({ list_workflows: new Error("Permission denied") });
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText(/Permission denied/)).toBeInTheDocument();
    });
  });

  it('shows dismiss button on error message', async () => {
    setupMocks({ list_workflows: new Error("Network error") });
    render(<WorkflowPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('✕')).toBeInTheDocument();
    });
  });
});
