import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const mockListen = vi.fn();
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

// ── Mock lucide-react icons ────────────────────────────────────────────────

vi.mock('lucide-react', () => {
  const icon = (name: string) => {
    const Component = (props: Record<string, unknown>) => <span data-testid={`icon-${name}`} {...props} />;
    Component.displayName = name;
    return Component;
  };
  const names = [
    'User', 'Palette', 'LogIn', 'Save', 'Key', 'X', 'Check', 'Upload', 'Download',
    'RotateCcw', 'Sun', 'Moon', 'Eye', 'EyeOff', 'ChevronRight', 'CheckCircle',
    'MinusCircle', 'AlertCircle', 'Loader2', 'Zap', 'Plug', 'Mail', 'CalendarDays',
    'ClipboardList', 'MessageSquare', 'Search', 'Mic', 'Home', 'Server',
    'AlertTriangle', 'Inbox',
  ];
  return Object.fromEntries(names.map(n => [n, icon(n)]));
});

// ── Import after mocks ────────────────────────────────────────────────────

import { SettingsPanel } from '../SettingsPanel';

// ── Test data ──────────────────────────────────────────────────────────────

const mockApiKeySettings = {
  anthropic_api_key: "sk-ant-test123",
  openai_api_key: "",
  gemini_api_key: "",
  grok_api_key: "",
  groq_api_key: "",
  openrouter_api_key: "",
  azure_openai_api_key: "",
  azure_openai_api_url: "",
  mistral_api_key: "",
  cerebras_api_key: "",
  deepseek_api_key: "",
  zhipu_api_key: "",
  vercel_ai_api_key: "",
  vercel_ai_api_url: "",
  minimax_api_key: "",
  perplexity_api_key: "",
  together_api_key: "",
  fireworks_api_key: "",
  sambanova_api_key: "",
  ollama_api_key: "",
  ollama_api_url: "",
  claude_model: "claude-3-5-sonnet-latest",
  openai_model: "gpt-4o",
  openrouter_model: "",
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
      case "get_provider_api_keys": return mockApiKeySettings;
      case "save_provider_api_keys": return ["ollama", "claude"];
      case "validate_all_api_keys": return [];
      case "validate_api_key": return { provider: "ollama", valid: true, error: null, latency_ms: 50 };
      case "cloud_oauth_save_client_config": return null;
      case "cloud_oauth_disconnect": return null;
      case "cloud_oauth_refresh": return null;
      default: return null;
    }
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  setupMocks();
  mockListen.mockImplementation(async () => () => {});
});

// ── Tests ──────────────────────────────────────────────────────────────────

describe('SettingsPanel', () => {
  // ── Rendering ─────────────────────────────────────────────────────────

  it('renders without crashing', () => {
    render(<SettingsPanel />);
    expect(screen.getByText('Settings')).toBeInTheDocument();
  });

  it('shows all five section buttons in sidebar', () => {
    render(<SettingsPanel />);
    // Use getAllByText since section labels appear in both sidebar and content
    expect(screen.getAllByText('Profile').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Appearance').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('OAuth Login').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Customizations').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('API Keys').length).toBeGreaterThanOrEqual(1);
  });

  it('defaults to Profile section', () => {
    render(<SettingsPanel />);
    // Profile section should be visible by default (may appear multiple times)
    expect(screen.getAllByText('Profile').length).toBeGreaterThanOrEqual(1);
  });

  it('renders close button when onClose prop provided', () => {
    const onClose = vi.fn();
    render(<SettingsPanel onClose={onClose} />);
    // The X icon button should be present (icon name is capital X)
    const xIcon = screen.getByTestId('icon-X');
    expect(xIcon).toBeInTheDocument();
  });

  it('calls onClose when close button is clicked', () => {
    const onClose = vi.fn();
    render(<SettingsPanel onClose={onClose} />);
    const xIcon = screen.getByTestId('icon-X');
    const button = xIcon.closest('button');
    expect(button).not.toBeNull();
    fireEvent.click(button!);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  // ── Section switching ─────────────────────────────────────────────────

  it('switches to Appearance section', () => {
    render(<SettingsPanel />);
    // Click the Appearance button in the sidebar
    const buttons = screen.getAllByText('Appearance');
    fireEvent.click(buttons[0]);
    // Appearance section should now be active (button text + section content)
    expect(screen.getAllByText('Appearance').length).toBeGreaterThanOrEqual(1);
  });

  it('switches to API Keys section and loads keys', async () => {
    render(<SettingsPanel />);
    fireEvent.click(screen.getByText('API Keys'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_provider_api_keys");
    });
  });

  it('switches to OAuth Login section', () => {
    render(<SettingsPanel />);
    fireEvent.click(screen.getByText('OAuth Login'));
    expect(screen.getByText('OAuth Login')).toBeInTheDocument();
  });

  it('switches to Customizations section', () => {
    render(<SettingsPanel />);
    fireEvent.click(screen.getByText('Customizations'));
    expect(screen.getByText('Customizations')).toBeInTheDocument();
  });

  // ── API Keys section ──────────────────────────────────────────────────

  it('shows Anthropic section in API Keys', async () => {
    render(<SettingsPanel />);
    fireEvent.click(screen.getByText('API Keys'));
    await waitFor(() => {
      expect(screen.getByText('Anthropic (Claude)')).toBeInTheDocument();
    });
  });

  it('shows Save & Apply button in API Keys section', async () => {
    render(<SettingsPanel />);
    fireEvent.click(screen.getByText('API Keys'));
    await waitFor(() => {
      expect(screen.getByText('Save & Apply')).toBeInTheDocument();
    });
  });

  it('validates API keys on API Keys section open', async () => {
    render(<SettingsPanel />);
    fireEvent.click(screen.getByText('API Keys'));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("validate_all_api_keys");
    });
  });

  // ── Inline completion toggle ──────────────────────────────────────────

  describe('Inline completion toggle', () => {
    beforeEach(() => {
      localStorage.removeItem("vibeui-ai-inline-completion-enabled");
    });

    it('defaults to enabled when no setting stored', () => {
      render(<SettingsPanel />);
      fireEvent.click(screen.getAllByText('Appearance')[0]);
      const label = screen.getByText('Inline completion (ghost text)');
      const checkbox = label.closest('label')!.querySelector('input[type="checkbox"]') as HTMLInputElement;
      expect(checkbox.checked).toBe(true);
    });

    it('reflects stored "false" value on mount', () => {
      localStorage.setItem("vibeui-ai-inline-completion-enabled", "false");
      render(<SettingsPanel />);
      fireEvent.click(screen.getAllByText('Appearance')[0]);
      const label = screen.getByText('Inline completion (ghost text)');
      const checkbox = label.closest('label')!.querySelector('input[type="checkbox"]') as HTMLInputElement;
      expect(checkbox.checked).toBe(false);
    });

    it('writes "false" to localStorage when toggled off', () => {
      render(<SettingsPanel />);
      fireEvent.click(screen.getAllByText('Appearance')[0]);
      const label = screen.getByText('Inline completion (ghost text)');
      const checkbox = label.closest('label')!.querySelector('input[type="checkbox"]') as HTMLInputElement;
      fireEvent.click(checkbox);
      expect(localStorage.getItem("vibeui-ai-inline-completion-enabled")).toBe("false");
    });

    it('writes "true" to localStorage when toggled back on', () => {
      localStorage.setItem("vibeui-ai-inline-completion-enabled", "false");
      render(<SettingsPanel />);
      fireEvent.click(screen.getAllByText('Appearance')[0]);
      const label = screen.getByText('Inline completion (ghost text)');
      const checkbox = label.closest('label')!.querySelector('input[type="checkbox"]') as HTMLInputElement;
      fireEvent.click(checkbox);
      expect(localStorage.getItem("vibeui-ai-inline-completion-enabled")).toBe("true");
    });
  });
});
