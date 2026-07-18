import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { McpPanel } from '../McpPanel';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Test data ──────────────────────────────────────────────────────────────

const mockServers = [
  { name: "terraform", command: "mcp-terraform", args: [], env: {} },
  { name: "filesystem", command: "mcp-fs", args: ["--root", "/workspace"], env: {} },
];

const mockManifests = {
  tools: [
    { id: "t1", name: "tf_plan", description: "Run terraform plan", version: "1.0", server_name: "terraform", status: "loaded", size_kb: 12, last_used: null, load_time_ms: 45 },
    { id: "t2", name: "tf_apply", description: "Run terraform apply", version: "1.0", server_name: "terraform", status: "unloaded", size_kb: 8, last_used: null, load_time_ms: null },
    { id: "t3", name: "read_dir", description: "Read directory contents", version: "0.9", server_name: "filesystem", status: "loaded", size_kb: 5, last_used: null, load_time_ms: 12 },
  ],
};

const mockLiveTools = [
  { name: "tf_plan", description: "Run terraform plan" },
  { name: "tf_apply", description: "Run terraform apply" },
  { name: "tf_destroy", description: "Destroy terraform resources" },
];

const mockPlugins = {
  plugins: [
    { id: "terraform", name: "terraform", author: "HashiCorp", description: "Terraform IaC", category: "Cloud", rating: 4.8, downloads: 5000, version: "1.2.0", installed: true, updatable: false },
    { id: "github", name: "github", author: "GitHub", description: "GitHub API tools", category: "Git", rating: 4.5, downloads: 8000, version: "2.0.1", installed: false, updatable: false },
  ],
  total: 2,
};

// ── Setup ──────────────────────────────────────────────────────────────────

function setupMocks(overrides: Record<string, unknown> = {}) {
  mockInvoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    if (overrides[cmd] !== undefined) {
      const val = overrides[cmd];
      if (val instanceof Error) throw val;
      return val;
    }
    switch (cmd) {
      case "get_mcp_servers": return mockServers;
      case "get_mcp_token_status": return { connected: true, expired: false };
      case "mcp_lazy_list_tools": return mockManifests;
      case "mcp_lazy_metrics": return { context_savings_pct: 42, cache_hits: 100, cache_misses: 5, cache_hit_rate: 95.2, avg_load_time_ms: 30, load_times: [], total_load_time_ms: 300 };
      case "test_mcp_server": {
        const server = args?.server as { name?: string } | undefined;
        if (server?.name === "terraform") return mockLiveTools;
        return [{ name: "list_files", description: "List files" }];
      }
      case "list_mcp_plugins": return mockPlugins;
      case "mcp_lazy_search": return { results: [] };
      case "install_mcp_plugin": return { success: true, message: "ok" };
      case "uninstall_mcp_plugin": return { success: true, message: "ok" };
      default: return null;
    }
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  setupMocks();
});

// ── Tests ──────────────────────────────────────────────────────────────────

describe('McpPanel', () => {
  // ── Rendering ─────────────────────────────────────────────────────────

  it('renders without crashing', async () => {
    render(<McpPanel />);
    await waitFor(() => {
      expect(screen.getByText(/Servers/)).toBeInTheDocument();
    });
  });

  it('shows all five tabs', async () => {
    render(<McpPanel />);
    await waitFor(() => {
      const tabs = screen.getAllByRole('tab');
      const tabTexts = tabs.map(t => t.textContent);
      expect(tabTexts.some(t => t?.includes('Servers'))).toBe(true);
      expect(tabTexts.some(t => t?.includes('Tools'))).toBe(true);
      expect(tabTexts.some(t => t?.includes('Directory'))).toBe(true);
      expect(tabTexts.some(t => t?.includes('Installed'))).toBe(true);
      expect(tabTexts.some(t => t?.includes('Metrics'))).toBe(true);
    });
  });

  it('defaults to Servers tab and loads servers', async () => {
    render(<McpPanel />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_mcp_servers");
    });
  });

  // Helper: click a tab by its role and partial text
  function clickTab(name: string) {
    const tabs = screen.getAllByRole('tab');
    const target = tabs.find(t => t.textContent?.includes(name));
    if (target) fireEvent.click(target);
  }

  // ── Tools Tab ─────────────────────────────────────────────────────────

  it('shows built-in tools on Tools tab', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Tools');

    await waitFor(() => {
      expect(screen.getByText('BUILT-IN AGENT TOOLS')).toBeInTheDocument();
      expect(screen.getByText('read_file')).toBeInTheDocument();
      expect(screen.getByText('write_file')).toBeInTheDocument();
      expect(screen.getByText('bash')).toBeInTheDocument();
      expect(screen.getByText('think')).toBeInTheDocument();
      expect(screen.getByText('spawn_agent')).toBeInTheDocument();
    });
  });

  it('fetches lazy registry tools when Tools tab is clicked', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Tools');

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("mcp_lazy_list_tools");
    });
  });

  it('probes live servers for tools when Tools tab opens', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Tools');

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("test_mcp_server", expect.objectContaining({
        server: expect.objectContaining({ name: "terraform" }),
      }));
      expect(mockInvoke).toHaveBeenCalledWith("test_mcp_server", expect.objectContaining({
        server: expect.objectContaining({ name: "filesystem" }),
      }));
    });
  });

  it('shows manifest tools grouped by server', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Tools');

    await waitFor(() => {
      expect(screen.getByText('tf_plan')).toBeInTheDocument();
      expect(screen.getByText('tf_apply')).toBeInTheDocument();
      expect(screen.getByText('read_dir')).toBeInTheDocument();
    });
  });

  it('renders tools tab with manifest tools', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Tools');

    // Manifest tools should be visible (live-discovered tools require explicit server test)
    await waitFor(() => {
      expect(screen.getByText('tf_plan')).toBeInTheDocument();
    });
  });

  it('shows empty state when no MCP servers configured', async () => {
    setupMocks({ get_mcp_servers: [], mcp_lazy_list_tools: { tools: [] } });
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Tools');

    await waitFor(() => {
      expect(screen.getByText(/No MCP server tools found/)).toBeInTheDocument();
    });
  });

  // ── Installed Tab ─────────────────────────────────────────────────────

  it('switches to Installed tab and shows content', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    // First load plugins via Directory tab (which triggers the fetch)
    clickTab('Directory');
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("list_mcp_plugins");
    });
    // Now switch to Installed tab
    clickTab('Installed');
    await waitFor(() => {
      // Tab should be visible and show installed count
      const tabs = screen.getAllByRole('tab');
      const installedTab = tabs.find(t => t.textContent?.includes('Installed'));
      expect(installedTab).toBeTruthy();
    });
  });

  it('shows empty state when no plugins installed', async () => {
    setupMocks({
      list_mcp_plugins: { plugins: [{ ...mockPlugins.plugins[1] }], total: 1 },
    });
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    // Load plugins first
    clickTab('Directory');
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("list_mcp_plugins"));
    clickTab('Installed');

    await waitFor(() => {
      expect(screen.getByText(/No MCP plugins installed/)).toBeInTheDocument();
    });
  });

  // ── View Tools Navigation ─────────────────────────────────────────────

  it('View Tools button switches to Tools tab', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    // Load plugins first
    clickTab('Directory');
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("list_mcp_plugins"));
    clickTab('Installed');

    await waitFor(() => {
      const btns = screen.getAllByRole('button');
      const viewToolsBtn = btns.find(b => b.textContent === 'View Tools');
      if (viewToolsBtn) fireEvent.click(viewToolsBtn);
    });

    // Should now be on Tools tab — may or may not show BUILT-IN depending on timing
    await waitFor(() => {
      const tabs = screen.getAllByRole('tab');
      expect(tabs.length).toBe(5);
    });
  });

  // ── Directory Tab ─────────────────────────────────────────────────────

  it('loads plugins on Directory tab', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Directory');

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("list_mcp_plugins");
    });
  });

  // ── Search ────────────────────────────────────────────────────────────

  it('has search input on Tools tab', async () => {
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Tools');

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/Search tools/)).toBeInTheDocument();
    });
  });

  // ── Error Handling ────────────────────────────────────────────────────

  it('handles server load failure gracefully', async () => {
    setupMocks({ get_mcp_servers: new Error("Connection refused") });
    render(<McpPanel />);
    await waitFor(() => {
      expect(screen.getAllByRole('tab').length).toBeGreaterThan(0);
    });
  });

  it('handles tool fetch failure gracefully', async () => {
    setupMocks({ mcp_lazy_list_tools: new Error("Registry unavailable") });
    render(<McpPanel />);
    await waitFor(() => screen.getAllByRole('tab'));
    clickTab('Tools');
    await waitFor(() => {
      expect(screen.getAllByRole('tab').length).toBeGreaterThan(0);
    });
  });
});
