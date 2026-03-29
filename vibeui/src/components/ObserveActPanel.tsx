import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type SubTab = "setup" | "monitor" | "history" | "safety";

const card: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, border: "1px solid var(--border-color)" };
const label: React.CSSProperties = { fontSize: 12, color: "var(--text-secondary)", marginBottom: 4, display: "block" };
const input: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, boxSizing: "border-box" as const };
const btn: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "none", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", cursor: "pointer", fontSize: 12, fontWeight: 600 };

interface Step {
  num: number;
  reasoning: string;
  actions: string[];
  verified: boolean;
  durationMs: number;
}

interface ObserveActConfig {
  mode: "cautious" | "autonomous" | "restricted";
  maxSteps: number;
  interval: number;
  maxActionsPerStep: number;
  rateLimitMs: number;
  maxConsecutiveFailures: number;
  forbiddenKeyCombos: string;
  forbiddenScreenRegions: string;
  verifyAfterAction: boolean;
}

export function ObserveActPanel() {
  const [tab, setTab] = useState<SubTab>("setup");
  const [task, setTask] = useState("");
  const [mode, setMode] = useState<"cautious" | "autonomous" | "restricted">("cautious");
  const [maxSteps, setMaxSteps] = useState(50);
  const [interval, setInterval_] = useState(2000);
  const [status, setStatus] = useState<"idle" | "running" | "completed" | "failed">("idle");
  const [steps, setSteps] = useState<Step[]>([]);
  const [config, setConfig] = useState<ObserveActConfig | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      try {
        const [stepsData, configData] = await Promise.all([
          invoke<Step[]>("get_observeact_steps"),
          invoke<ObserveActConfig>("get_observeact_config"),
        ]);
        setSteps(stepsData);
        setConfig(configData);
        if (configData) {
          setMode(configData.mode);
          setMaxSteps(configData.maxSteps);
          setInterval_(configData.interval);
        }
      } catch (err) {
        console.error("Failed to load observe-act data:", err);
      } finally {
        setLoading(false);
      }
    };
    loadData();
  }, []);

  const handleSaveConfig = async (newConfig: ObserveActConfig) => {
    setConfig(newConfig);
    try {
      await invoke("save_observeact_config", { config: newConfig });
    } catch (err) {
      console.error("Failed to save observe-act config:", err);
    }
  };

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 16, fontSize: 13, color: "var(--text-primary)" }}>
      <div style={{ display: "flex", gap: 2, borderBottom: "1px solid var(--border-color)", padding: "0 16px", flexShrink: 0 }}>
        {(["setup", "monitor", "history", "safety"] as const).map(t => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: "6px 12px", border: "none", background: "transparent", cursor: "pointer",
            borderBottom: tab === t ? "2px solid var(--accent-blue)" : "2px solid transparent",
            color: tab === t ? "var(--text-primary)" : "var(--text-secondary)", fontSize: 12, fontFamily: "inherit", textTransform: "capitalize",
          }}>{t}</button>
        ))}
      </div>

      {tab === "setup" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Observe-Act Agent</div>
          <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 12px" }}>
            Continuous visual grounding loop: screenshot → LLM vision → action → verify → repeat.
            Comparable to Anthropic Computer Use and OpenClaw.
          </p>
          <div style={card}>
            <label style={label}>Task Description</label>
            <textarea style={{ ...input, height: 60, resize: "vertical" as const }} value={task} onChange={e => setTask(e.target.value)} placeholder="Log into the admin panel and export the user report as CSV..." />
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, marginTop: 8 }}>
              <div>
                <label style={label}>Safety Mode</label>
                <select style={input} value={mode} onChange={e => setMode(e.target.value as any)}>
                  <option value="cautious">Cautious (confirm destructive)</option>
                  <option value="autonomous">Autonomous (full auto)</option>
                  <option value="restricted">Restricted (read-only)</option>
                </select>
              </div>
              <div>
                <label style={label}>Max Steps</label>
                <input type="number" style={input} value={maxSteps} onChange={e => setMaxSteps(+e.target.value)} min={1} max={200} />
              </div>
              <div>
                <label style={label}>Interval (ms)</label>
                <input type="number" style={input} value={interval} onChange={e => setInterval_(+e.target.value)} min={500} max={10000} step={500} />
              </div>
            </div>
            <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
              <button style={{ ...btn, opacity: !task ? 0.5 : 1 }} disabled={!task} onClick={() => setStatus("running")}>Start Observe-Act Loop</button>
              {status === "running" && <button style={{ ...btn, background: "var(--accent-rose)" }} onClick={() => setStatus("idle")}>Stop</button>}
              <button style={{ ...btn, background: "var(--bg-tertiary)" }} onClick={() => handleSaveConfig({
                mode, maxSteps, interval,
                maxActionsPerStep: config?.maxActionsPerStep ?? 3,
                rateLimitMs: config?.rateLimitMs ?? 200,
                maxConsecutiveFailures: config?.maxConsecutiveFailures ?? 3,
                forbiddenKeyCombos: config?.forbiddenKeyCombos ?? "Ctrl+Alt+Del",
                forbiddenScreenRegions: config?.forbiddenScreenRegions ?? "",
                verifyAfterAction: config?.verifyAfterAction ?? true,
              })}>Save Config</button>
            </div>
          </div>
        </div>
      )}

      {tab === "monitor" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Live Monitor</div>
          {loading ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>Loading monitor data...</div>
          ) : (
            <>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 8, marginBottom: 12 }}>
                {[
                  { label: "Status", value: status.toUpperCase(), color: status === "running" ? "var(--accent-green)" : status === "completed" ? "#2196f3" : "var(--text-secondary)" },
                  { label: "Steps", value: `${steps.length}/${maxSteps}`, color: "var(--text-primary)" },
                  { label: "Actions", value: `${steps.reduce((a, s) => a + s.actions.length, 0)}`, color: "var(--text-primary)" },
                  { label: "Success Rate", value: `${steps.length > 0 ? Math.round(steps.filter(s => s.verified).length / steps.length * 100) : 0}%`, color: "var(--accent-green)" },
                ].map(m => (
                  <div key={m.label} style={card}>
                    <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>{m.label}</div>
                    <div style={{ fontSize: 18, fontWeight: 700, color: m.color, marginTop: 2 }}>{m.value}</div>
                  </div>
                ))}
              </div>
              <div style={card}>
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Latest Screenshot</div>
                <div style={{ background: "var(--bg-tertiary)", borderRadius: 4, height: 200, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-secondary)", fontSize: 12 }}>
                  {status === "running" ? "Capturing..." : "No active session"}
                </div>
              </div>
            </>
          )}
        </div>
      )}

      {tab === "history" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Step History</div>
          {loading ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>Loading step history...</div>
          ) : steps.length === 0 ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>No steps recorded yet. Start an observe-act session to see history.</div>
          ) : (
            steps.map(s => (
              <div key={s.num} style={{ ...card, marginBottom: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <span style={{ fontWeight: 600 }}>Step {s.num}</span>
                  <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                    <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>{s.durationMs}ms</span>
                    <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 3, background: s.verified ? "#4caf5022" : "#f4433622", color: s.verified ? "var(--accent-green)" : "var(--accent-rose)" }}>
                      {s.verified ? "Verified" : "Failed"}
                    </span>
                  </div>
                </div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>{s.reasoning}</div>
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                  {s.actions.map((a, i) => (
                    <span key={i} style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: "var(--bg-tertiary)", fontFamily: "var(--font-mono)" }}>{a}</span>
                  ))}
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {tab === "safety" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Safety Configuration</div>
          {loading ? (
            <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)", fontSize: 13 }}>Loading safety config...</div>
          ) : (
            <div style={card}>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Safety Rails</div>
              <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
                <tbody>
                  <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Max Actions per Step</td><td style={{ padding: "4px 0" }}>{config?.maxActionsPerStep ?? 5}</td></tr>
                  <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Rate Limit (ms between actions)</td><td style={{ padding: "4px 0" }}>{config?.rateLimitMs ?? 200}ms</td></tr>
                  <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Max Consecutive Failures</td><td style={{ padding: "4px 0" }}>{config?.maxConsecutiveFailures ?? 3}</td></tr>
                  <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Forbidden Key Combos</td><td style={{ padding: "4px 0" }}>{config?.forbiddenKeyCombos ?? "Alt+F4, Ctrl+Alt+Del, Cmd+Q"}</td></tr>
                  <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Forbidden Screen Regions</td><td style={{ padding: "4px 0" }}>{config?.forbiddenScreenRegions ?? "System tray, menu bar"}</td></tr>
                  <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Verify After Action</td><td style={{ padding: "4px 0" }}>{config?.verifyAfterAction !== false ? "Enabled" : "Disabled"}</td></tr>
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
