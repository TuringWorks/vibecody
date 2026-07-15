import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { WorkflowDef } from "../workflow/dsl";
import { WorkflowDesigner } from "./WorkflowDesigner";
import { WorkflowRunGraph } from "./WorkflowRunGraph";

// Fluxo workflow interface: a full visual **Designer** (editable React Flow canvas
// + JSON authoring) and a **Runs** monitor (live DAG + task table + human signals),
// both over the daemon's `/fluxo/*` API via the `fluxo_*` Tauri commands.

interface TaskExec {
  taskId: string;
  referenceName: string;
  taskType: string;
  taskName: string;
  status: string;
  output?: unknown;
}

interface RunDetail {
  workflowId: string;
  workflowName: string;
  workflowVersion: number;
  status: string;
  output?: unknown;
  tasks: TaskExec[];
  reasonForIncompletion?: string;
}

const TERMINAL = new Set(["COMPLETED", "FAILED", "TERMINATED", "TIMED_OUT"]);

function statusColor(status: string): string {
  if (status === "COMPLETED") return "var(--success-color)";
  if (status === "FAILED" || status === "TIMED_OUT" || status === "TERMINATED") return "var(--text-danger)";
  if (status === "RUNNING" || status === "IN_PROGRESS" || status === "SCHEDULED") return "var(--accent-blue)";
  return "var(--text-secondary)";
}

export function WorkflowsPanel() {
  const [tab, setTab] = useState<"designer" | "runs">("designer");
  const [runs, setRuns] = useState<RunDetail[]>([]);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [runDetail, setRunDetail] = useState<RunDetail | null>(null);
  const [runDef, setRunDef] = useState<WorkflowDef | null>(null);
  const [runView, setRunView] = useState<"graph" | "table">("graph");
  const [signalRef, setSignalRef] = useState<string>("");
  const [signalOutput, setSignalOutput] = useState<string>("{}");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<boolean>(false);

  const run = useCallback(async <T,>(fn: () => Promise<T>): Promise<T | null> => {
    setBusy(true);
    setError(null);
    try {
      return await fn();
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      setBusy(false);
    }
  }, []);

  const loadRuns = useCallback(async () => {
    const res = await run(() => invoke<{ runs: RunDetail[] }>("fluxo_list_runs", { status: null }));
    if (res) setRuns(res.runs ?? []);
  }, [run]);

  const openRun = useCallback(
    async (id: string) => {
      setTab("runs");
      setSelectedRunId(id);
      const detail = await run(() => invoke<RunDetail>("fluxo_get_run", { workflowId: id }));
      if (!detail) return;
      setRunDetail(detail);
      const def = await run(() =>
        invoke<WorkflowDef>("fluxo_get_workflow", { name: detail.workflowName, version: detail.workflowVersion }),
      );
      setRunDef(def);
    },
    [run],
  );

  useEffect(() => {
    if (tab === "runs") void loadRuns();
  }, [tab, loadRuns]);

  // Poll the open run until it is terminal.
  useEffect(() => {
    if (!selectedRunId || (runDetail && TERMINAL.has(runDetail.status))) return;
    const t = setInterval(() => {
      void invoke<RunDetail>("fluxo_get_run", { workflowId: selectedRunId })
        .then((r) => setRunDetail(r))
        .catch(() => undefined);
    }, 1500);
    return () => clearInterval(t);
  }, [selectedRunId, runDetail]);

  const control = useCallback(
    async (cmd: "fluxo_pause_run" | "fluxo_resume_run" | "fluxo_terminate_run") => {
      if (!selectedRunId) return;
      await run(() => invoke(cmd, { workflowId: selectedRunId, reason: null }));
      await openRun(selectedRunId);
      await loadRuns();
    },
    [selectedRunId, run, openRun, loadRuns],
  );

  const doSignal = useCallback(async () => {
    if (!selectedRunId || !signalRef) return;
    let output: unknown;
    try {
      output = JSON.parse(signalOutput || "{}");
    } catch (e) {
      setError(`signal output is not valid JSON: ${String(e)}`);
      return;
    }
    await run(() => invoke("fluxo_signal", { workflowId: selectedRunId, referenceName: signalRef, output }));
    setSignalRef("");
    await openRun(selectedRunId);
  }, [selectedRunId, signalRef, signalOutput, run, openRun]);

  const waitingTasks = useMemo(
    () => (runDetail?.tasks ?? []).filter((t) => t.taskType === "HUMAN" || t.taskType === "WAIT"),
    [runDetail],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", fontSize: "var(--font-size-sm)", color: "var(--text-primary)" }}>
      <div style={{ display: "flex", gap: 4, padding: 6, borderBottom: "1px solid var(--border-color)" }}>
        <button style={tabBtn(tab === "designer")} onClick={() => setTab("designer")}>Designer</button>
        <button style={tabBtn(tab === "runs")} onClick={() => setTab("runs")}>Runs</button>
      </div>

      {tab === "designer" ? (
        <div style={{ flex: 1, minHeight: 0 }}>
          <WorkflowDesigner onRun={(id) => void openRun(id)} />
        </div>
      ) : (
        <div style={{ flex: 1, minHeight: 0, display: "flex" }}>
          {/* Runs list */}
          <div style={{ width: 260, borderRight: "1px solid var(--border-color)", overflowY: "auto", padding: 8 }}>
            <div style={{ display: "flex", alignItems: "center", marginBottom: 4 }}>
              <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)" }}>RUNS ({runs.length})</span>
              <div style={{ flex: 1 }} />
              <button style={btn(false)} onClick={loadRuns}>↻</button>
            </div>
            {runs.map((r) => (
              <div
                key={r.workflowId}
                onClick={() => openRun(r.workflowId)}
                style={{ display: "flex", justifyContent: "space-between", gap: 8, padding: "3px 6px", borderRadius: "var(--radius-xs-plus)", cursor: "pointer", background: r.workflowId === selectedRunId ? "var(--accent-bg)" : "transparent" }}
              >
                <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.workflowName}</span>
                <span style={{ color: statusColor(r.status) }}>{r.status}</span>
              </div>
            ))}
          </div>

          {/* Run detail */}
          <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", minHeight: 0 }}>
            {error && <div style={{ padding: 8, color: "var(--text-danger)", borderBottom: "1px solid var(--border-color)" }}>{error}</div>}
            {!runDetail ? (
              <div style={{ padding: 16, color: "var(--text-secondary)" }}>Select a run, or design + run a workflow.</div>
            ) : (
              <>
                <div style={{ display: "flex", alignItems: "center", gap: 10, padding: 8, borderBottom: "1px solid var(--border-color)" }}>
                  <strong>{runDetail.workflowName}</strong>
                  <span style={{ color: "var(--text-secondary)" }}>v{runDetail.workflowVersion}</span>
                  <span style={{ color: statusColor(runDetail.status), fontWeight: 600 }}>{runDetail.status}</span>
                  <div style={{ flex: 1 }} />
                  <button style={btn(false)} onClick={() => setRunView(runView === "graph" ? "table" : "graph")}>{runView === "graph" ? "Table" : "Graph"}</button>
                  <button style={btn(false)} disabled={busy} onClick={() => openRun(runDetail.workflowId)}>Refresh</button>
                  <button style={btn(false)} disabled={busy} onClick={() => control("fluxo_pause_run")}>Pause</button>
                  <button style={btn(false)} disabled={busy} onClick={() => control("fluxo_resume_run")}>Resume</button>
                  <button style={btn(false)} disabled={busy} onClick={() => control("fluxo_terminate_run")}>Terminate</button>
                </div>

                {waitingTasks.length > 0 && (
                  <div style={{ padding: 8, borderBottom: "1px solid var(--border-color)", display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
                    <span style={{ color: "var(--text-secondary)" }}>Waiting:</span>
                    {waitingTasks.map((t) => (
                      <button key={t.taskId} style={btn(false)} onClick={() => setSignalRef(t.referenceName)}>{t.referenceName}</button>
                    ))}
                    <input style={{ ...inputStyle, width: 130 }} placeholder="referenceName" value={signalRef} onChange={(e) => setSignalRef(e.target.value)} />
                    <input style={{ ...inputStyle, width: 130 }} placeholder="output JSON" value={signalOutput} onChange={(e) => setSignalOutput(e.target.value)} />
                    <button style={btn(true)} disabled={busy || !signalRef} onClick={doSignal}>Signal</button>
                  </div>
                )}

                <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
                  {runView === "graph" && runDef ? (
                    <WorkflowRunGraph def={runDef} tasks={runDetail.tasks} />
                  ) : (
                    <table style={{ width: "100%", borderCollapse: "collapse" }}>
                      <thead>
                        <tr style={{ textAlign: "left", color: "var(--text-secondary)" }}>
                          <th style={th}>Task</th><th style={th}>Type</th><th style={th}>Status</th>
                        </tr>
                      </thead>
                      <tbody>
                        {runDetail.tasks.map((t) => (
                          <tr key={t.taskId} style={{ borderTop: "1px solid var(--border-color)" }}>
                            <td style={td}>{t.referenceName}</td>
                            <td style={{ ...td, color: "var(--text-secondary)" }}>{t.taskType}</td>
                            <td style={{ ...td, color: statusColor(t.status) }}>{t.status}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>

                {runDetail.output != null && (
                  <div style={{ padding: 8, borderTop: "1px solid var(--border-color)" }}>
                    <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", marginBottom: 4 }}>OUTPUT</div>
                    <pre style={{ margin: 0, padding: 8, background: "var(--bg-tertiary)", borderRadius: "var(--radius-sm)", overflowX: "auto", fontSize: "var(--font-size-xs)" }}>{JSON.stringify(runDetail.output, null, 2)}</pre>
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  padding: "4px 6px",
  background: "var(--bg-tertiary)",
  border: "1px solid var(--border-color)",
  borderRadius: "var(--radius-xs-plus)",
  color: "var(--text-primary)",
  fontSize: "var(--font-size-sm)",
};

const th: React.CSSProperties = { padding: "4px 8px", fontWeight: 500, fontSize: "var(--font-size-xs)" };
const td: React.CSSProperties = { padding: "4px 8px" };

function tabBtn(active: boolean): React.CSSProperties {
  return {
    padding: "4px 14px",
    background: active ? "var(--accent-color)" : "var(--bg-tertiary)",
    color: active ? "var(--btn-primary-fg, #fff)" : "var(--text-primary)",
    border: "1px solid var(--border-color)",
    borderRadius: "var(--radius-xs-plus)",
    cursor: "pointer",
    fontSize: "var(--font-size-sm)",
  };
}

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
