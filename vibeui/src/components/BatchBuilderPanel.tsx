/**
 * BatchBuilderPanel — Bulk/Batch Development "Hands-Off" UI.
 *
 * Tabs: New Run | Monitor | QA Review | Migration | History
 * - New Run: configure project, requirements, user stories, APIs, models, estimate, launch
 * - Monitor: live progress, agent pool, phase timeline, metrics, logs
 * - QA Review: multi-agent validation results, findings, cross-validation, scoring
 * - Migration: legacy code migration workflow with strategy, risk, progress
 * - History: past batch runs with expandable details and statistics
 */
import React, { useState } from "react";

/* ── Types ───────────────────────────────────────────────────────────── */

type TabKey = "newrun" | "monitor" | "qa" | "migration" | "history";
type Priority = "low" | "normal" | "high" | "critical";
type TechStack =
  | "react-node"
  | "react-python"
  | "vue-node"
  | "angular-java"
  | "nextjs-prisma"
  | "rust-actix"
  | "django-htmx"
  | "flutter-firebase"
  | "rails-postgres"
  | "go-gin"
  | "custom";

type RunStatus =
  | "Queued"
  | "Planning"
  | "Generating"
  | "Validating"
  | "Compiling"
  | "Testing"
  | "Completed"
  | "Failed";

type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
type FieldType = "string" | "number" | "boolean" | "date" | "uuid" | "json" | "array" | "enum";
type Severity = "Critical" | "High" | "Medium" | "Low";
type LogLevel = "Debug" | "Info" | "Warning" | "Error";
type MigrationPhase = "Analysis" | "Planning" | "Translation" | "Validation" | "Testing" | "Deployment" | "Monitoring";
type MigrationStrategy = "direct" | "rewrite" | "strangler" | "bigbang" | "incremental" | "hybrid";

interface Requirement {
  id: number;
  text: string;
}

interface UserStory {
  id: number;
  persona: string;
  action: string;
  benefit: string;
  acceptanceCriteria: string;
}

interface ApiEndpoint {
  id: number;
  method: HttpMethod;
  path: string;
  description: string;
  auth: boolean;
}

interface ModelField {
  id: number;
  name: string;
  type: FieldType;
  required: boolean;
}

interface DataModel {
  id: number;
  name: string;
  fields: ModelField[];
}

interface Estimate {
  files: number;
  lines: number;
  duration: string;
  complexity: number;
  agents: string[];
}

interface AgentCard {
  role: string;
  icon: string;
  status: "idle" | "active" | "done" | "error";
  filesCreated: number;
  linesGenerated: number;
  assignedModules: string[];
}

interface Phase {
  name: string;
  status: "completed" | "active" | "pending";
}

interface LogEntry {
  id: number;
  level: LogLevel;
  timestamp: string;
  agentId: string;
  message: string;
}

interface QaAgent {
  name: string;
  status: "pass" | "fail" | "running" | "pending";
  critical: number;
  high: number;
  medium: number;
  low: number;
  passRate: number;
}

interface Finding {
  id: number;
  severity: Severity;
  file: string;
  line: number;
  category: string;
  message: string;
  suggestion: string;
  autoFixable: boolean;
  resolved: boolean;
}

interface CrossValidation {
  agentA: string;
  agentB: string;
  confidence: number;
  agreements: number;
  disagreements: number;
}

interface MigrationComponent {
  id: number;
  name: string;
  compType: string;
  language: string;
  lines: number;
  complexity: "Low" | "Medium" | "High";
  risk: "Low" | "Medium" | "High" | "Critical";
  status: "Pending" | "In Progress" | "Completed" | "Failed";
}

interface TranslationRule {
  id: number;
  sourcePattern: string;
  targetPattern: string;
  confidence: number;
  example: string;
}

interface HistoryRun {
  id: string;
  title: string;
  status: "Completed" | "Failed" | "Cancelled";
  files: number;
  lines: number;
  duration: string;
  agents: number;
  date: string;
  qaScore: number;
  fileTree: string[];
}

/* ── Constants ───────────────────────────────────────────────────────── */

const TABS: { key: TabKey; label: string }[] = [
  { key: "newrun", label: "New Run" },
  { key: "monitor", label: "Monitor" },
  { key: "qa", label: "QA Review" },
  { key: "migration", label: "Migration" },
  { key: "history", label: "History" },
];

const TECH_STACKS: { value: TechStack; label: string }[] = [
  { value: "react-node", label: "React + Node" },
  { value: "react-python", label: "React + Python" },
  { value: "vue-node", label: "Vue + Node" },
  { value: "angular-java", label: "Angular + Java" },
  { value: "nextjs-prisma", label: "Next.js + Prisma" },
  { value: "rust-actix", label: "Rust + Actix" },
  { value: "django-htmx", label: "Django + HTMX" },
  { value: "flutter-firebase", label: "Flutter + Firebase" },
  { value: "rails-postgres", label: "Rails + Postgres" },
  { value: "go-gin", label: "Go + Gin" },
  { value: "custom", label: "Custom" },
];

const PRIORITY_COLORS: Record<Priority, string> = {
  low: "var(--info-color)",
  normal: "var(--success-color)",
  high: "var(--warning-color)",
  critical: "var(--error-color)",
};

const STATUS_COLORS: Record<RunStatus, string> = {
  Queued: "var(--text-muted)",
  Planning: "var(--info-color)",
  Generating: "var(--warning-color)",
  Validating: "#9c27b0",
  Compiling: "#00bcd4",
  Testing: "var(--warning-color)",
  Completed: "var(--success-color)",
  Failed: "var(--error-color)",
};

const LOG_COLORS: Record<LogLevel, string> = {
  Debug: "var(--text-muted)",
  Info: "var(--info-color)",
  Warning: "var(--warning-color)",
  Error: "var(--error-color)",
};

const SEVERITY_COLORS: Record<Severity, string> = {
  Critical: "var(--error-color)",
  High: "var(--error-color)",
  Medium: "var(--warning-color)",
  Low: "var(--info-color)",
};

const AGENT_ROLES: { role: string; icon: string }[] = [
  { role: "Architect", icon: "A" },
  { role: "Backend", icon: "B" },
  { role: "Frontend", icon: "F" },
  { role: "Database", icon: "D" },
  { role: "Infrastructure", icon: "I" },
  { role: "Testing", icon: "T" },
  { role: "Documentation", icon: "W" },
  { role: "Security", icon: "S" },
  { role: "Performance", icon: "P" },
  { role: "Integration", icon: "G" },
];

const QA_AGENTS: string[] = [
  "CompileChecker",
  "TestRunner",
  "SecurityAuditor",
  "StyleEnforcer",
  "DocValidator",
  "PerformanceAnalyzer",
  "DependencyAuditor",
  "IntegrationTester",
];

const SOURCE_LANGS = [
  "COBOL", "Fortran", "Java 4-7", "C# Legacy", "VB6", "VB.NET",
  "Delphi", "PowerBuilder", "ColdFusion", "Perl", "Classic ASP",
];

const TARGET_LANGS = [
  "Rust", "Go", "Python", "TypeScript", "Java 21", "C# 12",
  "Kotlin", "Swift", "Elixir", "Scala 3",
];

const STRATEGY_DESCRIPTIONS: Record<MigrationStrategy, string> = {
  direct: "Line-by-line translation preserving original structure",
  rewrite: "Complete rewrite using idiomatic target language patterns",
  strangler: "Gradual replacement wrapping legacy with new services",
  bigbang: "Full system replacement in a single cutover",
  incremental: "Module-by-module migration with parallel operation",
  hybrid: "Combined approach using bridge adapters between old and new",
};

const MIGRATION_PHASES: MigrationPhase[] = [
  "Analysis", "Planning", "Translation", "Validation", "Testing", "Deployment", "Monitoring",
];

/* ── Helpers ─────────────────────────────────────────────────────────── */

let nextId = 1000;
const genId = () => ++nextId;

const badge = (_: string, color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 4,
  fontSize: 11,
  fontWeight: 600,
  color: "white",
  background: color,
  marginRight: 4,
});

const inputStyle: React.CSSProperties = {
  background: "var(--vscode-input-background)",
  color: "var(--vscode-input-foreground)",
  border: "1px solid var(--vscode-input-border)",
  borderRadius: 4,
  padding: "6px 10px",
  fontSize: 13,
  width: "100%",
  boxSizing: "border-box",
};

const btnStyle: React.CSSProperties = {
  background: "var(--vscode-button-background)",
  color: "var(--vscode-button-foreground)",
  border: "none",
  borderRadius: 4,
  padding: "6px 14px",
  cursor: "pointer",
  fontSize: 13,
  fontWeight: 500,
};

const btnDanger: React.CSSProperties = {
  ...btnStyle,
  background: "var(--error-color)",
};

const sectionTitle: React.CSSProperties = {
  fontSize: 14,
  fontWeight: 600,
  marginBottom: 8,
  marginTop: 16,
  color: "var(--vscode-editor-foreground)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--vscode-input-background)",
  border: "1px solid var(--vscode-input-border)",
  borderRadius: 6,
  padding: 12,
  marginBottom: 8,
};

const tableStyle: React.CSSProperties = {
  width: "100%",
  borderCollapse: "collapse" as const,
  fontSize: 12,
};

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "6px 8px",
  borderBottom: "1px solid var(--vscode-input-border)",
  fontWeight: 600,
  color: "var(--vscode-editor-foreground)",
};

const tdStyle: React.CSSProperties = {
  padding: "6px 8px",
  borderBottom: "1px solid var(--vscode-input-border)",
  color: "var(--vscode-editor-foreground)",
};

/* ── Mock Data Generators ────────────────────────────────────────────── */

function buildMockAgents(): AgentCard[] {
  return AGENT_ROLES.map((r) => ({
    role: r.role,
    icon: r.icon,
    status: "idle" as const,
    filesCreated: 0,
    linesGenerated: 0,
    assignedModules: [],
  }));
}

function buildMockPhases(): Phase[] {
  return [
    { name: "Initialization", status: "completed" },
    { name: "Architecture Planning", status: "completed" },
    { name: "Database Schema", status: "completed" },
    { name: "Backend Services", status: "active" },
    { name: "Frontend Components", status: "pending" },
    { name: "API Integration", status: "pending" },
    { name: "Testing Suite", status: "pending" },
    { name: "Documentation", status: "pending" },
    { name: "Security Scan", status: "pending" },
    { name: "Final Validation", status: "pending" },
  ];
}

function buildMockLogs(): LogEntry[] {
  const entries: LogEntry[] = [];
  const msgs = [
    { level: "Info" as LogLevel, agent: "Architect", msg: "Project structure created with 12 modules" },
    { level: "Info" as LogLevel, agent: "Database", msg: "Generated 8 migration files for PostgreSQL" },
    { level: "Debug" as LogLevel, agent: "Backend", msg: "Creating service layer for UserModule" },
    { level: "Warning" as LogLevel, agent: "Security", msg: "Detected unvalidated input in /api/users endpoint" },
    { level: "Info" as LogLevel, agent: "Frontend", msg: "Generated 14 React components with TypeScript" },
    { level: "Error" as LogLevel, agent: "Testing", msg: "Test compile failure in integration/auth_test.rs" },
    { level: "Info" as LogLevel, agent: "Backend", msg: "Implemented CRUD operations for 6 entities" },
    { level: "Debug" as LogLevel, agent: "Infrastructure", msg: "Docker compose file generated" },
  ];
  msgs.forEach((m, i) => {
    entries.push({
      id: genId(),
      level: m.level,
      timestamp: `2026-03-08T10:${String(i * 3).padStart(2, "0")}:00Z`,
      agentId: m.agent,
      message: m.msg,
    });
  });
  return entries;
}

function buildMockQaAgents(): QaAgent[] {
  return QA_AGENTS.map((name) => ({
    name,
    status: "pass" as const,
    critical: Math.floor(Math.random() * 2),
    high: Math.floor(Math.random() * 5),
    medium: Math.floor(Math.random() * 10),
    low: Math.floor(Math.random() * 15),
    passRate: 70 + Math.floor(Math.random() * 30),
  }));
}

function buildMockFindings(): Finding[] {
  const items: Finding[] = [
    { id: genId(), severity: "Critical", file: "src/auth/jwt.rs", line: 42, category: "Security", message: "JWT secret hardcoded", suggestion: "Use environment variable", autoFixable: true, resolved: false },
    { id: genId(), severity: "High", file: "src/api/users.rs", line: 88, category: "Validation", message: "Missing input sanitization", suggestion: "Add validator middleware", autoFixable: true, resolved: false },
    { id: genId(), severity: "Medium", file: "src/db/pool.rs", line: 15, category: "Performance", message: "Connection pool size too small", suggestion: "Increase max_connections to 20", autoFixable: true, resolved: true },
    { id: genId(), severity: "Low", file: "src/models/user.rs", line: 3, category: "Style", message: "Missing doc comment on public struct", suggestion: "Add /// documentation", autoFixable: true, resolved: false },
    { id: genId(), severity: "High", file: "src/api/orders.rs", line: 112, category: "Security", message: "SQL injection via string concatenation", suggestion: "Use parameterized queries", autoFixable: false, resolved: false },
    { id: genId(), severity: "Medium", file: "src/services/email.rs", line: 67, category: "Error Handling", message: "Unwrap on fallible operation", suggestion: "Use proper error handling with ?", autoFixable: true, resolved: false },
  ];
  return items;
}

function buildMockCrossValidations(): CrossValidation[] {
  return [
    { agentA: "CompileChecker", agentB: "TestRunner", confidence: 94, agreements: 47, disagreements: 3 },
    { agentA: "SecurityAuditor", agentB: "StyleEnforcer", confidence: 78, agreements: 32, disagreements: 9 },
    { agentA: "DocValidator", agentB: "PerformanceAnalyzer", confidence: 85, agreements: 28, disagreements: 5 },
    { agentA: "DependencyAuditor", agentB: "IntegrationTester", confidence: 91, agreements: 40, disagreements: 4 },
  ];
}

function buildMockMigrationComponents(): MigrationComponent[] {
  return [
    { id: genId(), name: "CustomerModule", compType: "Service", language: "COBOL", lines: 4200, complexity: "High", risk: "High", status: "Completed" },
    { id: genId(), name: "OrderProcessor", compType: "Batch", language: "COBOL", lines: 6800, complexity: "High", risk: "Critical", status: "In Progress" },
    { id: genId(), name: "InventoryTracker", compType: "Service", language: "COBOL", lines: 2100, complexity: "Medium", risk: "Medium", status: "Pending" },
    { id: genId(), name: "ReportGenerator", compType: "Utility", language: "COBOL", lines: 1500, complexity: "Low", risk: "Low", status: "Pending" },
    { id: genId(), name: "AuthGateway", compType: "Middleware", language: "COBOL", lines: 900, complexity: "Medium", risk: "Medium", status: "Completed" },
  ];
}

function buildMockTranslationRules(): TranslationRule[] {
  return [
    { id: genId(), sourcePattern: "PERFORM VARYING", targetPattern: "for i in 0..n", confidence: 92, example: "PERFORM VARYING I FROM 1 BY 1 UNTIL I > 10 -> for i in 0..10" },
    { id: genId(), sourcePattern: "MOVE X TO Y", targetPattern: "y = x.clone()", confidence: 88, example: "MOVE WS-NAME TO OUT-NAME -> out_name = ws_name.clone()" },
    { id: genId(), sourcePattern: "IF ... ELSE", targetPattern: "if ... else", confidence: 97, example: "IF X > 0 THEN ... ELSE ... -> if x > 0 { ... } else { ... }" },
    { id: genId(), sourcePattern: "EVALUATE TRUE", targetPattern: "match ... {}", confidence: 85, example: "EVALUATE TRUE WHEN X > 0 ... -> match true { _ if x > 0 => ... }" },
  ];
}

function buildMockHistory(): HistoryRun[] {
  return [
    { id: "BR-001", title: "E-Commerce Platform", status: "Completed", files: 87, lines: 12450, duration: "2h 14m", agents: 8, date: "2026-03-07", qaScore: 92, fileTree: ["src/", "src/api/", "src/models/", "src/services/", "src/frontend/", "tests/", "docs/"] },
    { id: "BR-002", title: "Chat Application", status: "Completed", files: 42, lines: 6800, duration: "1h 05m", agents: 6, date: "2026-03-06", qaScore: 88, fileTree: ["src/", "src/ws/", "src/rooms/", "src/ui/", "tests/"] },
    { id: "BR-003", title: "ML Pipeline Service", status: "Failed", files: 31, lines: 4200, duration: "0h 48m", agents: 5, date: "2026-03-05", qaScore: 45, fileTree: ["src/", "src/pipeline/", "src/models/"] },
    { id: "BR-004", title: "CMS Backend", status: "Cancelled", files: 15, lines: 2100, duration: "0h 22m", agents: 4, date: "2026-03-04", qaScore: 0, fileTree: ["src/", "src/content/"] },
    { id: "BR-005", title: "IoT Dashboard", status: "Completed", files: 64, lines: 9300, duration: "1h 52m", agents: 7, date: "2026-03-03", qaScore: 85, fileTree: ["src/", "src/devices/", "src/telemetry/", "src/dashboard/", "tests/"] },
  ];
}

/* ── Component ───────────────────────────────────────────────────────── */

const BatchBuilderPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabKey>("newrun");

  /* ── New Run state ────────────────────────────────────────────────── */
  const [projectTitle, setProjectTitle] = useState("");
  const [projectDesc, setProjectDesc] = useState("");
  const [techStack, setTechStack] = useState<TechStack>("react-node");
  const [priority, setPriority] = useState<Priority>("normal");
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [reqInput, setReqInput] = useState("");
  const [userStories, setUserStories] = useState<UserStory[]>([]);
  const [storyForm, setStoryForm] = useState({ persona: "", action: "", benefit: "", criteria: "" });
  const [endpoints, setEndpoints] = useState<ApiEndpoint[]>([]);
  const [epForm, setEpForm] = useState<{ method: HttpMethod; path: string; desc: string; auth: boolean }>({ method: "GET", path: "", desc: "", auth: false });
  const [dataModels, setDataModels] = useState<DataModel[]>([]);
  const [modelName, setModelName] = useState("");
  const [modelFields, setModelFields] = useState<ModelField[]>([]);
  const [fieldForm, setFieldForm] = useState<{ name: string; type: FieldType; required: boolean }>({ name: "", type: "string", required: true });
  const [estimate, setEstimate] = useState<Estimate | null>(null);

  /* ── Monitor state ────────────────────────────────────────────────── */
  const [runId] = useState("BR-" + String(Math.floor(Math.random() * 900) + 100));
  const [runStatus, setRunStatus] = useState<RunStatus>("Queued");
  const [elapsed, setElapsed] = useState("0m 0s");
  const [progress, setProgress] = useState(0);
  const [phaseLabel, setPhaseLabel] = useState("Waiting...");
  const [tokenUsed, setTokenUsed] = useState(0);
  const [tokenTotal] = useState(500000);
  const [agents, setAgents] = useState<AgentCard[]>(buildMockAgents);
  const [phases, setPhases] = useState<Phase[]>(buildMockPhases);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [metrics, setMetrics] = useState({ files: 0, lines: 0, linesPerHour: 0, filesPerHour: 0, compilePass: 0, testPass: 0 });

  /* ── QA state ─────────────────────────────────────────────────────── */
  const [qaRound, setQaRound] = useState(1);
  const [qaAgents, setQaAgents] = useState<QaAgent[]>(buildMockQaAgents);
  const [findings, setFindings] = useState<Finding[]>(buildMockFindings);
  const [crossValidations] = useState<CrossValidation[]>(buildMockCrossValidations);
  const [overallScore] = useState(78);
  const [findingSortKey, setFindingSortKey] = useState<"severity" | "file" | "category">("severity");

  /* ── Migration state ──────────────────────────────────────────────── */
  const [sourceLang, setSourceLang] = useState("COBOL");
  const [targetLang, setTargetLang] = useState("Rust");
  const [strategy, setStrategy] = useState<MigrationStrategy>("strangler");
  const [migComponents] = useState<MigrationComponent[]>(buildMockMigrationComponents);
  const [translationRules] = useState<TranslationRule[]>(buildMockTranslationRules);
  const [migPhaseIndex] = useState(2);

  /* ── History state ────────────────────────────────────────────────── */
  const [historyRuns] = useState<HistoryRun[]>(buildMockHistory);
  const [expandedRun, setExpandedRun] = useState<string | null>(null);
  const [historyFilter, setHistoryFilter] = useState<"all" | "Completed" | "Failed" | "Cancelled">("all");

  /* ── New Run actions ──────────────────────────────────────────────── */

  const addRequirement = () => {
    if (!reqInput.trim()) return;
    setRequirements((prev) => [...prev, { id: genId(), text: reqInput.trim() }]);
    setReqInput("");
  };

  const removeRequirement = (id: number) => {
    setRequirements((prev) => prev.filter((r) => r.id !== id));
  };

  const addUserStory = () => {
    if (!storyForm.persona.trim() || !storyForm.action.trim()) return;
    setUserStories((prev) => [
      ...prev,
      { id: genId(), persona: storyForm.persona, action: storyForm.action, benefit: storyForm.benefit, acceptanceCriteria: storyForm.criteria },
    ]);
    setStoryForm({ persona: "", action: "", benefit: "", criteria: "" });
  };

  const removeUserStory = (id: number) => {
    setUserStories((prev) => prev.filter((s) => s.id !== id));
  };

  const addEndpoint = () => {
    if (!epForm.path.trim()) return;
    setEndpoints((prev) => [...prev, { id: genId(), method: epForm.method, path: epForm.path, description: epForm.desc, auth: epForm.auth }]);
    setEpForm({ method: "GET", path: "", desc: "", auth: false });
  };

  const removeEndpoint = (id: number) => {
    setEndpoints((prev) => prev.filter((e) => e.id !== id));
  };

  const addField = () => {
    if (!fieldForm.name.trim()) return;
    setModelFields((prev) => [...prev, { id: genId(), name: fieldForm.name, type: fieldForm.type, required: fieldForm.required }]);
    setFieldForm({ name: "", type: "string", required: true });
  };

  const removeField = (id: number) => {
    setModelFields((prev) => prev.filter((f) => f.id !== id));
  };

  const addModel = () => {
    if (!modelName.trim() || modelFields.length === 0) return;
    setDataModels((prev) => [...prev, { id: genId(), name: modelName, fields: [...modelFields] }]);
    setModelName("");
    setModelFields([]);
  };

  const removeModel = (id: number) => {
    setDataModels((prev) => prev.filter((m) => m.id !== id));
  };

  const runEstimate = () => {
    const baseFiles = 20 + requirements.length * 3 + endpoints.length * 4 + dataModels.length * 5 + userStories.length * 2;
    const baseLines = baseFiles * 120;
    const complexityScore = Math.min(100, 20 + requirements.length * 5 + endpoints.length * 8 + dataModels.length * 10);
    const hours = Math.max(0.5, baseFiles * 0.03);
    const recAgents = AGENT_ROLES.slice(0, Math.min(10, 3 + Math.floor(complexityScore / 15))).map((a) => a.role);
    setEstimate({
      files: baseFiles,
      lines: baseLines,
      duration: `${Math.floor(hours)}h ${Math.round((hours % 1) * 60)}m`,
      complexity: complexityScore,
      agents: recAgents,
    });
  };

  const startBatchRun = () => {
    setRunStatus("Planning");
    setElapsed("0m 12s");
    setProgress(8);
    setPhaseLabel("Architecture Planning");
    setTokenUsed(12400);
    setLogs(buildMockLogs());
    setAgents(
      AGENT_ROLES.map((r, i) => ({
        role: r.role,
        icon: r.icon,
        status: i < 3 ? ("active" as const) : ("idle" as const),
        filesCreated: i < 3 ? Math.floor(Math.random() * 8) + 1 : 0,
        linesGenerated: i < 3 ? Math.floor(Math.random() * 1200) + 100 : 0,
        assignedModules: i < 3 ? ["Module-" + (i + 1)] : [],
      }))
    );
    setPhases(buildMockPhases());
    setMetrics({ files: 14, lines: 2450, linesPerHour: 8200, filesPerHour: 47, compilePass: 92, testPass: 78 });
    setActiveTab("monitor");
  };

  /* ── Monitor actions ──────────────────────────────────────────────── */

  const pauseRun = () => setRunStatus("Queued");
  const resumeRun = () => setRunStatus("Generating");
  const cancelRun = () => {
    setRunStatus("Failed");
    setPhaseLabel("Cancelled by user");
  };

  /* ── QA actions ───────────────────────────────────────────────────── */

  const toggleResolved = (id: number) => {
    setFindings((prev) => prev.map((f) => (f.id === id ? { ...f, resolved: !f.resolved } : f)));
  };

  const runAnotherRound = () => {
    setQaRound((r) => r + 1);
    setQaAgents(buildMockQaAgents());
  };

  const sortedFindings = [...findings].sort((a, b) => {
    if (findingSortKey === "severity") {
      const order: Record<Severity, number> = { Critical: 0, High: 1, Medium: 2, Low: 3 };
      return order[a.severity] - order[b.severity];
    }
    if (findingSortKey === "file") return a.file.localeCompare(b.file);
    return a.category.localeCompare(b.category);
  });

  /* ── History helpers ──────────────────────────────────────────────── */

  const filteredHistory = historyRuns.filter((r) => historyFilter === "all" || r.status === historyFilter);
  const allTimeLines = historyRuns.reduce((s, r) => s + r.lines, 0);
  const allTimeFiles = historyRuns.reduce((s, r) => s + r.files, 0);

  /* ── Render helpers ────────────────────────────────────────────────── */

  const scoreColor = (score: number) => {
    if (score >= 80) return "var(--success-color)";
    if (score >= 60) return "var(--warning-color)";
    return "var(--error-color)";
  };

  const recommendation = (score: number) => {
    if (score >= 90) return { text: "Approve", color: "var(--success-color)" };
    if (score >= 80) return { text: "Approve with Warnings", color: "var(--success-color)" };
    if (score >= 60) return { text: "Request Changes", color: "var(--warning-color)" };
    return { text: "Reject", color: "var(--error-color)" };
  };

  /* ── Tab: New Run ─────────────────────────────────────────────────── */

  const renderNewRun = () => (
    <div style={{ padding: 16, overflowY: "auto", maxHeight: "calc(100vh - 80px)" }}>
      {/* Title & Description */}
      <div style={sectionTitle}>Project Details</div>
      <input
        style={{ ...inputStyle, marginBottom: 8 }}
        placeholder="Project title..."
        value={projectTitle}
        onChange={(e) => setProjectTitle(e.target.value)}
      />
      <textarea
        style={{ ...inputStyle, minHeight: 80, resize: "vertical" }}
        placeholder="Project description..."
        value={projectDesc}
        onChange={(e) => setProjectDesc(e.target.value)}
      />

      {/* Tech Stack & Priority */}
      <div style={{ display: "flex", gap: 12, marginTop: 12 }}>
        <div style={{ flex: 1 }}>
          <label style={{ fontSize: 12, color: "var(--vscode-editor-foreground)", display: "block", marginBottom: 4 }}>Tech Stack</label>
          <select
            style={{ ...inputStyle }}
            value={techStack}
            onChange={(e) => setTechStack(e.target.value as TechStack)}
          >
            {TECH_STACKS.map((ts) => (
              <option key={ts.value} value={ts.value}>{ts.label}</option>
            ))}
          </select>
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ fontSize: 12, color: "var(--vscode-editor-foreground)", display: "block", marginBottom: 4 }}>Priority</label>
          <div style={{ display: "flex", gap: 4 }}>
            {(["low", "normal", "high", "critical"] as Priority[]).map((p) => (
              <button
                key={p}
                onClick={() => setPriority(p)}
                style={{
                  ...btnStyle,
                  flex: 1,
                  background: priority === p ? PRIORITY_COLORS[p] : "var(--vscode-input-background)",
                  color: priority === p ? "white" : "var(--vscode-editor-foreground)",
                  border: `1px solid ${priority === p ? PRIORITY_COLORS[p] : "var(--vscode-input-border)"}`,
                  textTransform: "capitalize",
                  fontSize: 11,
                  padding: "4px 6px",
                }}
              >
                {p}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Requirements */}
      <div style={sectionTitle}>Requirements</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
        <input
          style={{ ...inputStyle, flex: 1 }}
          placeholder="Add a requirement..."
          value={reqInput}
          onChange={(e) => setReqInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addRequirement()}
        />
        <button style={btnStyle} onClick={addRequirement}>Add</button>
      </div>
      {requirements.map((r) => (
        <div key={r.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center", padding: "6px 12px" }}>
          <span style={{ fontSize: 13 }}>{r.text}</span>
          <button style={{ ...btnDanger, padding: "2px 8px", fontSize: 11 }} onClick={() => removeRequirement(r.id)}>Remove</button>
        </div>
      ))}

      {/* User Stories */}
      <div style={sectionTitle}>User Stories</div>
      <div style={{ ...cardStyle, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
        <input style={inputStyle} placeholder="As a [persona]..." value={storyForm.persona} onChange={(e) => setStoryForm({ ...storyForm, persona: e.target.value })} />
        <input style={inputStyle} placeholder="I want to [action]..." value={storyForm.action} onChange={(e) => setStoryForm({ ...storyForm, action: e.target.value })} />
        <input style={inputStyle} placeholder="So that [benefit]..." value={storyForm.benefit} onChange={(e) => setStoryForm({ ...storyForm, benefit: e.target.value })} />
        <input style={inputStyle} placeholder="Acceptance criteria..." value={storyForm.criteria} onChange={(e) => setStoryForm({ ...storyForm, criteria: e.target.value })} />
        <button style={{ ...btnStyle, gridColumn: "span 2" }} onClick={addUserStory}>Add User Story</button>
      </div>
      {userStories.map((s) => (
        <div key={s.id} style={{ ...cardStyle, fontSize: 12 }}>
          <div><strong>As a</strong> {s.persona}, <strong>I want to</strong> {s.action}, <strong>so that</strong> {s.benefit}</div>
          {s.acceptanceCriteria && <div style={{ marginTop: 4, color: "var(--text-muted)" }}>Criteria: {s.acceptanceCriteria}</div>}
          <button style={{ ...btnDanger, padding: "2px 8px", fontSize: 11, marginTop: 4 }} onClick={() => removeUserStory(s.id)}>Remove</button>
        </div>
      ))}

      {/* API Endpoints */}
      <div style={sectionTitle}>API Endpoints</div>
      <div style={{ ...cardStyle, display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
        <select style={{ ...inputStyle, width: 90 }} value={epForm.method} onChange={(e) => setEpForm({ ...epForm, method: e.target.value as HttpMethod })}>
          {(["GET", "POST", "PUT", "PATCH", "DELETE"] as HttpMethod[]).map((m) => <option key={m} value={m}>{m}</option>)}
        </select>
        <input style={{ ...inputStyle, flex: 1, minWidth: 140 }} placeholder="/api/resource" value={epForm.path} onChange={(e) => setEpForm({ ...epForm, path: e.target.value })} />
        <input style={{ ...inputStyle, flex: 1, minWidth: 140 }} placeholder="Description" value={epForm.desc} onChange={(e) => setEpForm({ ...epForm, desc: e.target.value })} />
        <label style={{ fontSize: 12, display: "flex", alignItems: "center", gap: 4, cursor: "pointer", color: "var(--vscode-editor-foreground)" }}>
          <input type="checkbox" checked={epForm.auth} onChange={(e) => setEpForm({ ...epForm, auth: e.target.checked })} /> Auth
        </label>
        <button style={btnStyle} onClick={addEndpoint}>Add</button>
      </div>
      {endpoints.length > 0 && (
        <table style={tableStyle}>
          <thead>
            <tr><th style={thStyle}>Method</th><th style={thStyle}>Path</th><th style={thStyle}>Description</th><th style={thStyle}>Auth</th><th style={thStyle}></th></tr>
          </thead>
          <tbody>
            {endpoints.map((ep) => (
              <tr key={ep.id}>
                <td style={tdStyle}><span style={badge(ep.method, ep.method === "GET" ? "var(--success-color)" : ep.method === "DELETE" ? "var(--error-color)" : "var(--info-color)")}>{ep.method}</span></td>
                <td style={{ ...tdStyle, fontFamily: "monospace" }}>{ep.path}</td>
                <td style={tdStyle}>{ep.description}</td>
                <td style={tdStyle}>{ep.auth ? "Yes" : "No"}</td>
                <td style={tdStyle}><button style={{ ...btnDanger, padding: "2px 8px", fontSize: 11 }} onClick={() => removeEndpoint(ep.id)}>Remove</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {/* Data Models */}
      <div style={sectionTitle}>Data Models</div>
      <div style={cardStyle}>
        <input style={{ ...inputStyle, marginBottom: 8 }} placeholder="Model name (e.g., User)" value={modelName} onChange={(e) => setModelName(e.target.value)} />
        <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
          <input style={{ ...inputStyle, flex: 1 }} placeholder="Field name" value={fieldForm.name} onChange={(e) => setFieldForm({ ...fieldForm, name: e.target.value })} />
          <select style={{ ...inputStyle, width: 100 }} value={fieldForm.type} onChange={(e) => setFieldForm({ ...fieldForm, type: e.target.value as FieldType })}>
            {(["string", "number", "boolean", "date", "uuid", "json", "array", "enum"] as FieldType[]).map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
          <label style={{ fontSize: 12, display: "flex", alignItems: "center", gap: 4, cursor: "pointer", color: "var(--vscode-editor-foreground)" }}>
            <input type="checkbox" checked={fieldForm.required} onChange={(e) => setFieldForm({ ...fieldForm, required: e.target.checked })} /> Req
          </label>
          <button style={btnStyle} onClick={addField}>+ Field</button>
        </div>
        {modelFields.length > 0 && (
          <table style={{ ...tableStyle, marginBottom: 8 }}>
            <thead><tr><th style={thStyle}>Name</th><th style={thStyle}>Type</th><th style={thStyle}>Required</th><th style={thStyle}></th></tr></thead>
            <tbody>
              {modelFields.map((f) => (
                <tr key={f.id}>
                  <td style={tdStyle}>{f.name}</td>
                  <td style={tdStyle}>{f.type}</td>
                  <td style={tdStyle}>{f.required ? "Yes" : "No"}</td>
                  <td style={tdStyle}><button style={{ ...btnDanger, padding: "2px 6px", fontSize: 11 }} onClick={() => removeField(f.id)}>X</button></td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <button style={btnStyle} onClick={addModel}>Add Model</button>
      </div>
      {dataModels.map((dm) => (
        <div key={dm.id} style={{ ...cardStyle, fontSize: 12 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong>{dm.name}</strong> ({dm.fields.length} fields)
            <button style={{ ...btnDanger, padding: "2px 8px", fontSize: 11 }} onClick={() => removeModel(dm.id)}>Remove</button>
          </div>
          <div style={{ marginTop: 4, color: "var(--text-muted)" }}>
            {dm.fields.map((f) => `${f.name}: ${f.type}${f.required ? "*" : ""}`).join(", ")}
          </div>
        </div>
      ))}

      {/* Estimate & Start */}
      <div style={{ display: "flex", gap: 12, marginTop: 20 }}>
        <button style={{ ...btnStyle, background: "var(--info-color)" }} onClick={runEstimate}>Estimate</button>
        <button style={{ ...btnStyle, background: "var(--success-color)" }} onClick={startBatchRun} disabled={!projectTitle.trim()}>
          Start Batch Run
        </button>
      </div>
      {estimate && (
        <div style={{ ...cardStyle, marginTop: 12, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(120px, 1fr))", gap: 12 }}>
          <div><div style={{ fontSize: 11, color: "var(--text-muted)" }}>Est. Files</div><div style={{ fontSize: 20, fontWeight: 700 }}>{estimate.files}</div></div>
          <div><div style={{ fontSize: 11, color: "var(--text-muted)" }}>Est. Lines</div><div style={{ fontSize: 20, fontWeight: 700 }}>{estimate.lines.toLocaleString()}</div></div>
          <div><div style={{ fontSize: 11, color: "var(--text-muted)" }}>Duration</div><div style={{ fontSize: 20, fontWeight: 700 }}>{estimate.duration}</div></div>
          <div><div style={{ fontSize: 11, color: "var(--text-muted)" }}>Complexity</div><div style={{ fontSize: 20, fontWeight: 700, color: scoreColor(100 - estimate.complexity) }}>{estimate.complexity}/100</div></div>
          <div style={{ gridColumn: "span 2" }}>
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 4 }}>Recommended Agents</div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
              {estimate.agents.map((a) => <span key={a} style={badge(a, "var(--vscode-badge-background)")}>{a}</span>)}
            </div>
          </div>
        </div>
      )}
    </div>
  );

  /* ── Tab: Monitor ─────────────────────────────────────────────────── */

  const renderMonitor = () => (
    <div style={{ padding: 16, overflowY: "auto", maxHeight: "calc(100vh - 80px)" }}>
      {/* Status bar */}
      <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <span style={{ fontSize: 12, fontFamily: "monospace", color: "var(--text-muted)" }}>{runId}</span>
          <span style={badge(runStatus, STATUS_COLORS[runStatus])}>{runStatus}</span>
        </div>
        <span style={{ fontSize: 12, color: "var(--vscode-editor-foreground)" }}>Elapsed: {elapsed}</span>
      </div>

      {/* Progress bar */}
      <div style={{ marginTop: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, marginBottom: 4 }}>
          <span>{phaseLabel}</span>
          <span>{progress}%</span>
        </div>
        <div style={{ height: 8, borderRadius: 4, background: "var(--vscode-input-background)" }}>
          <div style={{ height: 8, borderRadius: 4, background: "var(--vscode-button-background)", width: `${progress}%`, transition: "width 0.3s" }} />
        </div>
      </div>

      {/* Token budget */}
      <div style={{ marginTop: 12, fontSize: 12, color: "var(--vscode-editor-foreground)" }}>
        Tokens: {tokenUsed.toLocaleString()} / {tokenTotal.toLocaleString()} ({Math.round((tokenUsed / tokenTotal) * 100)}%)
        <div style={{ height: 4, borderRadius: 2, background: "var(--vscode-input-background)", marginTop: 4 }}>
          <div style={{ height: 4, borderRadius: 2, background: tokenUsed / tokenTotal > 0.9 ? "var(--error-color)" : "var(--success-color)", width: `${(tokenUsed / tokenTotal) * 100}%` }} />
        </div>
      </div>

      {/* Agent pool grid */}
      <div style={sectionTitle}>Agent Pool</div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))", gap: 8 }}>
        {agents.map((a) => (
          <div key={a.role} style={{
            ...cardStyle,
            borderLeft: `3px solid ${a.status === "active" ? "var(--success-color)" : a.status === "done" ? "var(--info-color)" : a.status === "error" ? "var(--error-color)" : "var(--text-muted)"}`,
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
              <span style={{
                width: 24, height: 24, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center",
                fontSize: 11, fontWeight: 700, background: "var(--vscode-badge-background)", color: "var(--vscode-badge-foreground)",
              }}>{a.icon}</span>
              <span style={{ fontSize: 12, fontWeight: 600 }}>{a.role}</span>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
              <div>Files: {a.filesCreated} | Lines: {a.linesGenerated}</div>
              {a.assignedModules.length > 0 && <div>Modules: {a.assignedModules.join(", ")}</div>}
            </div>
          </div>
        ))}
      </div>

      {/* Phase timeline */}
      <div style={sectionTitle}>Phase Timeline</div>
      <div style={{ marginLeft: 8 }}>
        {phases.map((p, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6, fontSize: 13 }}>
            <span style={{ width: 16, textAlign: "center", fontSize: 14 }}>
              {p.status === "completed" ? "\u2713" : p.status === "active" ? "\u25CF" : "\u25CB"}
            </span>
            <span style={{
              color: p.status === "completed" ? "var(--success-color)" : p.status === "active" ? "var(--vscode-button-background)" : "var(--text-muted)",
              fontWeight: p.status === "active" ? 600 : 400,
            }}>{p.name}</span>
          </div>
        ))}
      </div>

      {/* Generation metrics */}
      <div style={sectionTitle}>Generation Metrics</div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(120px, 1fr))", gap: 8 }}>
        {[
          { label: "Total Files", value: metrics.files },
          { label: "Total Lines", value: metrics.lines.toLocaleString() },
          { label: "Lines/Hour", value: metrics.linesPerHour.toLocaleString() },
          { label: "Files/Hour", value: metrics.filesPerHour },
          { label: "Compile Pass", value: `${metrics.compilePass}%` },
          { label: "Test Pass", value: `${metrics.testPass}%` },
        ].map((m) => (
          <div key={m.label} style={cardStyle}>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>{m.label}</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{m.value}</div>
          </div>
        ))}
      </div>

      {/* Action buttons */}
      <div style={{ display: "flex", gap: 8, marginTop: 16 }}>
        <button style={btnStyle} onClick={pauseRun}>Pause</button>
        <button style={{ ...btnStyle, background: "var(--success-color)" }} onClick={resumeRun}>Resume</button>
        <button style={btnDanger} onClick={cancelRun}>Cancel</button>
      </div>

      {/* Log viewer */}
      <div style={sectionTitle}>Logs</div>
      <div style={{
        background: "var(--vscode-input-background)",
        border: "1px solid var(--vscode-input-border)",
        borderRadius: 6,
        maxHeight: 240,
        overflowY: "auto",
        padding: 8,
        fontFamily: "monospace",
        fontSize: 11,
      }}>
        {logs.length === 0 && <div style={{ color: "var(--text-muted)" }}>No log entries yet. Start a batch run to see logs.</div>}
        {logs.map((entry) => (
          <div key={entry.id} style={{ marginBottom: 4, display: "flex", gap: 8, alignItems: "flex-start" }}>
            <span style={badge(entry.level, LOG_COLORS[entry.level])}>{entry.level.slice(0, 4)}</span>
            <span style={{ color: "var(--text-muted)", minWidth: 55 }}>{entry.timestamp.slice(11, 19)}</span>
            <span style={{ color: "var(--info-color)", minWidth: 80 }}>[{entry.agentId}]</span>
            <span>{entry.message}</span>
          </div>
        ))}
      </div>
    </div>
  );

  /* ── Tab: QA Review ───────────────────────────────────────────────── */

  const renderQa = () => {
    const rec = recommendation(overallScore);
    return (
      <div style={{ padding: 16, overflowY: "auto", maxHeight: "calc(100vh - 80px)" }}>
        {/* Round selector & Overall score */}
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <label style={{ fontSize: 12, color: "var(--vscode-editor-foreground)" }}>QA Round:</label>
            <select style={{ ...inputStyle, width: 100 }} value={qaRound} onChange={(e) => setQaRound(Number(e.target.value))}>
              {Array.from({ length: qaRound }, (_, i) => (
                <option key={i + 1} value={i + 1}>Round {i + 1}</option>
              ))}
            </select>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <div style={{ textAlign: "center" }}>
              <div style={{ fontSize: 32, fontWeight: 700, color: scoreColor(overallScore) }}>{overallScore}</div>
              <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Overall Score</div>
            </div>
            <span style={badge(rec.text, rec.color)}>{rec.text}</span>
          </div>
        </div>

        {/* QA Agent cards */}
        <div style={sectionTitle}>QA Agent Results</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 8 }}>
          {qaAgents.map((qa) => (
            <div key={qa.name} style={{
              ...cardStyle,
              borderLeft: `3px solid ${qa.status === "pass" ? "var(--success-color)" : qa.status === "fail" ? "var(--error-color)" : "var(--warning-color)"}`,
            }}>
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 6 }}>{qa.name}</div>
              <div style={{ display: "flex", gap: 4, marginBottom: 6, flexWrap: "wrap" }}>
                {qa.critical > 0 && <span style={badge(`C:${qa.critical}`, SEVERITY_COLORS.Critical)}>{`C:${qa.critical}`}</span>}
                {qa.high > 0 && <span style={badge(`H:${qa.high}`, SEVERITY_COLORS.High)}>{`H:${qa.high}`}</span>}
                {qa.medium > 0 && <span style={badge(`M:${qa.medium}`, SEVERITY_COLORS.Medium)}>{`M:${qa.medium}`}</span>}
                {qa.low > 0 && <span style={badge(`L:${qa.low}`, SEVERITY_COLORS.Low)}>{`L:${qa.low}`}</span>}
              </div>
              <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 4 }}>Pass rate: {qa.passRate}%</div>
              <div style={{ height: 4, borderRadius: 2, background: "var(--vscode-input-background)" }}>
                <div style={{ height: 4, borderRadius: 2, background: scoreColor(qa.passRate), width: `${qa.passRate}%` }} />
              </div>
            </div>
          ))}
        </div>

        {/* Findings table */}
        <div style={{ ...sectionTitle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span>Findings ({findings.length})</span>
          <div style={{ display: "flex", gap: 4 }}>
            {(["severity", "file", "category"] as const).map((key) => (
              <button
                key={key}
                onClick={() => setFindingSortKey(key)}
                style={{
                  ...btnStyle,
                  fontSize: 11,
                  padding: "2px 8px",
                  background: findingSortKey === key ? "var(--vscode-button-background)" : "var(--vscode-input-background)",
                  color: findingSortKey === key ? "var(--vscode-button-foreground)" : "var(--vscode-editor-foreground)",
                }}
              >
                {key}
              </button>
            ))}
          </div>
        </div>
        <div style={{ overflowX: "auto" }}>
          <table style={tableStyle}>
            <thead>
              <tr>
                <th style={thStyle}>Severity</th>
                <th style={thStyle}>File</th>
                <th style={thStyle}>Line</th>
                <th style={thStyle}>Message</th>
                <th style={thStyle}>Suggestion</th>
                <th style={thStyle}>Auto-Fix</th>
                <th style={thStyle}>Resolved</th>
              </tr>
            </thead>
            <tbody>
              {sortedFindings.map((f) => (
                <tr key={f.id} style={{ opacity: f.resolved ? 0.5 : 1 }}>
                  <td style={tdStyle}><span style={badge(f.severity, SEVERITY_COLORS[f.severity])}>{f.severity}</span></td>
                  <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 11 }}>{f.file}</td>
                  <td style={tdStyle}>{f.line}</td>
                  <td style={tdStyle}>{f.message}</td>
                  <td style={{ ...tdStyle, fontSize: 11, color: "var(--text-muted)" }}>{f.suggestion}</td>
                  <td style={tdStyle}>{f.autoFixable ? "Yes" : "No"}</td>
                  <td style={tdStyle}>
                    <input type="checkbox" checked={f.resolved} onChange={() => toggleResolved(f.id)} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Cross-validation */}
        <div style={sectionTitle}>Cross-Validation</div>
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Agent A</th>
              <th style={thStyle}>Agent B</th>
              <th style={thStyle}>Confidence</th>
              <th style={thStyle}>Agreements</th>
              <th style={thStyle}>Disagreements</th>
            </tr>
          </thead>
          <tbody>
            {crossValidations.map((cv, i) => (
              <tr key={i}>
                <td style={tdStyle}>{cv.agentA}</td>
                <td style={tdStyle}>{cv.agentB}</td>
                <td style={tdStyle}><span style={{ color: scoreColor(cv.confidence), fontWeight: 600 }}>{cv.confidence}%</span></td>
                <td style={tdStyle}>{cv.agreements}</td>
                <td style={tdStyle}>{cv.disagreements}</td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* Run another round */}
        {overallScore < 90 && (
          <button style={{ ...btnStyle, marginTop: 16, background: "var(--info-color)" }} onClick={runAnotherRound}>
            Run Another Round
          </button>
        )}
      </div>
    );
  };

  /* ── Tab: Migration ───────────────────────────────────────────────── */

  const renderMigration = () => {
    const completedCount = migComponents.filter((c) => c.status === "Completed").length;
    const totalSourceLines = migComponents.reduce((s, c) => s + c.lines, 0);
    const completedLines = migComponents.filter((c) => c.status === "Completed").reduce((s, c) => s + c.lines, 0);
    const overallConfidence = Math.round(translationRules.reduce((s, r) => s + r.confidence, 0) / translationRules.length);
    const manualReviews = migComponents.filter((c) => c.risk === "Critical" || c.risk === "High").length;

    return (
      <div style={{ padding: 16, overflowY: "auto", maxHeight: "calc(100vh - 80px)" }}>
        {/* Source / Target / Strategy */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12, marginBottom: 16 }}>
          <div>
            <label style={{ fontSize: 12, color: "var(--vscode-editor-foreground)", display: "block", marginBottom: 4 }}>Source Language</label>
            <select style={inputStyle} value={sourceLang} onChange={(e) => setSourceLang(e.target.value)}>
              {SOURCE_LANGS.map((l) => <option key={l} value={l}>{l}</option>)}
            </select>
          </div>
          <div>
            <label style={{ fontSize: 12, color: "var(--vscode-editor-foreground)", display: "block", marginBottom: 4 }}>Target Language</label>
            <select style={inputStyle} value={targetLang} onChange={(e) => setTargetLang(e.target.value)}>
              {TARGET_LANGS.map((l) => <option key={l} value={l}>{l}</option>)}
            </select>
          </div>
          <div>
            <label style={{ fontSize: 12, color: "var(--vscode-editor-foreground)", display: "block", marginBottom: 4 }}>Strategy</label>
            <select style={inputStyle} value={strategy} onChange={(e) => setStrategy(e.target.value as MigrationStrategy)}>
              {(Object.keys(STRATEGY_DESCRIPTIONS) as MigrationStrategy[]).map((s) => <option key={s} value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>)}
            </select>
          </div>
        </div>
        <div style={{ ...cardStyle, fontSize: 12, color: "var(--text-muted)" }}>
          {STRATEGY_DESCRIPTIONS[strategy]}
        </div>

        {/* Component list */}
        <div style={sectionTitle}>Components</div>
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Name</th>
              <th style={thStyle}>Type</th>
              <th style={thStyle}>Language</th>
              <th style={thStyle}>Lines</th>
              <th style={thStyle}>Complexity</th>
              <th style={thStyle}>Risk</th>
              <th style={thStyle}>Status</th>
            </tr>
          </thead>
          <tbody>
            {migComponents.map((c) => (
              <tr key={c.id}>
                <td style={tdStyle}>{c.name}</td>
                <td style={tdStyle}>{c.compType}</td>
                <td style={tdStyle}>{c.language}</td>
                <td style={tdStyle}>{c.lines.toLocaleString()}</td>
                <td style={tdStyle}><span style={badge(c.complexity, c.complexity === "High" ? "var(--error-color)" : c.complexity === "Medium" ? "var(--warning-color)" : "var(--success-color)")}>{c.complexity}</span></td>
                <td style={tdStyle}><span style={badge(c.risk, SEVERITY_COLORS[c.risk === "Critical" ? "Critical" : c.risk === "High" ? "High" : c.risk === "Medium" ? "Medium" : "Low"])}>{c.risk}</span></td>
                <td style={tdStyle}><span style={badge(c.status, c.status === "Completed" ? "var(--success-color)" : c.status === "In Progress" ? "var(--info-color)" : c.status === "Failed" ? "var(--error-color)" : "var(--text-muted)")}>{c.status}</span></td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* Service boundary visualization */}
        <div style={sectionTitle}>Service Boundaries</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 8 }}>
          {["Core Services", "Data Access", "API Gateway"].map((group) => (
            <div key={group} style={{ ...cardStyle, borderTop: "3px solid var(--vscode-button-background)" }}>
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 6 }}>{group}</div>
              <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
                {migComponents
                  .filter((_, i) => (group === "Core Services" ? i < 2 : group === "Data Access" ? i >= 2 && i < 4 : i >= 4))
                  .map((c) => c.name)
                  .join(", ") || "N/A"}
              </div>
              <div style={{ fontSize: 11, marginTop: 4 }}>
                API Surface: {group === "API Gateway" ? "REST + gRPC" : "Internal"}
              </div>
              <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
                Data Store: {group === "Data Access" ? "PostgreSQL" : group === "Core Services" ? "Redis Cache" : "N/A"}
              </div>
            </div>
          ))}
        </div>

        {/* Translation rules */}
        <div style={sectionTitle}>Translation Rules</div>
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={thStyle}>Source Pattern</th>
              <th style={thStyle}>Target Pattern</th>
              <th style={thStyle}>Confidence</th>
              <th style={thStyle}>Example</th>
            </tr>
          </thead>
          <tbody>
            {translationRules.map((rule) => (
              <tr key={rule.id}>
                <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 11 }}>{rule.sourcePattern}</td>
                <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 11 }}>{rule.targetPattern}</td>
                <td style={tdStyle}><span style={{ color: scoreColor(rule.confidence), fontWeight: 600 }}>{rule.confidence}%</span></td>
                <td style={{ ...tdStyle, fontSize: 11, color: "var(--text-muted)" }}>{rule.example}</td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* Risk assessment */}
        <div style={sectionTitle}>Risk Assessment</div>
        <div style={cardStyle}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
            <span style={{ fontSize: 13, fontWeight: 600 }}>Overall Risk:</span>
            <span style={badge(
              manualReviews > 2 ? "High" : manualReviews > 0 ? "Medium" : "Low",
              manualReviews > 2 ? "var(--error-color)" : manualReviews > 0 ? "var(--warning-color)" : "var(--success-color)",
            )}>
              {manualReviews > 2 ? "High" : manualReviews > 0 ? "Medium" : "Low"}
            </span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, fontSize: 12 }}>
            <div>High complexity modules: {migComponents.filter((c) => c.complexity === "High").length}</div>
            <div>Critical risk components: {migComponents.filter((c) => c.risk === "Critical").length}</div>
            <div>Manual review needed: {manualReviews} components</div>
            <div>Untranslatable patterns: 0</div>
          </div>
        </div>

        {/* Migration progress timeline */}
        <div style={sectionTitle}>Migration Progress</div>
        <div style={{ display: "flex", gap: 4, alignItems: "center", marginBottom: 12 }}>
          {MIGRATION_PHASES.map((phase, i) => (
            <React.Fragment key={phase}>
              <div style={{
                padding: "6px 12px",
                borderRadius: 4,
                fontSize: 11,
                fontWeight: i <= migPhaseIndex ? 600 : 400,
                background: i < migPhaseIndex ? "var(--success-color)" : i === migPhaseIndex ? "var(--vscode-button-background)" : "var(--vscode-input-background)",
                color: i <= migPhaseIndex ? "white" : "var(--vscode-editor-foreground)",
              }}>
                {phase}
              </div>
              {i < MIGRATION_PHASES.length - 1 && <span style={{ color: "var(--text-muted)" }}>&rarr;</span>}
            </React.Fragment>
          ))}
        </div>

        {/* Report summary */}
        <div style={sectionTitle}>Report Summary</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))", gap: 8 }}>
          <div style={cardStyle}>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Components Migrated</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{completedCount} / {migComponents.length}</div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Lines (Source → Target)</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{completedLines.toLocaleString()} → {Math.round(completedLines * 0.6).toLocaleString()}</div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Total Source Lines</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{totalSourceLines.toLocaleString()}</div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Overall Confidence</div>
            <div style={{ fontSize: 18, fontWeight: 700, color: scoreColor(overallConfidence) }}>{overallConfidence}%</div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Manual Reviews Needed</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{manualReviews}</div>
          </div>
        </div>
      </div>
    );
  };

  /* ── Tab: History ─────────────────────────────────────────────────── */

  const renderHistory = () => (
    <div style={{ padding: 16, overflowY: "auto", maxHeight: "calc(100vh - 80px)" }}>
      {/* Total statistics */}
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 8, marginBottom: 16 }}>
        <div style={cardStyle}>
          <div style={{ fontSize: 11, color: "var(--text-muted)" }}>All-Time Lines Generated</div>
          <div style={{ fontSize: 20, fontWeight: 700 }}>{allTimeLines.toLocaleString()}</div>
        </div>
        <div style={cardStyle}>
          <div style={{ fontSize: 11, color: "var(--text-muted)" }}>All-Time Files Created</div>
          <div style={{ fontSize: 20, fontWeight: 700 }}>{allTimeFiles}</div>
        </div>
        <div style={cardStyle}>
          <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Average Run Duration</div>
          <div style={{ fontSize: 20, fontWeight: 700 }}>1h 16m</div>
        </div>
      </div>

      {/* Filter */}
      <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
        {(["all", "Completed", "Failed", "Cancelled"] as const).map((f) => (
          <button
            key={f}
            onClick={() => setHistoryFilter(f)}
            style={{
              ...btnStyle,
              fontSize: 11,
              padding: "4px 10px",
              background: historyFilter === f ? "var(--vscode-button-background)" : "var(--vscode-input-background)",
              color: historyFilter === f ? "var(--vscode-button-foreground)" : "var(--vscode-editor-foreground)",
              textTransform: "capitalize",
            }}
          >
            {f === "all" ? "All" : f}
          </button>
        ))}
      </div>

      {/* Runs table */}
      <table style={tableStyle}>
        <thead>
          <tr>
            <th style={thStyle}>ID</th>
            <th style={thStyle}>Title</th>
            <th style={thStyle}>Status</th>
            <th style={thStyle}>Files</th>
            <th style={thStyle}>Lines</th>
            <th style={thStyle}>Duration</th>
            <th style={thStyle}>Agents</th>
            <th style={thStyle}>Date</th>
          </tr>
        </thead>
        <tbody>
          {filteredHistory.map((run) => (
            <React.Fragment key={run.id}>
              <tr
                onClick={() => setExpandedRun(expandedRun === run.id ? null : run.id)}
                style={{ cursor: "pointer" }}
                onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--vscode-list-hoverBackground)"; }}
                onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
              >
                <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 11 }}>{run.id}</td>
                <td style={tdStyle}>{run.title}</td>
                <td style={tdStyle}>
                  <span style={badge(run.status, run.status === "Completed" ? "var(--success-color)" : run.status === "Failed" ? "var(--error-color)" : "var(--text-muted)")}>
                    {run.status}
                  </span>
                </td>
                <td style={tdStyle}>{run.files}</td>
                <td style={tdStyle}>{run.lines.toLocaleString()}</td>
                <td style={tdStyle}>{run.duration}</td>
                <td style={tdStyle}>{run.agents}</td>
                <td style={tdStyle}>{run.date}</td>
              </tr>
              {expandedRun === run.id && (
                <tr>
                  <td colSpan={8} style={{ ...tdStyle, padding: 16, background: "var(--vscode-input-background)" }}>
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
                      <div>
                        <div style={{ fontWeight: 600, marginBottom: 8, fontSize: 13 }}>Detailed Metrics</div>
                        <div style={{ fontSize: 12 }}>
                          <div>QA Score: <span style={{ color: scoreColor(run.qaScore), fontWeight: 600 }}>{run.qaScore}/100</span></div>
                          <div>Avg Lines/File: {run.files > 0 ? Math.round(run.lines / run.files) : 0}</div>
                          <div>Status: {run.status}</div>
                        </div>
                      </div>
                      <div>
                        <div style={{ fontWeight: 600, marginBottom: 8, fontSize: 13 }}>Generated File Tree</div>
                        <div style={{ fontFamily: "monospace", fontSize: 11 }}>
                          {run.fileTree.map((f, i) => (
                            <div key={i} style={{ color: "var(--text-muted)" }}>{f}</div>
                          ))}
                        </div>
                      </div>
                    </div>
                  </td>
                </tr>
              )}
            </React.Fragment>
          ))}
        </tbody>
      </table>
    </div>
  );

  /* ── Main render ──────────────────────────────────────────────────── */

  return (
    <div style={{
      height: "100%",
      display: "flex",
      flexDirection: "column",
      background: "var(--vscode-editor-background)",
      color: "var(--vscode-editor-foreground)",
      fontFamily: "var(--vscode-font-family, sans-serif)",
      fontSize: 13,
    }}>
      {/* Tab bar */}
      <div style={{
        display: "flex",
        borderBottom: "1px solid var(--vscode-input-border)",
        background: "var(--vscode-input-background)",
      }}>
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            style={{
              flex: 1,
              padding: "10px 0",
              border: "none",
              borderBottom: activeTab === tab.key ? "2px solid var(--vscode-button-background)" : "2px solid transparent",
              background: "transparent",
              color: activeTab === tab.key ? "var(--vscode-button-background)" : "var(--vscode-editor-foreground)",
              fontWeight: activeTab === tab.key ? 600 : 400,
              cursor: "pointer",
              fontSize: 13,
            }}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div style={{ flex: 1, overflow: "hidden" }}>
        {activeTab === "newrun" && renderNewRun()}
        {activeTab === "monitor" && renderMonitor()}
        {activeTab === "qa" && renderQa()}
        {activeTab === "migration" && renderMigration()}
        {activeTab === "history" && renderHistory()}
      </div>
    </div>
  );
};

export default BatchBuilderPanel;
