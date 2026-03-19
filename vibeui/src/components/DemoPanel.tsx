import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ────────────────────────────────────────────────────────────────────

interface DemoStep {
  action: string;
  url?: string;
  selector?: string;
  text?: string;
  description?: string;
  caption?: string;
  assertion?: string;
  ms?: number;
  script?: string;
  x?: number;
  y?: number;
  timeout_ms?: number;
}

interface DemoFrame {
  step_index: number;
  step: DemoStep;
  screenshot_path: string | null;
  result: string | null;
  timestamp: number;
  duration_ms: number;
}

interface DemoRecording {
  id: string;
  name: string;
  description: string;
  steps: DemoStep[];
  frames: DemoFrame[];
  started_at: number;
  finished_at: number | null;
  feature_description: string | null;
  browser_url: string | null;
  status: string;
}

// ── Styles ───────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = {
  padding: 12,
  height: "100%",
  overflow: "auto",
  background: "var(--bg-tertiary)",
  color: "var(--text-primary)",
  fontFamily: "var(--font-family, 'Segoe UI', system-ui, sans-serif)",
  fontSize: 13,
};

const headerStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  marginBottom: 12,
};

const tabBarStyle: React.CSSProperties = {
  display: "flex",
  gap: 2,
  marginBottom: 12,
  borderBottom: "1px solid var(--border-color)",
  paddingBottom: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px",
  cursor: "pointer",
  borderRadius: "4px 4px 0 0",
  background: active ? "var(--accent-color)" : "transparent",
  color: active ? "var(--text-primary)" : "var(--text-secondary)",
  border: "none",
  fontSize: 12,
  fontWeight: active ? 600 : 400,
});

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 6,
  padding: 10,
  marginBottom: 8,
  cursor: "pointer",
  border: "1px solid transparent",
  transition: "border-color 0.15s",
};

const badgeStyle = (status: string): React.CSSProperties => ({
  borderRadius: 10,
  padding: "2px 8px",
  fontSize: 11,
  fontWeight: 600,
  background:
    status === "completed"
      ? "var(--success-color)"
      : status === "running"
        ? "var(--warning-color)"
        : status === "failed"
          ? "var(--error-color)"
          : "var(--accent-color)",
  color: "var(--btn-primary-fg)",
});

const frameRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 10,
  padding: "6px 0",
  borderBottom: "1px solid var(--border-color)",
};

const thumbStyle: React.CSSProperties = {
  width: 100,
  height: 64,
  objectFit: "cover",
  borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)",
};

const btnStyle: React.CSSProperties = {
  background: "var(--accent-color)",
  color: "var(--text-primary)",
  border: "none",
  borderRadius: 4,
  padding: "6px 14px",
  cursor: "pointer",
  fontSize: 12,
  fontWeight: 500,
};

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "6px 10px",
  borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  fontSize: 13,
  boxSizing: "border-box",
};

const selectStyle: React.CSSProperties = {
  ...inputStyle,
  width: "auto",
  minWidth: 140,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatTs(ts: number): string {
  return new Date(ts * 1000).toLocaleString();
}

function stepSummary(step: DemoStep): string {
  switch (step.action) {
    case "navigate":
      return `Navigate to ${step.url}`;
    case "click":
      return `Click: ${step.description || step.selector}`;
    case "type":
      return `Type: ${step.description || step.text}`;
    case "wait":
      return `Wait ${step.ms}ms`;
    case "screenshot":
      return `Screenshot: ${step.caption}`;
    case "assert":
      return `Assert: ${step.assertion}`;
    case "narrate":
      return `${step.text}`;
    case "eval_js":
      return `Eval: ${step.description}`;
    case "scroll":
      return `Scroll ${step.y}px`;
    case "wait_for_selector":
      return `Wait for ${step.selector}`;
    default:
      return step.action;
  }
}

// ── Component ────────────────────────────────────────────────────────────────

type TabKey = "demos" | "create" | "generate";

export function DemoPanel() {
  const [tab, setTab] = useState<TabKey>("demos");
  const [demos, setDemos] = useState<DemoRecording[]>([]);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Create tab state
  const [demoName, setDemoName] = useState("");
  const [demoDesc, setDemoDesc] = useState("");
  const [steps, setSteps] = useState<DemoStep[]>([]);
  const [newStepAction, setNewStepAction] = useState("navigate");
  const [newStepUrl, setNewStepUrl] = useState("");
  const [newStepSelector, setNewStepSelector] = useState("");
  const [newStepText, setNewStepText] = useState("");
  const [newStepCaption, setNewStepCaption] = useState("");
  const [cdpPort, setCdpPort] = useState("9222");

  // Generate tab state
  const [featureDesc, setFeatureDesc] = useState("");
  const [appUrl, setAppUrl] = useState("http://localhost:3000");
  const [generatedSteps, setGeneratedSteps] = useState<DemoStep[] | null>(null);
  const [generating, setGenerating] = useState(false);

  const loadDemos = async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await invoke<DemoRecording[]>("demo_list");
      setDemos(list);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadDemos();
  }, []);

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const addStep = () => {
    const step: DemoStep = { action: newStepAction };
    switch (newStepAction) {
      case "navigate":
        step.url = newStepUrl || "http://localhost:3000";
        break;
      case "click":
        step.selector = newStepSelector;
        step.description = newStepCaption || "Click element";
        break;
      case "type":
        step.selector = newStepSelector;
        step.text = newStepText;
        step.description = newStepCaption || "Type text";
        break;
      case "screenshot":
        step.caption = newStepCaption || "Screenshot";
        break;
      case "wait":
        step.ms = parseInt(newStepText) || 1000;
        step.description = newStepCaption || "Wait";
        break;
      case "assert":
        step.assertion = newStepCaption;
        break;
      case "narrate":
        step.text = newStepCaption;
        break;
    }
    setSteps([...steps, step]);
    setNewStepUrl("");
    setNewStepSelector("");
    setNewStepText("");
    setNewStepCaption("");
  };

  const runDemo = async (name: string, desc: string, demoSteps: DemoStep[]) => {
    setLoading(true);
    setError(null);
    try {
      await invoke("demo_run", {
        name,
        description: desc,
        stepsJson: JSON.stringify(demoSteps),
        cdpPort: parseInt(cdpPort),
      });
      setTab("demos");
      await loadDemos();
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const generateSteps = async () => {
    setGenerating(true);
    setError(null);
    try {
      const result = await invoke<DemoStep[]>("demo_generate_steps", {
        featureDescription: featureDesc,
        appUrl,
      });
      setGeneratedSteps(result);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const exportDemo = async (id: string, format: string) => {
    try {
      const path = await invoke<string>("demo_export", { id, format });
      setError(null);
      setError(`Exported to: ${path}`);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  // ── Render ─────────────────────────────────────────────────────────────

  return (
    <div style={panelStyle}>
      <div style={headerStyle}>
        <span style={{ fontWeight: 700, fontSize: 15 }}>Feature Demos</span>
        <button onClick={loadDemos} disabled={loading} style={btnStyle}>
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      <div style={tabBarStyle}>
        {(["demos", "create", "generate"] as TabKey[]).map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t === "demos" ? "Demos" : t === "create" ? "Create" : "AI Generate"}
          </button>
        ))}
      </div>

      {error && (
        <div style={{ color: "var(--error-color)", marginBottom: 8, fontSize: 12 }}>{error}</div>
      )}

      {/* ── Demos Tab ────────────────────────────────────────────────── */}
      {tab === "demos" && (
        <div>
          {!loading && demos.length === 0 && (
            <div style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: 24 }}>
              No demos yet. Create one using the "Create" or "AI Generate" tabs.
            </div>
          )}

          {demos.map((demo) => (
            <div
              key={demo.id}
              style={{
                ...cardStyle,
                borderColor: expanded.has(demo.id) ? "var(--accent-color)" : "transparent",
              }}
              onClick={() => toggle(demo.id)}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontWeight: 600 }}>{demo.name}</span>
                <span style={badgeStyle(demo.status)}>{demo.status}</span>
              </div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>
                {demo.description}
              </div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>
                {formatTs(demo.started_at)}
                {demo.finished_at && <span> — {formatTs(demo.finished_at)}</span>}
                <span style={{ marginLeft: 8 }}>{demo.frames.length} frames</span>
              </div>

              {expanded.has(demo.id) && (
                <div style={{ marginTop: 10 }}>
                  <div style={{ display: "flex", gap: 6, marginBottom: 10 }}>
                    <button
                      style={{ ...btnStyle, fontSize: 11, padding: "4px 10px" }}
                      onClick={(e) => {
                        e.stopPropagation();
                        exportDemo(demo.id, "html");
                      }}
                    >
                      Export HTML
                    </button>
                    <button
                      style={{ ...btnStyle, fontSize: 11, padding: "4px 10px" }}
                      onClick={(e) => {
                        e.stopPropagation();
                        exportDemo(demo.id, "markdown");
                      }}
                    >
                      Export Markdown
                    </button>
                  </div>

                  {demo.frames.map((frame, i) => (
                    <div key={i} style={frameRowStyle}>
                      {frame.screenshot_path ? (
                        <img
                          src={`asset://localhost/${frame.screenshot_path}`}
                          alt={stepSummary(frame.step)}
                          style={thumbStyle}
                          onError={(e) => {
                            (e.target as HTMLImageElement).style.display = "none";
                          }}
                        />
                      ) : (
                        <div
                          style={{
                            ...thumbStyle,
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            fontSize: 10,
                            color: "var(--text-secondary)",
                          }}
                        >
                          No image
                        </div>
                      )}
                      <div>
                        <div style={{ fontWeight: 500, fontSize: 12 }}>{stepSummary(frame.step)}</div>
                        {frame.result && (
                          <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                            {frame.result}
                          </div>
                        )}
                        <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>
                          {frame.duration_ms}ms
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* ── Create Tab ───────────────────────────────────────────────── */}
      {tab === "create" && (
        <div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              Demo Name
            </label>
            <input
              style={inputStyle}
              placeholder="e.g., Login Flow"
              value={demoName}
              onChange={(e) => setDemoName(e.target.value)}
            />
          </div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              Description
            </label>
            <input
              style={inputStyle}
              placeholder="What this demo shows"
              value={demoDesc}
              onChange={(e) => setDemoDesc(e.target.value)}
            />
          </div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              CDP Port
            </label>
            <input
              style={{ ...inputStyle, width: 100 }}
              value={cdpPort}
              onChange={(e) => setCdpPort(e.target.value)}
            />
          </div>

          <div
            style={{
              background: "var(--bg-secondary)",
              borderRadius: 6,
              padding: 10,
              marginBottom: 10,
            }}
          >
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Add Step</div>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
              <select
                style={selectStyle}
                value={newStepAction}
                onChange={(e) => setNewStepAction(e.target.value)}
              >
                <option value="navigate">Navigate</option>
                <option value="click">Click</option>
                <option value="type">Type</option>
                <option value="screenshot">Screenshot</option>
                <option value="wait">Wait</option>
                <option value="assert">Assert</option>
                <option value="narrate">Narrate</option>
              </select>
              {(newStepAction === "navigate") && (
                <input
                  style={{ ...inputStyle, flex: 1 }}
                  placeholder="URL"
                  value={newStepUrl}
                  onChange={(e) => setNewStepUrl(e.target.value)}
                />
              )}
              {(newStepAction === "click" || newStepAction === "type") && (
                <input
                  style={{ ...inputStyle, flex: 1 }}
                  placeholder="CSS Selector"
                  value={newStepSelector}
                  onChange={(e) => setNewStepSelector(e.target.value)}
                />
              )}
              {(newStepAction === "type" || newStepAction === "wait") && (
                <input
                  style={{ ...inputStyle, flex: 1 }}
                  placeholder={newStepAction === "wait" ? "Milliseconds" : "Text to type"}
                  value={newStepText}
                  onChange={(e) => setNewStepText(e.target.value)}
                />
              )}
              <input
                style={{ ...inputStyle, flex: 1 }}
                placeholder={
                  newStepAction === "assert"
                    ? "Assertion"
                    : newStepAction === "narrate"
                      ? "Narration text"
                      : "Caption / Description"
                }
                value={newStepCaption}
                onChange={(e) => setNewStepCaption(e.target.value)}
              />
            </div>
            <button style={btnStyle} onClick={addStep}>
              + Add Step
            </button>
          </div>

          {steps.length > 0 && (
            <div style={{ marginBottom: 10 }}>
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 6 }}>
                Steps ({steps.length})
              </div>
              {steps.map((s, i) => (
                <div
                  key={i}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "4px 8px",
                    background: "var(--bg-secondary)",
                    borderRadius: 4,
                    marginBottom: 4,
                    fontSize: 12,
                  }}
                >
                  <span>
                    {i + 1}. {stepSummary(s)}
                  </span>
                  <button
                    style={{
                      background: "none",
                      border: "none",
                      color: "var(--error-color)",
                      cursor: "pointer",
                      fontSize: 14,
                    }}
                    onClick={() => setSteps(steps.filter((_, idx) => idx !== i))}
                  >
                    x
                  </button>
                </div>
              ))}
            </div>
          )}

          <button
            style={{ ...btnStyle, width: "100%", padding: "10px 0", fontSize: 14 }}
            disabled={!demoName || steps.length === 0 || loading}
            onClick={() => runDemo(demoName, demoDesc, steps)}
          >
            {loading ? "Running Demo..." : "Run Demo"}
          </button>
        </div>
      )}

      {/* ── AI Generate Tab ──────────────────────────────────────────── */}
      {tab === "generate" && (
        <div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              Feature Description
            </label>
            <textarea
              style={{ ...inputStyle, minHeight: 80, resize: "vertical" }}
              placeholder="Describe the feature to demo, e.g.: 'Login form with email/password validation, error messages, and redirect to dashboard on success'"
              value={featureDesc}
              onChange={(e) => setFeatureDesc(e.target.value)}
            />
          </div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              App URL
            </label>
            <input
              style={inputStyle}
              value={appUrl}
              onChange={(e) => setAppUrl(e.target.value)}
            />
          </div>
          <div style={{ marginBottom: 10 }}>
            <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>
              CDP Port
            </label>
            <input
              style={{ ...inputStyle, width: 100 }}
              value={cdpPort}
              onChange={(e) => setCdpPort(e.target.value)}
            />
          </div>
          <button
            style={{ ...btnStyle, marginBottom: 12 }}
            disabled={!featureDesc || generating}
            onClick={generateSteps}
          >
            {generating ? "Generating..." : "Generate Demo Steps"}
          </button>

          {generatedSteps && (
            <div>
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>
                Generated Steps ({generatedSteps.length})
              </div>
              {generatedSteps.map((s, i) => (
                <div
                  key={i}
                  style={{
                    padding: "6px 10px",
                    background: "var(--bg-secondary)",
                    borderRadius: 4,
                    marginBottom: 4,
                    fontSize: 12,
                  }}
                >
                  {i + 1}. {stepSummary(s)}
                </div>
              ))}
              <button
                style={{ ...btnStyle, width: "100%", padding: "10px 0", fontSize: 14, marginTop: 10 }}
                disabled={loading}
                onClick={() =>
                  runDemo(
                    featureDesc.slice(0, 40).replace(/[^a-zA-Z0-9 ]/g, "").trim().replace(/ /g, "-") || "ai-demo",
                    featureDesc,
                    generatedSteps
                  )
                }
              >
                {loading ? "Running Demo..." : "Run Generated Demo"}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
