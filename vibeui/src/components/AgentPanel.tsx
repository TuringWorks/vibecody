import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { flowContext } from "../utils/FlowContext";
import { runLinter, formatLintForAgent } from "../utils/LinterIntegration";
import { useToast } from "../hooks/useToast";
import { Toaster } from "./Toaster";

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

type AgentStatus = "idle" | "running" | "complete" | "error";

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
    const [expandedSteps, setExpandedSteps] = useState<Set<number>>(new Set());
    const [copiedStep, setCopiedStep] = useState<number | null>(null);
    const feedEndRef = useRef<HTMLDivElement>(null);

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
        const unlisteners: Array<() => void> = [];

        (async () => {
            unlisteners.push(
                await listen<string>("agent:chunk", (e) => {
                    setStreaming((prev) => prev + e.payload);
                })
            );
            unlisteners.push(
                await listen<AgentStep>("agent:step", (e) => {
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
                                    setSteps((prev) => [...prev, {
                                        step_num: step.step_num + 0.5,
                                        tool_name: "linter",
                                        tool_summary: `Auto-lint: ${filePath.split("/").pop() || "file"}`,
                                        output: msg,
                                        success: result.errors.length === 0,
                                        approved: true,
                                    }]);
                                }
                            });
                        }
                    }
                })
            );
            unlisteners.push(
                await listen<PendingCall>("agent:pending", (e) => {
                    setStreaming("");
                    setPending(e.payload);
                })
            );
            unlisteners.push(
                await listen<string>("agent:complete", (e) => {
                    setStreaming(e.payload);
                    setPending(null);
                    setStatus("complete");
                    // Record in Cascade flow
                    flowContext.add({
                        kind: "agent_complete",
                        summary: e.payload || "Agent task complete",
                        detail: "",
                    });
                })
            );
            unlisteners.push(
                await listen<string>("agent:error", (e) => {
                    setStreaming((prev) => (prev ? prev + "\n\n" : "") + "❌ " + e.payload);
                    setPending(null);
                    setStatus("error");
                })
            );
        })();

        return () => unlisteners.forEach((fn) => fn());
    }, []);

    const startAgent = async () => {
        if (!task.trim() || !provider) return;
        setSteps([]);
        setStreaming("");
        setPending(null);
        setStatus("running");

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

    const isRunning = status === "running";

    const statusLabel = status === "running" ? "Agent is running"
        : status === "complete" ? "Agent task complete"
        : status === "error" ? "Agent encountered an error"
        : "Agent idle";

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "8px" }}>
            <div className="sr-only" aria-live="polite">{statusLabel}</div>
            <div style={{ fontWeight: 600, fontSize: "14px" }}>🤖 Agent Mode</div>
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
                style={{
                    background: "var(--bg-tertiary)",
                    border: "1px solid var(--border-color)",
                    color: "var(--text-primary)",
                    borderRadius: "4px",
                    padding: "8px",
                    fontSize: "13px",
                    resize: "vertical",
                    fontFamily: "inherit",
                }}
            />

            <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                <select
                    value={approvalPolicy}
                    onChange={(e) => {
                        setApprovalPolicy(e.target.value);
                        setTurboMode(e.target.value === "full-auto");
                    }}
                    disabled={isRunning}
                    style={{
                        fontSize: "12px",
                        background: "var(--bg-tertiary)",
                        color: "var(--text-primary)",
                        border: "1px solid var(--border-color)",
                        borderRadius: "4px",
                        padding: "4px 6px",
                        flex: 1,
                    }}
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
                        background: turboMode ? "#fa0" : "var(--bg-tertiary)",
                        color: turboMode ? "#000" : "var(--text-secondary)",
                        border: `1px solid ${turboMode ? "#fa0" : "var(--border-color)"}`,
                        borderRadius: "4px",
                        cursor: isRunning ? "not-allowed" : "pointer",
                        fontWeight: turboMode ? 700 : 400,
                        transition: "all 0.15s",
                        whiteSpace: "nowrap",
                    }}
                >
                    ⚡ Turbo
                </button>

                <button
                    className="btn-primary"
                    onClick={startAgent}
                    disabled={!task.trim() || !provider || isRunning}
                    style={{ whiteSpace: "nowrap" }}
                >
                    {isRunning ? "⏳ Running…" : "▶ Run"}
                </button>

                {isRunning && (
                    <button
                        onClick={stopAgent}
                        style={{ whiteSpace: "nowrap", padding: "4px 10px", fontSize: "12px", background: "var(--text-danger, #ff4d4f)", color: "#fff", border: "none", borderRadius: "4px", cursor: "pointer" }}
                        title="Stop the agent"
                    >
                        ⏹ Stop
                    </button>
                )}

                {(status === "complete" || status === "error") && (
                    <button className="btn-secondary" onClick={reset} style={{ whiteSpace: "nowrap" }}>
                        ↺ Reset
                    </button>
                )}
            </div>

            {!provider && (
                <div style={{ fontSize: "12px", color: "#f4a", padding: "6px", background: "rgba(255,68,170,0.1)", borderRadius: "4px" }}>
                    ⚠️ Select an AI provider in the header first.
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
                    fontFamily: "monospace",
                    fontSize: "12px",
                    display: "flex",
                    flexDirection: "column",
                    gap: "6px",
                }}
            >
                {steps.length === 0 && !streaming && !pending && (
                    <div style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: "24px" }}>
                        {status === "idle"
                            ? "Enter a task above and click ▶ Run."
                            : "Agent initialising…"}
                    </div>
                )}

                {steps.map((step, i) => {
                    const isExpanded = expandedSteps.has(i);
                    const isTruncated = step.output.length > 600;
                    const displayOutput = isExpanded || !isTruncated
                        ? step.output
                        : step.output.slice(0, 600) + "\n…";
                    return (
                        <div
                            key={i}
                            style={{ borderBottom: "1px solid var(--border-color)", paddingBottom: "6px" }}
                        >
                            <div style={{ display: "flex", alignItems: "flex-start", gap: "6px" }}>
                                <div style={{ flex: 1, color: step.success ? "var(--accent-green, #4ec9b0)" : "var(--text-danger, #f44)", fontWeight: 500 }}>
                                    {step.success ? "✅" : "❌"} {step.tool_summary}
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
                                        style={{ flexShrink: 0, background: "none", border: "none", cursor: "pointer", fontSize: "10px", color: copiedStep === i ? "#3fb950" : "var(--text-secondary)", padding: "0 2px" }}
                                    >
                                        {copiedStep === i ? "✓" : "⎘"}
                                    </button>
                                )}
                            </div>
                            {step.output && (
                                <>
                                    <pre style={{ margin: "4px 0 0 16px", color: "var(--text-secondary)", whiteSpace: "pre-wrap", maxHeight: isExpanded ? "none" : "160px", overflowY: "auto", fontSize: "11px" }}>
                                        {displayOutput}
                                    </pre>
                                    {isTruncated && (
                                        <button
                                            onClick={() => setExpandedSteps(prev => {
                                                const next = new Set(prev);
                                                next.has(i) ? next.delete(i) : next.add(i);
                                                return next;
                                            })}
                                            style={{ marginLeft: "16px", fontSize: "10px", background: "none", border: "none", cursor: "pointer", color: "var(--accent-blue, #007acc)", padding: "2px 0" }}
                                        >
                                            {isExpanded ? "▲ Collapse" : "▼ Show all"}
                                        </button>
                                    )}
                                </>
                            )}
                        </div>
                    );
                })}

                {/* Streaming LLM text */}
                {streaming && (
                    <div style={{ color: "var(--text-primary)", whiteSpace: "pre-wrap" }}>
                        {streaming}
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

                {/* Approval prompt */}
                {pending && (
                    <div
                        style={{
                            background: "var(--bg-secondary)",
                            border: "1px solid var(--accent-blue, #007acc)",
                            borderRadius: "6px",
                            padding: "10px",
                        }}
                    >
                        <div style={{ fontWeight: 600, marginBottom: "4px" }}>
                            {pending.is_destructive ? "⚠️ Destructive action — approve?" : "🔧 Action — approve?"}
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
                                style={{ background: "#388e3c" }}
                            >
                                ✓ Approve
                            </button>
                            <button
                                className="btn-secondary"
                                onClick={reject}
                                style={{ background: "#c62828", color: "#fff" }}
                            >
                                ✗ Reject
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
