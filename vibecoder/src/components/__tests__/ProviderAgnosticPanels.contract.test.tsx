/**
 * Contract: a panel handed a provider must use it.
 *
 * Every panel in the AI Playground composite shipped ignoring the toolbar
 * selection — Counsel ran on a hard-coded claude/openai/gemini trio, Arena and
 * Compare pinned side A to ollama, SuperBrain enabled `claude || openai`. Four
 * panels, one bug, and it reached a user (AGENTS.md → Provider-Agnostic Panels
 * — STRICT).
 *
 * ArenaPanel already had a BDD suite when its bug shipped. Every case in it
 * renders `<ArenaPanel />` with no props, so the suite encoded the defect as
 * the expected usage: a test that never passes a provider cannot notice one
 * being dropped. Type-checking cannot either — passing a prop a component does
 * not declare is legal TypeScript and React discards it silently.
 *
 * So these tests deliberately do the thing the per-panel suites do not: mount
 * each panel the way `LazyPanels` actually mounts it, with a provider, and
 * assert the provider reaches the *outgoing invoke payload*. Asserting on the
 * rendered dropdown would pass on a panel that displays the selection and then
 * sends something else; the payload is what the backend acts on.
 *
 * `TOOLBAR` is deliberately none of the values any panel used to default to,
 * so a panel falling back to its old constant fails rather than coinciding
 * with the expectation.
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const TOOLBAR = "perplexity";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: async () => () => {},
}));

// `vi.mock` is hoisted above every const, so this factory repeats the literal
// "perplexity" rather than referencing TOOLBAR.
vi.mock("../../hooks/useModelRegistry", () => ({
  useModelRegistry: () => ({
    providers: ["ollama", "openai", "claude", "gemini", "perplexity"],
    modelsForProvider: (p: string) =>
      p === "perplexity" ? ["sonar-pro", "sonar"] : ["llama3", "gpt-4"],
  }),
  PROVIDER_DEFAULT_MODEL: {
    ollama: "llama3",
    openai: "gpt-4",
    claude: "claude-sonnet-4-6",
    gemini: "gemini-2.5-flash",
    perplexity: "sonar-pro",
  },
  getDefaultProvider: () => "openai",
  ALL_PROVIDERS: ["ollama", "openai", "claude", "gemini", "perplexity"],
  DEFAULT_PROVIDER: "openai",
}));

import { ArenaPanel } from "../ArenaPanel";
import { MultiModelPanel } from "../MultiModelPanel";
import { CounselPanel } from "../CounselPanel";
import { SuperBrainPanel } from "../SuperBrainPanel";

/** Every provider named anywhere in a recorded invoke payload. */
function providersSentTo(command: string): string[] {
  const found: string[] = [];
  for (const call of mockInvoke.mock.calls) {
    if (call[0] !== command) continue;
    JSON.stringify(call[1] ?? {}, (_k, v) => {
      if (typeof v === "string") found.push(v);
      return v;
    });
  }
  return found;
}

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case "get_arena_history":
        return [[], []];
      case "counsel_list_sessions":
        return [];
      case "counsel_create_session":
        return { id: "s1", topic: "t", participants: [], rounds: [], moderator_index: 0, status: "AwaitingUser" };
      default:
        return null;
    }
  });
});

describe("Given the toolbar has a provider selected", () => {
  it("When Arena runs a battle, Then side A is the selected provider", async () => {
    render(<ArenaPanel provider={TOOLBAR} />);

    fireEvent.change(screen.getByPlaceholderText(/prompt|ask/i), {
      target: { value: "compare these" },
    });
    fireEvent.click(screen.getByRole("button", { name: /battle/i }));

    await waitFor(() => {
      expect(providersSentTo("compare_models")).toContain(TOOLBAR);
    });
  });

  it("When Compare runs, Then side A is the selected provider", async () => {
    render(<MultiModelPanel provider={TOOLBAR} />);

    fireEvent.change(screen.getByPlaceholderText(/prompt|ask/i), {
      target: { value: "compare these" },
    });
    fireEvent.click(screen.getByRole("button", { name: /compare/i }));

    await waitFor(() => {
      expect(providersSentTo("compare_models")).toContain(TOOLBAR);
    });
  });

  it("When a Counsel session is created, Then every seat uses the selected provider", async () => {
    render(<CounselPanel provider={TOOLBAR} />);

    const topic = await screen.findByPlaceholderText(/topic|discuss|debate/i);
    fireEvent.change(topic, { target: { value: "should we ship" } });
    fireEvent.click(screen.getByRole("button", { name: /start counsel/i }));

    await waitFor(() => {
      const sent = providersSentTo("counsel_create_session");
      expect(sent).toContain(TOOLBAR);
      // The old hard-coded trio must not survive anywhere in the payload.
      expect(sent).not.toContain("claude");
      expect(sent).not.toContain("gemini");
    });
  });

  it("When SuperBrain runs, Then only the selected provider is queried", async () => {
    render(<SuperBrainPanel provider={TOOLBAR} />);

    fireEvent.change(screen.getByPlaceholderText(/ask anything/i), {
      target: { value: "orchestrate this" },
    });
    // A mode tile also carries the word "Think"; the submit control is the
    // real <button>, not the role="button" div.
    const submit = screen
      .getAllByRole("button", { name: /think/i })
      .find(el => el.tagName === "BUTTON");
    fireEvent.click(submit!);

    await waitFor(() => {
      const sent = providersSentTo("superbrain_query");
      expect(sent).toContain(TOOLBAR);
      // `enabled: p === "claude" || p === "openai"` put these in the payload
      // no matter what the toolbar said.
      expect(sent).not.toContain("claude");
      expect(sent).not.toContain("openai");
    });
  });
});

describe("Given the toolbar has no provider selected", () => {
  it("When Counsel is set up, Then it seats nobody rather than guessing", async () => {
    render(<CounselPanel />);

    // Silently falling back to a provider is the failure mode this whole
    // contract exists to stop, so "no selection" must produce no seats.
    expect(
      await screen.findByText(/no model selected/i)
    ).toBeInTheDocument();

    expect(providersSentTo("counsel_create_session")).toHaveLength(0);
  });
});
