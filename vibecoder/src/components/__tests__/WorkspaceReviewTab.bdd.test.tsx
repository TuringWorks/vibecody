/**
 * BDD: the shared workspace-review engine that Blue and Purple team run on.
 *
 * Red team ships its own inline copy; this is the extracted engine the other
 * two teams use. The behaviour under test is what makes it a real review rather
 * than the disconnected SOC consoles those panels used to be: it resolves a
 * scope, runs one provider-agnostic call per file, keeps partial findings, and
 * hands them to chat — with the same honesty rules (no fabricated CVSS, capped
 * runs say so). The `kind` drives the command names, so this also proves Blue
 * and Purple call *their* backend, not each other's.
 */
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
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

import { WorkspaceReviewTab, type WorkspaceReviewConfig } from "../WorkspaceReviewTab";

const BLUE: WorkspaceReviewConfig = {
  kind: "blueteam",
  runLabel: "Run Blue Team",
  intro: "defensive review",
  findingNoun: "defensive gaps",
  evidenceLabel: "Detection",
  fixLabel: "Add this control",
  vectorLabel: "Control area",
  fixSource: "blue team",
  fixInstructions: ["add the control"],
  emptyBody: "Run Blue Team to review defences.",
};

const finding = (over: Record<string, unknown> = {}) => ({
  id: "src/api.ts-0",
  attack_vector: "input validation",
  cvss_score: 0,
  severity: "high",
  location: "src/api.ts:12",
  title: "No validation on user input",
  description: "The body is trusted without a schema check",
  poc: "log every rejected request",
  remediation: "Validate with a schema before use",
  source_file: "src/api.ts",
  source_line: 12,
  confirmed: false,
  ...over,
});

function setup(
  targets: { files: string[]; matched: number; limit: number },
  byFile: Record<string, unknown[]> = {},
) {
  mockInvoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
    switch (cmd) {
      case "blueteam_workspace_targets":
      case "purpleteam_workspace_targets": return targets;
      case "read_file": return "// source";
      case "blueteam_file":
      case "purpleteam_file": return byFile[args.file as string] ?? [];
      case "blueteam_save_session":
      case "purpleteam_save_session": return null;
      default: return null;
    }
  });
}

const renderTab = (config = BLUE, props: Record<string, unknown> = {}) =>
  render(<WorkspaceReviewTab workspacePath="/ws" provider="Ollama (devstral-2)" config={config} {...props} />);

beforeEach(() => mockInvoke.mockReset());

describe("WorkspaceReviewTab — blue team", () => {
  it("calls the kind's own backend commands, not another team's", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 }, { "src/api.ts": [finding()] });
    renderTab();
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    await waitFor(() => expect(screen.getByText("1 defensive gaps")).toBeTruthy());
    expect(mockInvoke).toHaveBeenCalledWith("blueteam_workspace_targets", { workspace: "/ws", pattern: null });
    expect(mockInvoke).toHaveBeenCalledWith("blueteam_file", expect.objectContaining({ file: "src/api.ts" }));
    // Never the red or purple command.
    expect(mockInvoke).not.toHaveBeenCalledWith("redteam_file", expect.anything());
    expect(mockInvoke).not.toHaveBeenCalledWith("purpleteam_file", expect.anything());
  });

  it("labels the finding fields for this team", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 }, { "src/api.ts": [finding()] });
    renderTab();
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    fireEvent.click(await screen.findByText("No validation on user input"));
    // Blue team's words, not red team's "PoC"/"Vector".
    expect(screen.getByText("Control area:")).toBeTruthy();
    expect(screen.getByText("Detection:")).toBeTruthy();
    expect(screen.getByText("Add this control:")).toBeTruthy();
  });

  it("shows severity, never a fabricated CVSS", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 }, { "src/api.ts": [finding()] });
    renderTab();
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    await waitFor(() => expect(screen.getByText("No validation on user input")).toBeTruthy());
    expect(screen.getByText("HIGH")).toBeTruthy();
    expect(screen.queryByText(/CVSS/)).toBeNull();
  });

  it("forwards the toolbar provider and model to every file review", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 }, { "src/api.ts": [finding()] });
    renderTab();
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "blueteam_file",
        expect.objectContaining({ provider: "ollama", model: "devstral-2", effort: "medium" }),
      ),
    );
  });

  it("refuses to run without a provider rather than defaulting to one", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 });
    renderTab(BLUE, { provider: "" });
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    await waitFor(() => expect(screen.getByText(/Select a provider and model/)).toBeTruthy());
    expect(mockInvoke).not.toHaveBeenCalledWith("blueteam_file", expect.anything());
  });

  it("says when the scope was capped, so a partial run never reads as complete", async () => {
    setup({ files: ["a.ts"], matched: 200, limit: 40 }, { "a.ts": [] });
    renderTab();
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    await waitFor(() => expect(screen.getByText(/scope had 200, capped at 40/)).toBeTruthy());
  });

  it("persists the run for later export", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 }, { "src/api.ts": [finding()] });
    renderTab();
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "blueteam_save_session",
        expect.objectContaining({ session: expect.objectContaining({ current_stage: "Report" }) }),
      ),
    );
  });

  it("hands findings to chat via Fix with AI", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 }, { "src/api.ts": [finding()] });
    renderTab();
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    await waitFor(() => expect(screen.getByText("1 defensive gaps")).toBeTruthy());
    expect(screen.getByRole("button", { name: /Fix with AI/i })).toBeTruthy();
  });

  it("reports a clean file honestly rather than implying a problem", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 }, { "src/api.ts": [] });
    renderTab();
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));

    await waitFor(() => expect(screen.getByText(/nothing flagged/)).toBeTruthy());
  });
});

describe("WorkspaceReviewTab — purple team wiring", () => {
  const PURPLE: WorkspaceReviewConfig = { ...BLUE, kind: "purpleteam", runLabel: "Run Purple Team", findingNoun: "coverage gaps" };

  it("runs the purple backend and shows coverage gaps", async () => {
    setup({ files: ["src/api.ts"], matched: 1, limit: 40 }, { "src/api.ts": [finding({ id: "src/api.ts-p" })] });
    renderTab(PURPLE);
    fireEvent.click(screen.getByRole("button", { name: "Run Purple Team" }));

    await waitFor(() => expect(screen.getByText("1 coverage gaps")).toBeTruthy());
    expect(mockInvoke).toHaveBeenCalledWith("purpleteam_file", expect.objectContaining({ file: "src/api.ts" }));
  });
});
