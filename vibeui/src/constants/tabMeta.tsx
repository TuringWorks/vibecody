import type { LucideIcon } from "lucide-react";
import {
  MessageSquare, Brain, UsersRound, Swords, Infinity, Factory, Store,
  ClipboardList, Ruler, Activity, Palette,
  Shield, TestTube, TrendingUp, Network,
  GitBranch, GitPullRequest, Users,
  Hammer, Container, RefreshCw, CloudCog, Workflow, Cpu,
  Database, Globe, Radio,
  Cog, TerminalSquare, Wrench,
  Binary, Regex, PenTool,
  Settings, Plug, UserCog, DollarSign, Package, Building2,
  Zap, ShieldCheck,
} from "lucide-react";

export interface TabMeta {
  icon: LucideIcon;
  label: string;
  /** Searchable aliases — old panel names and keywords for discoverability */
  aliases?: string[];
}

export const TAB_META: Record<string, TabMeta> = {
  // --- AI ---
  chat:            { icon: MessageSquare,  label: "Chat",              aliases: ["sandbox", "filesystem chat", "file ai", "ai sandbox", "sandbox chat"] },
  "agent-os":      { icon: Cpu,             label: "Agent-OS",          aliases: ["agent os", "agentos", "agent dashboard", "agent host", "branch agent", "browser agent", "orchestration", "agent modes", "agent pool"] },
  "ai-teams":      { icon: UsersRound,     label: "AI Teams",          aliases: ["teams", "agentteams", "subagents", "spawn", "cloud agent", "cibot", "agent modes", "hierarchy", "multi-agent"] },
  "ai-playground": { icon: Swords,         label: "AI Council",        aliases: ["counsel", "superbrain", "compare", "arena", "playground", "debate", "ensemble", "multi-model"] },
  "ai-context":    { icon: Infinity,       label: "Context & Memory",  aliases: ["icontext", "bundles", "openmemory", "fastcontext", "infinite context", "artifacts"] },
  "ai-generation": { icon: Factory,        label: "Generation",        aliases: ["batchbuilder", "imagegen", "autoresearch", "batch", "image", "research", "transform"] },
  marketplace:     { icon: Store,          label: "Marketplace" },

  // --- Project ---
  "project-hub":   { icon: ClipboardList,  label: "Project Hub",       aliases: ["projects", "workmanagement", "work management", "dashboard", "steering", "soul", "memory", "rules", "discuss"] },
  planning:        { icon: Ruler,          label: "Planning",          aliases: ["specs", "plandoc", "workflow", "orchestration", "clarify", "codesearch"] },
  observability:   { icon: Activity,       label: "Observability",     aliases: ["traces", "recording", "demo"] },
  design:          { icon: Palette,        label: "Design",            aliases: ["remotecontrol", "remote control", "img2app", "screenshot to app", "sketch"] },

  // --- Code Quality ---
  security:        { icon: Shield,         label: "Security",          aliases: ["redteam", "blueteam", "purpleteam", "securityscan", "red team", "blue team", "purple team"] },
  testing:         { icon: TestTube,       label: "Testing",           aliases: ["tests", "coverage", "bugbot", "autofix", "cloudautofix", "qa-validation", "qa", "visualtest", "visual test", "visual verify"] },
  "code-analysis": { icon: TrendingUp,     label: "Code Analysis",     aliases: ["metrics", "astedit", "editpredict", "snippets", "ast edit"] },
  architecture:    { icon: Network,        label: "Architecture",      aliases: ["aireview", "ai review", "archspec", "policy", "policy engine", "code review", "togaf", "c4", "adr"] },

  // --- Source Control ---
  "version-control": { icon: GitBranch,    label: "Version Control",   aliases: ["history", "checkpoints", "bisect"] },
  github:            { icon: GitPullRequest, label: "GitHub",           aliases: ["ghactions", "gh sync", "github actions"] },
  collaboration:     { icon: Users,        label: "Collaboration",     aliases: ["collab", "compliance", "gateway", "gateway-sandbox", "msg gateway", "telegram", "slack", "discord", "messaging", "webhook bot"] },

  // --- Infrastructure ---
  "build-deploy":    { icon: Hammer,       label: "Build & Deploy",    aliases: ["build", "deploy", "scaffold", "appbuilder", "fullstack", "app builder", "full stack"] },
  containers:        { icon: Container,    label: "Containers",        aliases: ["docker", "k8s", "sandbox", "cloudsandbox", "kubernetes"] },
  "ci-cd":           { icon: RefreshCw,    label: "CI/CD",             aliases: ["cicd", "cistatus", "pipeline", "cigates", "ci gates", "quality gates"] },
  "cloud-platform":  { icon: CloudCog,     label: "Cloud & Platform",  aliases: ["cloudproviders", "env", "health", "idp", "environment"] },
  "ai-ml":           { icon: Workflow,     label: "AI/ML",             aliases: ["aiml", "modelwizard", "inference", "quantum"] },
  "rl-os":           { icon: Brain,        label: "RL-OS",             aliases: ["reinforcement learning", "rl", "rlos", "training", "ppo", "sac", "dqn", "rlhf", "dpo", "distillation", "quantization", "environment", "evaluation", "deployment", "multi-agent", "marl", "reward", "policy", "gymnasium"] },

  // --- Data & APIs ---
  database:          { icon: Database,     label: "Database",          aliases: ["migrations", "vectordb", "vector db"] },
  "api-tools":       { icon: Globe,        label: "API Tools",         aliases: ["http", "graphql", "mock", "websocket", "apidocs", "api docs"] },
  "data-pipeline":   { icon: Radio,        label: "Data Pipeline",     aliases: ["streaming", "ingest", "crawler"] },

  // --- Developer Tools ---
  "system-monitor":  { icon: Cog,          label: "System Monitor",    aliases: ["processes", "profiler", "debugmode", "debug", "browser", "observeact", "desktop", "browse"] },
  terminal:          { icon: TerminalSquare, label: "Terminal",         aliases: ["scripts", "ssh", "notebook", "logs"] },
  diagnostics:       { icon: Wrench,       label: "Diagnostics",       aliases: ["deps", "network", "loadtest", "renderopt", "swebench", "sessionmemory", "resilience", "load test"] },

  // --- Toolkit ---
  converters:        { icon: Binary,       label: "Converters",        aliases: ["encoding", "numbers", "colorconv", "units", "unicode", "timestamp"] },
  formatters:        { icon: Regex,        label: "Formatters",        aliases: ["regex", "jwt", "jsontools", "cron", "csv", "cidr", "datagen", "utils", "json"] },
  editors:           { icon: PenTool,      label: "Editors",           aliases: ["difftool", "markdown", "canvas", "colors", "diff", "palette"] },

  // --- Settings ---
  config:            { icon: Settings,     label: "Configuration",     aliases: ["settings", "hooks", "markers", "jobs", "bookmarks", "keys"] },
  integrations:      { icon: Plug,         label: "Integrations",      aliases: ["mcp", "acpprotocol", "webhooks", "acp"] },
  productivity:      { icon: ClipboardList, label: "Productivity",      aliases: ["email", "gmail", "outlook", "calendar", "google calendar", "todoist", "todo", "notion", "jira", "home assistant", "smart home", "ha"] },
  administration:    { icon: UserCog,      label: "Administration",    aliases: ["admin", "auth", "governance", "sessions", "manager"] },
  billing:           { icon: DollarSign,   label: "Billing",           aliases: ["cost", "usagemetering", "usage"] },
  "tools-settings":  { icon: Package,      label: "Tools",             aliases: ["automations", "self-review", "selfreview"] },
  company:           { icon: Building2,    label: "Company",           aliases: ["paperclip", "org chart", "orgchart", "zero human", "autonomous company", "company dashboard", "approvals", "routines", "heartbeat", "agents org", "company orchestration", "agent tasks", "agent goals", "agent docs", "hire", "fire agent", "budget", "secrets vault"] },

  // --- FIT-GAP v8 ---
  "agent-intelligence":      { icon: Zap,          label: "Agent Intelligence",   aliases: ["env dispatch", "nested agents", "thought stream", "hard problem", "repro agent", "recursive agents", "agent tree", "cross-env"] },
  "enterprise-governance":   { icon: ShieldCheck,  label: "Enterprise Governance", aliases: ["mcp governance", "msaf", "team onboarding", "audit", "sso", "oidc", "saml", "enterprise", "governance"] },
};

export const DEFAULT_TAB_META: TabMeta = { icon: Workflow, label: "Panel" };
