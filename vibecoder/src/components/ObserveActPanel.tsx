/**
 * ObserveActPanel — the visual grounding loop's control surface.
 *
 * Everything here talks to the VibeCLI daemon over `daemonFetch`: the daemon
 * owns the screenshot pipeline, the desktop automation tools and the provider
 * keys, and it owns the session registry because there is one screen per
 * machine. A shell-local copy of any of that would give three shells three
 * disagreeing answers to "is a session running".
 *
 * The live view is driven by the session's SSE stream rather than by polling.
 * A 500 ms poll of a loop that steps every two seconds is three wasted
 * requests out of four, and the daemon's rate limiter (60/min) would start
 * refusing them partway through a run.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { daemonFetch, getDaemonToken } from "../lib/daemonFetch";
import { ExperimentalBadge } from "./ExperimentalBadge";
import { ModelSelector } from "./shared/ModelSelector";
import { PROVIDER_DEFAULT_MODEL } from "../hooks/useModelRegistry";

type SubTab = "setup" | "monitor" | "history" | "safety";

type SafetyMode = "cautious" | "autonomous" | "restricted";

type SessionStatus =
  | "idle"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "aborted";

/** Mirrors `observe_act::ObserveActAction` — serde-tagged on `type`. */
type ObserveAction =
  | { type: "click"; x: number; y: number }
  | { type: "double_click"; x: number; y: number }
  | { type: "right_click"; x: number; y: number }
  | { type: "type"; text: string }
  | { type: "key_combo"; keys: string[] }
  | { type: "scroll"; direction: "up" | "down" | "left" | "right"; amount: number }
  | { type: "wait"; ms: number }
  | { type: "screenshot" }
  | { type: "move_mouse"; x: number; y: number }
  | { type: "drag"; from_x: number; from_y: number; to_x: number; to_y: number }
  | { type: "done"; summary: string };

interface VerificationResult {
  expected_change: string;
  actual_observation: string;
  success: boolean;
  confidence: number;
}

interface Step {
  step_num: number;
  timestamp_ms: number;
  screenshot_path: string | null;
  llm_reasoning: string;
  actions_taken: ObserveAction[];
  proposed_actions: ObserveAction[];
  verification_result: VerificationResult | null;
  duration_ms: number;
}

interface ScreenRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  label: string;
}

/** Mirrors `observe_act::SafetyRails`. */
interface SafetyRails {
  forbidden_regions: ScreenRegion[];
  max_actions_per_step: number;
  require_confirmation_for: string[];
  forbidden_key_combos: string[][];
  rate_limit_ms: number;
}

/** Mirrors `observe_act::ObserveActConfig`. */
interface LoopConfig {
  observation_interval_ms: number;
  max_steps: number;
  max_consecutive_failures: number;
  screenshot_width: number;
  screenshot_height: number;
  vision_provider: string;
  verify_after_action: boolean;
  safety_mode: SafetyMode;
}

/** `StoredConfig` flattens `LoopConfig` and nests `safety`. */
type StoredConfig = LoopConfig & { safety: SafetyRails };

interface PendingApproval {
  id: string;
  step_num: number;
  action: ObserveAction;
  description: string;
  requested_at_ms: number;
}

interface SessionSummary {
  total_steps: number;
  total_actions: number;
  success_rate: number;
  duration_ms: number;
  final_status: SessionStatus;
  task: string;
  completion_summary: string | null;
}

interface Session {
  id: string;
  model: string;
  task: string;
  status: SessionStatus;
  config: LoopConfig;
  started_at_ms: number;
  consecutive_failures: number;
  summary: SessionSummary;
  steps: Step[];
  pending_approval: PendingApproval | null;
  has_screenshot: boolean;
  provider_claims_vision?: boolean;
}

interface SessionRow {
  id: string;
  model: string;
  task: string;
  status: SessionStatus;
  total_steps: number;
  total_actions: number;
  success_rate: number;
  duration_ms: number;
  completion_summary: string | null;
}

interface Preflight {
  platform: string;
  missing_tools: string[];
  logical_screen: [number, number] | null;
  screen_error?: string;
  ready: boolean;
}

/** Server-sent event payloads — mirrors `observe_act::ObserveActEvent`. */
type LoopEvent =
  | { event: "step_started"; step_num: number }
  | { event: "screenshot_captured"; path: string }
  | { event: "llm_reasoning"; text: string }
  | { event: "action_executed"; action: ObserveAction; success: boolean }
  | { event: "verification_done"; result: VerificationResult }
  | { event: "task_completed"; summary: string }
  | { event: "error"; message: string }
  | { event: "safety_halt"; reason: string }
  | {
      event: "approval_required";
      approval_id: string;
      step_num: number;
      action: ObserveAction;
      description: string;
    }
  | { event: "approval_resolved"; approval_id: string; approved: boolean };

interface ObserveActPanelProps {
  /** Provider selected in the toolbar. */
  provider?: string;
  daemonUrl?: string;
}

const TERMINAL: readonly SessionStatus[] = ["completed", "failed", "aborted"];

/** Render an action the way the Rust `Display` impl does. */
function describeAction(a: ObserveAction): string {
  switch (a.type) {
    case "click":
      return `Click(${a.x}, ${a.y})`;
    case "double_click":
      return `DoubleClick(${a.x}, ${a.y})`;
    case "right_click":
      return `RightClick(${a.x}, ${a.y})`;
    case "type":
      return `Type("${a.text}")`;
    case "key_combo":
      return `KeyCombo(${a.keys.join("+")})`;
    case "scroll":
      return `Scroll(${a.direction}, ${a.amount})`;
    case "wait":
      return `Wait(${a.ms}ms)`;
    case "screenshot":
      return "Screenshot";
    case "move_mouse":
      return `MoveMouse(${a.x}, ${a.y})`;
    case "drag":
      return `Drag(${a.from_x},${a.from_y} → ${a.to_x},${a.to_y})`;
    case "done":
      return `Done("${a.summary}")`;
    default: {
      // Exhaustiveness: a new action variant on the daemon shows up here as a
      // type error rather than as a blank chip in the history.
      const never: never = a;
      return JSON.stringify(never);
    }
  }
}

/** Read an error body the daemon sent, falling back to the status line. */
async function errorFrom(res: Response): Promise<string> {
  try {
    const body: unknown = await res.json();
    if (body && typeof body === "object" && "error" in body) {
      return String((body as { error: unknown }).error);
    }
  } catch {
    // Not JSON — the status is all we have.
  }
  return `${res.status} ${res.statusText}`;
}

const statusColor = (s: SessionStatus): string => {
  switch (s) {
    case "running":
      return "var(--accent-green)";
    case "paused":
      return "var(--warning-color)";
    case "completed":
      return "var(--info-color)";
    case "failed":
    case "aborted":
      return "var(--accent-rose)";
    default:
      return "var(--text-secondary)";
  }
};

export function ObserveActPanel({
  provider,
  daemonUrl = "http://localhost:7878",
}: ObserveActPanelProps) {
  const [tab, setTab] = useState<SubTab>("setup");
  const [task, setTask] = useState("");

  // The vision model is chosen here rather than inherited wholesale: the model
  // selected for coding is frequently not vision-capable, and a loop that
  // silently fell back to some other vendor would be sending screenshots of
  // the user's desktop to a service they did not pick. Seeded from the
  // toolbar's provider, per AGENTS.md → Provider-Agnostic Panels.
  const [visionProvider, setVisionProvider] = useState(provider ?? "");
  const [visionModel, setVisionModel] = useState(
    provider ? (PROVIDER_DEFAULT_MODEL[provider] ?? "") : ""
  );

  // Follow the toolbar when it changes, unless the operator has picked
  // something else here.
  const touchedRef = useRef(false);
  useEffect(() => {
    if (touchedRef.current || !provider) return;
    setVisionProvider(provider);
    setVisionModel(PROVIDER_DEFAULT_MODEL[provider] ?? "");
  }, [provider]);

  const [config, setConfig] = useState<StoredConfig | null>(null);
  const [preflight, setPreflight] = useState<Preflight | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [sessions, setSessions] = useState<SessionRow[]>([]);
  const [liveEvents, setLiveEvents] = useState<LoopEvent[]>([]);
  const [shotNonce, setShotNonce] = useState(0);
  const [token, setToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  const sessionId = session?.id ?? null;
  const status = session?.status ?? "idle";
  const isLive = status === "running" || status === "paused";

  const refreshSession = useCallback(
    async (id: string) => {
      const res = await daemonFetch(`${daemonUrl}/observe/sessions/${id}`);
      if (!res.ok) return;
      setSession((await res.json()) as Session);
    },
    [daemonUrl]
  );

  const refreshSessions = useCallback(async () => {
    const res = await daemonFetch(`${daemonUrl}/observe/sessions`);
    if (!res.ok) return;
    const body = (await res.json()) as { sessions: SessionRow[] };
    setSessions(body.sessions);
    return body.sessions;
  }, [daemonUrl]);

  // Initial load: config, preflight, and any session a previous run left
  // behind — including one still going, which the panel must adopt rather than
  // show an empty Start form over.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const [cfgRes, preRes] = await Promise.all([
          daemonFetch(`${daemonUrl}/observe/config`),
          daemonFetch(`${daemonUrl}/observe/preflight`),
        ]);
        if (cancelled) return;
        if (cfgRes.ok) setConfig((await cfgRes.json()) as StoredConfig);
        if (preRes.ok) setPreflight((await preRes.json()) as Preflight);

        const rows = await refreshSessions();
        if (cancelled || !rows?.length) return;
        const live = rows.find((r) => !TERMINAL.includes(r.status)) ?? rows[0];
        await refreshSession(live.id);
      } catch (e) {
        if (!cancelled) {
          setError(
            `Could not reach the VibeCLI daemon at ${daemonUrl}: ${String(e)}`
          );
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [daemonUrl, refreshSession, refreshSessions]);

  useEffect(() => {
    getDaemonToken().then(setToken).catch(() => setToken(null));
  }, []);

  // Live stream. `EventSource` cannot set a header, so the token rides in the
  // query string — the daemon accepts it for exactly this case.
  useEffect(() => {
    if (!sessionId || !token || TERMINAL.includes(status)) return;
    const url = `${daemonUrl}/observe/sessions/${sessionId}/events?token=${encodeURIComponent(token)}`;
    const source = new EventSource(url);

    const onSnapshot = (e: MessageEvent<string>) => {
      try {
        setSession(JSON.parse(e.data) as Session);
      } catch {
        // A malformed frame is not worth tearing the stream down for.
      }
    };
    const onStep = (e: MessageEvent<string>) => {
      let event: LoopEvent;
      try {
        event = JSON.parse(e.data) as LoopEvent;
      } catch {
        return;
      }
      setLiveEvents((prev) => [...prev.slice(-199), event]);
      if (event.event === "screenshot_captured") setShotNonce((n) => n + 1);
      // Re-read the authoritative record at the points where it changed in a
      // way the event stream cannot express on its own — a finished step, an
      // approval that needs its id, a terminal status.
      if (
        event.event === "step_started" ||
        event.event === "task_completed" ||
        event.event === "approval_required" ||
        event.event === "approval_resolved" ||
        event.event === "safety_halt"
      ) {
        void refreshSession(sessionId);
      }
    };
    const onClosed = () => void refreshSession(sessionId);

    source.addEventListener("snapshot", onSnapshot as EventListener);
    source.addEventListener("step", onStep as EventListener);
    source.addEventListener("closed", onClosed);
    source.addEventListener("lagged", onClosed);

    return () => source.close();
  }, [daemonUrl, sessionId, token, status, refreshSession]);

  const post = useCallback(
    async (path: string, body?: unknown): Promise<Response> =>
      daemonFetch(`${daemonUrl}${path}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
      }),
    [daemonUrl]
  );

  const start = useCallback(async () => {
    setError(null);
    setNotice(null);
    setBusy(true);
    try {
      const res = await post("/observe/sessions", {
        task,
        provider: visionProvider,
        model: visionModel,
        config: config
          ? { ...configOnly(config), safety_mode: config.safety_mode }
          : undefined,
        safety: config?.safety,
      });
      if (!res.ok) {
        setError(await errorFrom(res));
        return;
      }
      const started = (await res.json()) as Session;
      setLiveEvents([]);
      setSession(started);
      setTab("monitor");
      if (started.provider_claims_vision === false) {
        setNotice(
          `${started.model} does not advertise vision support. The run will proceed — several providers accept images without advertising it — but if every step comes back blind, pick a vision model.`
        );
      }
      void refreshSessions();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [post, task, visionProvider, visionModel, config, refreshSessions]);

  const control = useCallback(
    async (action: "pause" | "resume" | "abort") => {
      if (!sessionId) return;
      setBusy(true);
      try {
        const res = await post(`/observe/sessions/${sessionId}/${action}`);
        if (res.ok) setSession((await res.json()) as Session);
        else setError(await errorFrom(res));
      } finally {
        setBusy(false);
      }
    },
    [post, sessionId]
  );

  const answerApproval = useCallback(
    async (approvalId: string, approve: boolean) => {
      if (!sessionId) return;
      const res = await post(`/observe/sessions/${sessionId}/approve`, {
        approval_id: approvalId,
        approve,
      });
      if (!res.ok) setError(await errorFrom(res));
      void refreshSession(sessionId);
    },
    [post, sessionId, refreshSession]
  );

  const saveConfig = useCallback(
    async (next: StoredConfig) => {
      setError(null);
      try {
        const res = await daemonFetch(`${daemonUrl}/observe/config`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(next),
        });
        if (!res.ok) {
          setError(await errorFrom(res));
          return;
        }
        // Apply only once the write succeeded. Setting it first and never
        // rolling back showed settings that were never persisted — they came
        // back on the next load with nothing saying anything had failed.
        setConfig((await res.json()) as StoredConfig);
        setNotice("Configuration saved.");
      } catch (e) {
        setError(String(e));
      }
    },
    [daemonUrl]
  );

  const patchConfig = useCallback(
    (patch: Partial<StoredConfig>) =>
      setConfig((prev) => (prev ? { ...prev, ...patch } : prev)),
    []
  );

  const patchSafety = useCallback(
    (patch: Partial<SafetyRails>) =>
      setConfig((prev) =>
        prev ? { ...prev, safety: { ...prev.safety, ...patch } } : prev
      ),
    []
  );

  const modelChosen = visionProvider !== "" && visionModel !== "";
  const canStart =
    task.trim().length > 0 && modelChosen && !isLive && !busy && !!config;

  const liveReasoning = useMemo(() => {
    for (let i = liveEvents.length - 1; i >= 0; i -= 1) {
      const e = liveEvents[i];
      if (e.event === "llm_reasoning") return e.text;
    }
    return null;
  }, [liveEvents]);

  const screenshotSrc =
    sessionId && token && session?.has_screenshot
      ? `${daemonUrl}/observe/sessions/${sessionId}/screenshot?token=${encodeURIComponent(token)}&n=${shotNonce}`
      : null;

  return (
    <div className="panel-container" style={{ fontSize: "var(--font-size-md)" }}>
      <ExperimentalBadge
        as="banner"
        feature="Observe & Act"
        tooltip="Autonomous observe-then-act agent loop. Safety guardrails are still being tightened — review every action before approving."
      />

      {error && (
        <div role="alert" style={bannerStyle("var(--error-color)")}>
          {error}
        </div>
      )}
      {notice && (
        <div role="status" style={bannerStyle("var(--info-color)")}>
          {notice}
        </div>
      )}

      <div className="panel-tab-bar">
        {(["setup", "monitor", "history", "safety"] as const).map((t) => (
          <button
            key={t}
            className={`panel-tab ${tab === t ? "active" : ""}`}
            onClick={() => setTab(t)}
            style={{ textTransform: "capitalize" }}
          >
            {t}
          </button>
        ))}
      </div>

      {tab === "setup" && (
        <div>
          <div style={headingStyle}>Observe-Act Agent</div>
          <p
            style={{
              fontSize: "var(--font-size-base)",
              color: "var(--text-secondary)",
              margin: "0 0 12px",
            }}
          >
            Continuous visual grounding loop: screenshot → LLM vision → action →
            verify → repeat. The agent drives your real mouse and keyboard.
          </p>

          <PreflightCard preflight={preflight} />

          <div className="panel-card">
            <label className="panel-label" htmlFor="observeact-task">
              Task Description
            </label>
            <textarea
              id="observeact-task"
              className="panel-input panel-textarea panel-input-full"
              style={{ height: 60, resize: "vertical" }}
              value={task}
              onChange={(e) => setTask(e.target.value)}
              placeholder="Log into the admin panel and export the user report as CSV..."
            />

            <div style={{ marginTop: 8 }}>
              <label className="panel-label">Vision Model</label>
              {visionProvider === "" ? (
                <div className="panel-empty" style={{ padding: 8 }}>
                  Select a model in the toolbar — the loop will not fall back to
                  a provider you did not pick.
                </div>
              ) : (
                <ModelSelector
                  provider={visionProvider}
                  model={visionModel}
                  onProviderChange={(p) => {
                    touchedRef.current = true;
                    setVisionProvider(p);
                  }}
                  onModelChange={(m) => {
                    touchedRef.current = true;
                    setVisionModel(m);
                  }}
                />
              )}
            </div>

            {config && (
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr 1fr",
                  gap: 8,
                  marginTop: 8,
                }}
              >
                <div>
                  <label className="panel-label" htmlFor="observeact-mode">
                    Safety Mode
                  </label>
                  <select
                    id="observeact-mode"
                    className="panel-select"
                    style={{ width: "100%" }}
                    value={config.safety_mode}
                    onChange={(e) =>
                      patchConfig({ safety_mode: e.target.value as SafetyMode })
                    }
                  >
                    <option value="cautious">Cautious (confirm destructive)</option>
                    <option value="autonomous">Autonomous (full auto)</option>
                    <option value="restricted">Restricted (read-only)</option>
                  </select>
                </div>
                <div>
                  <label className="panel-label" htmlFor="observeact-max-steps">
                    Max Steps
                  </label>
                  <input
                    id="observeact-max-steps"
                    type="number"
                    className="panel-input panel-input-full"
                    value={config.max_steps}
                    onChange={(e) =>
                      patchConfig({ max_steps: clamp(+e.target.value, 1, 200) })
                    }
                    min={1}
                    max={200}
                  />
                </div>
                <div>
                  <label className="panel-label" htmlFor="observeact-interval">
                    Interval (ms)
                  </label>
                  <input
                    id="observeact-interval"
                    type="number"
                    className="panel-input panel-input-full"
                    value={config.observation_interval_ms}
                    onChange={(e) =>
                      patchConfig({
                        observation_interval_ms: clamp(+e.target.value, 0, 60000),
                      })
                    }
                    min={0}
                    max={60000}
                    step={500}
                  />
                </div>
              </div>
            )}

            {config?.safety_mode === "restricted" && (
              <p style={hintStyle}>
                Read-only: the agent observes and records what it would do, and
                executes nothing.
              </p>
            )}
            {config?.safety_mode === "autonomous" && (
              <p style={{ ...hintStyle, color: "var(--accent-rose)" }}>
                Full auto: destructive actions run without asking. Stop is the
                only gate.
              </p>
            )}

            <div style={{ marginTop: 12, display: "flex", gap: 8, flexWrap: "wrap" }}>
              <button
                className="panel-btn panel-btn-primary"
                style={{ opacity: canStart ? 1 : 0.5 }}
                disabled={!canStart}
                onClick={() => void start()}
              >
                Start Observe-Act Loop
              </button>
              {isLive && (
                <>
                  <button
                    className="panel-btn panel-btn-secondary"
                    disabled={busy}
                    onClick={() =>
                      void control(status === "paused" ? "resume" : "pause")
                    }
                  >
                    {status === "paused" ? "Resume" : "Pause"}
                  </button>
                  <button
                    className="panel-btn panel-btn-danger"
                    disabled={busy}
                    onClick={() => void control("abort")}
                  >
                    Stop
                  </button>
                </>
              )}
              <button
                className="panel-btn panel-btn-secondary"
                disabled={!config}
                onClick={() => config && void saveConfig(config)}
              >
                Save Config
              </button>
            </div>
          </div>
        </div>
      )}

      {tab === "monitor" && (
        <div>
          <div style={headingStyle}>Live Monitor</div>
          {loading ? (
            <div className="panel-loading">Loading monitor data...</div>
          ) : !session ? (
            <div className="panel-empty">
              No session yet. Start one from Setup.
            </div>
          ) : (
            <>
              {session.pending_approval && (
                <ApprovalPrompt
                  approval={session.pending_approval}
                  onAnswer={answerApproval}
                />
              )}

              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr 1fr 1fr",
                  gap: 8,
                  marginBottom: 12,
                }}
              >
                <Metric
                  label="Status"
                  value={session.status.toUpperCase()}
                  color={statusColor(session.status)}
                />
                <Metric
                  label="Steps"
                  value={`${session.summary.total_steps}/${session.config.max_steps}`}
                />
                <Metric
                  label="Actions"
                  value={String(session.summary.total_actions)}
                />
                <Metric
                  label="Verified"
                  value={verifiedLabel(session.steps)}
                  color="var(--accent-green)"
                />
              </div>

              <div className="panel-card">
                <div style={subHeadingStyle}>Latest Screenshot</div>
                {screenshotSrc ? (
                  <img
                    src={screenshotSrc}
                    alt={`Screen as of step ${session.summary.total_steps}`}
                    style={{
                      width: "100%",
                      borderRadius: "var(--radius-xs-plus)",
                      background: "var(--bg-tertiary)",
                    }}
                  />
                ) : (
                  <div style={placeholderStyle}>
                    {isLive ? "Capturing..." : "No screenshot captured"}
                  </div>
                )}
              </div>

              {liveReasoning && (
                <div className="panel-card" style={{ marginTop: 8 }}>
                  <div style={subHeadingStyle}>Current Reasoning</div>
                  <div
                    style={{
                      fontSize: "var(--font-size-base)",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {liveReasoning}
                  </div>
                </div>
              )}

              <div className="panel-card" style={{ marginTop: 8 }}>
                <div style={subHeadingStyle}>Event Log</div>
                {liveEvents.length === 0 ? (
                  <div style={{ color: "var(--text-secondary)" }}>
                    No events yet on this connection.
                  </div>
                ) : (
                  <div style={{ maxHeight: 220, overflowY: "auto" }}>
                    {liveEvents
                      .slice()
                      .reverse()
                      .map((e, i) => (
                        <div
                          key={`${e.event}-${liveEvents.length - i}`}
                          className="panel-mono"
                          style={{
                            fontSize: "var(--font-size-xs)",
                            padding: "2px 0",
                            color: eventColor(e),
                          }}
                        >
                          {describeEvent(e)}
                        </div>
                      ))}
                  </div>
                )}
              </div>
            </>
          )}
        </div>
      )}

      {tab === "history" && (
        <div>
          <div style={headingStyle}>Step History</div>
          {loading ? (
            <div className="panel-loading">Loading step history...</div>
          ) : (
            <>
              {sessions.length > 1 && (
                <div className="panel-card" style={{ marginBottom: 8 }}>
                  <label className="panel-label" htmlFor="observeact-session">
                    Session
                  </label>
                  <select
                    id="observeact-session"
                    className="panel-select"
                    style={{ width: "100%" }}
                    value={sessionId ?? ""}
                    onChange={(e) => void refreshSession(e.target.value)}
                  >
                    {sessions.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.status} — {s.task.slice(0, 60)} ({s.total_steps} steps)
                      </option>
                    ))}
                  </select>
                </div>
              )}
              {!session || session.steps.length === 0 ? (
                <div className="panel-empty">
                  No steps recorded yet. Start an observe-act session to see
                  history.
                </div>
              ) : (
                session.steps.map((s) => <StepCard key={s.step_num} step={s} />)
              )}
            </>
          )}
        </div>
      )}

      {tab === "safety" && (
        <div>
          <div style={headingStyle}>Safety Configuration</div>
          {loading ? (
            <div className="panel-loading">Loading safety config...</div>
          ) : !config ? (
            <div className="panel-empty">
              The daemon did not return a configuration.
            </div>
          ) : (
            <div className="panel-card">
              <div style={subHeadingStyle}>Safety Rails</div>
              <NumberRow
                label="Max Actions per Step"
                value={config.safety.max_actions_per_step}
                min={1}
                max={20}
                onChange={(v) => patchSafety({ max_actions_per_step: v })}
              />
              <NumberRow
                label="Rate Limit (ms between actions)"
                value={config.safety.rate_limit_ms}
                min={0}
                max={10000}
                step={50}
                onChange={(v) => patchSafety({ rate_limit_ms: v })}
              />
              <NumberRow
                label="Max Consecutive Failures"
                value={config.max_consecutive_failures}
                min={1}
                max={20}
                onChange={(v) => patchConfig({ max_consecutive_failures: v })}
              />
              <TextRow
                label="Forbidden Key Combos"
                hint="Comma-separated, each combo joined with +. e.g. alt+f4, ctrl+alt+del"
                value={config.safety.forbidden_key_combos
                  .map((c) => c.join("+"))
                  .join(", ")}
                onChange={(v) =>
                  patchSafety({
                    forbidden_key_combos: v
                      .split(",")
                      .map((c) => c.trim().toLowerCase())
                      .filter(Boolean)
                      .map((c) => c.split("+").map((k) => k.trim()).filter(Boolean))
                      .filter((c) => c.length > 0),
                  })
                }
              />
              <RegionEditor
                regions={config.safety.forbidden_regions}
                onChange={(forbidden_regions) => patchSafety({ forbidden_regions })}
              />
              <label
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "6px 0",
                }}
              >
                <input
                  type="checkbox"
                  checked={config.verify_after_action}
                  onChange={(e) =>
                    patchConfig({ verify_after_action: e.target.checked })
                  }
                />
                <span>Verify after action</span>
              </label>
              <p style={hintStyle}>
                Verification costs a second screenshot and a second model call
                per step. With it off, steps are recorded unverified — which is
                neither a pass nor a fail, and does not count toward the failure
                limit.
              </p>
              <div style={{ marginTop: 12 }}>
                <button
                  className="panel-btn panel-btn-primary"
                  onClick={() => void saveConfig(config)}
                >
                  Save Safety Config
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Sub-components ─────────────────────────────────────────────────────────

function PreflightCard({ preflight }: { preflight: Preflight | null }) {
  if (!preflight) return null;
  if (preflight.ready) {
    return (
      <div className="panel-card" style={{ marginBottom: 8 }}>
        <span style={{ color: "var(--accent-green)" }}>Ready</span>{" "}
        <span style={{ color: "var(--text-secondary)" }}>
          — {preflight.platform}
          {preflight.logical_screen
            ? `, ${preflight.logical_screen[0]}×${preflight.logical_screen[1]}`
            : ""}
        </span>
      </div>
    );
  }
  return (
    <div
      className="panel-card"
      style={{ marginBottom: 8, borderLeft: "3px solid var(--error-color)" }}
      role="alert"
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>
        This machine cannot run a session yet
      </div>
      {preflight.missing_tools.length > 0 && (
        <div style={{ fontSize: "var(--font-size-base)" }}>
          Missing tools:{" "}
          <span className="panel-mono">{preflight.missing_tools.join(", ")}</span>
          {preflight.platform === "macOS" && (
            <> — install with <span className="panel-mono">brew install cliclick</span>.</>
          )}
        </div>
      )}
      {preflight.screen_error && (
        <div style={{ fontSize: "var(--font-size-base)", marginTop: 4 }}>
          Screen geometry: {preflight.screen_error}
        </div>
      )}
    </div>
  );
}

function ApprovalPrompt({
  approval,
  onAnswer,
}: {
  approval: PendingApproval;
  onAnswer: (id: string, approve: boolean) => void;
}) {
  return (
    <div
      className="panel-card"
      role="alert"
      style={{
        marginBottom: 12,
        borderLeft: "3px solid var(--warning-color)",
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>
        Confirm destructive action — step {approval.step_num}
      </div>
      <div className="panel-mono" style={{ marginBottom: 8 }}>
        {approval.description}
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <button
          className="panel-btn panel-btn-primary"
          onClick={() => onAnswer(approval.id, true)}
        >
          Approve
        </button>
        <button
          className="panel-btn panel-btn-danger"
          onClick={() => onAnswer(approval.id, false)}
        >
          Deny
        </button>
      </div>
      <p style={hintStyle}>
        Unanswered for five minutes, this is treated as a refusal.
      </p>
    </div>
  );
}

function StepCard({ step }: { step: Step }) {
  const skipped = step.proposed_actions.filter(
    (p) => !step.actions_taken.some((a) => describeAction(a) === describeAction(p))
  );
  return (
    <div className="panel-card" style={{ marginBottom: 8 }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 6,
        }}
      >
        <span style={{ fontWeight: 600 }}>Step {step.step_num}</span>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <span
            style={{
              fontSize: "var(--font-size-xs)",
              color: "var(--text-secondary)",
            }}
          >
            {step.duration_ms}ms
          </span>
          <VerificationBadge result={step.verification_result} />
        </div>
      </div>
      <div
        style={{
          fontSize: "var(--font-size-base)",
          color: "var(--text-secondary)",
          marginBottom: 4,
        }}
      >
        {step.llm_reasoning}
      </div>
      {step.actions_taken.length > 0 && (
        <ActionChips actions={step.actions_taken} tone="ran" />
      )}
      {skipped.length > 0 && <ActionChips actions={skipped} tone="skipped" />}
      {step.verification_result && (
        <div
          style={{
            fontSize: "var(--font-size-xs)",
            color: "var(--text-secondary)",
            marginTop: 6,
          }}
        >
          Expected: {step.verification_result.expected_change || "—"} · Saw:{" "}
          {step.verification_result.actual_observation}
        </div>
      )}
    </div>
  );
}

function ActionChips({
  actions,
  tone,
}: {
  actions: ObserveAction[];
  tone: "ran" | "skipped";
}) {
  return (
    <div
      style={{ display: "flex", gap: 4, flexWrap: "wrap", marginTop: 4 }}
      aria-label={tone === "ran" ? "Actions executed" : "Actions not executed"}
    >
      {tone === "skipped" && (
        <span
          style={{
            fontSize: "var(--font-size-xs)",
            color: "var(--text-secondary)",
          }}
        >
          not executed:
        </span>
      )}
      {actions.map((a, i) => (
        <span
          key={`${tone}-${i}`}
          className="panel-mono"
          style={{
            fontSize: "var(--font-size-xs)",
            padding: "2px 8px",
            borderRadius: 3,
            background: "var(--bg-tertiary)",
            opacity: tone === "skipped" ? 0.6 : 1,
            textDecoration: tone === "skipped" ? "line-through" : "none",
          }}
        >
          {describeAction(a)}
        </span>
      ))}
    </div>
  );
}

/**
 * Three states, not two. An unverified step is neither a pass nor a fail, and
 * showing it as "Failed" — which the old panel did, via `verified: boolean` —
 * turned "we did not check" into "it went wrong".
 */
function VerificationBadge({ result }: { result: VerificationResult | null }) {
  const [label, fg, bg] = result
    ? result.success
      ? ["Verified", "var(--accent-green)", "var(--success-bg)"]
      : ["Failed", "var(--accent-rose)", "var(--error-bg)"]
    : ["Unverified", "var(--text-secondary)", "var(--bg-tertiary)"];
  return (
    <span
      title={
        result
          ? `${Math.round(result.confidence * 100)}% confidence`
          : "No verification was performed for this step"
      }
      style={{
        fontSize: "var(--font-size-xs)",
        padding: "1px 8px",
        borderRadius: 3,
        background: bg,
        color: fg,
      }}
    >
      {label}
    </span>
  );
}

function Metric({
  label,
  value,
  color = "var(--text-primary)",
}: {
  label: string;
  value: string;
  color?: string;
}) {
  return (
    <div className="panel-card">
      <div
        style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}
      >
        {label}
      </div>
      <div style={{ fontSize: 18, fontWeight: 700, color, marginTop: 2 }}>
        {value}
      </div>
    </div>
  );
}

function NumberRow({
  label,
  value,
  min,
  max,
  step = 1,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 12,
        padding: "4px 0",
      }}
    >
      <label className="panel-label" htmlFor={`observeact-${label}`}>
        {label}
      </label>
      <input
        id={`observeact-${label}`}
        type="number"
        className="panel-input"
        style={{ width: 110 }}
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(e) => onChange(clamp(+e.target.value, min, max))}
      />
    </div>
  );
}

function TextRow({
  label,
  hint,
  value,
  onChange,
}: {
  label: string;
  hint: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div style={{ padding: "4px 0" }}>
      <label className="panel-label" htmlFor={`observeact-${label}`}>
        {label}
      </label>
      <input
        id={`observeact-${label}`}
        type="text"
        className="panel-input panel-input-full"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      <p style={hintStyle}>{hint}</p>
    </div>
  );
}

function RegionEditor({
  regions,
  onChange,
}: {
  regions: ScreenRegion[];
  onChange: (r: ScreenRegion[]) => void;
}) {
  const [draft, setDraft] = useState<ScreenRegion>({
    x: 0,
    y: 0,
    width: 0,
    height: 0,
    label: "",
  });
  const complete = draft.label.trim() !== "" && draft.width > 0 && draft.height > 0;

  return (
    <div style={{ padding: "4px 0" }}>
      <div className="panel-label">Forbidden Screen Regions</div>
      {regions.length === 0 ? (
        <div
          style={{
            color: "var(--text-secondary)",
            fontSize: "var(--font-size-base)",
          }}
        >
          None. Clicks are allowed anywhere on screen.
        </div>
      ) : (
        regions.map((r, i) => (
          <div
            key={`${r.label}-${i}`}
            style={{ display: "flex", alignItems: "center", gap: 8, padding: "2px 0" }}
          >
            <span className="panel-mono" style={{ fontSize: "var(--font-size-xs)" }}>
              {r.label}: {r.width}×{r.height} at ({r.x}, {r.y})
            </span>
            <button
              className="panel-btn panel-btn-secondary"
              style={{ padding: "0 8px" }}
              onClick={() => onChange(regions.filter((_, j) => j !== i))}
              aria-label={`Remove ${r.label}`}
            >
              Remove
            </button>
          </div>
        ))
      )}
      <div style={{ display: "flex", gap: 4, marginTop: 6, flexWrap: "wrap" }}>
        <input
          className="panel-input"
          style={{ width: 120 }}
          placeholder="label"
          value={draft.label}
          onChange={(e) => setDraft({ ...draft, label: e.target.value })}
        />
        {(["x", "y", "width", "height"] as const).map((f) => (
          <input
            key={f}
            className="panel-input"
            style={{ width: 70 }}
            type="number"
            min={0}
            placeholder={f}
            value={draft[f]}
            onChange={(e) => setDraft({ ...draft, [f]: Math.max(0, +e.target.value) })}
          />
        ))}
        <button
          className="panel-btn panel-btn-secondary"
          disabled={!complete}
          style={{ opacity: complete ? 1 : 0.5 }}
          onClick={() => {
            onChange([...regions, draft]);
            setDraft({ x: 0, y: 0, width: 0, height: 0, label: "" });
          }}
        >
          Add Region
        </button>
      </div>
      <p style={hintStyle}>
        Coordinates are in the display's own units — the same ones a click uses,
        not screenshot pixels.
      </p>
    </div>
  );
}

// ── Helpers ────────────────────────────────────────────────────────────────

const clamp = (v: number, lo: number, hi: number): number =>
  Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : lo;

/** The loop config without the nested safety rails, which the API takes apart. */
function configOnly(stored: StoredConfig): LoopConfig {
  const { safety: _safety, ...rest } = stored;
  return rest;
}

/**
 * Verified count over *checked* steps, never over all of them.
 *
 * A rate whose denominator includes unverified steps reports "50%" for a
 * flawless run that simply had verification turned off. With nothing checked
 * the honest answer is `n/a`, not `0%`.
 */
function verifiedLabel(steps: Step[]): string {
  const checked = steps.filter((s) => s.verification_result !== null);
  if (checked.length === 0) return "n/a";
  const passed = checked.filter((s) => s.verification_result?.success).length;
  return `${Math.round((passed / checked.length) * 100)}% (${passed}/${checked.length})`;
}

function describeEvent(e: LoopEvent): string {
  switch (e.event) {
    case "step_started":
      return `▸ step ${e.step_num}`;
    case "screenshot_captured":
      return "  screenshot captured";
    case "llm_reasoning":
      return `  reasoning: ${e.text.slice(0, 160)}`;
    case "action_executed":
      return `  ${e.success ? "ok" : "FAILED"} ${describeAction(e.action)}`;
    case "verification_done":
      return `  verify: ${e.result.success ? "pass" : "fail"} — ${e.result.actual_observation.slice(0, 120)}`;
    case "task_completed":
      return `✓ done: ${e.summary}`;
    case "error":
      return `✕ ${e.message}`;
    case "safety_halt":
      return `⚠ halt: ${e.reason}`;
    case "approval_required":
      return `? awaiting approval: ${e.description}`;
    case "approval_resolved":
      return `  approval ${e.approved ? "granted" : "denied"}`;
    default: {
      const never: never = e;
      return JSON.stringify(never);
    }
  }
}

function eventColor(e: LoopEvent): string {
  switch (e.event) {
    case "error":
      return "var(--accent-rose)";
    case "safety_halt":
    case "approval_required":
      return "var(--warning-color)";
    case "task_completed":
      return "var(--accent-green)";
    default:
      return "var(--text-secondary)";
  }
}

const headingStyle: React.CSSProperties = {
  fontSize: "var(--font-size-lg)",
  fontWeight: 600,
  marginBottom: 12,
};

const subHeadingStyle: React.CSSProperties = {
  fontSize: "var(--font-size-md)",
  fontWeight: 600,
  marginBottom: 8,
};

const hintStyle: React.CSSProperties = {
  fontSize: "var(--font-size-xs)",
  color: "var(--text-secondary)",
  margin: "6px 0 0",
};

const placeholderStyle: React.CSSProperties = {
  background: "var(--bg-tertiary)",
  borderRadius: "var(--radius-xs-plus)",
  height: 200,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  color: "var(--text-secondary)",
  fontSize: "var(--font-size-base)",
};

const bannerStyle = (color: string): React.CSSProperties => ({
  margin: "0 12px 8px",
  padding: "8px 10px",
  borderLeft: `3px solid ${color}`,
  background: "var(--bg-secondary)",
  color,
  fontSize: "var(--font-size-sm)",
});
