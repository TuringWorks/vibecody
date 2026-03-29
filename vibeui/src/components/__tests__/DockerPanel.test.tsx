import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Mock lucide-react icons ────────────────────────────────────────────────

vi.mock('lucide-react', () => {
  const icon = (name: string) => {
    const Component = (props: Record<string, unknown>) => <span data-testid={`icon-${name}`} {...props} />;
    Component.displayName = name;
    return Component;
  };
  return {
    // DockerPanel
    CheckCircle2: icon('CheckCircle2'), XCircle: icon('XCircle'),
    PauseCircle: icon('PauseCircle'), MinusCircle: icon('MinusCircle'),
    Package: icon('Package'),
    // StatusMessage dependencies
    AlertTriangle: icon('AlertTriangle'), Loader2: icon('Loader2'),
    Inbox: icon('Inbox'), CheckCircle: icon('CheckCircle'),
  };
});

// ── Import after mocks ────────────────────────────────────────────────────

import { DockerPanel } from '../DockerPanel';

// ── Test data ──────────────────────────────────────────────────────────────

const mockContainers = [
  { id: "abc123", name: "web-app", image: "nginx:latest", status: "Up 2 hours", ports: "0.0.0.0:80->80/tcp", created: "2024-01-01" },
  { id: "def456", name: "db-postgres", image: "postgres:16", status: "Exited (0) 5 minutes ago", ports: "", created: "2024-01-01" },
  { id: "ghi789", name: "redis-cache", image: "redis:7", status: "Paused", ports: "6379/tcp", created: "2024-01-01" },
];

const mockImages = [
  { id: "sha256:aaa111", repository: "nginx", tag: "latest", size: "187MB", created: "2 weeks ago" },
  { id: "sha256:bbb222", repository: "postgres", tag: "16", size: "412MB", created: "3 weeks ago" },
  { id: "sha256:ccc333", repository: "node", tag: "20-alpine", size: "128MB", created: "1 week ago" },
];

// ── Setup ──────────────────────────────────────────────────────────────────

function setupMocks(overrides: Record<string, unknown> = {}) {
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (overrides[cmd] !== undefined) {
      const val = overrides[cmd];
      if (val instanceof Error) throw val;
      return val;
    }
    switch (cmd) {
      case "list_docker_containers": return mockContainers;
      case "list_docker_images": return mockImages;
      case "docker_container_action": return "action completed";
      case "docker_pull_image": return "Pull complete";
      case "docker_compose_action": return "compose output";
      default: return null;
    }
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  setupMocks();
});

// ── Tests ──────────────────────────────────────────────────────────────────

describe('DockerPanel', () => {
  // ── Rendering ─────────────────────────────────────────────────────────

  it('renders without crashing', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('Containers')).toBeInTheDocument();
    });
  });

  it('shows all three sub-tabs', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('Containers')).toBeInTheDocument();
      expect(screen.getByText('Images')).toBeInTheDocument();
      expect(screen.getByText('Compose')).toBeInTheDocument();
    });
  });

  it('defaults to Containers tab and loads containers', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("list_docker_containers");
    });
  });

  // ── Container list ────────────────────────────────────────────────────

  it('displays container names and images', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('web-app')).toBeInTheDocument();
      expect(screen.getByText('db-postgres')).toBeInTheDocument();
      expect(screen.getByText('redis-cache')).toBeInTheDocument();
      expect(screen.getByText('nginx:latest')).toBeInTheDocument();
    });
  });

  it('shows container count', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('3 containers')).toBeInTheDocument();
    });
  });

  it('shows empty state when no containers', async () => {
    setupMocks({ list_docker_containers: [] });
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('No containers found')).toBeInTheDocument();
    });
  });

  it('shows refresh button on containers tab', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText('↻ Refresh')).toBeInTheDocument();
    });
  });

  it('clicking refresh reloads containers', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('↻ Refresh'));
    fireEvent.click(screen.getByText('↻ Refresh'));
    await waitFor(() => {
      // Called once on mount, once on refresh
      const calls = mockInvoke.mock.calls.filter(c => c[0] === 'list_docker_containers');
      expect(calls.length).toBeGreaterThanOrEqual(2);
    });
  });

  // ── Error handling ────────────────────────────────────────────────────

  it('displays error when container loading fails', async () => {
    setupMocks({ list_docker_containers: new Error("Docker daemon not running") });
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => {
      expect(screen.getByText(/Docker daemon not running/)).toBeInTheDocument();
    });
  });

  // ── Images tab ────────────────────────────────────────────────────────

  it('switches to Images tab and loads images', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Images'));
    fireEvent.click(screen.getByText('Images'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("list_docker_images");
    });
  });

  it('displays image list on Images tab', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Images'));
    fireEvent.click(screen.getByText('Images'));
    await waitFor(() => {
      expect(screen.getByText('nginx:latest')).toBeInTheDocument();
      expect(screen.getByText('187MB')).toBeInTheDocument();
    });
  });

  it('shows pull image input on Images tab', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Images'));
    fireEvent.click(screen.getByText('Images'));
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/Pull image/)).toBeInTheDocument();
    });
  });

  it('shows Pull button on Images tab', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Images'));
    fireEvent.click(screen.getByText('Images'));
    await waitFor(() => {
      expect(screen.getByText('Pull')).toBeInTheDocument();
    });
  });

  it('shows empty state when no images', async () => {
    setupMocks({ list_docker_images: [] });
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Images'));
    fireEvent.click(screen.getByText('Images'));
    await waitFor(() => {
      expect(screen.getByText('No local images')).toBeInTheDocument();
    });
  });

  // ── Compose tab ───────────────────────────────────────────────────────

  it('switches to Compose tab and shows compose buttons', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Compose'));
    fireEvent.click(screen.getByText('Compose'));
    await waitFor(() => {
      expect(screen.getByText('Up')).toBeInTheDocument();
      expect(screen.getByText('Down')).toBeInTheDocument();
      expect(screen.getByText('Logs')).toBeInTheDocument();
      expect(screen.getByText('Build')).toBeInTheDocument();
    });
  });

  it('shows service name input on Compose tab', async () => {
    render(<DockerPanel workspacePath="/workspace" />);
    await waitFor(() => screen.getByText('Compose'));
    fireEvent.click(screen.getByText('Compose'));
    await waitFor(() => {
      expect(screen.getByPlaceholderText(/Service name/)).toBeInTheDocument();
    });
  });
});
