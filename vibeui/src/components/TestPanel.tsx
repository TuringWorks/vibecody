/**
 * TestPanel — Run project tests and display pass/fail results.
 *
 * Auto-detects the test framework from workspace files:
 *   Cargo.toml  → `cargo test`
 *   package.json (with test script) → `npm test`
 *   pytest.ini / pyproject.toml / setup.py → `pytest`
 *   go.mod → `go test ./...`
 *
 * Uses `run_tests` Tauri command which streams `test:event` events
 * to the frontend and returns a summary on completion.
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export interface TestResult {
    name: string;
    status: "passed" | "failed" | "ignored" | "running";
    duration_ms: number | null;
    output: string | null;
}

interface TestRunResult {
    framework: string;
    passed: number;
    failed: number;
    ignored: number;
    total: number;
    duration_ms: number;
    tests: TestResult[];
}

interface TestPanelProps {
    workspacePath: string | null;
}

export function TestPanel({ workspacePath }: TestPanelProps) {
    const [running, setRunning] = useState(false);
    const [result, setResult] = useState<TestRunResult | null>(null);
    const [filter, setFilter] = useState<"all" | "failed" | "passed">("all");
    const [expanded, setExpanded] = useState<Set<string>>(new Set());
    const [customCmd, setCustomCmd] = useState("");
    const [liveLog, setLiveLog] = useState<string[]>([]);
    const [framework, setFramework] = useState<string | null>(null);
    const unlistenRef = useRef<UnlistenFn | null>(null);
    const logEndRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        return () => {
            unlistenRef.current?.();
        };
    }, []);

    useEffect(() => {
        logEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [liveLog]);

    // Detect framework on workspace change
    useEffect(() => {
        if (!workspacePath) { setFramework(null); return; }
        invoke<string>("detect_test_framework", { workspace: workspacePath })
            .then(setFramework)
            .catch(() => setFramework(null));
    }, [workspacePath]);

    async function runTests() {
        if (!workspacePath) return;
        setRunning(true);
        setResult(null);
        setLiveLog([]);
        setExpanded(new Set());

        // Clean up any previous listener before subscribing
        unlistenRef.current?.();
        unlistenRef.current = null;
        const unlisten = await listen<string>("test:log", (e) => {
            setLiveLog((prev) => [...prev.slice(-199), e.payload]);
        });
        unlistenRef.current = unlisten;

        try {
            const res = await invoke<TestRunResult>("run_tests", {
                workspace: workspacePath,
                command: customCmd.trim() || null,
            });
            setResult(res);
        } catch (e) {
            setResult({
                framework: "unknown",
                passed: 0, failed: 1, ignored: 0, total: 1,
                duration_ms: 0,
                tests: [{ name: "Test run", status: "failed", duration_ms: null, output: String(e) }],
            });
        } finally {
            setRunning(false);
            // Only clean up if we still own the listener (a second runTests
            // call may have already replaced it with its own listener).
            if (unlistenRef.current === unlisten) {
                unlisten();
                unlistenRef.current = null;
            }
        }
    }

    function toggleExpand(name: string) {
        setExpanded((prev) => {
            const next = new Set(prev);
            if (next.has(name)) next.delete(name); else next.add(name);
            return next;
        });
    }

    const visibleTests = result?.tests.filter((t) => {
        if (filter === "failed") return t.status === "failed";
        if (filter === "passed") return t.status === "passed";
        return true;
    }) ?? [];

    const passRate = result && result.total > 0
        ? Math.round((result.passed / result.total) * 100)
        : 0;

    if (!workspacePath) {
        return (
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>
                Open a workspace folder to run tests.
            </div>
        );
    }

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "10px", fontFamily: "var(--font-mono, monospace)", fontSize: 12 }}>
            {/* Header */}
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontWeight: 700, fontSize: 14 }}>🧪 Test Runner</span>
                {framework && (
                    <span style={{ fontSize: 10, padding: "2px 6px", background: "rgba(137,180,250,0.2)", color: "#89b4fa", borderRadius: 3 }}>
                        {framework}
                    </span>
                )}
                <button
                    onClick={runTests}
                    disabled={running}
                    style={{
                        marginLeft: "auto", padding: "4px 12px", fontSize: 12,
                        background: running ? "var(--bg-tertiary)" : "#6366f1",
                        color: running ? "var(--text-secondary)" : "#fff",
                        border: "none", borderRadius: 4, cursor: running ? "not-allowed" : "pointer",
                        fontWeight: 600,
                    }}
                >
                    {running ? "Running…" : "▶ Run Tests"}
                </button>
            </div>

            {/* Custom command override */}
            <input
                type="text"
                value={customCmd}
                onChange={(e) => setCustomCmd(e.target.value)}
                placeholder={`Custom command (default: auto-detect${framework ? ` → ${framework}` : ""})`}
                style={{
                    padding: "5px 8px", fontSize: 11, fontFamily: "monospace",
                    background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
                    borderRadius: 4, color: "var(--text-primary)", outline: "none",
                }}
            />

            {/* Summary bar */}
            {result && !running && (
                <div style={{ background: "var(--bg-secondary)", borderRadius: 6, padding: "10px 12px", display: "flex", gap: 16, alignItems: "center" }}>
                    {/* Pass-rate ring (simple colored bar) */}
                    <div style={{ flex: 1 }}>
                        <div style={{ height: 4, borderRadius: 2, background: "var(--bg-tertiary)", overflow: "hidden" }}>
                            <div style={{ height: "100%", width: `${passRate}%`, background: result.failed > 0 ? "#f38ba8" : "#a6e3a1", transition: "width 0.4s" }} />
                        </div>
                        <div style={{ marginTop: 4, display: "flex", gap: 12, fontSize: 11 }}>
                            <span style={{ color: "#a6e3a1" }}>✓ {result.passed}</span>
                            {result.failed > 0 && <span style={{ color: "#f38ba8" }}>✗ {result.failed}</span>}
                            {result.ignored > 0 && <span style={{ color: "#a6adc8" }}>⊘ {result.ignored}</span>}
                            <span style={{ color: "var(--text-secondary)", marginLeft: "auto" }}>
                                {result.duration_ms < 1000
                                    ? `${result.duration_ms}ms`
                                    : `${(result.duration_ms / 1000).toFixed(1)}s`}
                            </span>
                        </div>
                    </div>
                </div>
            )}

            {/* Filter tabs */}
            {result && result.tests.length > 0 && (
                <div style={{ display: "flex", gap: 4 }}>
                    {(["all", "failed", "passed"] as const).map((f) => (
                        <button
                            key={f}
                            onClick={() => setFilter(f)}
                            style={{
                                padding: "2px 10px", fontSize: 11, borderRadius: 3, cursor: "pointer",
                                background: filter === f ? "var(--accent-blue, #6366f1)" : "var(--bg-secondary)",
                                color: filter === f ? "#fff" : "var(--text-secondary)",
                                border: "1px solid var(--border-color)",
                            }}
                        >
                            {f === "all" ? `All (${result.total})` : f === "failed" ? `Failed (${result.failed})` : `Passed (${result.passed})`}
                        </button>
                    ))}
                </div>
            )}

            {/* Test list */}
            <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 3 }}>
                {visibleTests.map((t) => (
                    <div
                        key={t.name}
                        style={{
                            borderRadius: 4, padding: "5px 8px",
                            background: t.status === "failed" ? "rgba(243,139,168,0.08)" : "var(--bg-secondary)",
                            border: `1px solid ${t.status === "failed" ? "rgba(243,139,168,0.3)" : "var(--border-color)"}`,
                            cursor: t.output ? "pointer" : "default",
                        }}
                        onClick={() => t.output && toggleExpand(t.name)}
                    >
                        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                            <span style={{
                                fontSize: 10, flexShrink: 0,
                                color: t.status === "passed" ? "#a6e3a1" : t.status === "failed" ? "#f38ba8" : t.status === "ignored" ? "#a6adc8" : "#f9e2af",
                            }}>
                                {t.status === "passed" ? "✓" : t.status === "failed" ? "✗" : t.status === "ignored" ? "⊘" : "…"}
                            </span>
                            <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11 }}>
                                {t.name}
                            </span>
                            {t.duration_ms !== null && (
                                <span style={{ fontSize: 10, color: "var(--text-secondary)", flexShrink: 0 }}>
                                    {t.duration_ms}ms
                                </span>
                            )}
                            {t.output && (
                                <span style={{ fontSize: 10, color: "var(--text-secondary)", flexShrink: 0 }}>
                                    {expanded.has(t.name) ? "▲" : "▼"}
                                </span>
                            )}
                        </div>
                        {expanded.has(t.name) && t.output && (
                            <pre style={{ margin: "4px 0 0 14px", fontSize: 10, color: "#f38ba8", whiteSpace: "pre-wrap", wordBreak: "break-all", maxHeight: 200, overflowY: "auto" }}>
                                {t.output}
                            </pre>
                        )}
                    </div>
                ))}

                {/* Live log during run */}
                {running && liveLog.length > 0 && (
                    <div style={{ background: "var(--bg-secondary)", borderRadius: 4, padding: "6px 8px", maxHeight: 160, overflowY: "auto", fontFamily: "monospace", fontSize: 10 }}>
                        {liveLog.map((line, i) => (
                            <div key={i} style={{ color: "var(--text-secondary)" }}>{line}</div>
                        ))}
                        <div ref={logEndRef} />
                    </div>
                )}

                {!running && !result && (
                    <div style={{ textAlign: "center", padding: "32px 16px", color: "var(--text-secondary)", fontSize: 12 }}>
                        Click ▶ Run Tests to start.
                    </div>
                )}
            </div>
        </div>
    );
}
