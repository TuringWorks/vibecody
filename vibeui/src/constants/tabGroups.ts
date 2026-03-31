/** Grouped tab categories for the AI panel sidebar. */

export interface TabGroup {
  label: string;
  tabs: string[];
}

export const TAB_GROUPS: TabGroup[] = [
  {
    label: "AI",
    tabs: ["chat", "agent", "ai-teams", "ai-playground", "ai-context", "ai-generation", "marketplace"],
  },
  {
    label: "Project",
    tabs: ["project-hub", "planning", "observability", "design"],
  },
  {
    label: "Code Quality",
    tabs: ["security", "testing", "code-analysis"],
  },
  {
    label: "Source Control",
    tabs: ["version-control", "github", "collaboration"],
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
