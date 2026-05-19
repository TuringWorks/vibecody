import { createComposite } from "./createComposite";

// FIT-GAP v8: MCP enterprise governance, MSAF compatibility, team onboarding.
// B2.6 adds Plugin Governance — install / policy / uninstall for signed
// MCPB plugin bundles. Sits beside MCP Governance because the conceptual
// neighbour is "which plugin components run in this workspace".
// 2026-05-18 adds Security Posture — unified scanner aggregator + work-item
// bridge. Lives between MCP Governance and Plugin Governance per the
// docs/design/security-posture/panel.md slot rationale (all three are
// "what's running and is it safe" surfaces).
export const EnterpriseGovernanceComposite = createComposite([
  { id: "mcp-governance", label: "MCP Governance", importFn: () => import("../McpGovernancePanel"), exportName: "McpGovernancePanel" },
  { id: "security-posture", label: "Security Posture", importFn: () => import("../SecurityPosturePanel"), exportName: "SecurityPosturePanel" },
  { id: "plugin-governance", label: "Plugin Governance", importFn: () => import("../PluginGovernancePanel"), exportName: "PluginGovernancePanel" },
  { id: "msaf", label: "MSAF", importFn: () => import("../MsafPanel"), exportName: "MsafPanel" },
  { id: "team-onboarding", label: "Team Onboarding", importFn: () => import("../TeamOnboardingPanel"), exportName: "TeamOnboardingPanel" },
]);
