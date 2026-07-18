// Fluxo workflow designer: an editable React Flow canvas (Option B) over the
// nested Conductor DSL, plus a raw JSON authoring surface (Option A). The
// WorkflowDef is the single source of truth; the canvas is a projection and all
// structural edits go through workflow/operations.ts.

import {
  Background,
  Controls,
  MiniMap,
  Panel,
  ReactFlow,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  blankWorkflow,
  colorForType,
  isContainer,
  TASK_TYPES,
  validate,
  type TaskType,
  type WorkflowDef,
  type WorkflowTask,
} from "../workflow/dsl";
import { defToGraph, type FlowEdge, type FlowNode, type LayoutDir } from "../workflow/graph";
import { nodeTypes } from "../workflow/nodes";
import {
  addForkBranch,
  addSwitchCase,
  addTask,
  deleteTask,
  findTask,
  moveTask,
  removeForkBranch,
  removeSwitchCase,
  uniqueRef,
  updateTask,
  type AddTarget,
} from "../workflow/operations";

function targetLabel(t: AddTarget): string {
  switch (t.kind) {
    case "top":
      return "top level";
    case "case":
      return `case '${t.key}' of ${t.ref}`;
    case "default":
      return `default of ${t.ref}`;
    case "branch":
      return `branch ${t.index} of ${t.ref}`;
    case "loop":
      return `loop body of ${t.ref}`;
  }
}

export function WorkflowDesigner({
  onRun,
  initialDef,
}: {
  onRun?: (workflowId: string) => void;
  initialDef?: WorkflowDef;
}) {
  const [def, setDef] = useState<WorkflowDef>(() => initialDef ?? blankWorkflow());
  const [selectedRef, setSelectedRef] = useState<string | null>(null);
  const [addTarget, setAddTarget] = useState<AddTarget>({ kind: "top" });
  const [dir, setDir] = useState<LayoutDir>("TB");
  const [view, setView] = useState<"canvas" | "json">("canvas");
  const [jsonText, setJsonText] = useState<string>("");
  const [msg, setMsg] = useState<string | null>(null);

  const [nodes, setNodes, onNodesChange] = useNodesState<FlowNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<FlowEdge>([]);
  const positionsRef = useRef<Record<string, { x: number; y: number }>>({});

  // Recompute the canvas whenever the structure or direction changes.
  useEffect(() => {
    const g = defToGraph(def, dir, positionsRef.current);
    setNodes(g.nodes);
    setEdges(g.edges);
  }, [def, dir, setNodes, setEdges]);

  const selected = selectedRef ? findTask(def.tasks, selectedRef) : null;
  const patch = useCallback(
    (partial: Partial<WorkflowTask>) => {
      if (selectedRef) setDef((d) => updateTask(d, selectedRef, partial));
    },
    [selectedRef],
  );

  const addOfType = useCallback(
    (type: TaskType) => {
      const meta = TASK_TYPES.find((m) => m.type === type);
      if (!meta) return;
      const ref = uniqueRef(def, type.toLowerCase());
      setDef((d) => addTask(d, addTarget, meta.template(ref)));
      setSelectedRef(ref);
    },
    [def, addTarget],
  );

  const autoLayout = useCallback(() => {
    positionsRef.current = {};
    const g = defToGraph(def, dir);
    setNodes(g.nodes);
    setEdges(g.edges);
  }, [def, dir, setNodes, setEdges]);

  const doRegister = useCallback(async (): Promise<boolean> => {
    const errs = validate(def);
    if (errs.length) {
      setMsg(`invalid: ${errs.join("; ")}`);
      return false;
    }
    try {
      await invoke("fluxo_register", { definition: def });
      setMsg(`registered ${def.name} v${def.version ?? 1}`);
      return true;
    } catch (e) {
      setMsg(String(e));
      return false;
    }
  }, [def]);

  const doRun = useCallback(async () => {
    if (!(await doRegister())) return;
    try {
      const res = await invoke<{ workflowId: string }>("fluxo_execute", {
        name: def.name,
        input: {},
        version: null,
        correlationId: null,
      });
      if (res?.workflowId) onRun?.(res.workflowId);
    } catch (e) {
      setMsg(String(e));
    }
  }, [def.name, doRegister, onRun]);

  const enterJson = useCallback(() => {
    setJsonText(JSON.stringify(def, null, 2));
    setView("json");
  }, [def]);

  const applyJson = useCallback(() => {
    try {
      const parsed = JSON.parse(jsonText) as WorkflowDef;
      positionsRef.current = {};
      setDef(parsed);
      setSelectedRef(null);
      setMsg("applied JSON");
      setView("canvas");
    } catch (e) {
      setMsg(`JSON error: ${String(e)}`);
    }
  }, [jsonText]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", fontSize: "var(--font-size-sm)", color: "var(--text-primary)" }}>
      {/* Toolbar */}
      <div style={{ display: "flex", gap: 6, alignItems: "center", padding: 6, borderBottom: "1px solid var(--border-color)" }}>
        <input value={def.name} onChange={(e) => setDef({ ...def, name: e.target.value })} style={{ ...inputStyle, width: 160 }} placeholder="workflow name" />
        <input type="number" value={def.version ?? 1} onChange={(e) => setDef({ ...def, version: Number(e.target.value) || 1 })} style={{ ...inputStyle, width: 60 }} title="version" />
        <button style={btn(false)} onClick={() => { positionsRef.current = {}; setDef(blankWorkflow()); setSelectedRef(null); }}>New</button>
        <button style={btn(false)} onClick={autoLayout}>Auto-layout</button>
        <button style={btn(false)} onClick={() => setDir(dir === "TB" ? "LR" : "TB")}>{dir === "TB" ? "↕" : "↔"}</button>
        <button style={btn(false)} onClick={() => (view === "json" ? setView("canvas") : enterJson())}>{view === "json" ? "Canvas" : "JSON"}</button>
        <div style={{ flex: 1 }} />
        <button style={btn(false)} onClick={() => setMsg(validate(def).join("; ") || "valid ✓")}>Validate</button>
        <button style={btn(false)} onClick={doRegister}>Register</button>
        <button style={btn(true)} onClick={doRun}>Run ▶</button>
      </div>
      {msg && <div style={{ padding: "4px 8px", color: msg.startsWith("invalid") || msg.includes("error") ? "var(--text-danger)" : "var(--text-secondary)", borderBottom: "1px solid var(--border-color)" }}>{msg}</div>}

      {view === "json" ? (
        <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column", padding: 8, gap: 6 }}>
          <textarea value={jsonText} onChange={(e) => setJsonText(e.target.value)} spellCheck={false} style={{ ...inputStyle, flex: 1, fontFamily: "var(--font-mono, monospace)", resize: "none" }} />
          <div><button style={btn(true)} onClick={applyJson}>Apply JSON → Canvas</button></div>
        </div>
      ) : (
        <div style={{ flex: 1, minHeight: 0, display: "flex" }}>
          {/* Palette */}
          <div style={{ width: 150, borderRight: "1px solid var(--border-color)", overflowY: "auto", padding: 6 }}>
            <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", marginBottom: 4 }}>ADD TO: {targetLabel(addTarget)}</div>
            {addTarget.kind !== "top" && (
              <button style={{ ...btn(false), width: "100%", marginBottom: 6 }} onClick={() => setAddTarget({ kind: "top" })}>↩ top level</button>
            )}
            {TASK_TYPES.map((m) => (
              <button key={m.type} onClick={() => addOfType(m.type)} style={{ ...btn(false), width: "100%", marginBottom: 4, borderLeft: `3px solid ${m.color}`, textAlign: "left" }}>
                {m.label}
              </button>
            ))}
          </div>

          {/* Canvas */}
          <div style={{ flex: 1, minWidth: 0 }}>
            <ReactFlow
              nodes={nodes}
              edges={edges}
              nodeTypes={nodeTypes}
              onNodesChange={onNodesChange}
              onEdgesChange={onEdgesChange}
              onNodeClick={(_, node) => setSelectedRef(node.id)}
              onNodeDragStop={(_, node) => {
                positionsRef.current[node.id] = node.position;
              }}
              onPaneClick={() => setSelectedRef(null)}
              fitView
              proOptions={{ hideAttribution: true }}
            >
              <Background />
              <Controls />
              <MiniMap pannable zoomable />
              <Panel position="top-left">
                <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)" }}>{nodes.length} tasks</span>
              </Panel>
            </ReactFlow>
          </div>

          {/* Inspector */}
          <div style={{ width: 310, borderLeft: "1px solid var(--border-color)", overflowY: "auto", padding: 8 }}>
            {!selected ? (
              <div style={{ color: "var(--text-secondary)" }}>Select a task, or add one from the palette.</div>
            ) : (
              <Inspector
                def={def}
                task={selected}
                onPatch={patch}
                onDelete={() => { setDef((d) => deleteTask(d, selected.taskReferenceName)); setSelectedRef(null); }}
                onMove={(dirn) => setDef((d) => moveTask(d, selected.taskReferenceName, dirn))}
                onRename={(r) => { patch({ taskReferenceName: r }); setSelectedRef(r); }}
                onAddCase={(k) => setDef((d) => addSwitchCase(d, selected.taskReferenceName, k))}
                onRemoveCase={(k) => setDef((d) => removeSwitchCase(d, selected.taskReferenceName, k))}
                onAddBranch={() => setDef((d) => addForkBranch(d, selected.taskReferenceName))}
                onRemoveBranch={(i) => setDef((d) => removeForkBranch(d, selected.taskReferenceName, i))}
                onSetTarget={setAddTarget}
              />
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function Inspector(props: {
  def: WorkflowDef;
  task: WorkflowTask;
  onPatch: (p: Partial<WorkflowTask>) => void;
  onDelete: () => void;
  onMove: (dir: -1 | 1) => void;
  onRename: (ref: string) => void;
  onAddCase: (key: string) => void;
  onRemoveCase: (key: string) => void;
  onAddBranch: () => void;
  onRemoveBranch: (index: number) => void;
  onSetTarget: (t: AddTarget) => void;
}) {
  const { task, onPatch } = props;
  const ref = task.taskReferenceName;
  const [ipText, setIpText] = useState("{}");
  const [ipErr, setIpErr] = useState(false);
  const [caseKey, setCaseKey] = useState("");

  useEffect(() => {
    setIpText(JSON.stringify(task.inputParameters ?? {}, null, 2));
    setIpErr(false);
  }, [ref]); // eslint-disable-line react-hooks/exhaustive-deps

  const onIp = (text: string) => {
    setIpText(text);
    try {
      onPatch({ inputParameters: JSON.parse(text) });
      setIpErr(false);
    } catch {
      setIpErr(true);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ width: 8, height: 8, borderRadius: 8, background: colorForType(task.type) }} />
        <strong>{task.type ?? "SIMPLE"}</strong>
        <div style={{ flex: 1 }} />
        <button style={btn(false)} onClick={() => props.onMove(-1)} title="up">↑</button>
        <button style={btn(false)} onClick={() => props.onMove(1)} title="down">↓</button>
        <button style={{ ...btn(false), color: "var(--text-danger)" }} onClick={props.onDelete}>Delete</button>
      </div>

      <Field label="Name"><input style={inputStyle} value={task.name} onChange={(e) => onPatch({ name: e.target.value })} /></Field>
      <Field label="Reference"><input style={inputStyle} value={ref} onChange={(e) => props.onRename(e.target.value)} /></Field>
      <Field label="Type">
        <select style={inputStyle} value={task.type ?? "SIMPLE"} onChange={(e) => onPatch({ type: e.target.value as TaskType })}>
          {TASK_TYPES.map((m) => <option key={m.type} value={m.type}>{m.type}</option>)}
        </select>
      </Field>
      <label style={{ display: "flex", gap: 6, alignItems: "center", color: "var(--text-secondary)" }}>
        <input type="checkbox" checked={!!task.optional} onChange={(e) => onPatch({ optional: e.target.checked })} /> optional
      </label>

      <Field label={`Input parameters${ipErr ? " (invalid JSON)" : ""}`}>
        <textarea style={{ ...inputStyle, height: 90, fontFamily: "var(--font-mono, monospace)", borderColor: ipErr ? "var(--text-danger)" : "var(--border-color)" }} value={ipText} onChange={(e) => onIp(e.target.value)} spellCheck={false} />
      </Field>

      {task.type === "SWITCH" && (
        <div style={sectionStyle}>
          <Field label="Evaluator type"><input style={inputStyle} value={task.evaluatorType ?? "value-param"} onChange={(e) => onPatch({ evaluatorType: e.target.value })} /></Field>
          <Field label="Expression (case key / ${…})"><input style={inputStyle} value={task.expression ?? ""} onChange={(e) => onPatch({ expression: e.target.value })} /></Field>
          <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", margin: "4px 0" }}>CASES</div>
          {Object.keys(task.decisionCases ?? {}).map((k) => (
            <div key={k} style={rowBtns}>
              <button style={{ ...btn(false), flex: 1, textAlign: "left" }} onClick={() => props.onSetTarget({ kind: "case", ref, key: k })}>＋ case '{k}'</button>
              <button style={btn(false)} onClick={() => props.onRemoveCase(k)}>✕</button>
            </div>
          ))}
          <div style={rowBtns}>
            <input style={{ ...inputStyle, flex: 1 }} placeholder="new case key" value={caseKey} onChange={(e) => setCaseKey(e.target.value)} />
            <button style={btn(false)} disabled={!caseKey} onClick={() => { props.onAddCase(caseKey); setCaseKey(""); }}>Add</button>
          </div>
          <button style={{ ...btn(false), width: "100%", marginTop: 4 }} onClick={() => props.onSetTarget({ kind: "default", ref })}>＋ default case</button>
        </div>
      )}

      {task.type === "FORK_JOIN" && (
        <div style={sectionStyle}>
          <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", marginBottom: 4 }}>BRANCHES</div>
          {(task.forkTasks ?? []).map((_, i) => (
            <div key={i} style={rowBtns}>
              <button style={{ ...btn(false), flex: 1, textAlign: "left" }} onClick={() => props.onSetTarget({ kind: "branch", ref, index: i })}>＋ branch {i}</button>
              <button style={btn(false)} onClick={() => props.onRemoveBranch(i)}>✕</button>
            </div>
          ))}
          <button style={{ ...btn(false), width: "100%" }} onClick={props.onAddBranch}>Add branch</button>
        </div>
      )}

      {task.type === "DO_WHILE" && (
        <div style={sectionStyle}>
          <Field label="Loop condition"><input style={inputStyle} value={task.loopCondition ?? ""} onChange={(e) => onPatch({ loopCondition: e.target.value })} placeholder="iteration < 3" /></Field>
          <button style={{ ...btn(false), width: "100%" }} onClick={() => props.onSetTarget({ kind: "loop", ref })}>＋ add to loop body</button>
        </div>
      )}

      {task.type === "JOIN" && (
        <Field label="Join on (comma-separated refs)">
          <input style={inputStyle} value={(task.joinOn ?? []).join(", ")} onChange={(e) => onPatch({ joinOn: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} />
        </Field>
      )}

      {task.type === "SUB_WORKFLOW" && (
        <div style={sectionStyle}>
          <Field label="Sub-workflow name"><input style={inputStyle} value={task.subWorkflowParam?.name ?? ""} onChange={(e) => onPatch({ subWorkflowParam: { ...task.subWorkflowParam, name: e.target.value } })} /></Field>
        </div>
      )}

      {isContainer(task.type) ? null : (
        <div style={sectionStyle}>
          <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", marginBottom: 4 }}>RELIABILITY</div>
          <div style={rowBtns}>
            <Field label="retryCount"><input style={inputStyle} type="number" value={task.retryCount ?? 0} onChange={(e) => onPatch({ retryCount: Number(e.target.value) || 0 })} /></Field>
            <Field label="timeoutSeconds"><input style={inputStyle} type="number" value={task.timeoutSeconds ?? 0} onChange={(e) => onPatch({ timeoutSeconds: Number(e.target.value) || undefined })} /></Field>
          </div>
        </div>
      )}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 3, flex: 1 }}>
      <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)" }}>{label}</span>
      {children}
    </label>
  );
}

const inputStyle: React.CSSProperties = {
  padding: "4px 6px",
  background: "var(--bg-tertiary)",
  border: "1px solid var(--border-color)",
  borderRadius: "var(--radius-xs-plus)",
  color: "var(--text-primary)",
  fontSize: "var(--font-size-sm)",
  width: "100%",
  boxSizing: "border-box",
};

const sectionStyle: React.CSSProperties = {
  border: "1px solid var(--border-color)",
  borderRadius: "var(--radius-sm)",
  padding: 6,
  display: "flex",
  flexDirection: "column",
  gap: 4,
};

const rowBtns: React.CSSProperties = { display: "flex", gap: 4, alignItems: "flex-end" };

function btn(primary: boolean): React.CSSProperties {
  return {
    padding: "3px 8px",
    background: primary ? "var(--accent-color)" : "var(--bg-tertiary)",
    color: primary ? "var(--btn-primary-fg, #fff)" : "var(--text-primary)",
    border: "1px solid var(--border-color)",
    borderRadius: "var(--radius-xs-plus)",
    cursor: "pointer",
    fontSize: "var(--font-size-sm)",
  };
}
