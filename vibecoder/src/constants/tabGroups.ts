/** Grouped tab categories for the AI panel sidebar. */

export interface TabGroup {
  label: string;
  tabs: string[];
}

export const TAB_GROUPS: TabGroup[] = [
  {
    label: "AI",
    tabs: ["chat", "agent-os", "ai-teams", "ai-playground", "ai-context", "ai-generation", "marketplace"],
  },
  {
    label: "Project",
    // "goals" sits next to "planning" on purpose: it is the execution-goal
    // panel, and it was reachable only from Chat's "turn this into a goal"
    // handoff until it was listed here — rendered by LazyPanels, named in
    // tabMeta and PANEL_CATALOG, but absent from every nav group.
    tabs: ["project-hub", "planning", "goals", "observability", "design", "productivity"],
  },
  {
    // The client-engagement spine: four phases, their promised deliverables,
    // and the gates that decide whether a phase may close. Placed after
    // "Project" deliberately — inserting it earlier would shift the first nine
    // entries of ALL_TABS, which App.tsx slices for the Ctrl+1..9 shortcuts.
    label: "Delivery",
    tabs: ["engagement"],
  },
  {
    label: "Code Quality",
    tabs: ["security", "testing", "code-analysis", "architecture"],
  },
  {
    label: "Source Control",
    // "github" is not here: its tabs render inside the Git sidebar panel.
    tabs: ["version-control", "collaboration"],
  },
  {
    label: "Infrastructure",
    tabs: ["build-deploy", "containers", "ci-cd", "cloud-platform", "ai-ml", "rl-os"],
  },
  {
    label: "Data & APIs",
    tabs: ["database", "api-tools", "data-pipeline"],
  },
  {
    label: "Developer Tools",
    tabs: ["system-monitor", "terminal", "diagnostics"],
  },
  {
    label: "Toolkit",
    tabs: ["converters", "formatters", "editors", "tools-settings", "integrations"],
  },
  {
    label: "Settings",
    tabs: ["config", "administration", "billing"],
  },
  {
    label: "Company",
    tabs: ["company"],
  },
  {
    label: "Agent Intelligence",
    tabs: ["agent-intelligence", "enterprise-governance"],
  },
];

/** Flat lookup: tab id -> group label */
export const TAB_TO_GROUP: Record<string, string> = {};
for (const group of TAB_GROUPS) {
  for (const tab of group.tabs) {
    TAB_TO_GROUP[tab] = group.label;
  }
}

/** All tab ids in grouped order */
export const ALL_TABS: string[] = TAB_GROUPS.flatMap(g => g.tabs);
