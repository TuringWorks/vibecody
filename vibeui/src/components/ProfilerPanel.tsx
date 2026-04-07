/**
 * ProfilerPanel — CPU/Memory Performance Profiler.
 *
 * Auto-detects profiling tool (cargo-flamegraph, clinic, py-spy, go pprof),
 * runs profiler, parses output into hotspot table sorted by self%.
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ProfileHotspot {
  function_name: string;
  file: string | null;
  self_pct: number;
  total_pct: number;
  samples: number;
}

interface ProfileResult {
  tool: string;
  hotspots: ProfileHotspot[];
  total_samples: number;
  duration_secs: number;
  raw_output: string;
}

interface ProfilerPanelProps {
  workspacePath: string | null;
}

const toolLabel: Record<string, string> = {
  "cargo-flamegraph": "Cargo Flamegraph",
  clinic: "Clinic (Node.js)",
  "py-spy": "py-spy (Python)",
  "go-pprof": "Go pprof",
};

const pctColor = (pct: number) => {
  if (pct >= 20) return "var(--error-color)";
  if (pct >= 10) return "var(--warning-color)";
  if (pct >= 5) return "var(--warning-color)";
  return "var(--success-color)";
};

export function ProfilerPanel({ workspacePath }: ProfilerPanelProps) {
  const [tool, setTool] = useState<string | null>(null);
  const [result, setResult] = useState<ProfileResult | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [target, setTarget] = useState("");
  const [showRaw, setShowRaw] = useState(false);
  const cancelRef = useRef(false);
  const taskIdRef = useRef(0);

  useEffect(() => {
    if (!workspacePath) return;
    invoke<string>("detect_profiler_tool", { workspace: workspacePath })
      .then(setTool)
      .catch(() => setTool(null));
  }, [workspacePath]);

  if (!workspacePath) {
    return (
      <div style={{ padding: 16, opacity: 0.6, textAlign: "center" }}>
        <p>Open a workspace folder to use the profiler.</p>
      </div>
    );
  }

  const handleSuspend = () => {
    cancelRef.current = true;
    setRunning(false);
    setError("Profiling suspended by user.");
  };

  const handleRun = async () => {
    if (!tool) return;
    cancelRef.current = false;
    taskIdRef.current += 1;
    const thisId = taskIdRef.current;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<ProfileResult>("run_profiler", {
        workspace: workspacePath,
        tool,
        target: target.trim() || null,
      });
      if (cancelRef.current || taskIdRef.current !== thisId) return;
      setResult(r);
    } catch (e: unknown) {
      if (cancelRef.current || taskIdRef.current !== thisId) return;
      setError(String(e));
    } finally {
      if (!cancelRef.current && taskIdRef.current === thisId) {
        setRunning(false);
      }
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, overflow: "hidden" }}>
    <div style={{ flex: 1, overflowY: "auto", padding: "12px 16px", display: "flex", flexDirection: "column", gap: 12 }}>
      {/* Tool badge */}
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <div style={{
          background: "var(--bg-secondary)", borderRadius: 6, padding: "6px 12px",
          border: "1px solid var(--border-color)", fontSize: 12, fontWeight: 600,
        }}>
          {tool ? toolLabel[tool] || tool : "No profiler detected"}
        </div>
        {tool && (
          <span style={{ fontSize: 11, opacity: 0.5, fontFamily: "var(--font-mono)" }}>{tool}</span>
        )}
      </div>

      {/* Target input + Run button */}
      <div style={{ display: "flex", gap: 8 }}>
        <input
          type="text"
          value={target}
          onChange={(e) => setTarget(e.target.value)}
          placeholder={tool === "cargo-flamegraph" ? "target binary (optional)" : tool === "py-spy" ? "script.py" : tool === "clinic" ? "server.js" : "target (optional)"}
          style={{
            flex: 1, padding: "8px 10px", fontSize: 12, fontFamily: "var(--font-mono)",
            background: "var(--bg-secondary)",
            border: "1px solid var(--border-color)", borderRadius: 4,
            color: "var(--text-primary)", outline: "none",
          }}
        />
        {running ? (
          <button
            onClick={handleSuspend}
            style={{
              padding: "8px 18px", fontSize: 13, fontWeight: 700,
              background: "var(--error-color)",
              color: "var(--btn-primary-fg)", border: "none", borderRadius: 6,
              cursor: "pointer",
              whiteSpace: "nowrap",
            }}
          >
            Suspend
          </button>
        ) : (
          <button
            onClick={handleRun}
            disabled={!tool}
            style={{
              padding: "8px 18px", fontSize: 13, fontWeight: 700,
              background: "var(--accent-color)",
              color: "var(--btn-primary-fg)", border: "none", borderRadius: 6,
              cursor: !tool ? "not-allowed" : "pointer",
              whiteSpace: "nowrap",
            }}
          >
            Profile
          </button>
        )}
      </div>

      {/* Error */}
      {error && (
        <div style={{ background: "color-mix(in srgb, var(--accent-rose) 15%, transparent)", border: "1px solid var(--error-color)", borderRadius: 6, padding: 8, fontSize: 11, color: "var(--error-color)" }}>
          {error}
        </div>
      )}

      {/* Results */}
      {result && (
        <>
          {/* Summary bar */}
          <div style={{
            background: "var(--bg-secondary)", borderRadius: 6, padding: 10,
            border: "1px solid var(--border-color)", display: "flex", gap: 16, fontSize: 12,
          }}>
            <span>Duration: <strong>{result.duration_secs.toFixed(1)}s</strong></span>
            <span>Samples: <strong>{result.total_samples.toLocaleString()}</strong></span>
            <span>Hotspots: <strong>{result.hotspots.length}</strong></span>
            {result.hotspots.length > 0 && (
              <span>Top: <strong style={{ color: pctColor(result.hotspots[0].self_pct) }}>
                {result.hotspots[0].function_name.split("::").pop()} ({result.hotspots[0].self_pct.toFixed(1)}%)
              </strong></span>
            )}
          </div>

          {/* Hotspot table */}
          {result.hotspots.length > 0 ? (
            <div style={{ flex: 1, overflowY: "auto" }}>
              {/* Header */}
              <div style={{
                display: "grid", gridTemplateColumns: "1fr 70px 70px 80px",
                gap: 4, padding: "6px 8px", fontSize: 11, fontWeight: 600,
                borderBottom: "1px solid var(--border-color)", opacity: 0.7,
              }}>
                <span>Function</span>
                <span style={{ textAlign: "right" }}>Self%</span>
                <span style={{ textAlign: "right" }}>Total%</span>
                <span style={{ textAlign: "right" }}>Samples</span>
              </div>
              {/* Rows */}
              {result.hotspots.slice(0, 50).map((h, i) => (
                <div
                  key={i}
                  style={{
                    display: "grid", gridTemplateColumns: "1fr 70px 70px 80px",
                    gap: 4, padding: "5px 8px", fontSize: 11,
                    borderBottom: "1px solid var(--border-color)",
                    background: i % 2 === 0 ? "transparent" : "rgba(255,255,255,0.02)",
                  }}
                >
                  <span style={{ fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                    title={h.function_name}
                  >
                    {h.function_name}
                  </span>
                  <span style={{ textAlign: "right", position: "relative" }}>
                    <span style={{
                      position: "absolute", left: 0, top: 0, bottom: 0,
                      width: `${Math.min(h.self_pct, 100)}%`,
                      background: pctColor(h.self_pct), opacity: 0.2, borderRadius: 2,
                    }} />
                    <span style={{ position: "relative", color: pctColor(h.self_pct), fontWeight: 600 }}>
                      {h.self_pct.toFixed(1)}%
                    </span>
                  </span>
                  <span style={{ textAlign: "right", opacity: 0.7 }}>{h.total_pct.toFixed(1)}%</span>
                  <span style={{ textAlign: "right", fontFamily: "var(--font-mono)", opacity: 0.6 }}>
                    {h.samples > 0 ? h.samples.toLocaleString() : "-"}
                  </span>
                </div>
              ))}
              {result.hotspots.length > 50 && (
                <div style={{ padding: 8, fontSize: 11, opacity: 0.5, textAlign: "center" }}>
                  Showing 50 of {result.hotspots.length} hotspots
                </div>
              )}
            </div>
          ) : (
            <div style={{ padding: 16, opacity: 0.5, fontSize: 12, textAlign: "center" }}>
              No structured hotspot data available. Check raw output below.
            </div>
          )}

          {/* Raw output toggle */}
          <div>
            <button
              onClick={() => setShowRaw(!showRaw)}
              style={{
                background: "none", border: "none", cursor: "pointer", fontSize: 11,
                color: "var(--accent-color)", padding: 0, textDecoration: "underline",
              }}
            >
              {showRaw ? "Hide raw output" : "Show raw output"}
            </button>
            {showRaw && (
              <pre style={{
                marginTop: 8, background: "var(--bg-secondary)", borderRadius: 6,
                padding: 10, fontSize: 10, fontFamily: "var(--font-mono)", maxHeight: 200,
                overflowY: "auto", whiteSpace: "pre-wrap", color: "var(--text-primary)",
                border: "1px solid var(--border-color)",
              }}>
                {result.raw_output || "(no output)"}
              </pre>
            )}
          </div>
        </>
      )}

      {/* Empty state when no results */}
      {!result && !running && !error && (
        <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", opacity: 0.4, fontSize: 12 }}>
          {tool ? "Click Profile to start profiling your application." : "No profiling tool detected for this workspace."}
        </div>
      )}
    </div>
    </div>
  );
}
