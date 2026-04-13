import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { flowContext } from "../utils/FlowContext";
import { runLinter, formatLintForAgent } from "../utils/LinterIntegration";
import { useToast } from "../hooks/useToast";
import { Toaster } from "./Toaster";
import { AgentUIRenderer, parseVibeUIBlocks, stripVibeUIBlocks } from "./AgentUIRenderer";
import type { VibeUIAction } from "./AgentUIRenderer";
import { Bot, GitBranch, Loader2, Square, Zap, ShieldCheck, ListOrdered, Play, RotateCcw, Check, X, Copy, ChevronDown, ChevronUp } from "lucide-react";

interface AgentStep {
 step_num: number;
 tool_name: string;
 tool_summary: string;
 output: string;
 success: boolean;
 approved: boolean;
}

interface PendingCall {
 name: string;
 summary: string;
 is_destructive: boolean;
}

type AgentStatus = "idle" | "running" | "complete" | "partial" | "error";

interface AgentPanelProps {
 provider: string;
 workspacePath?: string | null;
}

export function AgentPanel({ provider, workspacePath }: AgentPanelProps) {
 const { toasts, toast, dismiss } = useToast();
 const [task, setTask] = useState("");
 const [steps, setSteps] = useState<AgentStep[]>([]);
 const [streaming, setStreaming] = useState("");
 const [pending, setPending] = useState<PendingCall | null>(null);
 const [status, setStatus] = useState<AgentStatus>("idle");
 const [approvalPolicy, setApprovalPolicy] = useState("auto-edit");
 const [turboMode, setTurboMode] = useState(false);
 const [parallelMode, setParallelMode] = useState(false);
 const [worktreeIsolation, setWorktreeIsolation] = useState<"unknown" | "worktree" | "sequential">("unknown");
 const [expandedSteps, setExpandedSteps] = useState<Set<number>>(new Set());
 const [copiedStep, setCopiedStep] = useState<number | null>(null);
 const feedEndRef = useRef<HTMLDivElement>(null);

 // ── Streaming metrics ─────────────────────────────────────────────────────
 // Track tokens-per-second and time-to-first-token during LLM streaming.
 const streamStartMsRef = useRef<number | null>(null);
 const streamCharsRef = useRef<number>(0);
 const [streamMetrics, setStreamMetrics] = useState<{
 tokensPerSec: number;
 ttftMs: number | null;
 totalTokens: number;
 } | null>(null);

 // Turbo Mode: shortcut that sets approval to full-auto
 function toggleTurbo() {
 const next = !turboMode;
 setTurboMode(next);
 setApprovalPolicy(next ? "full-auto" : "auto-edit");
 }

 // Auto-scroll step feed
 useEffect(() => {
 feedEndRef.current?.scrollIntoView({ behavior: "smooth" });
 }, [steps, streaming, pending]);

 // Register Tauri event listeners
 useEffect(() => {
 let cancelled = false;
 const unlisteners: Array<() => void> = [];

 (async () => {
 const u1 = await listen<string>("agent:chunk", (e) => {
 const now = Date.now();
 const chunk = e.payload;
 setStreaming((prev) => prev + chunk);

 // Detect worktree isolation mode from backend status messages
 if (chunk.includes("using worktree isolation")) {
   setWorktreeIsolation("worktree");
 } else if (chunk.includes("running chunks sequentially")) {
   setWorktreeIsolation("sequential");
 }

 // Compute TTFT on first chunk only (before chars are accumulated)
 const isFirstChunk = streamCharsRef.current === 0;
 const ttftMs = isFirstChunk && streamStartMsRef.current
 ? now - streamStartMsRef.current
 : null;

 // Accumulate chars; estimate 1 token ≈ 4 chars
 streamCharsRef.current += chunk.length;
 const startTime = streamStartMsRef.current ?? now;
 const elapsedSec = (now - startTime) / 1000;
 const estimatedTokens = Math.round(streamCharsRef.current / 4);
 const tokensPerSec = elapsedSec > 0
 ? Math.round(estimatedTokens / elapsedSec)
 : 0;

 setStreamMetrics({
 tokensPerSec,
 ttftMs,
 totalTokens: estimatedTokens,
 });
 });
 if (cancelled) { u1(); return; }
 unlisteners.push(u1);

 const u2 = await listen<AgentStep>("agent:step", (e) => {
 const step = e.payload;
 setSteps((prev) => [...prev, step]);
 setStreaming("");
 setPending(null);
 // Record in Cascade flow
 flowContext.add({
 kind: "agent_step",
 summary: `${step.tool_name}: ${step.tool_summary}`,
 detail: step.output || "",
 });
 // Auto-lint after write_file steps
 if (step.tool_name === "write_file" && step.success) {
 const filePath = step.tool_summary.split("'")[1] || step.tool_summary;
 if (filePath) {
 runLinter(filePath).then((result) => {
 const msg = formatLintForAgent(result);
 if (msg) {
 const hasErrors = result.errors.length > 0;
 setSteps((prev) => [...prev, {
 step_num: step.step_num + 0.5,
 tool_name: "linter",
 tool_summary: hasErrors
 ? `Linter ERRORS - agent must fix before proceeding (${filePath.split("/").pop() || "file"})`
 : `Auto-lint OK: ${filePath.split("/").pop() || "file"}`,
 output: msg,
 success: !hasErrors,
 approved: true,
 }]);
 }
 }).catch((e: unknown) => console.error("Agent event error:", e));
 }
 }
 });
 if (cancelled) { u2(); return; }
 unlisteners.push(u2);

 const u3 = await listen<PendingCall>("agent:pending", (e) => {
 setStreaming("");
 setPending(e.payload);
 });
 if (cancelled) { u3(); return; }
 unlisteners.push(u3);

 const u4 = await listen<string>("agent:complete", (e) => {
 setStreaming(e.payload);
 setPending(null);
 setStatus("complete");
 // Record in Cascade flow
 flowContext.add({
 kind: "agent_complete",
 summary: e.payload || "Agent task complete",
 detail: "",
 });
 });
 if (cancelled) { u4(); return; }
 unlisteners.push(u4);

 const uPartial = await listen<{ summary: string; steps_completed: number; steps_planned: number; remaining_plan: string[] }>("agent:partial", (e) => {
 const { summary, steps_completed, steps_planned, remaining_plan } = e.payload;
 setStreaming(
   `⚠ Partial completion (${steps_completed}/${steps_planned} steps done)\n\n${summary}` +
   (remaining_plan.length > 0 ? `\n\nRemaining:\n${remaining_plan.map((s, i) => `  ${steps_completed + i + 1}. ${s}`).join("\n")}` : "")
 );
 setPending(null);
 setStatus("partial");
 flowContext.add({
   kind: "agent_partial",
   summary: `Partial: ${steps_completed}/${steps_planned} steps`,
   detail: remaining_plan.join(", "),
 });
 });
 if (cancelled) { uPartial(); return; }
 unlisteners.push(uPartial);

 const u5 = await listen<string>("agent:error", (e) => {
 setStreaming((prev) => (prev ? prev + "\n\n" : "") + "Error: " + e.payload);
 setPending(null);
 setStatus("error");
 });
 if (cancelled) { u5(); return; }
 unlisteners.push(u5);

 const u6 = await listen<{ error: string; attempt: number; max_attempts: number; backoff_ms: number }>("agent:retry", (e) => {
 const { error, attempt, max_attempts, backoff_ms } = e.payload;
 setStreaming((prev) =>
   (prev ? prev + "\n" : "") +
   `⟳ Retrying (${attempt + 1}/${max_attempts}) in ${(backoff_ms / 1000).toFixed(1)}s — ${error}`
 );
 });
 if (cancelled) { u6(); return; }
 unlisteners.push(u6);
 })();

 return () => {
 cancelled = true;
 unlisteners.forEach((fn) => fn());
 };
 }, []);

 const startAgent = async () => {
 if (!task.trim() || !provider) return;
 setSteps([]);
 setStreaming("");
 setPending(null);
 setStatus("running");
 setWorktreeIsolation("unknown");
 // Reset streaming metrics — record submit time for TTFT calculation
 streamStartMsRef.current = Date.now();
 streamCharsRef.current = 0;
 setStreamMetrics(null);

 try {
 if (parallelMode) {
   await invoke("start_parallel_agent_task", {
     task: task.trim(),
     provider,
     maxChunks: 4,
   });
 } else {
   await invoke("start_agent_task", {
     task: task.trim(),
     approvalPolicy,
     provider,
   });
 }
 } catch (e) {
 setStatus("error");
 setStreaming(String(e));
 }
 };

 const stopAgent = async () => {
 try {
 await invoke("stop_agent_task");
 } catch {
 // Best-effort: even if the command fails, reset local state
 } finally {
 setStatus("idle");
 setPending(null);
 setStreaming("");
 }
 };

 const approve = async () => {
 try {
 await invoke("respond_to_agent_approval", { approved: true });
 } catch (e) {
 toast.error(`Approval failed: ${e}`);
 }
 };

 const reject = async () => {
 try {
 await invoke("respond_to_agent_approval", { approved: false });
 setPending(null);
 } catch (e) {
 toast.error(`Rejection failed: ${e}`);
 }
 };

 const reset = () => {
 setStatus("idle");
 setSteps([]);
 setStreaming("");
 setPending(null);
 setTask("");
 };

 /** Retry after error — preserves completed steps and work. */
 const retry = async () => {
 if (!task.trim() || !provider) return;
 setStreaming("");
 setPending(null);
 setStatus("running");
 streamStartMsRef.current = Date.now();
 streamCharsRef.current = 0;
 setStreamMetrics(null);
 try {
   await invoke("start_agent_task", {
     task: task.trim(),
     approvalPolicy,
     provider,
   });
 } catch (e) {
   setStatus("error");
   setStreaming(String(e));
 }
 };

 // Handle interactive UI actions from AgentUIRenderer blocks
 const handleVibeUIAction = useCallback((action: VibeUIAction) => {
 const message = action.type === "button_click"
 ? `User selected: ${action.value}`
 : `User submitted form: ${JSON.stringify(action.value)}`;
 // Inject the action as a follow-up agent task
 invoke("start_agent_task", {
 task: message,
 approvalPolicy,
 provider,
 }).catch((e) => {
 toast.error(`Action failed: ${e}`);
 });
 setStatus("running");
 setStreaming("");
 setPending(null);
 }, [approvalPolicy, provider, toast]);

 /** Resume a partial/failed run from its checkpoint. */
 const [lastCheckpointId, setLastCheckpointId] = useState<string | null>(null);
 const resumeAgent = async () => {
   if (!lastCheckpointId) return;
   setStreaming("");
   setPending(null);
   setStatus("running");
   streamStartMsRef.current = Date.now();
   streamCharsRef.current = 0;
   setStreamMetrics(null);
   try {
     await invoke("resume_agent_task", { checkpointId: lastCheckpointId });
   } catch (e) {
     setStatus("error");
     setStreaming(String(e));
   }
 };

 // Track the latest checkpoint id from agent:partial events so Resume works
 useEffect(() => {
   let cancelled = false;
   (async () => {
     const u = await listen<{ summary: string; steps_completed: number; steps_planned: number; remaining_plan: string[] }>("agent:partial", () => {
       // The checkpoint id is the agent task id — extract from sub-agents
       invoke<{ id: string; status: string }[]>("list_sub_agents").then((agents) => {
         const partial = agents?.find((a: { status: string }) => a.status === "partial");
         if (partial) setLastCheckpointId(partial.id);
       }).catch(() => {});
     });
     if (cancelled) { u(); return; }
     return () => { cancelled = true; u(); };
   })();
 }, []);

 const isRunning = status === "running";

 const statusLabel = status === "running" ? "Agent is running"
 : status === "complete" ? "Agent task complete"
 : status === "partial" ? "Agent partially complete — can resume"
 : status === "error" ? "Agent encountered an error"
 : "Agent idle";

 return (
 <div className="panel-container" style={{ padding: "12px", gap: "8px" }}>
 <div className="sr-only" aria-live="polite">{statusLabel}</div>
 <div style={{ fontWeight: 600, fontSize: "14px", display: "flex", alignItems: "center", gap: 6 }}><Bot size={16} strokeWidth={1.5} />Agent Mode</div>
 <p style={{ fontSize: "12px", color: "var(--text-secondary)", margin: 0 }}>
 Describe a task — the agent plans and executes it autonomously using file, search, and bash tools.
 </p>

 <textarea
 value={task}
 onChange={(e) => setTask(e.target.value)}
 onKeyDown={(e) => {
 if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
 e.preventDefault();
 startAgent();
 }
 }}
 placeholder={`e.g. Add a /health endpoint to src/server.ts\n\n(⌘Enter to run)`}
 rows={4}
 disabled={isRunning}
 className="panel-input panel-textarea panel-input-full"
 style={{ resize: "vertical", fontFamily: "inherit" }}
 />

 <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
 <select
 value={approvalPolicy}
 onChange={(e) => {
 setApprovalPolicy(e.target.value);
 setTurboMode(e.target.value === "full-auto");
 }}
 disabled={isRunning}
 className="panel-select"
 style={{ flex: 1 }}
 >
 <option value="suggest">Suggest — approve each step</option>
 <option value="auto-edit">Auto-Edit — auto files, approve bash</option>
 <option value="full-auto">Full Auto — no prompts</option>
 </select>

 {/* Turbo Mode toggle */}
 <button
 onClick={toggleTurbo}
 disabled={isRunning}
 title={turboMode ? "Turbo Mode ON — click to disable full-auto" : "Turbo Mode OFF — click to enable full-auto (no approval prompts)"}
 style={{
 padding: "4px 8px",
 fontSize: "13px",
 background: turboMode ? "var(--warning-color)" : "var(--bg-tertiary)",
 color: turboMode ? "var(--bg-primary)" : "var(--text-secondary)",
 border: `1px solid ${turboMode ? "var(--warning-color)" : "var(--border-color)"}`,
 borderRadius: "4px",
 cursor: isRunning ? "not-allowed" : "pointer",
 fontWeight: turboMode ? 700 : 400,
 transition: "all 0.15s",
 whiteSpace: "nowrap",
 }}
 >
 <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><Zap size={14} strokeWidth={1.5} />Turbo</span>
 </button>

 {/* Parallel Mode toggle */}
 <button
 onClick={() => setParallelMode(!parallelMode)}
 disabled={isRunning}
 title={parallelMode ? "Parallel Mode ON — task will be split into chunks and run concurrently" : "Parallel Mode OFF — single agent executes sequentially"}
 style={{
 padding: "4px 8px",
 fontSize: "13px",
 background: parallelMode ? "var(--info-color, #6c8cff)" : "var(--bg-tertiary)",
 color: parallelMode ? "var(--bg-primary)" : "var(--text-secondary)",
 border: `1px solid ${parallelMode ? "var(--info-color, #6c8cff)" : "var(--border-color)"}`,
 borderRadius: "4px",
 cursor: isRunning ? "not-allowed" : "pointer",
 fontWeight: parallelMode ? 700 : 400,
 transition: "all 0.15s",
 whiteSpace: "nowrap",
 }}
 >
 <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><GitBranch size={14} strokeWidth={1.5} />Parallel</span>
 </button>

 <button
 className="btn-primary"
 onClick={startAgent}
 disabled={!task.trim() || !provider || isRunning}
 style={{ whiteSpace: "nowrap" }}
 >
 {isRunning ? <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><Loader2 size={14} strokeWidth={1.5} className="spin" />Running...</span> : <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><span style={{ fontSize: "12px" }}>&#9654;</span>Run</span>}
 </button>

 {isRunning && (
 <button
 onClick={stopAgent}
 className="panel-btn panel-btn-danger panel-btn-sm"
 style={{ whiteSpace: "nowrap" }}
 title="Stop the agent"
 >
 <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><Square size={14} strokeWidth={1.5} />Stop</span>
 </button>
 )}

 {status === "error" && (
 <button
   onClick={retry}
   className="panel-btn panel-btn-primary panel-btn-sm"
   style={{ whiteSpace: "nowrap" }}
   title="Retry — keeps completed steps"
 >
   ⟳ Retry
 </button>
 )}
 {status === "partial" && lastCheckpointId && (
 <button
   onClick={resumeAgent}
   className="panel-btn panel-btn-primary panel-btn-sm"
   style={{ whiteSpace: "nowrap" }}
   title="Resume from last checkpoint — continues remaining plan steps"
 >
   <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><Play size={11} /> Resume</span>
 </button>
 )}
 {(status === "complete" || status === "error" || status === "partial") && (
 <button className="btn-secondary" onClick={reset} style={{ whiteSpace: "nowrap" }}>
 <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><RotateCcw size={11} /> Reset</span>
 </button>
 )}
 </div>

 {/* Worktree isolation badge — shown when parallel mode is active */}
 {parallelMode && isRunning && worktreeIsolation !== "unknown" && (
 <div style={{
   display: "flex",
   alignItems: "center",
   gap: 6,
   padding: "4px 8px",
   borderRadius: 4,
   fontSize: 11,
   fontWeight: 600,
   background: worktreeIsolation === "worktree"
     ? "color-mix(in srgb, var(--accent-green) 12%, transparent)"
     : "color-mix(in srgb, var(--warning-color) 12%, transparent)",
   color: worktreeIsolation === "worktree"
     ? "var(--accent-green)"
     : "var(--warning-color)",
   border: `1px solid ${worktreeIsolation === "worktree" ? "color-mix(in srgb, var(--accent-green) 25%, transparent)" : "color-mix(in srgb, var(--warning-color) 25%, transparent)"}`,
 }}>
   {worktreeIsolation === "worktree" ? (
     <><ShieldCheck size={13} strokeWidth={2} /> Worktree Isolated — each chunk runs on its own git branch</>
   ) : (
     <><ListOrdered size={13} strokeWidth={2} /> Sequential Mode — non-git project, chunks run one at a time</>
   )}
 </div>
 )}

 {/* Show isolation result after completion */}
 {parallelMode && (status === "complete" || status === "partial") && worktreeIsolation !== "unknown" && (
 <div style={{
   display: "flex",
   alignItems: "center",
   gap: 6,
   padding: "4px 8px",
   borderRadius: 4,
   fontSize: 11,
   fontWeight: 500,
   background: "var(--bg-tertiary)",
   color: "var(--text-secondary)",
 }}>
   {worktreeIsolation === "worktree" ? (
     <><ShieldCheck size={13} strokeWidth={1.5} /> Ran with worktree isolation — branches merged back</>
   ) : (
     <><ListOrdered size={13} strokeWidth={1.5} /> Ran sequentially — non-git project</>
   )}
 </div>
 )}

 {!provider && (
 <div style={{ fontSize: "12px", color: "var(--warning-color)", padding: "6px", background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", borderRadius: "4px" }}>
 Select an AI provider in the header first.
 </div>
 )}

 {/* Step feed */}
 <div
 style={{
 flex: 1,
 overflowY: "auto",
 background: "var(--bg-tertiary)",
 borderRadius: "6px",
 padding: "8px",
 fontFamily: "var(--font-mono)",
 fontSize: "12px",
 display: "flex",
 flexDirection: "column",
 gap: "6px",
 }}
 >
 {steps.length === 0 && !streaming && !pending && (
 <div style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: "24px" }}>
 {status === "idle"
 ? "Enter a task above and click Run."
 : "Agent initialising…"}
 </div>
 )}

 {steps.map((step, i) => {
 const isExpanded = expandedSteps.has(i);
 const vibeBlocks = step.output ? parseVibeUIBlocks(step.output) : [];
 const textOutput = vibeBlocks.length > 0 ? stripVibeUIBlocks(step.output) : step.output;
 const isTruncated = textOutput.length > 600;
 const displayOutput = isExpanded || !isTruncated
 ? textOutput
 : textOutput.slice(0, 600) + "\n…";
 return (
 <div
 key={i}
 style={{ borderBottom: "1px solid var(--border-color)", paddingBottom: "6px" }}
 >
 <div style={{ display: "flex", alignItems: "flex-start", gap: "6px" }}>
 <div style={{ flex: 1, color: step.success ? "var(--accent-green)" : "var(--text-danger)", fontWeight: 500 }}>
 {step.success ? "" : ""} {step.tool_summary}
 </div>
 {step.output && (
 <button
 onClick={() => {
 navigator.clipboard.writeText(step.output).then(() => {
 setCopiedStep(i);
 setTimeout(() => setCopiedStep(null), 1500);
 }).catch(() => {});
 }}
 title="Copy step output"
 style={{ flexShrink: 0, background: "none", border: "none", cursor: "pointer", fontSize: "10px", color: copiedStep === i ? "var(--success-color)" : "var(--text-secondary)", padding: "0 2px" }}
 >
 {copiedStep === i ? <Check size={10} /> : <Copy size={10} />}
 </button>
 )}
 </div>
 {textOutput && (
 <>
 <pre style={{ margin: "4px 0 0 16px", color: "var(--text-secondary)", whiteSpace: "pre-wrap", maxHeight: isExpanded ? "none" : "160px", overflowY: "auto", fontSize: "11px" }}>
 {displayOutput}
 </pre>
 {isTruncated && (
 <button
 onClick={() => setExpandedSteps(prev => {
 const next = new Set(prev);
 if (next.has(i)) next.delete(i); else next.add(i);
 return next;
 })}
 style={{ marginLeft: "16px", fontSize: "10px", background: "none", border: "none", cursor: "pointer", color: "var(--accent-color)", padding: "2px 0" }}
 >
 <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>{isExpanded ? <><ChevronUp size={10} /> Collapse</> : <><ChevronDown size={10} /> Show all</>}</span>
 </button>
 )}
 </>
 )}
 {vibeBlocks.length > 0 && (
 <div style={{ marginLeft: 16 }}>
 <AgentUIRenderer blocks={vibeBlocks} onAction={handleVibeUIAction} />
 </div>
 )}
 </div>
 );
 })}

 {/* Streaming LLM text + interactive UI blocks */}
 {streaming && (() => {
 const streamBlocks = parseVibeUIBlocks(streaming);
 const streamText = streamBlocks.length > 0 ? stripVibeUIBlocks(streaming) : streaming;
 return (
 <div
 className="panel-card"
 style={{ borderLeft: "3px solid var(--accent-color)", marginBottom: 8 }}
 >
 {/* Metrics header row */}
 <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
 <Loader2 size={12} style={{ animation: "spin 1s linear infinite", flexShrink: 0 }} />
 <span style={{ fontSize: 11, color: "var(--text-secondary)", fontVariantNumeric: "tabular-nums" }}>
 Thinking…{streamMetrics
 ? ` ${streamMetrics.tokensPerSec} tok/s · ${streamMetrics.totalTokens} tokens${streamMetrics.ttftMs !== null ? ` · ${streamMetrics.ttftMs}ms TTFT` : ""}`
 : ""}
 </span>
 </div>

 {/* Streaming text body */}
 {streamText && (
 <div
 style={{
 fontSize: 12,
 color: "var(--text-primary)",
 fontFamily: "var(--font-mono)",
 whiteSpace: "pre-wrap",
 maxHeight: 200,
 overflowY: "auto",
 }}
 >
 {streamText}
 {isRunning && (
 <span
 style={{
 display: "inline-block",
 width: "2px",
 height: "1em",
 background: "currentColor",
 verticalAlign: "text-bottom",
 animation: "blink 1s step-end infinite",
 }}
 />
 )}
 </div>
 )}

 {/* Interactive UI blocks embedded in stream */}
 {streamBlocks.length > 0 && (
 <AgentUIRenderer blocks={streamBlocks} onAction={handleVibeUIAction} />
 )}
 </div>
 );
 })()}

 {/* Approval prompt */}
 {pending && (
 <div
 style={{
 background: "var(--bg-secondary)",
 border: "1px solid var(--accent-color)",
 borderRadius: "6px",
 padding: "10px",
 }}
 >
 <div style={{ fontWeight: 600, marginBottom: "4px" }}>
 {pending.is_destructive ? "Destructive action — approve?" : "Action — approve?"}
 </div>
 <code
 style={{
 display: "block",
 background: "var(--bg-tertiary)",
 padding: "6px",
 borderRadius: "4px",
 marginBottom: "8px",
 fontSize: "12px",
 whiteSpace: "pre-wrap",
 }}
 >
 {pending.summary}
 </code>
 <div style={{ display: "flex", gap: "8px" }}>
 <button
 className="btn-primary"
 onClick={approve}
 style={{ background: "var(--success-color)" }}
 >
 <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><Check size={12} /> Approve</span>
 </button>
 <button
 className="btn-secondary"
 onClick={reject}
 style={{ background: "var(--error-color)", color: "var(--text-primary)" }}
 >
 <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}><X size={12} /> Reject</span>
 </button>
 </div>
 </div>
 )}

 <div ref={feedEndRef} />
 </div>

 <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
 {steps.length > 0 && `${steps.length} step${steps.length !== 1 ? "s" : ""} completed`}
 {workspacePath && ` · ${workspacePath.split("/").pop()}`}
 </div>
 <Toaster toasts={toasts} onDismiss={dismiss} />
 </div>
 );
}
