// Immutable structural edits on a WorkflowDef. The nested DSL is the source of
// truth; the canvas edits call these. Every op returns a new def (structuredClone).

import { walkTasks, type WorkflowDef, type WorkflowTask } from "./dsl";

function locate(tasks: WorkflowTask[], ref: string): { list: WorkflowTask[]; index: number } | null {
  for (let i = 0; i < tasks.length; i++) {
    const t = tasks[i];
    if (t.taskReferenceName === ref) return { list: tasks, index: i };
    for (const sub of Object.values(t.decisionCases ?? {})) {
      const r = locate(sub, ref);
      if (r) return r;
    }
    const d = locate(t.defaultCase ?? [], ref);
    if (d) return d;
    for (const b of t.forkTasks ?? []) {
      const r = locate(b, ref);
      if (r) return r;
    }
    const l = locate(t.loopOver ?? [], ref);
    if (l) return l;
  }
  return null;
}

export function findTask(tasks: WorkflowTask[], ref: string): WorkflowTask | null {
  const loc = locate(tasks, ref);
  return loc ? loc.list[loc.index] : null;
}

/** Where to insert a new task. */
export type AddTarget =
  | { kind: "top" }
  | { kind: "case"; ref: string; key: string }
  | { kind: "default"; ref: string }
  | { kind: "branch"; ref: string; index: number }
  | { kind: "loop"; ref: string };

export function addTask(def: WorkflowDef, target: AddTarget, task: WorkflowTask): WorkflowDef {
  const d = structuredClone(def);
  if (target.kind === "top") {
    d.tasks.push(task);
    return d;
  }
  const parent = findTask(d.tasks, target.ref);
  if (!parent) return d;
  if (target.kind === "case") {
    parent.decisionCases ??= {};
    (parent.decisionCases[target.key] ??= []).push(task);
  } else if (target.kind === "default") {
    (parent.defaultCase ??= []).push(task);
  } else if (target.kind === "branch") {
    parent.forkTasks ??= [];
    (parent.forkTasks[target.index] ??= []).push(task);
  } else if (target.kind === "loop") {
    (parent.loopOver ??= []).push(task);
  }
  return d;
}

export function deleteTask(def: WorkflowDef, ref: string): WorkflowDef {
  const d = structuredClone(def);
  const loc = locate(d.tasks, ref);
  if (loc) loc.list.splice(loc.index, 1);
  return d;
}

export function updateTask(def: WorkflowDef, ref: string, patch: Partial<WorkflowTask>): WorkflowDef {
  const d = structuredClone(def);
  const t = findTask(d.tasks, ref);
  if (t) Object.assign(t, patch);
  return d;
}

export function moveTask(def: WorkflowDef, ref: string, dir: -1 | 1): WorkflowDef {
  const d = structuredClone(def);
  const loc = locate(d.tasks, ref);
  if (loc) {
    const j = loc.index + dir;
    if (j >= 0 && j < loc.list.length) {
      const [x] = loc.list.splice(loc.index, 1);
      loc.list.splice(j, 0, x);
    }
  }
  return d;
}

export function addSwitchCase(def: WorkflowDef, ref: string, key: string): WorkflowDef {
  const d = structuredClone(def);
  const t = findTask(d.tasks, ref);
  if (t) {
    t.decisionCases ??= {};
    t.decisionCases[key] ??= [];
  }
  return d;
}

export function removeSwitchCase(def: WorkflowDef, ref: string, key: string): WorkflowDef {
  const d = structuredClone(def);
  const t = findTask(d.tasks, ref);
  if (t?.decisionCases) delete t.decisionCases[key];
  return d;
}

export function addForkBranch(def: WorkflowDef, ref: string): WorkflowDef {
  const d = structuredClone(def);
  const t = findTask(d.tasks, ref);
  if (t) (t.forkTasks ??= []).push([]);
  return d;
}

export function removeForkBranch(def: WorkflowDef, ref: string, index: number): WorkflowDef {
  const d = structuredClone(def);
  const t = findTask(d.tasks, ref);
  if (t?.forkTasks) t.forkTasks.splice(index, 1);
  return d;
}

/** A reference name not already used in the definition. */
export function uniqueRef(def: WorkflowDef, base: string): string {
  const seen = new Set<string>();
  walkTasks(def.tasks, (t) => seen.add(t.taskReferenceName));
  if (!seen.has(base)) return base;
  let i = 2;
  while (seen.has(`${base}_${i}`)) i += 1;
  return `${base}_${i}`;
}
