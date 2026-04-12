import { createComposite } from "./createComposite";

// FIT-GAP v8: MCP enterprise governance, MSAF compatibility, team onboarding.
export const EnterpriseGovernanceComposite = createComposite([
  { id: "mcp-governance", label: "MCP Governance", importFn: () => import("../McpGovernancePanel"), exportName: "McpGovernancePanel" },
  { id: "msaf", label: "MSAF", importFn: () => import("../MsafPanel"), exportName: "MsafPanel" },
  { id: "team-onboarding", label: "Team Onboarding", importFn: () => import("../TeamOnboardingPanel"), exportName: "TeamOnboardingPanel" },
]);
