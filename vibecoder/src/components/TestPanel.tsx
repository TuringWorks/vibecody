/**
 * TestPanel — Run project tests and display pass/fail results.
 *
 * Auto-detects the test framework from workspace files:
 * Cargo.toml → `cargo test`
 * package.json (with test script) → `npm test`
 * pytest.ini / pyproject.toml / setup.py → `pytest`
 * go.mod → `go test ./...`
 *
 * `run_tests` streams a `test:log` event per output line while the run happens.
 * The console below is the only thing that distinguishes a suite that is
 * compiling from one that has hung, so it stays on screen during the run and
 * after it — a long suite otherwise leaves the panel blank for minutes.
 */
import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { ChevronDown } from "lucide-react";
import { formatElapsed } from "../lib/duration";
import { FixWithAIButton } from "./FixWithAIButton";
import type { FixItem } from "../lib/fixWithAI";

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

/** Lines kept in the console. Older ones are dropped, and the header says so. */
const LOG_LIMIT = 2000;

/**
 * How much of a failure's output a fix request carries.
 *
 * A panicking test can print a megabyte; the useful part is the assertion and
 * the frames around it. The request says when it truncated, so nobody reads a
 * clipped stack as the whole story.
 */
const OUTPUT_IN_REQUEST = 1200;

/** One failing test as the shared chat hand-off carries it. */
function toFixItem(t: TestResult): FixItem {
 const output = t.output ?? "";
 const clipped = output.length > OUTPUT_IN_REQUEST;
 const shown = clipped ? output.slice(0, OUTPUT_IN_REQUEST) : output;
 return {
  severity: "failing test",
  title: t.name,
  message: output
   ? `Output:\n${shown}${clipped ? `\n… (${output.length - OUTPUT_IN_REQUEST} more characters not shown)` : ""}`
   : "The test failed and produced no output.",
 };
}

export function TestPanel({ workspacePath }: TestPanelProps) {
 const [running, setRunning] = useState(false);
 const [suspending, setSuspending] = useState(false);
 const [result, setResult] = useState<TestRunResult | null>(null);
 const [filter, setFilter] = useState<"all" | "failed" | "passed">("all");
 const [expanded, setExpanded] = useState<Set<string>>(new Set());
 const [customCmd, setCustomCmd] = useState("");
 const [liveLog, setLiveLog] = useState<string[]>([]);
 const [logCount, setLogCount] = useState(0);
 const [elapsedMs, setElapsedMs] = useState(0);
 const [framework, setFramework] = useState<string | null>(null);
 const unlistenRef = useRef<UnlistenFn | null>(null);
 const logBoxRef = useRef<HTMLDivElement>(null);
 // Auto-scroll only while the user is reading the tail; scrolling up to look
 // at an earlier failure must not be yanked back down by the next line.
 const pinnedRef = useRef(true);
 const taskIdRef = useRef(0);

 // One subscription for the panel's lifetime: re-subscribing per run raced the
 // first lines of the run against the `listen` promise.
 useEffect(() => {
  let disposed = false;
  listen<string>("test:log", (e) => {
   setLiveLog((prev) => [...prev.slice(-(LOG_LIMIT - 1)), e.payload]);
   setLogCount((n) => n + 1);
  }).then((un) => {
   if (disposed) { un(); return; }
   unlistenRef.current = un;
  });
  return () => {
   disposed = true;
   unlistenRef.current?.();
   unlistenRef.current = null;
  };
 }, []);

 useEffect(() => {
  if (!pinnedRef.current) return;
  const box = logBoxRef.current;
  // `scrollTo` is absent under jsdom.
  box?.scrollTo?.({ top: box.scrollHeight });
 }, [liveLog]);

 const onLogScroll = useCallback(() => {
  const box = logBoxRef.current;
  if (!box) return;
  pinnedRef.current = box.scrollHeight - box.scrollTop - box.clientHeight < 24;
 }, []);

 // Elapsed time is the panel's proof of life while a runner is silent
 // (cargo prints nothing for the whole compile).
 useEffect(() => {
  if (!running) return;
  const startedAt = Date.now();
  setElapsedMs(0);
  const id = setInterval(() => setElapsedMs(Date.now() - startedAt), 500);
  return () => clearInterval(id);
 }, [running]);

 // Detect framework on workspace change
 useEffect(() => {
  if (!workspacePath) { setFramework(null); return; }
  invoke<string>("detect_test_framework", { workspace: workspacePath })
   .then(setFramework)
   .catch(() => setFramework(null));
 }, [workspacePath]);

 /** Ask the backend to kill the run. The pending `run_tests` call then fails. */
 async function handleSuspend() {
  setSuspending(true);
  try {
   await invoke("stop_tests");
  } catch (e) {
   setLiveLog((prev) => [...prev, `Could not stop the run: ${e}`]);
   setSuspending(false);
  }
 }

 async function runTests() {
  if (!workspacePath) return;
  taskIdRef.current += 1;
  const thisId = taskIdRef.current;
  setRunning(true);
  setSuspending(false);
  setResult(null);
  setLiveLog([]);
  setLogCount(0);
  setExpanded(new Set());
  pinnedRef.current = true;

  try {
   const res = await invoke<TestRunResult>("run_tests", {
    workspace: workspacePath,
    command: customCmd.trim() || null,
   });
   if (taskIdRef.current !== thisId) return;
   setResult(res);
  } catch (e) {
   if (taskIdRef.current !== thisId) return;
   setResult({
    framework: framework || "unknown",
    passed: 0, failed: 1, ignored: 0, total: 1,
    duration_ms: 0,
    tests: [{ name: "Test run", status: "failed", duration_ms: null, output: String(e) }],
   });
  } finally {
   if (taskIdRef.current === thisId) {
    setRunning(false);
    setSuspending(false);
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
   <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>
    Open a workspace folder to run tests.
   </div>
  );
 }

 return (
  <div className="panel-container" style={{ fontSize: "var(--font-size-base)" }}>
   <div className="panel-header">
    <h3>Test Runner</h3>
    {framework && (
     <span style={{ fontSize: "var(--font-size-xs)", padding: "2px 8px", background: "color-mix(in srgb, var(--accent-blue) 20%, transparent)", color: "var(--text-info)", borderRadius: 3 }}>
      {framework}
     </span>
    )}
    {running ? (
     <button onClick={handleSuspend} disabled={suspending} className="panel-btn panel-btn-danger" style={{ marginLeft: "auto" }}>
      {suspending ? "Stopping…" : "Suspend"}
     </button>
    ) : (
     <button onClick={runTests} className="panel-btn panel-btn-primary" style={{ marginLeft: "auto" }}>Run Tests</button>
    )}
   </div>
   <div className="panel-body" style={{ gap: "12px" }}>
    {/* Custom command override */}
    <input
     type="text"
     value={customCmd}
     onChange={(e) => setCustomCmd(e.target.value)}
     placeholder={`Custom command (default: auto-detect${framework ? ` → ${framework}` : ""})`}
     className="panel-input panel-input-full"
     style={{ fontFamily: "var(--font-mono)" }}
    />

    {/* Live status: what is happening, for how long, how much it has said */}
    {running && (
     <div
      role="status"
      aria-live="polite"
      style={{ display: "flex", alignItems: "center", gap: 8, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}
     >
      <span
       aria-hidden="true"
       style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--accent-color)", animation: "pulse 1.4s ease-in-out infinite" }}
      />
      <span>
       {suspending ? "Stopping" : "Running"} · {formatElapsed(elapsedMs)} · {logCount} {logCount === 1 ? "line" : "lines"}
      </span>
      {logCount <= 1 && elapsedMs > 3000 && (
       <span style={{ color: "var(--text-secondary)" }}>— no output yet (the runner may still be compiling)</span>
      )}
     </div>
    )}

    {/* Summary bar */}
    {result && !running && (
     <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)", padding: "12px 12px", display: "flex", gap: 16, alignItems: "center" }}>
      {/* Pass-rate ring (simple colored bar) */}
      <div style={{ flex: 1 }}>
       <div style={{ height: 4, borderRadius: 2, background: "var(--bg-tertiary)", overflow: "hidden" }}>
        <div style={{ height: "100%", width: `${passRate}%`, background: result.failed > 0 ? "var(--error-color)" : "var(--success-color)", transition: "width 0.4s" }} />
       </div>
       <div style={{ marginTop: 4, display: "flex", gap: 12, fontSize: "var(--font-size-sm)" }}>
        <span style={{ color: "var(--text-success)" }}>✓ {result.passed}</span>
        {result.failed > 0 && <span style={{ color: "var(--text-danger)" }}>✗ {result.failed}</span>}
        {result.ignored > 0 && <span style={{ color: "var(--text-secondary)" }}>⊘ {result.ignored}</span>}
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
     <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
      {(["all", "failed", "passed"] as const).map((f) => (
       <button
        key={f}
        onClick={() => setFilter(f)}
        style={{
         padding: "2px 12px", fontSize: "var(--font-size-sm)", borderRadius: 3, cursor: "pointer",
         background: filter === f ? "var(--accent-blue)" : "var(--bg-secondary)",
         color: filter === f ? "var(--text-primary)" : "var(--text-secondary)",
         border: "1px solid var(--border-color)",
        }}
       >
        {f === "all" ? `All (${result.total})` : f === "failed" ? `Failed (${result.failed})` : `Passed (${result.passed})`}
       </button>
      ))}
      <span style={{ flex: 1 }} />
      <FixWithAIButton
       items={result.tests.filter((t) => t.status === "failed").map(toFixItem)}
       source="failing test"
       instructions={[
        "Read each failure's output before changing anything, and fix the code the test is about — do not edit the test to make it pass unless the test is itself wrong, and say so if it is.",
       ]}
       resetKey={result}
       label={`Fix all ${result.failed} with AI`}
       title="Write a fix request for the failing tests into the chat composer"
      />
     </div>
    )}

    {/* Test list */}
    {visibleTests.length > 0 && (
     <div style={{ flex: "0 1 auto", overflowY: "auto", display: "flex", flexDirection: "column", gap: 3 }}>
      {visibleTests.map((t) => (
       <div role="button" tabIndex={0}
        key={t.name}
        style={{
         borderRadius: "var(--radius-xs-plus)", padding: "4px 8px",
         background: t.status === "failed" ? "color-mix(in srgb, var(--accent-rose) 8%, transparent)" : "var(--bg-secondary)",
         border: `1px solid ${t.status === "failed" ? "rgba(243,139,168,0.3)" : "var(--border-color)"}`,
         cursor: t.output ? "pointer" : "default",
        }}
        onClick={() => t.output && toggleExpand(t.name)}
       >
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
         <span style={{
          fontSize: "var(--font-size-xs)", flexShrink: 0,
          color: t.status === "passed" ? "var(--success-color)" : t.status === "failed" ? "var(--error-color)" : t.status === "ignored" ? "var(--text-secondary)" : "var(--warning-color)",
         }}>
          {t.status === "passed" ? "✓" : t.status === "failed" ? "✗" : t.status === "ignored" ? "⊘" : "…"}
         </span>
         <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: "var(--font-size-sm)" }}>
          {t.name}
         </span>
         {t.duration_ms !== null && (
          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", flexShrink: 0 }}>
           {t.duration_ms}ms
          </span>
         )}
         {t.status === "failed" && (
          <FixWithAIButton
           items={[toFixItem(t)]}
           source="failing test"
           instructions={[
            "Read the failure output before changing anything, and fix the code the test is about — do not edit the test to make it pass unless the test is itself wrong, and say so if it is.",
           ]}
           resetKey={result}
           title="Write a fix request for this failure into the chat composer"
          />
         )}
         {t.output && (
          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", flexShrink: 0 }}>
           {expanded.has(t.name) ? "" : <ChevronDown size={10} />}
          </span>
         )}
        </div>
        {expanded.has(t.name) && t.output && (
         <pre style={{ margin: "4px 0 0 16px", fontSize: "var(--font-size-xs)", color: "var(--text-danger)", whiteSpace: "pre-wrap", wordBreak: "break-all", maxHeight: 200, overflowY: "auto" }}>
          {t.output}
         </pre>
        )}
       </div>
      ))}
     </div>
    )}

    {/* Console — kept after the run so a failure can be read in context */}
    {liveLog.length > 0 && (
     <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 120 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginBottom: 4 }}>
       <span>Console</span>
       <span>
        {logCount > LOG_LIMIT
         ? `last ${LOG_LIMIT} of ${logCount} lines`
         : `${logCount} ${logCount === 1 ? "line" : "lines"}`}
       </span>
      </div>
      <div
       ref={logBoxRef}
       onScroll={onLogScroll}
       data-testid="test-console"
       style={{
        flex: 1, minHeight: 0, overflowY: "auto",
        background: "var(--bg-secondary)", borderRadius: "var(--radius-xs-plus)",
        padding: "8px 8px", border: "1px solid var(--border-color)",
       }}
      >
       <pre style={{ margin: 0, fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)", lineHeight: 1.5, whiteSpace: "pre-wrap", wordBreak: "break-all", color: "var(--text-primary)" }}>
        {liveLog.join("\n")}
       </pre>
      </div>
     </div>
    )}

    {!running && !result && liveLog.length === 0 && (
     <div style={{ textAlign: "center", padding: "32px 16px", color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>
      Click Run Tests to start.
     </div>
    )}
   </div>
  </div>
 );
}
