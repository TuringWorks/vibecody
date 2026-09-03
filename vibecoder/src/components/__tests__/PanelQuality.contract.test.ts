import { describe, expect, it } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const COMPONENTS = resolve(__dirname, "..");
const panels = readdirSync(COMPONENTS).filter((name) => name.endsWith("Panel.tsx"));
const source = (name: string) => readFileSync(resolve(COMPONENTS, name), "utf8");

describe("panel runtime quality", () => {
  it("authenticates daemon requests and validates the two public health probes", () => {
    const violations: string[] = [];
    for (const name of panels) {
      const text = source(name).split("\n").filter((line) => !line.trimStart().startsWith("//")).join("\n");
      if (!/\bfetch\s*\(/.test(text)) continue;
      const rawCalls = [...text.matchAll(/\bfetch\s*\(\s*([^,\n]+)/g)].map((match) => match[1]);
      const healthOnly = rawCalls.every((call) => call.includes("/health"))
        && text.includes("isVibeCliHealth");
      if (!healthOnly) violations.push(name);
    }
    expect(violations, "use daemonFetch for protected routes; public /health must verify daemon identity").toEqual([]);
  });

  it("does not introduce more blocking browser confirmations", () => {
    // Existing callers are migration debt. New work must use ConfirmationDialog;
    // shrinking this list is encouraged, growing it fails loudly.
    const legacy = new Set([
      "AdminPanel.tsx", "AgilePanel.tsx", "ArchitectureSpecPanel.tsx",
      "CompanyAdapterPanel.tsx", "CompanyAgentDetailPanel.tsx", "CompanyDashboardPanel.tsx",
      "CompanyOrgChartPanel.tsx", "CostPanel.tsx", "DiffReviewPanel.tsx", "DockerPanel.tsx",
      "DrawioEditorPanel.tsx", "EnvPanel.tsx", "GoalPanel.tsx", "MigrationsPanel.tsx",
      "PluginGovernancePanel.tsx", "SandboxChatPanel.tsx", "SettingsPanel.tsx", "SshPanel.tsx",
      "WebhookPanel.tsx", "WorkManagementPanel.tsx",
    ]);
    const current = panels.filter((name) => /\b(?:window\.)?confirm\s*\(/.test(source(name)));
    expect(current.filter((name) => !legacy.has(name)), "use the shared ConfirmationDialog").toEqual([]);
  });

  it("keeps unmanaged intervals limited to active-job elapsed/progress timers", () => {
    const intentional = new Set([
      "AutofixPanel.tsx", "BatchBuilderPanel.tsx", "RedTeamPanel.tsx",
      "SkillForgePanel.tsx", "TestPanel.tsx",
    ]);
    const current = panels.filter((name) => /\bsetInterval\s*\(/.test(source(name)));
    expect(current.filter((name) => !intentional.has(name)), "recurring refreshes must use useVisibleInterval").toEqual([]);
  });
});
