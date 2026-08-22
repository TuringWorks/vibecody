/**
 * BDD: Blue Team and Purple Team open on a workspace review of the project.
 *
 * Both were disconnected consoles — a generic SOC toolkit, a generic ATT&CK
 * matrix — with no relationship to the code the user has open. These assert the
 * new default: opening either panel lands on a tab that reviews *this* project,
 * and the tab drives the team's own backend.
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

import { BlueTeamPanel } from "../BlueTeamPanel";
import { PurpleTeamPanel } from "../PurpleTeamPanel";

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd.endsWith("_workspace_targets")) return { files: ["src/api.ts"], matched: 1, limit: 40 };
    if (cmd === "read_file") return "// source";
    if (cmd.endsWith("_file")) return [];
    return [];
  });
});

describe("Blue Team", () => {
  it("opens on the Workspace tab, not the SOC console", () => {
    render(<BlueTeamPanel workspacePath="/ws" provider="Ollama (devstral-2)" />);
    expect(screen.getByRole("button", { name: "Run Blue Team" })).toBeTruthy();
  });

  it("reviews the workspace through the blue-team backend", async () => {
    render(<BlueTeamPanel workspacePath="/ws" provider="Ollama (devstral-2)" />);
    fireEvent.click(screen.getByRole("button", { name: "Run Blue Team" }));
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("blueteam_workspace_targets", { workspace: "/ws", pattern: null }),
    );
  });

  it("still offers the SOC console tabs", () => {
    render(<BlueTeamPanel workspacePath="/ws" provider="Ollama (devstral-2)" />);
    expect(screen.getByRole("button", { name: "Incidents" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Playbooks" })).toBeTruthy();
  });
});

describe("Purple Team", () => {
  it("opens on the Workspace tab, not the ATT&CK matrix", () => {
    render(<PurpleTeamPanel workspacePath="/ws" provider="Ollama (devstral-2)" />);
    expect(screen.getByRole("button", { name: "Run Purple Team" })).toBeTruthy();
  });

  it("reviews the workspace through the purple-team backend", async () => {
    render(<PurpleTeamPanel workspacePath="/ws" provider="Ollama (devstral-2)" />);
    fireEvent.click(screen.getByRole("button", { name: "Run Purple Team" }));
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("purpleteam_workspace_targets", { workspace: "/ws", pattern: null }),
    );
  });

  it("still offers the exercise tabs", () => {
    render(<PurpleTeamPanel workspacePath="/ws" provider="Ollama (devstral-2)" />);
    expect(screen.getByRole("button", { name: "Exercises" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Coverage Gaps" })).toBeTruthy();
  });
});
