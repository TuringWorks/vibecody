import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ScreenshotEntry {
  path: string;
  timestamp: number;
}

interface VisualStep {
  action: string;
  screenshot: ScreenshotEntry | null;
  assertion: { passed: boolean; confidence: number; details: string } | null;
}

interface VisualTestResult {
  steps: VisualStep[];
  status: string;
}

type SessionStatus = "idle" | "running" | "passed" | "failed";

const statusBadge = (status: SessionStatus) => {
  const map: Record<SessionStatus, { bg: string; label: string }> = {
    idle: { bg: "var(--text-secondary)", label: "Idle" },
    running: { bg: "var(--info-color)", label: "Running..." },
    passed: { bg: "var(--success-color)", label: "Passed" },
    failed: { bg: "var(--error-color)", label: "Failed" },
  };
  const s = map[status];
  return (
    <span
      style={{
        display: "inline-block",
        padding: "2px 8px",
        borderRadius: 4,
        fontSize: 11,
        fontWeight: 600,
        color: "var(--text-primary)",
        background: s.bg,
      }}
    >
      {s.label}
    </span>
  );
};

export function VisualTestPanel() {
  const [url, setUrl] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [status, setStatus] = useState<SessionStatus>("idle");
  const [steps, setSteps] = useState<VisualStep[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [assertion, setAssertion] = useState("");

  const handleStart = async () => {
    if (!url.trim()) return;
    setError(null);
    setStatus("running");
    setSteps([]);
    const id = `vt-${Date.now()}`;
    setSessionId(id);

    // Take an initial screenshot
    try {
      const home =
        "~";
      const outputDir = `${home}/.vibeui/visual-tests/${id}`;
      const result = await invoke<{ path: string; timestamp: number }>(
        "take_screenshot",
        { outputDir }
      );
      const step: VisualStep = {
        action: `Navigate to ${url}`,
        screenshot: result,
        assertion: null,
      };
      setSteps([step]);
    } catch (e: unknown) {
      setError(String(e));
      setStatus("failed");
    }
  };

  const handleScreenshot = async () => {
    if (!sessionId) return;
    setError(null);
    try {
      const home =
        "~";
      const outputDir = `${home}/.vibeui/visual-tests/${sessionId}`;
      const result = await invoke<{ path: string; timestamp: number }>(
        "take_screenshot",
        { outputDir }
      );
      setSteps((prev) => [
        ...prev,
        { action: "Screenshot", screenshot: result, assertion: null },
      ]);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleLoadResults = async () => {
    if (!sessionId) return;
    setError(null);
    try {
      const result = await invoke<VisualTestResult>(
        "get_visual_test_results",
        { sessionId }
      );
      if (result.status === "not_found") {
        setError("No saved results found for this session.");
        return;
      }
      setSteps(result.steps || []);
      const allPassed = (result.steps || []).every(
        (s) => !s.assertion || s.assertion.passed
      );
      const anyFailed = (result.steps || []).some(
        (s) => s.assertion && !s.assertion.passed
      );
      setStatus(anyFailed ? "failed" : allPassed ? "passed" : "running");
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const inputStyle: React.CSSProperties = {
    flex: 1,
    padding: "6px 10px",
    borderRadius: 4,
    border: "1px solid var(--border-color)",
    background: "var(--bg-primary)",
    color: "var(--text-primary)",
    fontSize: 13,
  };

  return (
    <div className="panel-container" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          flexWrap: "wrap",
        }}
      >
        <input
          type="text"
          placeholder="Application URL (e.g. http://localhost:3000)"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          style={inputStyle}
        />
        <button
          onClick={handleStart}
          disabled={status === "running" || !url.trim()}
          className="panel-btn panel-btn-primary"
          style={{
            opacity: status === "running" || !url.trim() ? 0.5 : 1,
          }}
        >
          Start Visual Test
        </button>
        {statusBadge(status)}
      </div>

      {/* Actions row */}
      {sessionId && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button onClick={handleScreenshot} className="panel-btn panel-btn-secondary">
            Take Screenshot
          </button>
          <button onClick={handleLoadResults} className="panel-btn panel-btn-secondary">
            Load Results
          </button>
          <input
            type="text"
            placeholder="Visual assertion (e.g. 'Login button is visible')"
            value={assertion}
            onChange={(e) => setAssertion(e.target.value)}
            style={inputStyle}
          />
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="panel-error">
          {error}
        </div>
      )}

      {/* Steps list */}
      {steps.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <div
            style={{
              fontWeight: 600,
              fontSize: 12,
              color: "var(--text-secondary)",
              textTransform: "uppercase",
            }}
          >
            Test Steps ({steps.length})
          </div>
          {steps.map((step, i) => (
            <div
              key={i}
              className="panel-card"
              style={{
                display: "flex",
                gap: 10,
                alignItems: "flex-start",
              }}
            >
              {/* Step number */}
              <div
                style={{
                  width: 28,
                  height: 28,
                  borderRadius: "50%",
                  background: "var(--accent-color)",
                  color: "var(--text-primary)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontWeight: 700,
                  fontSize: 12,
                  flexShrink: 0,
                }}
              >
                {i + 1}
              </div>

              {/* Details */}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontWeight: 600, marginBottom: 4 }}>
                  {step.action}
                </div>

                {/* Screenshot thumbnail */}
                {step.screenshot && (
                  <div
                    style={{
                      fontSize: 11,
                      color: "var(--text-secondary)",
                      marginBottom: 4,
                    }}
                  >
                    Screenshot: {step.screenshot.path}
                  </div>
                )}

                {/* Assertion result */}
                {step.assertion && (
                  <div
                    style={{
                      padding: 6,
                      borderRadius: 4,
                      background: step.assertion.passed
                        ? "rgba(76,175,80,0.12)"
                        : "rgba(244,67,54,0.12)",
                      fontSize: 12,
                      marginTop: 4,
                    }}
                  >
                    <span style={{ fontWeight: 600 }}>
                      {step.assertion.passed ? "PASS" : "FAIL"}
                    </span>{" "}
                    (confidence: {(step.assertion.confidence * 100).toFixed(0)}%)
                    <div
                      style={{
                        marginTop: 4,
                        color: "var(--text-secondary)",
                      }}
                    >
                      {step.assertion.details}
                    </div>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Empty state */}
      {steps.length === 0 && status === "idle" && (
        <div className="panel-empty" style={{ padding: 40 }}>
          <div style={{ fontSize: 32, marginBottom: 8 }}>
            Visual Self-Testing
          </div>
          <div style={{ fontSize: 13 }}>
            Enter an application URL and start a visual test session.
            <br />
            The agent will take screenshots and evaluate visual assertions via
            AI.
          </div>
        </div>
      )}
    </div>
  );
}
