/**
 * BDD: the harness panel says what will actually be sent, or says it cannot.
 *
 * This panel edits what every request to a paid API carries — tool schemas or
 * a prose catalogue, the output cap, the reasoning budget. Two failure modes
 * matter more here than in an ordinary settings pane:
 *
 *   * **Rendering a default the daemon never confirmed.** If the daemon is
 *     unreachable, showing the built-in values would tell the user their models
 *     are configured one way while the running daemon does something else.
 *   * **Rendering absent as a number.** An empty output cap means "the provider
 *     decides". Drawn as `0` it becomes a limit nobody set — the exact
 *     substitution `vibe_ai::harness` refuses to make on the Rust side.
 *
 * The reply is also narrowed rather than cast: `invoke`/`fetch` hand back
 * whatever the other end sent, and a cast is a compile-time story about it.
 */
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const mockDaemonFetch = vi.fn();
vi.mock("../../lib/daemonFetch", () => ({
  daemonFetch: (...args: unknown[]) => mockDaemonFetch(...args),
}));

vi.mock("../../hooks/useModelRegistry", async () => {
  const actual = await vi.importActual<Record<string, unknown>>(
    "../../hooks/useModelRegistry"
  );
  return {
    ...actual,
    useModelRegistry: () => ({
      providers: ["claude", "openai"],
      modelsForProvider: (p: string) =>
        p === "claude" ? ["claude-opus-5", "claude-sonnet-5"] : ["gpt-5.5"],
      loading: false,
      refresh: async () => {},
      lastUpdated: 0,
    }),
  };
});

import { HarnessSection } from "../settings/HarnessSection";

/**
 * Block body, not an expression body.
 *
 * `beforeEach(() => mock.mockReset())` returns the mock — and Vitest treats a
 * returned function as a teardown, so the mock is *called* after every test.
 * A throwing mock then fails its own test with no useful diff.
 */
beforeEach(() => {
  mockDaemonFetch.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

const NATIVE_PROFILE = {
  tool_transport: "native",
  prompt_dialect: "compact",
  prompt_cache: true,
};

function reply(body: unknown, ok = true, status = 200) {
  return Promise.resolve({
    ok,
    status,
    json: () => Promise.resolve(body),
  });
}

function resolved(overrides: Record<string, unknown> = {}) {
  return {
    provider: "claude",
    model: "*",
    effective: { ...NATIVE_PROFILE },
    builtin: { ...NATIVE_PROFILE },
    ...overrides,
  };
}

describe("Given the daemon answers with a resolved profile", () => {
  it("When the panel loads, Then it renders what the harness will actually use", async () => {
    mockDaemonFetch.mockReturnValue(reply(resolved()));
    render(<HarnessSection />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("Native schemas")).toBeTruthy();
    });
    expect(screen.getByDisplayValue("Compact")).toBeTruthy();
  });

  it("When a cap is absent, Then it stays empty rather than rendering as zero", async () => {
    mockDaemonFetch.mockReturnValue(reply(resolved()));
    render(<HarnessSection />);

    const field = await screen.findByLabelText("Max output tokens");
    // Absent means "the provider decides". A `0` here would assert a limit
    // nobody set — the substitution the Rust side refuses to make.
    expect((field as HTMLInputElement).value).toBe("");
  });

  it("When nothing is overridden, Then every reset control is disabled", async () => {
    mockDaemonFetch.mockReturnValue(reply(resolved()));
    render(<HarnessSection />);

    const reset = await screen.findByLabelText("Reset Tool transport");
    expect((reset as HTMLButtonElement).disabled).toBe(true);
    expect(screen.queryByText("changed")).toBeNull();
  });

  it("When a field is overridden, Then it is marked and its reset is enabled", async () => {
    mockDaemonFetch.mockReturnValue(
      reply(
        resolved({
          effective: { ...NATIVE_PROFILE, tool_transport: "prose" },
          provider_override: { tool_transport: "prose" },
        })
      )
    );
    render(<HarnessSection />);

    await waitFor(() => expect(screen.getByText("changed")).toBeTruthy());
    const reset = screen.getByLabelText("Reset Tool transport");
    expect((reset as HTMLButtonElement).disabled).toBe(false);
  });
});

describe("Given the user changes a setting", () => {
  it("When a value is picked, Then it is PUT as a patch of just that field", async () => {
    mockDaemonFetch.mockReturnValue(reply(resolved()));
    render(<HarnessSection />);
    await screen.findByDisplayValue("Native schemas");

    fireEvent.change(screen.getByDisplayValue("Native schemas"), {
      target: { value: "prose" },
    });

    await waitFor(() => {
      const put = mockDaemonFetch.mock.calls.find(
        (c) => (c[1] as RequestInit | undefined)?.method === "PUT"
      );
      expect(put).toBeTruthy();
      expect(JSON.parse((put?.[1] as RequestInit).body as string)).toEqual({
        tool_transport: "prose",
      });
    });
  });

  it("When the last override is cleared, Then it is DELETEd rather than stored as nothing", async () => {
    mockDaemonFetch.mockReturnValue(
      reply(
        resolved({
          effective: { ...NATIVE_PROFILE, max_output_tokens: 32000 },
          provider_override: { max_output_tokens: 32000 },
        })
      )
    );
    render(<HarnessSection />);

    const field = await screen.findByLabelText("Max output tokens");
    fireEvent.change(field, { target: { value: "" } });
    fireEvent.blur(field);

    // A stored patch of nothing would pin this pair to today's defaults
    // forever; the row has to be removed instead.
    await waitFor(() => {
      const del = mockDaemonFetch.mock.calls.find(
        (c) => (c[1] as RequestInit | undefined)?.method === "DELETE"
      );
      expect(del).toBeTruthy();
    });
  });

  it("When a number will not parse, Then nothing is sent", async () => {
    mockDaemonFetch.mockReturnValue(reply(resolved()));
    render(<HarnessSection />);
    const field = await screen.findByLabelText("Max output tokens");
    mockDaemonFetch.mockClear();

    fireEvent.change(field, { target: { value: "not a number" } });
    fireEvent.blur(field);

    // Committing NaN — or coercing it to 0 — would be worse than ignoring it.
    await waitFor(() => {
      expect(
        mockDaemonFetch.mock.calls.filter(
          (c) => (c[1] as RequestInit | undefined)?.method
        )
      ).toHaveLength(0);
    });
  });
});

describe("Given the daemon cannot be reached", () => {
  it("When the panel loads, Then it says so instead of rendering defaults", async () => {
    mockDaemonFetch.mockRejectedValue(new Error("ECONNREFUSED"));
    render(<HarnessSection />);

    await waitFor(() => {
      expect(screen.getByText(/Could not reach the VibeCLI daemon/)).toBeTruthy();
    });
    // Showing built-in values here would tell the user their models are
    // configured one way while the running daemon does something else.
    expect(screen.queryByDisplayValue("Native schemas")).toBeNull();
  });

  it("When the daemon answers with an error status, Then the status is reported", async () => {
    mockDaemonFetch.mockReturnValue(reply(null, false, 401));
    render(<HarnessSection />);

    await waitFor(() => {
      expect(screen.getByText(/Daemon returned 401/)).toBeTruthy();
    });
  });

  it("When the reply is not the shape we expect, Then it is rejected, not cast", async () => {
    mockDaemonFetch.mockReturnValue(reply({ unexpected: "shape" }));
    render(<HarnessSection />);

    await waitFor(() => {
      expect(screen.getByText(/Unreadable response/)).toBeTruthy();
    });
  });
});
