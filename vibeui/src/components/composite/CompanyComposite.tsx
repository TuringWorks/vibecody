import { createComposite } from "./createComposite";

/**
 * CompanyComposite — Zero-human company orchestration (Paperclip parity).
 *
 * Tabs:
 *   Dashboard   — Real-time company status, quick actions, command console
 *   Org Chart   — SVG/ASCII agent hierarchy with reporting tree
 *   Agents      — Agent detail lookup, hire/fire
 *   Agent Goals — Hierarchical goal tree with progress (agent-scoped)
 *   Agent Tasks — Kanban task lifecycle for agents (backlog → done)
 *   Approvals   — Pending approval workflows with decide actions
 *   Budget      — Per-agent monthly budgets and cost events
 *   Secrets     — Encrypted secrets vault (keys listed, values hidden)
 *   Routines    — Scheduled recurring agent tasks + heartbeat triggers
 *   Heartbeats  — Per-agent/company run history with manual trigger
 *   Agent Docs  — Markdown docs linked to agents/tasks
 *   Import/Export — Company blueprint portability
 *   Adapters    — BYOA adapter registry (HTTP, process, Claude, Codex)
 */
export const CompanyComposite = createComposite([
  {
    id: "company-dashboard",
    label: "Dashboard",
    importFn: () => import("../CompanyDashboardPanel"),
    exportName: "CompanyDashboardPanel",
  },
  {
    id: "company-org",
    label: "Org Chart",
    importFn: () => import("../CompanyOrgChartPanel"),
    exportName: "CompanyOrgChartPanel",
  },
  {
    id: "company-agents",
    label: "Agents",
    importFn: () => import("../CompanyAgentDetailPanel"),
    exportName: "CompanyAgentDetailPanel",
  },
  {
    id: "company-goals",
    label: "Agent Goals",
    importFn: () => import("../CompanyGoalsPanel"),
    exportName: "CompanyGoalsPanel",
  },
  {
    id: "company-tasks",
    label: "Agent Tasks",
    importFn: () => import("../CompanyTaskBoardPanel"),
    exportName: "CompanyTaskBoardPanel",
  },
  {
    id: "company-approvals",
    label: "Approvals",
    importFn: () => import("../CompanyApprovalsPanel"),
    exportName: "CompanyApprovalsPanel",
  },
  {
    id: "company-budget",
    label: "Budget",
    importFn: () => import("../CompanyBudgetPanel"),
    exportName: "CompanyBudgetPanel",
  },
  {
    id: "company-secrets",
    label: "Secrets",
    importFn: () => import("../CompanySecretsPanel"),
    exportName: "CompanySecretsPanel",
  },
  {
    id: "company-routines",
    label: "Routines",
    importFn: () => import("../CompanyRoutinesPanel"),
    exportName: "CompanyRoutinesPanel",
  },
  {
    id: "company-heartbeats",
    label: "Heartbeats",
    importFn: () => import("../CompanyHeartbeatPanel"),
    exportName: "CompanyHeartbeatPanel",
  },
  {
    id: "company-docs",
    label: "Agent Docs",
    importFn: () => import("../CompanyDocumentsPanel"),
    exportName: "CompanyDocumentsPanel",
  },
  {
    id: "company-portability",
    label: "Import/Export",
    importFn: () => import("../CompanyPortabilityPanel"),
    exportName: "CompanyPortabilityPanel",
  },
  {
    id: "company-adapters",
    label: "Adapters",
    importFn: () => import("../CompanyAdapterPanel"),
    exportName: "CompanyAdapterPanel",
  },
  {
    id: "workspace-config",
    label: "Workspace Config",
    importFn: () => import("../CompanyWorkspaceConfigPanel"),
    exportName: "CompanyWorkspaceConfigPanel",
  },
  {
    id: "priority-map",
    label: "Priority Map",
    importFn: () => import("../CompanyPriorityMapPanel"),
    exportName: "CompanyPriorityMapPanel",
  },
]);
