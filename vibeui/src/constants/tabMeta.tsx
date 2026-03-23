import type { LucideIcon } from "lucide-react";
import {
  MessageSquare, Bot, UsersRound, Swords, Infinity, Factory, Store,
  ClipboardList, Ruler, Activity, Palette,
  Shield, TestTube, TrendingUp,
  GitBranch, GitPullRequest, Users,
  Hammer, Container, RefreshCw, CloudCog, Workflow,
  Database, Globe, Radio,
  Cog, TerminalSquare, Wrench,
  Binary, Regex, PenTool,
  Settings, Plug, UserCog, DollarSign, Package,
} from "lucide-react";

export interface TabMeta {
  icon: LucideIcon;
  label: string;
  /** Searchable aliases — old panel names and keywords for discoverability */
  aliases?: string[];
}

export const TAB_META: Record<string, TabMeta> = {
  // --- AI ---
  chat:            { icon: MessageSquare,  label: "Chat" },
  agent:           { icon: Bot,            label: "Agent" },
  "ai-teams":      { icon: UsersRound,     label: "AI Teams",          aliases: ["teams", "agentteams", "subagents", "cloud", "cibot", "agent modes"] },
  "ai-playground": { icon: Swords,         label: "Playground",        aliases: ["compare", "arena", "cascade", "discuss", "multi model"] },
  "ai-context":    { icon: Infinity,       label: "Context & Memory",  aliases: ["icontext", "bundles", "openmemory", "fastcontext", "infinite context"] },
  "ai-generation": { icon: Factory,        label: "Generation",        aliases: ["batchbuilder", "imagegen", "autoresearch", "batch", "image", "research"] },
  marketplace:     { icon: Store,          label: "Marketplace" },

  // --- Project ---
  "project-hub":   { icon: ClipboardList,  label: "Project Hub",       aliases: ["workmanagement", "dashboard", "steering", "soul", "memory", "rules"] },
  planning:        { icon: Ruler,          label: "Planning",          aliases: ["specs", "plandoc", "workflow", "orchestration", "clarify", "codesearch"] },
  observability:   { icon: Activity,       label: "Observability",     aliases: ["traces", "recording", "demo"] },
  design:          { icon: Palette,        label: "Design",            aliases: ["remotecontrol", "remote control"] },

  // --- Code Quality ---
  security:        { icon: Shield,         label: "Security",          aliases: ["redteam", "blueteam", "purpleteam", "securityscan", "red team", "blue team", "purple team"] },
  testing:         { icon: TestTube,       label: "Testing",           aliases: ["tests", "coverage", "bugbot", "autofix", "cloudautofix", "qa-validation", "qa"] },
  "code-analysis": { icon: TrendingUp,     label: "Code Analysis",     aliases: ["transform", "metrics", "astedit", "editpredict", "snippets", "ast edit"] },

  // --- Source Control ---
  "version-control": { icon: GitBranch,    label: "Version Control",   aliases: ["history", "checkpoints", "bisect"] },
  github:            { icon: GitPullRequest, label: "GitHub",           aliases: ["ghactions", "gh sync", "github actions"] },
  collaboration:     { icon: Users,        label: "Collaboration",     aliases: ["collab", "compliance"] },

  // --- Infrastructure ---
  "build-deploy":    { icon: Hammer,       label: "Build & Deploy",    aliases: ["build", "deploy", "scaffold", "appbuilder", "fullstack", "app builder", "full stack"] },
  containers:        { icon: Container,    label: "Containers",        aliases: ["docker", "k8s", "sandbox", "cloudsandbox", "kubernetes"] },
  "ci-cd":           { icon: RefreshCw,    label: "CI/CD",             aliases: ["cicd", "cistatus", "pipeline"] },
  "cloud-platform":  { icon: CloudCog,     label: "Cloud & Platform",  aliases: ["cloudproviders", "env", "health", "idp", "environment"] },
  "ai-ml":           { icon: Workflow,     label: "AI/ML",             aliases: ["aiml", "modelwizard", "training", "inference", "quantum"] },

  // --- Data & APIs ---
  database:          { icon: Database,     label: "Database",          aliases: ["vibesql", "supabase", "migrations", "vectordb", "vector db"] },
  "api-tools":       { icon: Globe,        label: "API Tools",         aliases: ["http", "graphql", "mock", "websocket", "apidocs", "api docs"] },
  "data-pipeline":   { icon: Radio,        label: "Data Pipeline",     aliases: ["streaming", "ingest", "crawler"] },

  // --- Developer Tools ---
  "system-monitor":  { icon: Cog,          label: "System Monitor",    aliases: ["processes", "profiler", "debugmode", "debug"] },
  terminal:          { icon: TerminalSquare, label: "Terminal",         aliases: ["scripts", "ssh", "notebook", "logs"] },
  diagnostics:       { icon: Wrench,       label: "Diagnostics",       aliases: ["deps", "network", "loadtest", "renderopt", "swebench", "sessionmemory", "load test"] },

  // --- Toolkit ---
  converters:        { icon: Binary,       label: "Converters",        aliases: ["encoding", "numbers", "colorconv", "units", "unicode", "timestamp"] },
  formatters:        { icon: Regex,        label: "Formatters",        aliases: ["regex", "jwt", "jsontools", "cron", "csv", "cidr", "datagen", "utils", "json"] },
  editors:           { icon: PenTool,      label: "Editors",           aliases: ["difftool", "markdown", "canvas", "colors", "diff", "palette"] },

  // --- Settings ---
  config:            { icon: Settings,     label: "Configuration",     aliases: ["settings", "hooks", "markers", "jobs", "bookmarks", "keys"] },
  integrations:      { icon: Plug,         label: "Integrations",      aliases: ["mcp", "acpprotocol", "webhooks", "acp"] },
  administration:    { icon: UserCog,      label: "Administration",    aliases: ["admin", "auth", "governance", "sessions", "manager"] },
  billing:           { icon: DollarSign,   label: "Billing",           aliases: ["cost", "usagemetering", "usage"] },
  "tools-settings":  { icon: Package,      label: "Tools",             aliases: ["artifacts", "img2app", "visualtest", "visual test"] },
};

export const DEFAULT_TAB_META: TabMeta = { icon: Workflow, label: "Panel" };
