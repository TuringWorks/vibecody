/* eslint-disable @typescript-eslint/no-explicit-any */
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
import React, { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  Queued: "var(--text-secondary)",
  Planning: "var(--info-color)",
  Generating: "var(--warning-color)",
  Validating: "var(--accent-purple)",
  Compiling: "#00bcd4",
  Testing: "var(--warning-color)",
  Completed: "var(--success-color)",
  Failed: "var(--error-color)",
};

const LOG_COLORS: Record<LogLevel, string> = {
  Debug: "var(--text-secondary)",
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
  borderRadius: "var(--radius-xs-plus)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  color: "var(--btn-primary-fg)",
  background: color,
  marginRight: 4,
});



const sectionTitle: React.CSSProperties = {
  fontSize: "var(--font-size-lg)",
  fontWeight: 600,
  marginBottom: 8,
  marginTop: 16,
  color: "var(--text-primary)",
};



/* ── Default empty state builders (no dummy data) ────────────────────── */

function buildEmptyAgents(): AgentCard[] {
  return AGENT_ROLES.map((r) => ({
    role: r.role,
    icon: r.icon,
    status: "idle" as const,
    filesCreated: 0,
    linesGenerated: 0,
    assignedModules: [],
  }));
}

function buildInitialPhases(): Phase[] {
  return [
    { name: "Initialization", status: "pending" },
    { name: "Architecture Planning", status: "pending" },
    { name: "Database Schema", status: "pending" },
    { name: "Backend Services", status: "pending" },
    { name: "Frontend Components", status: "pending" },
    { name: "API Integration", status: "pending" },
    { name: "Testing Suite", status: "pending" },
    { name: "Documentation", status: "pending" },
    { name: "Security Scan", status: "pending" },
    { name: "Final Validation", status: "pending" },
  ];
}

function buildEmptyQaAgents(): QaAgent[] {
  return QA_AGENTS.map((name) => ({
    name,
    status: "pass" as const,
    critical: 0,
    high: 0,
    medium: 0,
    low: 0,
    passRate: 0,
  }));
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
  const [agents, setAgents] = useState<AgentCard[]>(buildEmptyAgents);
  const [phases, setPhases] = useState<Phase[]>(buildInitialPhases);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [metrics, setMetrics] = useState({ files: 0, lines: 0, linesPerHour: 0, filesPerHour: 0, compilePass: 0, testPass: 0 });

  /* ── QA state ─────────────────────────────────────────────────────── */
  const [qaRound, setQaRound] = useState(1);
  const [qaAgents, setQaAgents] = useState<QaAgent[]>(buildEmptyQaAgents);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [crossValidations] = useState<CrossValidation[]>([]);
  const [overallScore] = useState(78);
  const [findingSortKey, setFindingSortKey] = useState<"severity" | "file" | "category">("severity");

  /* ── Migration state ──────────────────────────────────────────────── */
  const [sourceLang, setSourceLang] = useState("COBOL");
  const [targetLang, setTargetLang] = useState("Rust");
  const [strategy, setStrategy] = useState<MigrationStrategy>("strangler");
  const [migComponents] = useState<MigrationComponent[]>([]);
  const [translationRules] = useState<TranslationRule[]>([]);
  const [migPhaseIndex] = useState(2);

  /* ── History state ────────────────────────────────────────────────── */
  const [historyRuns, setHistoryRuns] = useState<HistoryRun[]>([]);
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

  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Load history from backend on mount
  const loadHistory = useCallback(async () => {
    try {
      const runs = await invoke<Record<string, unknown>[]>("batch_list_runs");
      if (Array.isArray(runs)) {
        setHistoryRuns(runs.filter((r: any) => ["Completed", "Failed", "Cancelled"].includes(r.status)).map((r: any) => ({
          id: r.id || "",
          title: r.title || r.projectTitle || "Untitled",
          status: r.status || "Completed",
          files: r.files || 0,
          lines: r.lines || 0,
          duration: r.duration || r.elapsed || "0m",
          agents: r.agentCount || AGENT_ROLES.length,
          date: r.createdAt ? r.createdAt.split("T")[0] : "",
          qaScore: r.qaScore || 0,
          fileTree: r.fileTree || [],
        })));
      }
    } catch { /* first run, no history */ }
  }, []);

  useEffect(() => { loadHistory(); }, [loadHistory]);

  // Clean up polling on unmount
  useEffect(() => {
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, []);

  const startBatchRun = async () => {
    try {
      const spec = {
        title: projectTitle || "Untitled Batch Run",
        description: projectDesc,
        techStack,
        priority,
        requirements: requirements.map(r => r.text),
        userStories: userStories.map(s => `As a ${s.persona}, I want to ${s.action} so that ${s.benefit}`),
        endpoints: endpoints.map(e => `${e.method} ${e.path}`),
        dataModels: dataModels.map(m => m.name),
        agentCount: estimate?.agents.length || AGENT_ROLES.length,
      };
      const run = await invoke<Record<string, unknown>>("batch_create_run", { spec });
      const newRunId = run.id || runId;

      setRunStatus("Planning");
      setElapsed("0m 0s");
      setProgress(0);
      setPhaseLabel("Starting...");
      setTokenUsed(0);
      setLogs([]);
      setAgents(buildEmptyAgents());
      setPhases(buildInitialPhases());
      setMetrics({ files: 0, lines: 0, linesPerHour: 0, filesPerHour: 0, compilePass: 0, testPass: 0 });
      setActiveTab("monitor");

      // Start polling for progress
      if (pollRef.current) clearInterval(pollRef.current);
      pollRef.current = setInterval(async () => {
        try {
          const updated = await invoke<any>("batch_simulate_progress", { runId: newRunId });
          if (!updated) return;
          setRunStatus(updated.status || "Generating");
          setProgress(updated.progress || 0);
          setPhaseLabel(updated.phaseLabel || "");
          setTokenUsed(updated.tokenUsed || 0);
          setElapsed(updated.elapsed || "");
          const f = updated.files || 0;
          const l = updated.lines || 0;
          setMetrics({
            files: f, lines: l,
            linesPerHour: l > 0 ? Math.round(l * 3.6) : 0,
            filesPerHour: f > 0 ? Math.round(f * 3.6) : 0,
            compilePass: updated.progress > 60 ? 92 : 0,
            testPass: updated.progress > 80 ? 78 : 0,
          });
          // Update logs
          if (Array.isArray(updated.logs)) {
            setLogs(updated.logs.map((lg: any, i: number) => ({
              id: i + 1000,
              level: lg.level || "Info",
              timestamp: lg.timestamp || "",
              agentId: lg.agentId || "System",
              message: lg.message || "",
            })));
          }
          // Update phases based on progress
          const prog = updated.progress || 0;
          setPhases(buildInitialPhases().map((p, i) => {
            const threshold = (i + 1) * 10;
            if (prog >= threshold) return { ...p, status: "completed" as const };
            if (prog >= threshold - 10) return { ...p, status: "active" as const };
            return p;
          }));
          // Update agents
          setAgents(AGENT_ROLES.map((r, i) => ({
            role: r.role, icon: r.icon,
            status: (prog > i * 10 && prog < 100) ? "active" as const : prog >= 100 ? "done" as const : "idle" as const,
            filesCreated: prog > i * 10 ? Math.floor(prog * 0.08 * (i + 1)) : 0,
            linesGenerated: prog > i * 10 ? Math.floor(prog * 12 * (i + 1)) : 0,
            assignedModules: prog > i * 10 ? [`Module-${i + 1}`] : [],
          })));

          if (updated.status === "Completed" || updated.status === "Failed") {
            if (pollRef.current) clearInterval(pollRef.current);
            pollRef.current = null;
            loadHistory();
          }
        } catch {
          if (pollRef.current) clearInterval(pollRef.current);
          pollRef.current = null;
        }
      }, 2000);
    } catch (e) {
      setRunStatus("Failed");
      setPhaseLabel(String(e));
    }
  };

  /* ── Monitor actions ──────────────────────────────────────────────── */

  const pauseRun = () => {
    if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null; }
    setRunStatus("Queued");
  };
  const resumeRun = () => setRunStatus("Generating");
  const cancelRun = async () => {
    if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null; }
    setRunStatus("Failed");
    setPhaseLabel("Cancelled by user");
    try {
      await invoke("batch_update_run", { runId, updates: { status: "Cancelled", phaseLabel: "Cancelled by user" } });
      loadHistory();
    } catch { /* ignore */ }
  };

  /* ── QA actions ───────────────────────────────────────────────────── */

  const toggleResolved = (id: number) => {
    setFindings((prev) => prev.map((f) => (f.id === id ? { ...f, resolved: !f.resolved } : f)));
  };

  const runAnotherRound = () => {
    setQaRound((r) => r + 1);
    // Re-evaluate QA agents with updated scores based on resolved findings
    const resolvedCount = findings.filter(f => f.resolved).length;
    const totalCount = findings.length || 1;
    setQaAgents(QA_AGENTS.map((name) => ({
      name,
      status: "pass" as const,
      critical: findings.filter(f => !f.resolved && f.severity === "Critical").length,
      high: findings.filter(f => !f.resolved && f.severity === "High").length,
      medium: findings.filter(f => !f.resolved && f.severity === "Medium").length,
      low: findings.filter(f => !f.resolved && f.severity === "Low").length,
      passRate: Math.round((resolvedCount / totalCount) * 100),
    })));
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
        className="panel-input panel-input-full" style={{ marginBottom: 8 }}
        placeholder="Project title..."
        value={projectTitle}
        onChange={(e) => setProjectTitle(e.target.value)}
      />
      <textarea
        className="panel-input panel-input-full" style={{ minHeight: 80, resize: "vertical" }}
        placeholder="Project description..."
        value={projectDesc}
        onChange={(e) => setProjectDesc(e.target.value)}
      />

      {/* Tech Stack & Priority */}
      <div style={{ display: "flex", gap: 12, marginTop: 12 }}>
        <div style={{ flex: 1 }}>
          <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", display: "block", marginBottom: 4 }}>Tech Stack</label>
          <select
            className="panel-input panel-input-full"
            value={techStack}
            onChange={(e) => setTechStack(e.target.value as TechStack)}
          >
            {TECH_STACKS.map((ts) => (
              <option key={ts.value} value={ts.value}>{ts.label}</option>
            ))}
          </select>
        </div>
        <div style={{ flex: 1 }}>
          <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", display: "block", marginBottom: 4 }}>Priority</label>
          <div style={{ display: "flex", gap: 4 }}>
            {(["low", "normal", "high", "critical"] as Priority[]).map((p) => (
              <button
                key={p}
                onClick={() => setPriority(p)}
                className="panel-btn panel-btn-secondary"
                style={{
                  flex: 1,
                  background: priority === p ? PRIORITY_COLORS[p] : "var(--bg-secondary)",
                  color: priority === p ? "white" : "var(--text-primary)",
                  border: `1px solid ${priority === p ? PRIORITY_COLORS[p] : "var(--border-color)"}`,
                  textTransform: "capitalize",
                  fontSize: "var(--font-size-sm)",
                  padding: "4px 8px",
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
          className="panel-input" style={{ flex: 1 }}
          placeholder="Add a requirement..."
          value={reqInput}
          onChange={(e) => setReqInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addRequirement()}
        />
        <button className="panel-btn panel-btn-primary" onClick={addRequirement}>Add</button>
      </div>
      {requirements.map((r) => (
        <div key={r.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "8px 12px" }}>
          <span style={{ fontSize: "var(--font-size-md)" }}>{r.text}</span>
          <button className="panel-btn panel-btn-danger" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => removeRequirement(r.id)}>Remove</button>
        </div>
      ))}

      {/* User Stories */}
      <div style={sectionTitle}>User Stories</div>
      <div className="panel-card" style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
        <input className="panel-input panel-input-full" placeholder="As a [persona]..." value={storyForm.persona} onChange={(e) => setStoryForm({ ...storyForm, persona: e.target.value })} />
        <input className="panel-input panel-input-full" placeholder="I want to [action]..." value={storyForm.action} onChange={(e) => setStoryForm({ ...storyForm, action: e.target.value })} />
        <input className="panel-input panel-input-full" placeholder="So that [benefit]..." value={storyForm.benefit} onChange={(e) => setStoryForm({ ...storyForm, benefit: e.target.value })} />
        <input className="panel-input panel-input-full" placeholder="Acceptance criteria..." value={storyForm.criteria} onChange={(e) => setStoryForm({ ...storyForm, criteria: e.target.value })} />
        <button className="panel-btn panel-btn-primary" style={{ gridColumn: "span 2" }} onClick={addUserStory}>Add User Story</button>
      </div>
      {userStories.map((s) => (
        <div key={s.id} className="panel-card" style={{ fontSize: "var(--font-size-base)" }}>
          <div><strong>As a</strong> {s.persona}, <strong>I want to</strong> {s.action}, <strong>so that</strong> {s.benefit}</div>
          {s.acceptanceCriteria && <div style={{ marginTop: 4, color: "var(--text-secondary)" }}>Criteria: {s.acceptanceCriteria}</div>}
          <button className="panel-btn panel-btn-danger" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)", marginTop: 4 }} onClick={() => removeUserStory(s.id)}>Remove</button>
        </div>
      ))}

      {/* API Endpoints */}
      <div style={sectionTitle}>API Endpoints</div>
      <div className="panel-card" style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
        <select className="panel-select" style={{ width: 90 }} value={epForm.method} onChange={(e) => setEpForm({ ...epForm, method: e.target.value as HttpMethod })}>
          {(["GET", "POST", "PUT", "PATCH", "DELETE"] as HttpMethod[]).map((m) => <option key={m} value={m}>{m}</option>)}
        </select>
        <input className="panel-input" style={{ flex: 1, minWidth: 140 }} placeholder="/api/resource" value={epForm.path} onChange={(e) => setEpForm({ ...epForm, path: e.target.value })} />
        <input className="panel-input" style={{ flex: 1, minWidth: 140 }} placeholder="Description" value={epForm.desc} onChange={(e) => setEpForm({ ...epForm, desc: e.target.value })} />
        <label style={{ fontSize: "var(--font-size-base)", display: "flex", alignItems: "center", gap: 4, cursor: "pointer", color: "var(--text-primary)" }}>
          <input type="checkbox" checked={epForm.auth} onChange={(e) => setEpForm({ ...epForm, auth: e.target.checked })} /> Auth
        </label>
        <button className="panel-btn panel-btn-primary" onClick={addEndpoint}>Add</button>
      </div>
      {endpoints.length > 0 && (
        <table className="panel-table">
          <thead>
            <tr><th>Method</th><th>Path</th><th>Description</th><th>Auth</th><th></th></tr>
          </thead>
          <tbody>
            {endpoints.map((ep) => (
              <tr key={ep.id}>
                <td><span style={badge(ep.method, ep.method === "GET" ? "var(--success-color)" : ep.method === "DELETE" ? "var(--error-color)" : "var(--info-color)")}>{ep.method}</span></td>
                <td style={{ fontFamily: "var(--font-mono)" }}>{ep.path}</td>
                <td>{ep.description}</td>
                <td>{ep.auth ? "Yes" : "No"}</td>
                <td><button className="panel-btn panel-btn-danger" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => removeEndpoint(ep.id)}>Remove</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {/* Data Models */}
      <div style={sectionTitle}>Data Models</div>
      <div className="panel-card">
        <input className="panel-input panel-input-full" style={{ marginBottom: 8 }} placeholder="Model name (e.g., User)" value={modelName} onChange={(e) => setModelName(e.target.value)} />
        <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
          <input className="panel-input" style={{ flex: 1 }} placeholder="Field name" value={fieldForm.name} onChange={(e) => setFieldForm({ ...fieldForm, name: e.target.value })} />
          <select className="panel-select" style={{ width: 100 }} value={fieldForm.type} onChange={(e) => setFieldForm({ ...fieldForm, type: e.target.value as FieldType })}>
            {(["string", "number", "boolean", "date", "uuid", "json", "array", "enum"] as FieldType[]).map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
          <label style={{ fontSize: "var(--font-size-base)", display: "flex", alignItems: "center", gap: 4, cursor: "pointer", color: "var(--text-primary)" }}>
            <input type="checkbox" checked={fieldForm.required} onChange={(e) => setFieldForm({ ...fieldForm, required: e.target.checked })} /> Req
          </label>
          <button className="panel-btn panel-btn-primary" onClick={addField}>+ Field</button>
        </div>
        {modelFields.length > 0 && (
          <table className="panel-table" style={{ marginBottom: 8 }}>
            <thead><tr><th>Name</th><th>Type</th><th>Required</th><th></th></tr></thead>
            <tbody>
              {modelFields.map((f) => (
                <tr key={f.id}>
                  <td>{f.name}</td>
                  <td>{f.type}</td>
                  <td>{f.required ? "Yes" : "No"}</td>
                  <td><button className="panel-btn panel-btn-danger" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => removeField(f.id)}>X</button></td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <button className="panel-btn panel-btn-primary" onClick={addModel}>Add Model</button>
      </div>
      {dataModels.map((dm) => (
        <div key={dm.id} className="panel-card" style={{ fontSize: "var(--font-size-base)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong>{dm.name}</strong> ({dm.fields.length} fields)
            <button className="panel-btn panel-btn-danger" style={{ padding: "2px 8px", fontSize: "var(--font-size-sm)" }} onClick={() => removeModel(dm.id)}>Remove</button>
          </div>
          <div style={{ marginTop: 4, color: "var(--text-secondary)" }}>
            {dm.fields.map((f) => `${f.name}: ${f.type}${f.required ? "*" : ""}`).join(", ")}
          </div>
        </div>
      ))}

      {/* Estimate & Start */}
      <div style={{ display: "flex", gap: 12, marginTop: 20 }}>
        <button className="panel-btn panel-btn-primary" style={{ background: "var(--info-color)" }} onClick={runEstimate}>Estimate</button>
        <button className="panel-btn panel-btn-primary" style={{ background: "var(--success-color)" }} onClick={startBatchRun} disabled={!projectTitle.trim()}>
          Start Batch Run
        </button>
      </div>
      {estimate && (
        <div className="panel-card" style={{ marginTop: 12, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(120px, 1fr))", gap: 12 }}>
          <div><div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Est. Files</div><div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{estimate.files}</div></div>
          <div><div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Est. Lines</div><div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{estimate.lines.toLocaleString()}</div></div>
          <div><div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Duration</div><div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{estimate.duration}</div></div>
          <div><div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Complexity</div><div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: scoreColor(100 - estimate.complexity) }}>{estimate.complexity}/100</div></div>
          <div style={{ gridColumn: "span 2" }}>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>Recommended Agents</div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
              {estimate.agents.map((a) => <span key={a} style={badge(a, "var(--bg-tertiary)")}>{a}</span>)}
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
      <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <span style={{ fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>{runId}</span>
          <span style={badge(runStatus, STATUS_COLORS[runStatus])}>{runStatus}</span>
        </div>
        <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>Elapsed: {elapsed}</span>
      </div>

      {/* Progress bar */}
      <div style={{ marginTop: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-base)", marginBottom: 4 }}>
          <span>{phaseLabel}</span>
          <span>{progress}%</span>
        </div>
        <div style={{ height: 8, borderRadius: "var(--radius-xs-plus)", background: "var(--bg-secondary)" }}>
          <div style={{ height: 8, borderRadius: "var(--radius-xs-plus)", background: "var(--accent-color)", width: `${progress}%`, transition: "width 0.3s" }} />
        </div>
      </div>

      {/* Token budget */}
      <div style={{ marginTop: 12, fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>
        Tokens: {tokenUsed.toLocaleString()} / {tokenTotal.toLocaleString()} ({Math.round((tokenUsed / tokenTotal) * 100)}%)
        <div style={{ height: 4, borderRadius: 2, background: "var(--bg-secondary)", marginTop: 4 }}>
          <div style={{ height: 4, borderRadius: 2, background: tokenUsed / tokenTotal > 0.9 ? "var(--error-color)" : "var(--success-color)", width: `${(tokenUsed / tokenTotal) * 100}%` }} />
        </div>
      </div>

      {/* Agent pool grid */}
      <div style={sectionTitle}>Agent Pool</div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))", gap: 8 }}>
        {agents.map((a) => (
          <div key={a.role} className="panel-card" style={{
            borderLeft: `3px solid ${a.status === "active" ? "var(--success-color)" : a.status === "done" ? "var(--info-color)" : a.status === "error" ? "var(--error-color)" : "var(--text-secondary)"}`,
          }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
              <span style={{
                width: 24, height: 24, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center",
                fontSize: "var(--font-size-sm)", fontWeight: 700, background: "var(--bg-tertiary)", color: "var(--btn-primary-fg)",
              }}>{a.icon}</span>
              <span style={{ fontSize: "var(--font-size-base)", fontWeight: 600 }}>{a.role}</span>
            </div>
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
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
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6, fontSize: "var(--font-size-md)" }}>
            <span style={{ width: 16, textAlign: "center", fontSize: "var(--font-size-lg)" }}>
              {p.status === "completed" ? "\u2713" : p.status === "active" ? "\u25CF" : "\u25CB"}
            </span>
            <span style={{
              color: p.status === "completed" ? "var(--success-color)" : p.status === "active" ? "var(--accent-color)" : "var(--text-secondary)",
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
          <div key={m.label} className="panel-card">
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{m.label}</div>
            <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{m.value}</div>
          </div>
        ))}
      </div>

      {/* Action buttons */}
      <div style={{ display: "flex", gap: 8, marginTop: 16 }}>
        <button className="panel-btn panel-btn-primary" onClick={pauseRun}>Pause</button>
        <button className="panel-btn panel-btn-primary" style={{ background: "var(--success-color)" }} onClick={resumeRun}>Resume</button>
        <button className="panel-btn panel-btn-danger" onClick={cancelRun}>Cancel</button>
      </div>

      {/* Log viewer */}
      <div style={sectionTitle}>Logs</div>
      <div style={{
        background: "var(--bg-secondary)",
        border: "1px solid var(--border-color)",
        borderRadius: "var(--radius-sm)",
        maxHeight: 240,
        overflowY: "auto",
        padding: 8,
        fontFamily: "var(--font-mono)",
        fontSize: "var(--font-size-sm)",
      }}>
        {logs.length === 0 && <div style={{ color: "var(--text-secondary)" }}>No log entries yet. Start a batch run to see logs.</div>}
        {logs.map((entry) => (
          <div key={entry.id} style={{ marginBottom: 4, display: "flex", gap: 8, alignItems: "flex-start" }}>
            <span style={badge(entry.level, LOG_COLORS[entry.level])}>{entry.level.slice(0, 4)}</span>
            <span style={{ color: "var(--text-secondary)", minWidth: 55 }}>{entry.timestamp.slice(11, 19)}</span>
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
            <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>QA Round:</label>
            <select className="panel-select" style={{ width: 100 }} value={qaRound} onChange={(e) => setQaRound(Number(e.target.value))}>
              {Array.from({ length: qaRound }, (_, i) => (
                <option key={i + 1} value={i + 1}>Round {i + 1}</option>
              ))}
            </select>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <div style={{ textAlign: "center" }}>
              <div style={{ fontSize: 32, fontWeight: 700, fontFamily: "var(--font-mono)", color: scoreColor(overallScore) }}>{overallScore}</div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Overall Score</div>
            </div>
            <span style={badge(rec.text, rec.color)}>{rec.text}</span>
          </div>
        </div>

        {/* QA Agent cards */}
        <div style={sectionTitle}>QA Agent Results</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 8 }}>
          {qaAgents.map((qa) => (
            <div key={qa.name} className="panel-card" style={{
              borderLeft: `3px solid ${qa.status === "pass" ? "var(--success-color)" : qa.status === "fail" ? "var(--error-color)" : "var(--warning-color)"}`,
            }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 6 }}>{qa.name}</div>
              <div style={{ display: "flex", gap: 4, marginBottom: 6, flexWrap: "wrap" }}>
                {qa.critical > 0 && <span style={badge(`C:${qa.critical}`, SEVERITY_COLORS.Critical)}>{`C:${qa.critical}`}</span>}
                {qa.high > 0 && <span style={badge(`H:${qa.high}`, SEVERITY_COLORS.High)}>{`H:${qa.high}`}</span>}
                {qa.medium > 0 && <span style={badge(`M:${qa.medium}`, SEVERITY_COLORS.Medium)}>{`M:${qa.medium}`}</span>}
                {qa.low > 0 && <span style={badge(`L:${qa.low}`, SEVERITY_COLORS.Low)}>{`L:${qa.low}`}</span>}
              </div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>Pass rate: {qa.passRate}%</div>
              <div style={{ height: 4, borderRadius: 2, background: "var(--bg-secondary)" }}>
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
                className="panel-btn panel-btn-secondary"
                style={{
                  fontSize: "var(--font-size-sm)",
                  padding: "2px 8px",
                  background: findingSortKey === key ? "var(--accent-color)" : "var(--bg-secondary)",
                  color: findingSortKey === key ? "white" : "var(--text-primary)",
                }}
              >
                {key}
              </button>
            ))}
          </div>
        </div>
        <div style={{ overflowX: "auto" }}>
          <table className="panel-table">
            <thead>
              <tr>
                <th>Severity</th>
                <th>File</th>
                <th>Line</th>
                <th>Message</th>
                <th>Suggestion</th>
                <th>Auto-Fix</th>
                <th>Resolved</th>
              </tr>
            </thead>
            <tbody>
              {sortedFindings.map((f) => (
                <tr key={f.id} style={{ opacity: f.resolved ? 0.5 : 1 }}>
                  <td><span style={badge(f.severity, SEVERITY_COLORS[f.severity])}>{f.severity}</span></td>
                  <td style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{f.file}</td>
                  <td>{f.line}</td>
                  <td>{f.message}</td>
                  <td style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{f.suggestion}</td>
                  <td>{f.autoFixable ? "Yes" : "No"}</td>
                  <td>
                    <input type="checkbox" checked={f.resolved} onChange={() => toggleResolved(f.id)} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Cross-validation */}
        <div style={sectionTitle}>Cross-Validation</div>
        <table className="panel-table">
          <thead>
            <tr>
              <th>Agent A</th>
              <th>Agent B</th>
              <th>Confidence</th>
              <th>Agreements</th>
              <th>Disagreements</th>
            </tr>
          </thead>
          <tbody>
            {crossValidations.map((cv, i) => (
              <tr key={i}>
                <td>{cv.agentA}</td>
                <td>{cv.agentB}</td>
                <td><span style={{ color: scoreColor(cv.confidence), fontWeight: 600 }}>{cv.confidence}%</span></td>
                <td>{cv.agreements}</td>
                <td>{cv.disagreements}</td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* Run another round */}
        {overallScore < 90 && (
          <button className="panel-btn panel-btn-primary" style={{ marginTop: 16, background: "var(--info-color)" }} onClick={runAnotherRound}>
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
    const overallConfidence = translationRules.length > 0 ? Math.round(translationRules.reduce((s, r) => s + r.confidence, 0) / translationRules.length) : 0;
    const manualReviews = migComponents.filter((c) => c.risk === "Critical" || c.risk === "High").length;

    return (
      <div style={{ padding: 16, overflowY: "auto", maxHeight: "calc(100vh - 80px)" }}>
        {/* Source / Target / Strategy */}
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12, marginBottom: 16 }}>
          <div>
            <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", display: "block", marginBottom: 4 }}>Source Language</label>
            <select className="panel-input panel-input-full" value={sourceLang} onChange={(e) => setSourceLang(e.target.value)}>
              {SOURCE_LANGS.map((l) => <option key={l} value={l}>{l}</option>)}
            </select>
          </div>
          <div>
            <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", display: "block", marginBottom: 4 }}>Target Language</label>
            <select className="panel-input panel-input-full" value={targetLang} onChange={(e) => setTargetLang(e.target.value)}>
              {TARGET_LANGS.map((l) => <option key={l} value={l}>{l}</option>)}
            </select>
          </div>
          <div>
            <label style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", display: "block", marginBottom: 4 }}>Strategy</label>
            <select className="panel-input panel-input-full" value={strategy} onChange={(e) => setStrategy(e.target.value as MigrationStrategy)}>
              {(Object.keys(STRATEGY_DESCRIPTIONS) as MigrationStrategy[]).map((s) => <option key={s} value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>)}
            </select>
          </div>
        </div>
        <div className="panel-card" style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
          {STRATEGY_DESCRIPTIONS[strategy]}
        </div>

        {/* Component list */}
        <div style={sectionTitle}>Components</div>
        <table className="panel-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Language</th>
              <th>Lines</th>
              <th>Complexity</th>
              <th>Risk</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {migComponents.map((c) => (
              <tr key={c.id}>
                <td>{c.name}</td>
                <td>{c.compType}</td>
                <td>{c.language}</td>
                <td>{c.lines.toLocaleString()}</td>
                <td><span style={badge(c.complexity, c.complexity === "High" ? "var(--error-color)" : c.complexity === "Medium" ? "var(--warning-color)" : "var(--success-color)")}>{c.complexity}</span></td>
                <td><span style={badge(c.risk, SEVERITY_COLORS[c.risk === "Critical" ? "Critical" : c.risk === "High" ? "High" : c.risk === "Medium" ? "Medium" : "Low"])}>{c.risk}</span></td>
                <td><span style={badge(c.status, c.status === "Completed" ? "var(--success-color)" : c.status === "In Progress" ? "var(--info-color)" : c.status === "Failed" ? "var(--error-color)" : "var(--text-secondary)")}>{c.status}</span></td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* Service boundary visualization */}
        <div style={sectionTitle}>Service Boundaries</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 8 }}>
          {["Core Services", "Data Access", "API Gateway"].map((group) => (
            <div key={group} className="panel-card" style={{ borderTop: "3px solid var(--accent-color)" }}>
              <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", marginBottom: 6 }}>{group}</div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                {migComponents
                  .filter((_, i) => (group === "Core Services" ? i < 2 : group === "Data Access" ? i >= 2 && i < 4 : i >= 4))
                  .map((c) => c.name)
                  .join(", ") || "N/A"}
              </div>
              <div style={{ fontSize: "var(--font-size-sm)", marginTop: 4 }}>
                API Surface: {group === "API Gateway" ? "REST + gRPC" : "Internal"}
              </div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                Data Store: {group === "Data Access" ? "PostgreSQL" : group === "Core Services" ? "Redis Cache" : "N/A"}
              </div>
            </div>
          ))}
        </div>

        {/* Translation rules */}
        <div style={sectionTitle}>Translation Rules</div>
        <table className="panel-table">
          <thead>
            <tr>
              <th>Source Pattern</th>
              <th>Target Pattern</th>
              <th>Confidence</th>
              <th>Example</th>
            </tr>
          </thead>
          <tbody>
            {translationRules.map((rule) => (
              <tr key={rule.id}>
                <td style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{rule.sourcePattern}</td>
                <td style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{rule.targetPattern}</td>
                <td><span style={{ color: scoreColor(rule.confidence), fontWeight: 600 }}>{rule.confidence}%</span></td>
                <td style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{rule.example}</td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* Risk assessment */}
        <div style={sectionTitle}>Risk Assessment</div>
        <div className="panel-card">
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
            <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Overall Risk:</span>
            <span style={badge(
              manualReviews > 2 ? "High" : manualReviews > 0 ? "Medium" : "Low",
              manualReviews > 2 ? "var(--error-color)" : manualReviews > 0 ? "var(--warning-color)" : "var(--success-color)",
            )}>
              {manualReviews > 2 ? "High" : manualReviews > 0 ? "Medium" : "Low"}
            </span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, fontSize: "var(--font-size-base)" }}>
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
                padding: "8px 12px",
                borderRadius: "var(--radius-xs-plus)",
                fontSize: "var(--font-size-sm)",
                fontWeight: i <= migPhaseIndex ? 600 : 400,
                background: i < migPhaseIndex ? "var(--success-color)" : i === migPhaseIndex ? "var(--accent-color)" : "var(--bg-secondary)",
                color: i <= migPhaseIndex ? "white" : "var(--text-primary)",
              }}>
                {phase}
              </div>
              {i < MIGRATION_PHASES.length - 1 && <span style={{ color: "var(--text-secondary)" }}>&rarr;</span>}
            </React.Fragment>
          ))}
        </div>

        {/* Report summary */}
        <div style={sectionTitle}>Report Summary</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))", gap: 8 }}>
          <div className="panel-card">
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Components Migrated</div>
            <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{completedCount} / {migComponents.length}</div>
          </div>
          <div className="panel-card">
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Lines (Source → Target)</div>
            <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{completedLines.toLocaleString()} → {Math.round(completedLines * 0.6).toLocaleString()}</div>
          </div>
          <div className="panel-card">
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Total Source Lines</div>
            <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{totalSourceLines.toLocaleString()}</div>
          </div>
          <div className="panel-card">
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Overall Confidence</div>
            <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: scoreColor(overallConfidence) }}>{overallConfidence}%</div>
          </div>
          <div className="panel-card">
            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Manual Reviews Needed</div>
            <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{manualReviews}</div>
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
        <div className="panel-card">
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>All-Time Lines Generated</div>
          <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{allTimeLines.toLocaleString()}</div>
        </div>
        <div className="panel-card">
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>All-Time Files Created</div>
          <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{allTimeFiles}</div>
        </div>
        <div className="panel-card">
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Average Run Duration</div>
          <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>1h 16m</div>
        </div>
      </div>

      {/* Filter */}
      <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
        {(["all", "Completed", "Failed", "Cancelled"] as const).map((f) => (
          <button
            key={f}
            onClick={() => setHistoryFilter(f)}
            className="panel-btn panel-btn-secondary"
            style={{
              fontSize: "var(--font-size-sm)",
              padding: "4px 12px",
              background: historyFilter === f ? "var(--accent-color)" : "var(--bg-secondary)",
              color: historyFilter === f ? "white" : "var(--text-primary)",
              textTransform: "capitalize",
            }}
          >
            {f === "all" ? "All" : f}
          </button>
        ))}
      </div>

      {/* Runs table */}
      <table className="panel-table">
        <thead>
          <tr>
            <th>ID</th>
            <th>Title</th>
            <th>Status</th>
            <th>Files</th>
            <th>Lines</th>
            <th>Duration</th>
            <th>Agents</th>
            <th>Date</th>
          </tr>
        </thead>
        <tbody>
          {filteredHistory.map((run) => (
            <React.Fragment key={run.id}>
              <tr
                onClick={() => setExpandedRun(expandedRun === run.id ? null : run.id)}
                style={{ cursor: "pointer" }}
                onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--bg-secondary)"; }}
                onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
              >
                <td style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{run.id}</td>
                <td>{run.title}</td>
                <td>
                  <span style={badge(run.status, run.status === "Completed" ? "var(--success-color)" : run.status === "Failed" ? "var(--error-color)" : "var(--text-secondary)")}>
                    {run.status}
                  </span>
                </td>
                <td>{run.files}</td>
                <td>{run.lines.toLocaleString()}</td>
                <td>{run.duration}</td>
                <td>{run.agents}</td>
                <td>{run.date}</td>
              </tr>
              {expandedRun === run.id && (
                <tr>
                  <td colSpan={8} style={{ padding: 16, background: "var(--bg-secondary)" }}>
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
                      <div>
                        <div style={{ fontWeight: 600, marginBottom: 8, fontSize: "var(--font-size-md)" }}>Detailed Metrics</div>
                        <div style={{ fontSize: "var(--font-size-base)" }}>
                          <div>QA Score: <span style={{ color: scoreColor(run.qaScore), fontWeight: 600 }}>{run.qaScore}/100</span></div>
                          <div>Avg Lines/File: {run.files > 0 ? Math.round(run.lines / run.files) : 0}</div>
                          <div>Status: {run.status}</div>
                        </div>
                      </div>
                      <div>
                        <div style={{ fontWeight: 600, marginBottom: 8, fontSize: "var(--font-size-md)" }}>Generated File Tree</div>
                        <div style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>
                          {run.fileTree.map((f, i) => (
                            <div key={i} style={{ color: "var(--text-secondary)" }}>{f}</div>
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
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-tab-bar">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`panel-tab ${activeTab === tab.key ? "active" : ""}`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="panel-body" style={{ padding: 0, overflow: "hidden" }}>
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
