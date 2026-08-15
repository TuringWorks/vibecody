/**
 * BDD: Connectors as a first-class surface.
 *
 * Connectors were reachable only inside the Plugins marketplace, mixed into the
 * same list as plugin bundles. They are a different thing — a plugin extends
 * the agent with skills and hooks that ship in a bundle, a connector is an MCP
 * server the workspace talks to, with credentials and a process that either
 * starts or does not. This gives them their own view, with the one question a
 * connector list has to answer: does it actually work.
 *
 * Every scenario drives the real component and asserts on the *invoke payload*
 * where an action is involved, because that is what the daemon acts on. A test
 * that only checks the rendered row passes for a panel that displays one thing
 * and sends another.
 */

import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { ConnectorsView } from "../ConnectorsView";

const CATALOG = [
  {
    id: "vibecli",
    title: "VibeCLI tools",
    description: "This machine's own VibeCLI, exposed over MCP.",
    category: "VibeCody",
    command: "vibecli",
    args: ["--mcp-server"],
    runtime: "builtin",
    runtime_program: "vibecli",
    runtime_available: true,
    credentials: [],
    docs_url: "https://github.com/TuringWorks/vibecody",
  },
  {
    id: "github",
    title: "GitHub",
    description: "Issues, pull requests and code search.",
    category: "Development",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-github"],
    runtime: "npx",
    runtime_program: "npx",
    runtime_available: true,
    credentials: [
      { env: "GITHUB_PERSONAL_ACCESS_TOKEN", label: "Personal access token", help: "repo scope" },
    ],
    docs_url: "https://github.com/modelcontextprotocol/servers",
  },
  {
    id: "fetch",
    title: "Fetch",
    description: "Retrieve a URL and hand back its text.",
    category: "Utilities",
    command: "uvx",
    args: ["mcp-server-fetch"],
    runtime: "uvx",
    runtime_program: "uvx",
    runtime_available: true,
    credentials: [],
    docs_url: "https://github.com/modelcontextprotocol/servers",
  },
  {
    id: "sentry",
    title: "Sentry",
    description: "Error tracking.",
    category: "Observability",
    command: "uvx",
    args: ["mcp-server-sentry"],
    runtime: "uvx",
    runtime_program: "uvx",
    // Deliberately unavailable: the row must say so rather than offer a
    // connector that cannot start.
    runtime_available: false,
    credentials: [{ env: "SENTRY_AUTH_TOKEN", label: "Auth token", help: "" }],
    docs_url: "https://github.com/modelcontextprotocol/servers",
  },
];

const CONFIGURED = [
  {
    id: "vibecli",
    catalog_id: "vibecli",
    title: "VibeCLI tools",
    command: "vibecli",
    args: ["--mcp-server"],
    enabled: true,
    added_at: 1786750545130,
    credential_names: [],
    missing_credentials: [],
  },
];

function listReply(connectors = CONFIGURED) {
  return { connectors, catalog: CATALOG };
}

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case "list_connectors":
        return listReply();
      case "add_connector":
        return { id: "fetch", title: "Fetch", enabled: true, credential_names: [] };
      case "toggle_connector":
        return { id: "vibecli", enabled: false };
      case "remove_connector":
        return { id: "vibecli", removed: true, secrets_deleted: 0 };
      case "probe_connector":
        return { id: "vibecli", result: { state: "ok", tools: ["read_file", "bash"] }, checked_at: 1 };
      default:
        return null;
    }
  });
});

const props = { daemonUrl: "http://127.0.0.1:7878", path: "/tmp/ws", onClose: () => {} };

describe("Given the Connectors view is opened", () => {
  it("When it loads, Then every catalog connector is listed", async () => {
    render(<ConnectorsView {...props} />);

    // "list all connectors" is the requirement: every catalog entry appears,
    // not just the ones already configured.
    for (const spec of CATALOG) {
      expect(await screen.findByText(spec.title)).toBeInTheDocument();
    }
  });

  it("When a connector is already configured, Then it is shown as added rather than offered again", async () => {
    render(<ConnectorsView {...props} />);

    const row = await screen.findByTestId("connector-vibecli");
    expect(within(row).getByTestId("connector-state")).toHaveTextContent(/added|configured|enabled/i);
  });

  it("When a connector's runtime is missing, Then the row says so", async () => {
    render(<ConnectorsView {...props} />);

    const row = await screen.findByTestId("connector-sentry");
    expect(row).toHaveTextContent(/uvx/i);
    expect(within(row).getByTestId("connector-runtime-warning")).toBeInTheDocument();
  });
});

describe("Given a connector that needs no credentials", () => {
  it("When it is added, Then the daemon is asked by catalog id", async () => {
    render(<ConnectorsView {...props} />);

    const row = await screen.findByTestId("connector-fetch");
    fireEvent.click(within(row).getByRole("button", { name: /^add$/i }));

    await waitFor(() => {
      const call = mockInvoke.mock.calls.find(c => c[0] === "add_connector");
      expect(call, "add_connector should have been invoked").toBeTruthy();
      // `catalogId`, not `id`: the daemon treats an `id` without a command as a
      // custom connector and answers 400 "a command is required".
      expect(call![1]).toMatchObject({ catalogId: "fetch", path: "/tmp/ws" });
    });
  });
});

describe("Given a configured connector", () => {
  it("When it is probed, Then the tools it reported are shown", async () => {
    render(<ConnectorsView {...props} />);

    const row = await screen.findByTestId("connector-vibecli");
    fireEvent.click(within(row).getByRole("button", { name: /test|probe/i }));

    expect(await screen.findByText(/read_file/)).toBeInTheDocument();
  });

  /**
   * The failure this whole feature exists to surface.
   *
   * A server can complete the handshake and still offer nothing — that is
   * exactly what `server-everything` did while the MCP client read a
   * notification as its reply. "ok" alone would render that as working, so the
   * count has to be visible and zero has to read as a problem.
   */
  it("When a probe returns no tools, Then it does not read as working", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_connectors") return listReply();
      if (cmd === "probe_connector")
        return { id: "vibecli", result: { state: "ok", tools: [] }, checked_at: 1 };
      return null;
    });
    render(<ConnectorsView {...props} />);

    const row = await screen.findByTestId("connector-vibecli");
    fireEvent.click(within(row).getByRole("button", { name: /test|probe/i }));

    await waitFor(() => {
      // The row shows "Testing…" first, so assert the settled verdict rather
      // than whichever state happened to render first.
      expect(within(row).getByTestId("connector-probe")).toHaveTextContent(/no tools/i);
    });
    const verdict = within(row).getByTestId("connector-probe");
    expect(verdict).not.toHaveTextContent(/^\s*(ok|working)\s*$/i);
  });

  it("When a probe fails, Then the reason is shown rather than a bare failure", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_connectors") return listReply();
      if (cmd === "probe_connector")
        return {
          id: "vibecli",
          result: { state: "failed", error: "not a valid Git repository" },
          checked_at: 1,
        };
      return null;
    });
    render(<ConnectorsView {...props} />);

    const row = await screen.findByTestId("connector-vibecli");
    fireEvent.click(within(row).getByRole("button", { name: /test|probe/i }));

    expect(await screen.findByText(/not a valid Git repository/i)).toBeInTheDocument();
  });

  it("When it is toggled off, Then the daemon is told", async () => {
    render(<ConnectorsView {...props} />);

    const row = await screen.findByTestId("connector-vibecli");
    fireEvent.click(within(row).getByRole("button", { name: /disable|turn off/i }));

    await waitFor(() => {
      const call = mockInvoke.mock.calls.find(c => c[0] === "toggle_connector");
      expect(call![1]).toMatchObject({ id: "vibecli", enabled: false });
    });
  });
});

describe("Given the daemon cannot be reached", () => {
  it("When the list fails, Then the view says so instead of showing an empty catalog", async () => {
    mockInvoke.mockImplementation(async () => {
      throw new Error("Cannot reach daemon");
    });
    render(<ConnectorsView {...props} />);

    // An empty list would read as "no connectors exist", which is a different
    // and wrong statement.
    expect(await screen.findByRole("alert")).toHaveTextContent(/daemon/i);
  });
});
