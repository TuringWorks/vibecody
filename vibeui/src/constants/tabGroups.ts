/** Grouped tab categories for the AI panel sidebar. */

export interface TabGroup {
  label: string;
  tabs: string[];
}

export const TAB_GROUPS: TabGroup[] = [
  {
    label: "AI",
    tabs: ["chat", "agent", "cascade", "compare", "arena", "teams", "cloud", "cibot", "marketplace", "icontext", "batchbuilder", "subagents", "imagegen"],
  },
  {
    label: "Project",
    tabs: ["memory", "specs", "workflow", "orchestration", "design", "steering", "traces", "dashboard", "recording", "demo", "fastcontext", "plandoc", "remotecontrol", "clarify", "codesearch"],
  },
  {
    label: "Code",
    tabs: ["autofix", "cloudautofix", "bugbot", "redteam", "tests", "coverage", "transform", "metrics", "bisect", "snippets", "astedit", "securityscan", "editpredict"],
  },
  {
    label: "Git & Collab",
    tabs: ["history", "checkpoints", "github", "collab", "compliance"],
  },
  {
    label: "Infrastructure",
    tabs: ["deploy", "docker", "k8s", "cicd", "env", "sandbox", "cloudsandbox", "health", "scaffold", "appbuilder", "training", "inference", "cistatus"],
  },
  {
    label: "Data & API",
    tabs: ["database", "supabase", "migrations", "http", "graphql", "mock", "websocket", "apidocs", "streaming", "vectordb", "ingest", "crawler"],
  },
  {
    label: "DevTools",
    tabs: ["processes", "profiler", "scripts", "ssh", "notebook", "logs", "deps", "network", "loadtest"],
  },
  {
    label: "Utilities",
    tabs: [
      "regex", "jwt", "jsontools", "cron", "encoding", "numbers", "datagen",
      "timestamp", "colorconv", "cidr", "csv", "units", "unicode", "difftool",
      "markdown", "canvas", "colors", "utils",
    ],
  },
  {
    label: "Settings",
    tabs: ["settings", "hooks", "jobs", "mcp", "artifacts", "manager", "auth", "cost", "markers", "img2app", "visualtest", "webhooks", "admin", "qa-validation", "sessions", "governance"],
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
