import { createComposite } from "./createComposite";

// FIT-GAP v8: MCP enterprise governance, MSAF compatibility, team onboarding.
// B2.6 adds Plugin Governance — install / policy / uninstall for signed
// MCPB plugin bundles. Sits beside MCP Governance because the conceptual
// neighbour is "which plugin components run in this workspace".
export const EnterpriseGovernanceComposite = createComposite([
  { id: "mcp-governance", label: "MCP Governance", importFn: () => import("../McpGovernancePanel"), exportName: "McpGovernancePanel" },
  { id: "plugin-governance", label: "Plugin Governance", importFn: () => import("../PluginGovernancePanel"), exportName: "PluginGovernancePanel" },
  { id: "msaf", label: "MSAF", importFn: () => import("../MsafPanel"), exportName: "MsafPanel" },
  { id: "team-onboarding", label: "Team Onboarding", importFn: () => import("../TeamOnboardingPanel"), exportName: "TeamOnboardingPanel" },
]);
