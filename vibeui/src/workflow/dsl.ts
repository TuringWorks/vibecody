// TypeScript mirror of fluxo's Conductor-compatible workflow DSL
// (see fluxo/fluxo-core/src/model.rs). The designer treats a `WorkflowDef` as
// the single source of truth; the canvas is a projection of it (see graph.ts).

export type TaskType =
  | "SIMPLE"
  | "SWITCH"
  | "FORK_JOIN"
  | "FORK_JOIN_DYNAMIC"
  | "JOIN"
  | "DO_WHILE"
  | "SUB_WORKFLOW"
  | "SET_VARIABLE"
  | "INLINE"
  | "WAIT"
  | "HUMAN"
  | "HTTP"
  | "EVENT"
  | "TERMINATE";

export interface WorkflowTask {
  name: string;
  taskReferenceName: string;
  type?: TaskType;
  inputParameters?: Record<string, unknown>;
  description?: string;
  optional?: boolean;
  // SWITCH
  evaluatorType?: string;
  expression?: string;
  decisionCases?: Record<string, WorkflowTask[]>;
  defaultCase?: WorkflowTask[];
  // FORK_JOIN
  forkTasks?: WorkflowTask[][];
  // JOIN
  joinOn?: string[];
  // DO_WHILE
  loopCondition?: string;
  loopOver?: WorkflowTask[];
  // SUB_WORKFLOW
  subWorkflowParam?: { name: string; version?: number };
  // retry / timeout policy
  retryCount?: number;
  retryDelaySeconds?: number;
  retryLogic?: "FIXED" | "EXPONENTIAL_BACKOFF";
  timeoutSeconds?: number;
}

export interface WorkflowDef {
  name: string;
  version?: number;
  description?: string;
  tasks: WorkflowTask[];
  inputParameters?: string[];
  outputParameters?: Record<string, unknown>;
  timeoutSeconds?: number;
  schemaVersion?: number;
}

/** Palette metadata for a task type. */
export interface TaskTypeMeta {
  type: TaskType;
  label: string;
  category: "flow" | "control" | "data" | "wait" | "terminal";
  color: string;
  /** A fresh task of this type with the given reference name. */
  template: (ref: string) => WorkflowTask;
}

export const TASK_TYPES: TaskTypeMeta[] = [
  { type: "SIMPLE", label: "Task", category: "flow", color: "#3b82f6",
    template: (r) => ({ name: r, taskReferenceName: r, type: "SIMPLE", inputParameters: {} }) },
  { type: "SWITCH", label: "Switch", category: "control", color: "#a855f7",
    template: (r) => ({ name: r, taskReferenceName: r, type: "SWITCH", evaluatorType: "value-param",
      expression: "case", inputParameters: { case: "${workflow.input.case}" },
      decisionCases: { a: [] }, defaultCase: [] }) },
  { type: "FORK_JOIN", label: "Fork / Join", category: "control", color: "#ec4899",
    template: (r) => ({ name: r, taskReferenceName: r, type: "FORK_JOIN", forkTasks: [[], []] }) },
  { type: "FORK_JOIN_DYNAMIC", label: "Dynamic Fork", category: "control", color: "#ec4899",
    template: (r) => ({ name: r, taskReferenceName: r, type: "FORK_JOIN_DYNAMIC",
      inputParameters: { forkedTasks: "${workflow.input.tasks}" } }) },
  { type: "JOIN", label: "Join", category: "control", color: "#ec4899",
    template: (r) => ({ name: r, taskReferenceName: r, type: "JOIN", joinOn: [] }) },
  { type: "DO_WHILE", label: "Loop", category: "control", color: "#f59e0b",
    template: (r) => ({ name: r, taskReferenceName: r, type: "DO_WHILE",
      loopCondition: "iteration < 3", loopOver: [] }) },
  { type: "SET_VARIABLE", label: "Set Variable", category: "data", color: "#14b8a6",
    template: (r) => ({ name: r, taskReferenceName: r, type: "SET_VARIABLE", inputParameters: {} }) },
  { type: "INLINE", label: "Inline", category: "data", color: "#14b8a6",
    template: (r) => ({ name: r, taskReferenceName: r, type: "INLINE", inputParameters: {} }) },
  { type: "SUB_WORKFLOW", label: "Sub-Workflow", category: "flow", color: "#3b82f6",
    template: (r) => ({ name: r, taskReferenceName: r, type: "SUB_WORKFLOW",
      subWorkflowParam: { name: "" }, inputParameters: {} }) },
  { type: "WAIT", label: "Wait", category: "wait", color: "#6b7280",
    template: (r) => ({ name: r, taskReferenceName: r, type: "WAIT" }) },
  { type: "HUMAN", label: "Human", category: "wait", color: "#6b7280",
    template: (r) => ({ name: r, taskReferenceName: r, type: "HUMAN" }) },
  { type: "TERMINATE", label: "Terminate", category: "terminal", color: "#ef4444",
    template: (r) => ({ name: r, taskReferenceName: r, type: "TERMINATE",
      inputParameters: { terminationStatus: "COMPLETED", workflowOutput: {} } }) },
];

export function colorForType(type: string | undefined): string {
  return TASK_TYPES.find((t) => t.type === type)?.color ?? "#3b82f6";
}

const LEAF_TYPES = new Set(["SIMPLE", "SET_VARIABLE", "INLINE", "WAIT", "HUMAN", "HTTP", "EVENT", "TERMINATE", "SUB_WORKFLOW", "JOIN", "FORK_JOIN_DYNAMIC"]);

export function isContainer(type: string | undefined): boolean {
  return type === "SWITCH" || type === "FORK_JOIN" || type === "DO_WHILE";
}

export function isLeaf(type: string | undefined): boolean {
  return LEAF_TYPES.has(type ?? "SIMPLE");
}

/** Depth-first walk of every task in the tree. */
export function walkTasks(tasks: WorkflowTask[], visit: (t: WorkflowTask) => void): void {
  for (const t of tasks) {
    visit(t);
    for (const sub of Object.values(t.decisionCases ?? {})) walkTasks(sub, visit);
    walkTasks(t.defaultCase ?? [], visit);
    for (const branch of t.forkTasks ?? []) walkTasks(branch, visit);
    walkTasks(t.loopOver ?? [], visit);
  }
}

/** Structural validation, mirroring the Rust `dsl::validate`. Returns error strings (empty = valid). */
export function validate(def: WorkflowDef): string[] {
  const errors: string[] = [];
  if (!def.name?.trim()) errors.push("workflow name is empty");
  if (!def.tasks?.length) errors.push("workflow has no tasks");

  const seen = new Set<string>();
  walkTasks(def.tasks ?? [], (t) => {
    const ref = t.taskReferenceName;
    if (!ref?.trim()) errors.push(`task '${t.name}' has an empty taskReferenceName`);
    else if (ref.includes("__")) errors.push(`ref '${ref}' must not contain '__' (reserved for loop instancing)`);
    else if (seen.has(ref)) errors.push(`duplicate taskReferenceName: ${ref}`);
    else seen.add(ref);
  });

  const validateList = (tasks: WorkflowTask[]) => {
    tasks.forEach((t, i) => {
      if (t.type === "SWITCH" && Object.keys(t.decisionCases ?? {}).length === 0)
        errors.push(`switch '${t.taskReferenceName}' has no decisionCases`);
      if (t.type === "FORK_JOIN" && (t.forkTasks ?? []).length === 0)
        errors.push(`fork '${t.taskReferenceName}' has no forkTasks`);
      if (t.type === "JOIN") for (const dep of t.joinOn ?? [])
        if (!seen.has(dep)) errors.push(`join '${t.taskReferenceName}' waits on unknown ref '${dep}'`);
      if (t.type === "DO_WHILE") {
        if ((t.loopOver ?? []).length === 0) errors.push(`do-while '${t.taskReferenceName}' has an empty loopOver`);
        if (!t.loopCondition?.trim()) errors.push(`do-while '${t.taskReferenceName}' has no loopCondition`);
      }
      if (t.type === "FORK_JOIN_DYNAMIC" && tasks[i + 1]?.type !== "JOIN")
        errors.push(`dynamic fork '${t.taskReferenceName}' must be immediately followed by a JOIN`);
      for (const sub of Object.values(t.decisionCases ?? {})) validateList(sub);
      validateList(t.defaultCase ?? []);
      for (const branch of t.forkTasks ?? []) validateList(branch);
      validateList(t.loopOver ?? []);
    });
  };
  validateList(def.tasks ?? []);
  return errors;
}

/** A fresh, minimal workflow definition. */
export function blankWorkflow(name = "new_workflow"): WorkflowDef {
  return {
    name,
    version: 1,
    schemaVersion: 2,
    tasks: [{ name: "step_one", taskReferenceName: "step_one", type: "SIMPLE", inputParameters: {} }],
    outputParameters: {},
  };
}
