/** Grouped tab categories for the AI panel sidebar. */

export interface TabGroup {
  label: string;
  tabs: string[];
}

export const TAB_GROUPS: TabGroup[] = [
  {
    label: "AI",
    tabs: ["chat", "agent", "cascade", "compare", "arena", "teams", "agentteams", "cloud", "cibot", "marketplace", "icontext", "batchbuilder", "subagents", "imagegen", "discuss"],
  },
  {
    label: "Project",
    tabs: ["workmanagement", "agile", "memory", "specs", "soul", "bundles", "workflow", "orchestration", "design", "steering", "traces", "dashboard", "recording", "demo", "fastcontext", "plandoc", "remotecontrol", "clarify", "codesearch"],
  },
  {
    label: "Code",
    tabs: ["autofix", "cloudautofix", "bugbot", "redteam", "blueteam", "purpleteam", "tests", "coverage", "transform", "metrics", "bisect", "snippets", "astedit", "securityscan", "editpredict"],
  },
  {
    label: "Git & Collab",
    tabs: ["history", "checkpoints", "github", "collab", "compliance"],
  },
  {
    label: "Infrastructure",
    tabs: ["build", "deploy", "docker", "k8s", "cicd", "ghactions", "env", "sandbox", "cloudsandbox", "health", "scaffold", "appbuilder", "training", "inference", "cistatus", "fullstack", "cloudproviders", "idp", "quantum"],
  },
  {
    label: "Data & API",
    tabs: ["vibesql", "database", "supabase", "migrations", "http", "graphql", "mock", "websocket", "apidocs", "streaming", "vectordb", "ingest", "crawler"],
  },
  {
    label: "DevTools",
    tabs: ["processes", "profiler", "scripts", "ssh", "notebook", "logs", "deps", "network", "loadtest", "renderopt", "swebench", "sessionmemory"],
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
    tabs: ["settings", "hooks", "jobs", "mcp", "mcplazy", "mcpdirectory", "acpprotocol", "artifacts", "manager", "auth", "cost", "usagemetering", "markers", "img2app", "visualtest", "webhooks", "admin", "qa-validation", "sessions", "governance"],
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
