/**
 * BDD: the Plugins panel tells the truth about what it installed.
 *
 * The panel's whole job is claims — "Added", "N live components", "enabled",
 * "Started at 14:02". Each one is about a workspace the panel cannot see, so
 * each is a place where the UI can be confidently wrong. These scenarios drive
 * the real component and assert on the *invoke payload* wherever an action is
 * involved, because that is what the daemon acts on: a test that only checks
 * the rendered row passes for a panel that shows one thing and sends another.
 *
 * Three claims get their own scenarios because they are the ones that would
 * hurt: a connector is never described as working before Test has run it, a
 * plugin an administrator pinned offers no button that would fail, and one
 * failed load never blanks the panels that answered.
 */

import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { PluginsView } from "../PluginsView";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const testFirst = {
  name: "core-test-first",
  title: "Test first",
  version: "1.0.0",
  description: "Pin behaviour with a failing test before changing it.",
  category: "Engineering Practice",
  components: [{ kind: "skill", name: "test-first" }],
  includes: [],
  connectors: [],
  installed: false,
  policy: null as string | null,
};

const engineeringBundle = {
  name: "bundle-engineering",
  title: "Engineering",
  version: "1.0.0",
  description: "The everyday coding setup.",
  category: "Bundles",
  components: [],
  includes: ["core-test-first"],
  connectors: ["github"],
  installed: false,
  policy: null as string | null,
};

const githubSpec = {
  id: "github",
  title: "GitHub",
  description: "Issues, pull requests and code search.",
  category: "Web And Search",
  command: "npx",
  args: ["-y", "@modelcontextprotocol/server-github"],
  runtime: "npx",
  runtime_program: "npx",
  runtime_available: true,
  credentials: [
    { env: "GITHUB_PERSONAL_ACCESS_TOKEN", label: "Personal access token", help: "repo scope" },
  ],
  docs_url: "https://example.invalid/docs",
};

const emptyInventory = {
  available: true,
  total: 0,
  mcp_servers: [],
  skills: [],
  subagents: [],
  rules: [],
  hooks: [],
};

/**
 * Answer the three mount-time loads. Each override replaces one answer; an
 * override that is an `Error` makes that one load reject, which is how the
 * "one failure must not blank the others" scenarios are set up.
 */
function daemon(overrides: {
  inventory?: unknown;
  catalog?: unknown;
  connectors?: unknown;
  actions?: Record<string, unknown>;
} = {}) {
  const answers: Record<string, unknown> = {
    list_plugins: overrides.inventory ?? emptyInventory,
    plugin_catalog: overrides.catalog ?? { plugins: [] },
    list_connectors: overrides.connectors ?? { connectors: [], catalog: [] },
    ...(overrides.actions ?? {}),
  };
  mockInvoke.mockImplementation((cmd: string) => {
    const answer = answers[cmd];
    if (answer === undefined) return Promise.reject(new Error(`unstubbed command ${cmd}`));
    if (answer instanceof Error) return Promise.reject(answer);
    return Promise.resolve(answer);
  });
}

/**
 * Find the deepest element whose *whole* text matches. The panel builds several
 * of its sentences from a `<strong>` plus surrounding text, so the default
 * matcher — which tests one text node at a time — cannot see them.
 */
function findWholeText(re: RegExp) {
  return screen.findByText((_content, el) => {
    if (!el || !re.test(el.textContent ?? "")) return false;
    return !Array.from(el.children).some((c) => re.test(c.textContent ?? ""));
  });
}

/** The payload of the first call to `cmd`. */
function payloadOf(cmd: string): Record<string, unknown> {
  const call = mockInvoke.mock.calls.find(([name]) => name === cmd);
  if (!call) throw new Error(`${cmd} was never invoked`);
  return call[1] as Record<string, unknown>;
}

function open(props: Partial<React.ComponentProps<typeof PluginsView>> = {}) {
  return render(
    <PluginsView
      daemonUrl="http://127.0.0.1:7878"
      path="/work/repo"
      onClose={() => {}}
      {...props}
    />,
  );
}

beforeEach(() => {
  mockInvoke.mockReset();
});

// ── Loading the marketplace ─────────────────────────────────────────────────

describe("Given a daemon with a catalog", () => {
  it("When the panel opens, Then it asks for the scoped workspace, not the daemon's cwd", async () => {
    daemon({ catalog: { plugins: [testFirst] } });
    open();

    await screen.findByText("Test first");
    // A plugin policy is per workspace. Dropping `path` would silently show
    // and edit whichever workspace the daemon happens to have started in.
    for (const cmd of ["list_plugins", "plugin_catalog", "list_connectors"]) {
      expect(payloadOf(cmd)).toMatchObject({ url: "http://127.0.0.1:7878", path: "/work/repo" });
    }
  });

  it("When a bundle brings other plugins, Then the card says so before it is clicked", async () => {
    daemon({
      catalog: { plugins: [engineeringBundle] },
      connectors: { connectors: [], catalog: [githubSpec] },
    });
    open();

    // "Includes 1 plugin · GitHub" — the connector named by its catalog title,
    // not the raw id, so the card reads as what will be set up.
    const brings = await screen.findByText(/Includes/);
    expect(brings.textContent).toContain("1 plugin");
    expect(brings.textContent).toContain("GitHub");
  });

  it("When a category is newer than this build, Then its entries are still shown", async () => {
    daemon({
      catalog: {
        plugins: [{ ...testFirst, category: "Quantum Ops" }],
      },
    });
    open();

    // A newer daemon must not be able to hide entries from an older panel by
    // filing them under a section this build's ordered list has never heard of.
    expect(await screen.findByText("Quantum Ops")).toBeTruthy();
    expect(screen.getByText("Test first")).toBeTruthy();
  });

  it("When the catalog route fails, Then the connectors still render and the error is named", async () => {
    daemon({
      catalog: new Error("404 Not Found"),
      connectors: { connectors: [], catalog: [githubSpec] },
    });
    open();

    // Awaiting the three loads together meant one stale route blanked the
    // whole panel, including the parts that answered fine.
    expect(await screen.findByText("GitHub")).toBeTruthy();
    expect(screen.getByText(/404 Not Found/)).toBeTruthy();
  });

  it("When the search box is used, Then only matching entries remain", async () => {
    daemon({
      catalog: { plugins: [testFirst, engineeringBundle] },
    });
    open();

    await screen.findByText("Test first");
    fireEvent.change(screen.getByLabelText("Search plugins"), { target: { value: "everyday" } });

    // Matches the bundle's description, not the plugin's.
    expect(screen.getByText("Engineering")).toBeTruthy();
    expect(screen.queryByText("Test first")).toBeNull();
  });
});

// ── Installing ───────────────────────────────────────────────────────────────

describe("Given a plugin in the marketplace", () => {
  it("When Add is clicked, Then the daemon is asked to install that exact plugin", async () => {
    daemon({
      catalog: { plugins: [testFirst] },
      actions: {
        install_plugin: { components: 1, signing_key_persisted: true, included: [], connectors: [] },
      },
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: "Add" }));

    await waitFor(() => expect(payloadOf("install_plugin")).toBeTruthy());
    expect(payloadOf("install_plugin")).toMatchObject({
      url: "http://127.0.0.1:7878",
      path: "/work/repo",
      name: "core-test-first",
    });
  });

  it("When the install needs a credential it cannot invent, Then the panel says which", async () => {
    daemon({
      catalog: { plugins: [engineeringBundle] },
      connectors: { connectors: [], catalog: [githubSpec] },
      actions: {
        install_plugin: {
          components: 0,
          signing_key_persisted: true,
          included: ["core-test-first"],
          connectors: [
            {
              id: "github",
              title: "GitHub",
              state: "needs_credentials",
              fields: ["GITHUB_PERSONAL_ACCESS_TOKEN"],
            },
          ],
        },
      },
    });
    open();

    // The connector this bundle expects is in the marketplace too, so scope
    // the click to the bundle's own row.
    const bundleRow = (await screen.findByText("Engineering")).closest(
      ".vx-market__row",
    ) as HTMLElement;
    fireEvent.click(within(bundleRow).getByRole("button", { name: "Add" }));

    // Reporting a half-configured bundle as "set up" is the feature lying at
    // the moment it is most believed.
    const notice = await findWholeText(/1 connector still needs a credential/);
    expect(notice.textContent).toContain("Added Engineering");
    expect(await screen.findByRole("button", { name: "Set up GitHub" })).toBeTruthy();
  });

  it("When the signing key could not be stored, Then the fingerprint warning is shown", async () => {
    daemon({
      catalog: { plugins: [testFirst] },
      actions: {
        install_plugin: {
          components: 1,
          signing_key_persisted: false,
          included: [],
          connectors: [],
        },
      },
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: "Add" }));
    expect(await screen.findByText(/publisher fingerprint will differ/)).toBeTruthy();
  });

  it("When the install fails, Then the daemon's own sentence is shown, not a status code", async () => {
    daemon({
      catalog: { plugins: [testFirst] },
      actions: { install_plugin: new Error("signature does not verify") },
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: "Add" }));
    expect(await screen.findByText(/signature does not verify/)).toBeTruthy();
  });

  it("When a plugin is already installed, Then it offers no Add button", async () => {
    daemon({ catalog: { plugins: [{ ...testFirst, installed: true, policy: "on" }] } });
    open();

    await screen.findByText("Test first");
    expect(screen.queryByRole("button", { name: "Add" })).toBeNull();
    expect(screen.getAllByText("Added").length).toBeGreaterThan(0);
  });
});

// ── Managing what is installed ──────────────────────────────────────────────

describe("Given an installed plugin", () => {
  const installed = { ...testFirst, installed: true, policy: "on" };

  async function openYours(catalogPlugins: unknown[], actions: Record<string, unknown> = {}) {
    daemon({
      catalog: { plugins: catalogPlugins },
      inventory: { ...emptyInventory, total: 1 },
      actions,
    });
    open();
    fireEvent.click(await screen.findByRole("tab", { name: "Yours" }));
  }

  it("When Disable is clicked, Then the policy is set to off for this workspace", async () => {
    await openYours([installed], { set_plugin_policy: null });

    fireEvent.click(await screen.findByRole("button", { name: "Disable" }));

    await waitFor(() => expect(payloadOf("set_plugin_policy")).toBeTruthy());
    expect(payloadOf("set_plugin_policy")).toMatchObject({
      path: "/work/repo",
      name: "core-test-first",
      policy: "off",
    });
  });

  it("When it is disabled, Then the button offers to enable it again", async () => {
    await openYours([{ ...installed, policy: "off" }], { set_plugin_policy: null });

    fireEvent.click(await screen.findByRole("button", { name: "Enable" }));
    await waitFor(() => expect(payloadOf("set_plugin_policy")).toBeTruthy());
    expect(payloadOf("set_plugin_policy")).toMatchObject({ policy: "on" });
  });

  it("When an administrator pinned it, Then no button is offered that would fail", async () => {
    await openYours([{ ...installed, policy: "required" }]);

    // `required` cannot be lowered without admin. Offering Disable here would
    // produce a button whose only outcome is an error.
    await screen.findByText(/Pinned by an administrator/);
    expect(screen.queryByRole("button", { name: "Disable" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Remove" })).toBeNull();
  });

  it("When Remove finds no files on disk, Then it says the policy row was cleared", async () => {
    await openYours([installed], { uninstall_plugin: { removed: false } });

    fireEvent.click(await screen.findByRole("button", { name: "Remove" }));
    expect(await screen.findByText(/had no files on disk/)).toBeTruthy();
  });

  it("When the live-component count is known, Then it is shown as measured, not inferred", async () => {
    await openYours([installed]);
    // The count comes from the daemon's inventory of what actually loaded,
    // which is a different number from "how many components the manifest
    // declares" whenever a component file is unreadable.
    expect(await screen.findByText("1 live component")).toBeTruthy();
  });
});

// ── Connectors: never claim it works until it has run ───────────────────────

describe("Given a configured connector", () => {
  const configured = {
    id: "github",
    catalog_id: "github",
    title: "GitHub",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-github"],
    enabled: true,
    added_at: 1_700_000_000_000,
    credential_names: ["GITHUB_PERSONAL_ACCESS_TOKEN"],
    missing_credentials: [],
  };

  async function openYours(actions: Record<string, unknown> = {}) {
    daemon({
      connectors: { connectors: [configured], catalog: [githubSpec] },
      actions,
    });
    open();
    fireEvent.click(await screen.findByRole("tab", { name: "Yours" }));
  }

  it("When it has never been tested, Then the panel says so rather than implying it works", async () => {
    await openYours();
    // Having a credential is not evidence the process starts. There is no
    // state here inferred from configuration.
    expect(await screen.findByText("Not tested yet.")).toBeTruthy();
  });

  it("When Test fails, Then the failure is reported, not a success", async () => {
    await openYours({
      probe_connector: {
        result: { state: "failed", error: "npx: command not found" },
        checked_at: 1_700_000_000_000,
      },
    });

    fireEvent.click(await screen.findByRole("button", { name: "Test" }));
    const line = await findWholeText(/Failed at .*npx: command not found/);
    expect(line.textContent).toContain("npx: command not found");
  });

  it("When Test times out, Then it says started-but-silent, not started", async () => {
    await openYours({
      probe_connector: {
        result: { state: "timedout", after_secs: 10 },
        checked_at: 1_700_000_000_000,
      },
    });

    fireEvent.click(await screen.findByRole("button", { name: "Test" }));
    // "Started" alone would be read as working. It launched and said nothing,
    // which is a different fact.
    expect(await screen.findByText(/silent for 10s/)).toBeTruthy();
  });

  it("When Test succeeds, Then the tool count comes with the time it was measured", async () => {
    await openYours({
      probe_connector: {
        result: { state: "ok", tools: ["search_code", "list_issues"] },
        checked_at: 1_700_000_000_000,
      },
    });

    fireEvent.click(await screen.findByRole("button", { name: "Test" }));
    // A result with no time is a claim about now made from a measurement of
    // then, so the line carries both.
    const line = await screen.findByText(/Started at .*2 tools/);
    expect(line.textContent).toContain("search_code");
  });

  it("When Remove deletes stored secrets, Then it says how many", async () => {
    await openYours({ remove_connector: { secrets_deleted: 1 } });

    const row = (await screen.findByText("GitHub")).closest(".vx-market__manage") as HTMLElement;
    fireEvent.click(within(row).getByRole("button", { name: "Remove" }));
    expect(await screen.findByText(/deleted 1 stored credential/)).toBeTruthy();
  });

  it("When its runtime is missing from PATH, Then the row says it cannot start", async () => {
    daemon({
      connectors: {
        connectors: [configured],
        catalog: [{ ...githubSpec, runtime_available: false }],
      },
    });
    open();
    fireEvent.click(await screen.findByRole("tab", { name: "Yours" }));

    expect(await screen.findByText(/is not on PATH, so this cannot start/)).toBeTruthy();
  });
});

// ── The empty state ─────────────────────────────────────────────────────────

describe("Given a workspace with nothing installed", () => {
  it("When Yours is opened, Then it points at the marketplace rather than a CLI command", async () => {
    daemon();
    open();
    fireEvent.click(await screen.findByRole("tab", { name: "Yours" }));

    expect(await screen.findByText(/Nothing is extending the agent/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Browse the marketplace" }));
    expect(screen.getByRole("tab", { name: "Marketplace" }).getAttribute("aria-selected")).toBe(
      "true",
    );
  });
});
