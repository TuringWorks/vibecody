/**
 * Observe-Act panel — the behaviours that must not regress.
 *
 * The panel shipped as a mock: "Start Observe-Act Loop" set a local `status`
 * string to `"running"` and nothing else happened — no screenshot, no model
 * call, no action. The Save Config button wrote to a file the loop never read.
 * These tests assert on the *outgoing request*, because that is what the
 * daemon acts on; asserting on the rendered controls is exactly what let a
 * panel that looked right and did nothing pass review.
 */
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const TOOLBAR = "perplexity";

const mockDaemonFetch = vi.fn();
vi.mock("../../lib/daemonFetch", () => ({
  daemonFetch: (...args: unknown[]) => mockDaemonFetch(...args),
  getDaemonToken: async () => "test-token",
}));

vi.mock("../../hooks/useModelRegistry", () => ({
  useModelRegistry: () => ({
    providers: ["ollama", "openai", "claude", "perplexity"],
    modelsForProvider: (p: string) =>
      p === "perplexity" ? ["sonar-pro", "sonar"] : ["llama3", "gpt-4"],
    loading: false,
  }),
  PROVIDER_DEFAULT_MODEL: {
    ollama: "llama3",
    openai: "gpt-4",
    claude: "claude-sonnet-4-6",
    perplexity: "sonar-pro",
  },
}));

import { ObserveActPanel } from "../ObserveActPanel";

/** `EventSource` does not exist in jsdom; the panel opens one when live. */
class FakeEventSource {
  static last: FakeEventSource | null = null;
  listeners = new Map<string, EventListener>();
  constructor(public url: string) {
    FakeEventSource.last = this;
  }
  addEventListener(type: string, fn: EventListener) {
    this.listeners.set(type, fn);
  }
  close() {}
}

// Assigned at module scope, not in `beforeEach`: the panel opens the stream
// from an effect that runs during the first `render`, and a stub installed
// per-test was not in place in time.
(globalThis as unknown as { EventSource: unknown }).EventSource = FakeEventSource;

const json = (body: unknown, status = 200): Response =>
  ({
    ok: status >= 200 && status < 300,
    status,
    statusText: "",
    json: async () => body,
  }) as Response;

const CONFIG = {
  observation_interval_ms: 2000,
  max_steps: 50,
  max_consecutive_failures: 3,
  screenshot_width: 1280,
  screenshot_height: 720,
  vision_provider: "claude",
  verify_after_action: true,
  safety_mode: "cautious" as const,
  safety: {
    forbidden_regions: [],
    max_actions_per_step: 5,
    require_confirmation_for: [],
    forbidden_key_combos: [["alt", "f4"]],
    rate_limit_ms: 200,
  },
};

const READY_PREFLIGHT = {
  platform: "macOS",
  missing_tools: [],
  logical_screen: [1440, 900],
  ready: true,
};

const SESSION = {
  id: "sess-1",
  model: "perplexity/sonar-pro",
  task: "do the thing",
  status: "running",
  config: CONFIG,
  started_at_ms: 1,
  consecutive_failures: 0,
  summary: {
    total_steps: 1,
    total_actions: 1,
    success_rate: 1,
    duration_ms: 10,
    final_status: "running",
    task: "do the thing",
    completion_summary: null,
  },
  steps: [],
  pending_approval: null,
  has_screenshot: false,
};

/** Route the panel's daemon calls to canned answers. */
function route(overrides: Record<string, unknown> = {}) {
  mockDaemonFetch.mockImplementation(async (url: string, init?: RequestInit) => {
    const method = init?.method ?? "GET";
    for (const [key, value] of Object.entries(overrides)) {
      if (`${method} ${url}`.includes(key)) return json(value);
    }
    if (url.includes("/observe/config")) return json(CONFIG);
    if (url.includes("/observe/preflight")) return json(READY_PREFLIGHT);
    if (url.includes("/observe/sessions") && method === "POST") return json(SESSION, 201);
    if (url.endsWith("/observe/sessions")) return json({ sessions: [] });
    return json(SESSION);
  });
}

/** Every request body the panel sent to a matching URL. */
function bodiesSentTo(fragment: string): unknown[] {
  return mockDaemonFetch.mock.calls
    .filter(([url]) => String(url).includes(fragment))
    .map(([, init]) => {
      const body = (init as RequestInit | undefined)?.body;
      return typeof body === "string" ? JSON.parse(body) : undefined;
    })
    .filter((b) => b !== undefined);
}

beforeEach(() => {
  vi.clearAllMocks();
  FakeEventSource.last = null;
  route();
});

describe("Given the operator wants to start a session", () => {
  it("When they press Start, Then the daemon is asked to run it", async () => {
    render(<ObserveActPanel provider={TOOLBAR} />);
    await screen.findByText(/Ready/);

    fireEvent.change(screen.getByLabelText(/Task Description/i), {
      target: { value: "export the report" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Start Observe-Act Loop/i }));

    await waitFor(() => {
      const bodies = bodiesSentTo("/observe/sessions") as Array<{ task?: string }>;
      expect(bodies.some((b) => b.task === "export the report")).toBe(true);
    });
  });

  /**
   * The panel used to set a local `status` string and stop there. A test that
   * only checked the button's disabled state would still pass on that panel —
   * so this one asserts a request left the panel at all.
   */
  it("When they press Start, Then it is not merely a local state change", async () => {
    render(<ObserveActPanel provider={TOOLBAR} />);
    await screen.findByText(/Ready/);
    fireEvent.change(screen.getByLabelText(/Task Description/i), {
      target: { value: "x" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Start Observe-Act Loop/i }));

    await waitFor(() => {
      const posts = mockDaemonFetch.mock.calls.filter(
        ([url, init]) =>
          String(url).endsWith("/observe/sessions") &&
          (init as RequestInit | undefined)?.method === "POST"
      );
      expect(posts).toHaveLength(1);
    });
  });

  /**
   * Provider-agnostic (AGENTS.md → STRICT): the selection reaches the outgoing
   * payload. `TOOLBAR` is deliberately not any provider the panel could
   * default to, so a hard-coded fallback fails rather than coincides.
   */
  it("When they press Start, Then the toolbar's provider is the one sent", async () => {
    render(<ObserveActPanel provider={TOOLBAR} />);
    await screen.findByText(/Ready/);
    fireEvent.change(screen.getByLabelText(/Task Description/i), {
      target: { value: "x" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Start Observe-Act Loop/i }));

    await waitFor(() => {
      const bodies = bodiesSentTo("/observe/sessions") as Array<{
        provider?: string;
        model?: string;
      }>;
      expect(bodies[0]?.provider).toBe(TOOLBAR);
      expect(bodies[0]?.model).toBe("sonar-pro");
    });
  });

  it("When no provider is selected, Then it says so instead of picking one", async () => {
    render(<ObserveActPanel />);
    await screen.findByText(/Ready/);

    expect(screen.getByText(/Select a model in the toolbar/i)).toBeTruthy();
    expect(
      screen.getByRole("button", { name: /Start Observe-Act Loop/i })
    ).toHaveProperty("disabled", true);
  });
});

describe("Given the machine is not set up for desktop automation", () => {
  it("When the panel loads, Then it names the missing tool", async () => {
    route({
      "/observe/preflight": {
        platform: "macOS",
        missing_tools: ["cliclick"],
        logical_screen: [1440, 900],
        ready: false,
      },
    });
    render(<ObserveActPanel provider={TOOLBAR} />);

    expect(await screen.findByText(/cannot run a session yet/i)).toBeTruthy();
    expect(screen.getAllByText(/cliclick/).length).toBeGreaterThan(0);
    expect(screen.getByText(/brew install cliclick/)).toBeTruthy();
  });
});

describe("Given a destructive action is waiting on the operator", () => {
  it("When they approve it, Then the answer names the approval it answers", async () => {
    const withApproval = {
      ...SESSION,
      pending_approval: {
        id: "appr-9",
        step_num: 3,
        action: { type: "key_combo", keys: ["ctrl", "q"] },
        description: "KeyCombo(ctrl+q)",
        requested_at_ms: 5,
      },
    };
    mockDaemonFetch.mockImplementation(async (url: string, init?: RequestInit) => {
      const method = init?.method ?? "GET";
      if (url.includes("/observe/config")) return json(CONFIG);
      if (url.includes("/observe/preflight")) return json(READY_PREFLIGHT);
      if (url.includes("/approve")) return json({ resolved: true });
      if (url.endsWith("/observe/sessions") && method === "GET")
        return json({ sessions: [{ ...SESSION, total_steps: 1 }] });
      return json(withApproval);
    });

    render(<ObserveActPanel provider={TOOLBAR} />);
    fireEvent.click(screen.getByRole("button", { name: /monitor/i }));

    fireEvent.click(await screen.findByRole("button", { name: /^Approve$/ }));

    await waitFor(() => {
      const bodies = bodiesSentTo("/approve") as Array<{
        approval_id?: string;
        approve?: boolean;
      }>;
      expect(bodies[0]).toEqual({ approval_id: "appr-9", approve: true });
    });
  });
});

describe("Given a step was recorded without verification", () => {
  /**
   * The old panel modelled this as `verified: boolean`, so a step nobody
   * checked rendered as "Failed" — turning "we did not look" into "it went
   * wrong". Absent stays absent (AGENTS.md → Modelling Honesty).
   */
  it("When the history is shown, Then it reads Unverified, not Failed", async () => {
    const withSteps = {
      ...SESSION,
      status: "completed",
      steps: [
        {
          step_num: 1,
          timestamp_ms: 1,
          screenshot_path: null,
          llm_reasoning: "clicked the button",
          actions_taken: [{ type: "click", x: 10, y: 20 }],
          proposed_actions: [{ type: "click", x: 10, y: 20 }],
          verification_result: null,
          duration_ms: 42,
        },
      ],
    };
    mockDaemonFetch.mockImplementation(async (url: string) => {
      if (url.includes("/observe/config")) return json(CONFIG);
      if (url.includes("/observe/preflight")) return json(READY_PREFLIGHT);
      if (url.endsWith("/observe/sessions"))
        return json({ sessions: [{ ...SESSION, status: "completed" }] });
      return json(withSteps);
    });

    render(<ObserveActPanel provider={TOOLBAR} />);
    fireEvent.click(screen.getByRole("button", { name: /history/i }));

    expect(await screen.findByText("Unverified")).toBeTruthy();
    expect(screen.queryByText("Failed")).toBeNull();
    expect(screen.getByText("Click(10, 20)")).toBeTruthy();
  });

  it("And the verified rate reads n/a rather than 0%", async () => {
    const withSteps = {
      ...SESSION,
      steps: [
        {
          step_num: 1,
          timestamp_ms: 1,
          screenshot_path: null,
          llm_reasoning: "r",
          actions_taken: [],
          proposed_actions: [],
          verification_result: null,
          duration_ms: 1,
        },
      ],
    };
    mockDaemonFetch.mockImplementation(async (url: string) => {
      if (url.includes("/observe/config")) return json(CONFIG);
      if (url.includes("/observe/preflight")) return json(READY_PREFLIGHT);
      if (url.endsWith("/observe/sessions")) return json({ sessions: [SESSION] });
      return json(withSteps);
    });

    render(<ObserveActPanel provider={TOOLBAR} />);
    fireEvent.click(screen.getByRole("button", { name: /monitor/i }));

    expect(await screen.findByText("n/a")).toBeTruthy();
  });
});

describe("Given a restricted-mode step proposed something it did not run", () => {
  it("When the history is shown, Then the proposal is visible and marked unexecuted", async () => {
    const withSteps = {
      ...SESSION,
      steps: [
        {
          step_num: 1,
          timestamp_ms: 1,
          screenshot_path: null,
          llm_reasoning: "would have clicked",
          actions_taken: [],
          proposed_actions: [{ type: "click", x: 5, y: 6 }],
          verification_result: null,
          duration_ms: 1,
        },
      ],
    };
    mockDaemonFetch.mockImplementation(async (url: string) => {
      if (url.includes("/observe/config")) return json(CONFIG);
      if (url.includes("/observe/preflight")) return json(READY_PREFLIGHT);
      if (url.endsWith("/observe/sessions")) return json({ sessions: [SESSION] });
      return json(withSteps);
    });

    render(<ObserveActPanel provider={TOOLBAR} />);
    fireEvent.click(screen.getByRole("button", { name: /history/i }));

    expect(await screen.findByText(/not executed:/)).toBeTruthy();
    expect(screen.getByText("Click(5, 6)")).toBeTruthy();
  });
});

describe("Given the operator edits the safety rails", () => {
  it("When they save, Then the daemon is sent the edited value", async () => {
    render(<ObserveActPanel provider={TOOLBAR} />);
    fireEvent.click(screen.getByRole("button", { name: /safety/i }));

    const field = await screen.findByLabelText(/Max Actions per Step/i);
    fireEvent.change(field, { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: /Save Safety Config/i }));

    await waitFor(() => {
      const bodies = bodiesSentTo("/observe/config") as Array<{
        safety?: { max_actions_per_step?: number };
      }>;
      expect(bodies[0]?.safety?.max_actions_per_step).toBe(2);
    });
  });
});
