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

/**
 * The panel builds its own authenticated request from `invoke("daemon_port")`
 * and `invoke("daemon_token_effective")`, the same way `useVoiceSettings` does,
 * so the seam under test is `fetch` rather than a helper.
 */
const mockDaemonFetch = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string) =>
    cmd === "daemon_port" ? Promise.resolve(7878) : Promise.resolve("test-token"),
}));

import { HarnessSection } from "@vibe/shared/settings/HarnessSection";

/**
 * Block body, not an expression body.
 *
 * `beforeEach(() => mock.mockReset())` returns the mock — and Vitest treats a
 * returned function as a teardown, so the mock is *called* after every test.
 * A throwing mock then fails its own test with no useful diff.
 */
beforeEach(() => {
  mockDaemonFetch.mockReset();
  // `/models` is a separate, public call the panel makes to populate its model
  // list. It is not what these scenarios are about, so it answers empty and
  // everything else goes to the mock under test.
  vi.stubGlobal("fetch", (url: string, init?: RequestInit) =>
    String(url).includes("/models")
      ? Promise.resolve({ ok: true, json: () => Promise.resolve({ models: [] }) })
      : mockDaemonFetch(url, init)
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
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

const ALL_FIELDS = [
  "tool_transport",
  "prompt_dialect",
  "max_output_tokens",
  "temperature",
  "parallel_tool_calls",
  "thinking_budgets",
  "prompt_cache",
  "context_window_fallback",
  "system_prompt_suffix",
];

function resolved(overrides: Record<string, unknown> = {}) {
  return {
    provider: "anthropic",
    model: "*",
    effective: { ...NATIVE_PROFILE },
    builtin: { ...NATIVE_PROFILE },
    honored_fields: ALL_FIELDS,
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

describe("Given a provider that cannot act on every knob", () => {
  /**
   * Only `claude.rs` reads `prompt_cache` — it is Anthropic's `cache_control`.
   * The panel used to draw the checkbox for every provider, so toggling it on
   * OpenAI saved, read back as "changed", and did nothing: the
   * success-assuming failure this codebase names as its dominant bug family,
   * arrived at through the UI instead of a return value.
   */
  it("When the daemon says a field is not honored, Then no control is drawn for it", async () => {
    mockDaemonFetch.mockReturnValue(
      reply(
        resolved({
          provider: "openai",
          honored_fields: ALL_FIELDS.filter((f) => f !== "prompt_cache"),
        })
      )
    );
    render(<HarnessSection />);

    await screen.findByDisplayValue("Native schemas");
    expect(screen.queryByLabelText("Reset Prompt caching")).toBeNull();
  });

  it("When a field is honored, Then its control is drawn", async () => {
    mockDaemonFetch.mockReturnValue(reply(resolved()));
    render(<HarnessSection />);

    expect(await screen.findByLabelText("Reset Prompt caching")).toBeTruthy();
  });

  /**
   * An older daemon sends no `honored_fields`. Treating that as "everything"
   * would put the do-nothing controls straight back, so the fallback is the
   * four the agent loop acts on for every provider.
   */
  it("When the daemon is older and sends no list, Then only the universal knobs show", async () => {
    const { honored_fields: _omitted, ...withoutList } = resolved();
    mockDaemonFetch.mockReturnValue(reply(withoutList));
    render(<HarnessSection />);

    await screen.findByDisplayValue("Native schemas");
    expect(screen.queryByLabelText("Reset Prompt caching")).toBeNull();
    expect(screen.queryByLabelText("Reset Temperature")).toBeNull();
    // ...but the ones the agent loop always honors are still offered.
    expect(screen.getByLabelText("Reset Tool transport")).toBeTruthy();
    expect(screen.getByLabelText("Reset Context window fallback")).toBeTruthy();
  });
});
