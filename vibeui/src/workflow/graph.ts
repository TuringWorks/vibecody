// Compile the nested Conductor DSL into a flat React Flow graph (nodes + edges)
// for rendering and editing, and lay it out with dagre. This is a *projection*:
// the WorkflowDef stays the source of truth (edits go through operations.ts).

import Dagre from "@dagrejs/dagre";
import type { Edge, Node } from "@xyflow/react";
import type { WorkflowDef, WorkflowTask } from "./dsl";

export interface TaskNodeData {
  ref: string;
  label: string;
  type: string;
  status?: string;
  iterations?: number;
  [key: string]: unknown;
}

export type FlowNode = Node<TaskNodeData, "task">;
export type FlowEdge = Edge;

interface Acc {
  nodes: FlowNode[];
  edges: FlowEdge[];
  seen: Set<string>;
}

interface Sub {
  entries: string[];
  exits: string[];
}

function addNode(acc: Acc, task: WorkflowTask): void {
  acc.nodes.push({
    id: task.taskReferenceName,
    type: "task",
    position: { x: 0, y: 0 },
    data: { ref: task.taskReferenceName, label: task.name, type: task.type ?? "SIMPLE" },
  });
}

function connect(acc: Acc, from: string, to: string, label?: string): void {
  if (from === to) return;
  const id = `${from}->${to}`;
  if (acc.seen.has(id)) return;
  acc.seen.add(id);
  acc.edges.push({ id, source: from, target: to, label, type: "smoothstep" });
}

function dedupe(xs: string[]): string[] {
  return Array.from(new Set(xs));
}

function buildList(acc: Acc, tasks: WorkflowTask[]): Sub {
  if (!tasks.length) return { entries: [], exits: [] };
  let firstEntries: string[] = [];
  let prevExits: string[] = [];
  tasks.forEach((t, i) => {
    const sub = buildTask(acc, t);
    if (i === 0) firstEntries = sub.entries;
    else for (const pe of prevExits) for (const en of sub.entries) connect(acc, pe, en);
    prevExits = sub.exits;
  });
  return { entries: firstEntries, exits: prevExits };
}

function buildTask(acc: Acc, t: WorkflowTask): Sub {
  addNode(acc, t);
  const id = t.taskReferenceName;
  const type = t.type ?? "SIMPLE";

  if (type === "SWITCH") {
    const exits: string[] = [];
    for (const [caseKey, sub] of Object.entries(t.decisionCases ?? {})) {
      const b = buildList(acc, sub);
      if (b.entries.length) {
        for (const en of b.entries) connect(acc, id, en, caseKey);
        exits.push(...b.exits);
      } else exits.push(id);
    }
    const def = buildList(acc, t.defaultCase ?? []);
    if (def.entries.length) {
      for (const en of def.entries) connect(acc, id, en, "default");
      exits.push(...def.exits);
    } else exits.push(id);
    return { entries: [id], exits: dedupe(exits) };
  }

  if (type === "FORK_JOIN") {
    const exits: string[] = [];
    for (const branch of t.forkTasks ?? []) {
      const b = buildList(acc, branch);
      if (b.entries.length) {
        for (const en of b.entries) connect(acc, id, en);
        exits.push(...b.exits);
      } else exits.push(id);
    }
    return { entries: [id], exits: dedupe(exits) };
  }

  if (type === "DO_WHILE") {
    const body = buildList(acc, t.loopOver ?? []);
    if (body.entries.length) {
      for (const en of body.entries) connect(acc, id, en, "loop");
      for (const ex of body.exits) connect(acc, ex, id, "↺");
    }
    return { entries: [id], exits: [id] };
  }

  if (type === "TERMINATE") return { entries: [id], exits: [] };

  return { entries: [id], exits: [id] };
}

/** Directions dagre understands. */
export type LayoutDir = "TB" | "LR";

/** Lay out a graph with dagre, honoring any user position overrides. */
export function layoutGraph(
  nodes: FlowNode[],
  edges: FlowEdge[],
  dir: LayoutDir,
  positions?: Record<string, { x: number; y: number }>,
): FlowNode[] {
  const W = 190;
  const H = 60;
  const g = new Dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: dir, nodesep: 45, ranksep: 65, marginx: 20, marginy: 20 });
  for (const n of nodes) g.setNode(n.id, { width: W, height: H });
  for (const e of edges) g.setEdge(e.source, e.target);
  Dagre.layout(g);
  return nodes.map((n) => {
    const override = positions?.[n.id];
    if (override) return { ...n, position: override };
    const p = g.node(n.id);
    return { ...n, position: { x: (p?.x ?? 0) - W / 2, y: (p?.y ?? 0) - H / 2 } };
  });
}

/** Build a laid-out graph for a definition (design mode). */
export function defToGraph(
  def: WorkflowDef,
  dir: LayoutDir = "TB",
  positions?: Record<string, { x: number; y: number }>,
): { nodes: FlowNode[]; edges: FlowEdge[] } {
  const acc: Acc = { nodes: [], edges: [], seen: new Set() };
  buildList(acc, def.tasks ?? []);
  return { nodes: layoutGraph(acc.nodes, acc.edges, dir, positions), edges: acc.edges };
}

/** Aggregate a run's (possibly instanced) task statuses down to base references. */
export function statusByRef(
  runTasks: Array<{ referenceName: string; status: string }>,
): Map<string, { status: string; iterations: number }> {
  const m = new Map<string, { status: string; iterations: number }>();
  for (const t of runTasks) {
    const instanced = t.referenceName.includes("__");
    const base = instanced ? t.referenceName.split("__")[0] : t.referenceName;
    const prev = m.get(base);
    m.set(base, {
      status: t.status,
      iterations: (prev?.iterations ?? 0) + (instanced ? 1 : 0),
    });
  }
  return m;
}

/** Build a laid-out graph for a run, coloring nodes by their (aggregated) status. */
export function runToGraph(
  def: WorkflowDef,
  runTasks: Array<{ referenceName: string; status: string }>,
  dir: LayoutDir = "TB",
): { nodes: FlowNode[]; edges: FlowEdge[] } {
  const { nodes, edges } = defToGraph(def, dir);
  const statuses = statusByRef(runTasks);
  const withStatus = nodes.map((n) => {
    const s = statuses.get(n.id);
    return s ? { ...n, data: { ...n.data, status: s.status, iterations: s.iterations } } : n;
  });
  return { nodes: withStatus, edges };
}
