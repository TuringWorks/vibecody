/**
 * BDD: Red Team attacks the workspace's code and content, not only a URL.
 *
 * The panel shipped URL-only: it assumed a running website and its "pipeline"
 * was hardcoded sleeps against a backend that produced no findings. This covers
 * the workspace mode — the real per-file loop over `redteam_workspace_targets`
 * + `redteam_file` — and the two things that make it honest: no fabricated
 * CVSS, and a Fix-with-AI hand-off carrying the attack, not a raw dump.
 */

import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("../../hooks/useModelRegistry", () => ({
  parseProviderSelection: (display: string) =>
    display ? { provider: "ollama", model: "devstral-2" } : { provider: "", model: "" },
}));
vi.mock("../../utils/effort", () => ({ getSelectedEffort: () => "medium" }));

import { RedTeamPanel } from "../RedTeamPanel";

const finding = (over: Record<string, unknown> = {}) => ({
  id: "src/db.rs-0",
  attack_vector: "SQL injection",
  cvss_score: 0,
  severity: "high",
  url: "",
  location: "src/db.rs:42",
  title: "Unparameterised query",
  description: "User input concatenated into SQL",
  poc: "' OR 1=1--",
  remediation: "Use a parameterised query",
  source_file: "src/db.rs",
  source_line: 42,
  confirmed: false,
  ...over,
});

/** targets → files; findingsByFile → what each file's review returns. */
function setup(
  targets: { files: string[]; matched: number; limit: number },
  findingsByFile: Record<string, unknown[]> = {},
) {
  mockInvoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
    switch (cmd) {
      case "redteam_workspace_targets": return targets;
      case "read_file": return "// source";
      case "redteam_file": return findingsByFile[args.file as string] ?? [];
      case "redteam_save_session": return null;
      case "get_redteam_sessions": return [];
      default: return null;
    }
  });
}

const renderPanel = (props: Record<string, unknown> = {}) =>
  render(<RedTeamPanel workspacePath="/ws" provider="Ollama (devstral-2)" {...props} />);

// The activity log auto-scrolls; jsdom implements no scrolling.
Element.prototype.scrollIntoView = vi.fn();

beforeEach(() => mockInvoke.mockReset());

describe("Red Team — workspace mode", () => {
  it("defaults to Workspace when a workspace is open", () => {
    setup({ files: [], matched: 0, limit: 40 });
    renderPanel();
    expect(
      (screen.getByRole("button", { name: /Workspace/ })).getAttribute("aria-pressed"),
    ).toBe("true");
    expect(screen.getByRole("button", { name: /Run Red Team/ })).toBeTruthy();
  });

  it("reviews every file the scope resolves to and shows the findings", async () => {
    setup(
      { files: ["src/db.rs", "README.md"], matched: 2, limit: 40 },
      {
        "src/db.rs": [finding()],
        "README.md": [finding({ id: "README.md-0", severity: "critical", attack_vector: "prompt injection", location: "README.md", source_file: "README.md", source_line: null, title: "Jailbreak framing", poc: "" })],
      },
    );
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Run Red Team/ }));

    await waitFor(() => expect(screen.getByText("Findings (2)")).toBeTruthy());
    expect(mockInvoke).toHaveBeenCalledWith("redteam_workspace_targets", { workspace: "/ws", pattern: null });
    expect(screen.getByText("Unparameterised query")).toBeTruthy();
    expect(screen.getByText("Jailbreak framing")).toBeTruthy();
  });

  it("reviews content files, not only code — that is the whole point", async () => {
    // A doc that feeds a model is an injection surface; the security review
    // skips Markdown by design, so the red team must not.
    setup(
      { files: ["docs/agent.md"], matched: 1, limit: 40 },
      { "docs/agent.md": [finding({ id: "docs/agent.md-0", attack_vector: "prompt injection", source_file: "docs/agent.md", location: "docs/agent.md", title: "Instruction override" })] },
    );
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Run Red Team/ }));

    await waitFor(() => expect(screen.getByText("Instruction override")).toBeTruthy());
    expect(mockInvoke).toHaveBeenCalledWith("redteam_file", expect.objectContaining({ file: "docs/agent.md" }));
  });

  it("shows the severity, never a fabricated CVSS, for a workspace finding", async () => {
    setup({ files: ["src/db.rs"], matched: 1, limit: 40 }, { "src/db.rs": [finding()] });
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Run Red Team/ }));

    await waitFor(() => expect(screen.getByText("Unparameterised query")).toBeTruthy());
    // cvss_score is 0 → the badge is the severity word, not "CVSS 0.0".
    expect(screen.getByText("HIGH")).toBeTruthy();
    expect(screen.queryByText(/CVSS/)).toBeNull();
  });

  it("passes the toolbar's provider and model to every file review", async () => {
    setup({ files: ["src/db.rs"], matched: 1, limit: 40 }, { "src/db.rs": [finding()] });
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Run Red Team/ }));

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "redteam_file",
        expect.objectContaining({ provider: "ollama", model: "devstral-2", effort: "medium" }),
      ),
    );
  });

  it("refuses to run with no provider selected, rather than defaulting to one", async () => {
    setup({ files: ["src/db.rs"], matched: 1, limit: 40 });
    renderPanel({ provider: "" });
    fireEvent.click(screen.getByRole("button", { name: /Run Red Team/ }));

    await waitFor(() => expect(screen.getByText(/Select a provider and model/)).toBeTruthy());
    expect(mockInvoke).not.toHaveBeenCalledWith("redteam_file", expect.anything());
  });

  it("persists the run so it can be exported and revisited", async () => {
    setup({ files: ["src/db.rs"], matched: 1, limit: 40 }, { "src/db.rs": [finding()] });
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Run Red Team/ }));

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("redteam_save_session", expect.objectContaining({
        session: expect.objectContaining({ findings: expect.arrayContaining([expect.objectContaining({ title: "Unparameterised query" })]) }),
      })),
    );
  });

  it("says when the scope was capped, so a partial run never reads as complete", async () => {
    setup({ files: ["a.rs"], matched: 120, limit: 40 }, { "a.rs": [] });
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Run Red Team/ }));

    await waitFor(() => expect(screen.getByText(/scope had 120, capped at 40/)).toBeTruthy());
  });

  it("hands findings to chat as attacks to close, via Fix with AI", async () => {
    setup({ files: ["src/db.rs"], matched: 1, limit: 40 }, { "src/db.rs": [finding()] });
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Run Red Team/ }));

    await waitFor(() => expect(screen.getByText("Findings (1)")).toBeTruthy());
    // The shared hand-off button is present for workspace findings.
    expect(screen.getByRole("button", { name: /Fix with AI/i })).toBeTruthy();
  });
});

describe("Red Team — mode toggle", () => {
  it("still offers the website URL scan", () => {
    setup({ files: [], matched: 0, limit: 40 });
    renderPanel();
    fireEvent.click(screen.getByRole("button", { name: /Website/ }));
    expect(screen.getByPlaceholderText("http://localhost:3000")).toBeTruthy();
  });

  it("starts on Website when no workspace is open", () => {
    setup({ files: [], matched: 0, limit: 40 });
    render(<RedTeamPanel workspacePath={null} provider="Ollama (devstral-2)" />);
    expect(
      (screen.getByRole("button", { name: /Website/ })).getAttribute("aria-pressed"),
    ).toBe("true");
  });
});
