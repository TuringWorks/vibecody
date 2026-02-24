import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
    const [task, setTask] = useState("");
    const [steps, setSteps] = useState<AgentStep[]>([]);
    const [streaming, setStreaming] = useState("");
    const [pending, setPending] = useState<PendingCall | null>(null);
    const [status, setStatus] = useState<AgentStatus>("idle");
    const [approvalPolicy, setApprovalPolicy] = useState("auto-edit");
    const feedEndRef = useRef<HTMLDivElement>(null);

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
                    setSteps((prev) => [...prev, e.payload]);
                    setStreaming("");
                    setPending(null);
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

    const approve = async () => {
        try {
            await invoke("respond_to_agent_approval", { approved: true });
        } catch (e) {
            console.error("Approval failed:", e);
        }
    };

    const reject = async () => {
        try {
            await invoke("respond_to_agent_approval", { approved: false });
            setPending(null);
        } catch (e) {
            console.error("Rejection failed:", e);
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

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "8px" }}>
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
                    onChange={(e) => setApprovalPolicy(e.target.value)}
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

                <button
                    className="btn-primary"
                    onClick={startAgent}
                    disabled={!task.trim() || !provider || isRunning}
                    style={{ whiteSpace: "nowrap" }}
                >
                    {isRunning ? "⏳ Running…" : "▶ Run"}
                </button>

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

                {steps.map((step, i) => (
                    <div
                        key={i}
                        style={{
                            borderBottom: "1px solid var(--border-color)",
                            paddingBottom: "6px",
                        }}
                    >
                        <div
                            style={{
                                color: step.success
                                    ? "var(--accent-green, #4ec9b0)"
                                    : "var(--text-danger, #f44)",
                                fontWeight: 500,
                            }}
                        >
                            {step.success ? "✅" : "❌"} {step.tool_summary}
                        </div>
                        {step.output && (
                            <pre
                                style={{
                                    margin: "4px 0 0 16px",
                                    color: "var(--text-secondary)",
                                    whiteSpace: "pre-wrap",
                                    maxHeight: "160px",
                                    overflowY: "auto",
                                    fontSize: "11px",
                                }}
                            >
                                {step.output.length > 600
                                    ? step.output.slice(0, 600) + "\n…"
                                    : step.output}
                            </pre>
                        )}
                    </div>
                ))}

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
        </div>
    );
}
