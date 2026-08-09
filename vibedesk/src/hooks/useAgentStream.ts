import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AgentEventPayload } from "./useTasks";

/** A rendered item in the conversation stream. */
export type StreamItem =
  | { kind: "user"; text: string }
  | { kind: "agent"; text: string }
  | { kind: "system"; text: string }
  | { kind: "tool"; tool: string; summary: string }
  | { kind: "error"; text: string };

/**
 * `partial` is terminal like `done`, but the agent stopped with planned work
 * outstanding. It is deliberately distinct: folding it into `done` is what let
 * an unfinished run present as a finished one.
 */
export type RunState = "idle" | "running" | "done" | "partial" | "error" | "cancelled";

interface RunArgs {
  daemonUrl: string;
  task: string;
  provider: string;
  model?: string;
  approval: string;
  /** Effort tier, or undefined to leave the provider on its own default.
   *  "off" must reach the daemon as an absent field, not a value. */
  reasoning?: string;
  /** "agent" (tool loop) or "chat" (single reply, no tools). */
  mode?: string;
  /** Sandbox grants; only meaningful when mode is "sandbox". */
  sandbox?: Record<string, unknown>;
  /** When set, resume this session (continue the conversation) instead of
   *  starting a fresh one. */
  resumeSessionId?: string;
  /** Directory the run executes in — the selected project, or the task's
   *  worktree. Omitted → the daemon's own workspace root. */
  workspaceRoot?: string;
}

/** One forwarded daemon event, as reshaped by `stream_agent` in commands.rs. */
type StreamEvent =
  | { kind: "user" | "chunk" | "system" | "error"; text: string }
  | { kind: "step"; tool: string; summary: string }
  | {
      kind: "partial";
      text: string;
      steps_completed?: number | null;
      steps_planned?: number | null;
      remaining_plan?: string[] | null;
    }
  | { kind: "retry"; text: string; attempt?: number | null; max_attempts?: number | null; backoff_ms?: number | null }
  | { kind: "complete" | "closed" };

/** Render a `partial` terminal event as the notice shown in the transcript. */
function partialNotice(ev: Extract<StreamEvent, { kind: "partial" }>): string {
  const done = ev.steps_completed ?? 0;
  const planned = ev.steps_planned ?? 0;
  const head = `Stopped with work outstanding — ${done}/${planned} planned steps done.`;
  const remaining = (ev.remaining_plan ?? []).map((s, i) => `  ${done + i + 1}. ${s}`);
  return [head, ev.text, remaining.length ? `Remaining:\n${remaining.join("\n")}` : ""]
    .filter(Boolean)
    .join("\n\n");
}

/** Render a `retry` notice so a backing-off agent isn't mistaken for a hung one. */
function retryNotice(ev: Extract<StreamEvent, { kind: "retry" }>): string {
  const attempt = (ev.attempt ?? 0) + 1;
  const max = ev.max_attempts ?? "?";
  const secs = ((ev.backoff_ms ?? 0) / 1000).toFixed(1);
  return `Transient error — retrying (${attempt}/${max}) in ${secs}s: ${ev.text}`;
}

/** Tauri channel carrying one session's events. Mirrors `session_event_name`. */
const channelFor = (sessionId: string) => `agent://${sessionId}`;

/**
 * How many times a dropped stream is reattached before the run is reported as
 * lost. Bounded so a daemon that has genuinely gone away surfaces an error
 * instead of reconnecting forever.
 */
const MAX_RECONNECT_ATTEMPTS = 5;

/**
 * Append one event to the item list, folding consecutive agent chunks into a
 * single bubble and dropping a replayed `user` turn that duplicates the one the
 * composer already added optimistically.
 */
function applyEvent(prev: StreamItem[], ev: StreamEvent): StreamItem[] {
  const last = prev[prev.length - 1];
  switch (ev.kind) {
    case "chunk":
      return last?.kind === "agent"
        ? [...prev.slice(0, -1), { kind: "agent", text: last.text + ev.text }]
        : [...prev, { kind: "agent", text: ev.text }];
    case "user":
      // The live path adds the user turn optimistically before the daemon's
      // copy can arrive; a replay carries it for real. Skip the duplicate.
      return last?.kind === "user" && last.text === ev.text
        ? prev
        : [...prev, { kind: "user", text: ev.text }];
    case "system":
      return [...prev, { kind: "system", text: ev.text }];
    case "error":
      return [...prev, { kind: "error", text: ev.text }];
    case "step":
      return [...prev, { kind: "tool", tool: ev.tool, summary: ev.summary }];
    case "partial":
      return [...prev, { kind: "system", text: partialNotice(ev) }];
    case "retry":
      return [...prev, { kind: "system", text: retryNotice(ev) }];
    default:
      return prev;
  }
}

/**
 * Convert a finished chat's persisted events (job_events) into renderable
 * stream items, mirroring the live SSE mapping in `src-tauri/commands.rs`.
 * Consecutive `chunk` events are folded into one agent bubble; `complete` is
 * dropped (it's a duplicate summary). Used to re-render a selected chat.
 */
export function eventsToStreamItems(events: AgentEventPayload[]): StreamItem[] {
  return events.reduce<StreamItem[]>((acc, ev) => {
    const text = ev.content ?? "";
    switch (ev.type) {
      case "user":
      case "chunk":
      case "system":
      case "error":
        return applyEvent(acc, { kind: ev.type, text });
      case "step":
        return applyEvent(acc, { kind: "step", tool: ev.tool_name ?? "tool", summary: text });
      // Unlike "complete", a partial is not a duplicate of anything already in
      // the transcript — it is the only record that the run left work undone.
      // Dropping it made a reopened chat look like it had finished cleanly.
      case "partial":
        return applyEvent(acc, {
          kind: "partial",
          text,
          steps_completed: ev.steps_completed,
          steps_planned: ev.steps_planned,
          remaining_plan: ev.remaining_plan,
        });
      // "complete" carries only a duplicate summary — skip it.
      default:
        return acc;
    }
  }, []);
}

/**
 * Owns the live conversation for one chat. Subscribes to that session's Tauri
 * channel (`agent://<session_id>`), which `stream_agent` feeds from the
 * daemon's SSE stream. The daemon is the source of truth; this hook only
 * renders what it emits.
 *
 * Two ways in:
 *  - `runTask` starts (or resumes) a run and resolves with its `session_id`.
 *  - `attach` re-subscribes to an already-running session and replays its
 *    durable log first, so re-opening a chat mid-run catches up instead of
 *    showing a blank pane.
 *
 * `stop` cancels the in-flight run through the daemon's `/jobs/:id/cancel`.
 */
export function useAgentStream() {
  const [items, setItems] = useState<StreamItem[]>([]);
  const [state, setState] = useState<RunState>("idle");
  /** Wall-clock start of the current turn, for the elapsed-time readout. */
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const unlisteners = useRef<UnlistenFn[]>([]);
  // Synchronous re-entrancy guard. `state` alone can't gate a second run: it's
  // updated asynchronously, so two runTask calls that fire before React commits
  // `state = "running"` (StrictMode double-invoke, key autorepeat, a double
  // submit) would BOTH pass a `state`-based check and open two /agent runs +
  // two /stream forwarders — the daemon's single reply then arrives twice and
  // interleaves in one bubble. A ref flips synchronously, so it closes that gap.
  const inFlight = useRef(false);
  // Mirrors `sessionId` for callbacks that must read it without re-subscribing.
  const sessionRef = useRef<string | null>(null);
  // Set while a user-requested cancel is landing, so the daemon's terminal
  // event resolves to "cancelled" rather than a plain "done"/"error".
  const cancelling = useRef(false);
  // Daemon URL of the run in flight, so a dropped stream can be reconciled and
  // reattached without the caller passing it again.
  const daemonUrlRef = useRef<string | null>(null);
  // Consecutive reattach attempts for the current run; reset whenever a run
  // starts or reaches a terminal state.
  const reconnectsRef = useRef(0);
  // Breaks the subscribe → reconcile → attach → subscribe cycle. `subscribe`
  // reads the latest reconcile through this ref instead of depending on it.
  const reconcileRef = useRef<((sid: string) => Promise<void>) | null>(null);

  const cleanup = useCallback(() => {
    for (const u of unlisteners.current) u();
    unlisteners.current = [];
    inFlight.current = false;
  }, []);

  useEffect(() => cleanup, [cleanup]);

  /**
   * Subscribe to a session's channel. Returns once the listener is live, so
   * callers can subscribe before asking the daemon to stream (no lost events).
   */
  const subscribe = useCallback(
    async (sid: string) => {
      const off = await listen<StreamEvent>(channelFor(sid), (e) => {
        const ev = e.payload;
        if (ev.kind === "closed") {
          cleanup();
          if (cancelling.current) {
            cancelling.current = false;
            setState((s) => (s === "running" ? "cancelled" : s));
            return;
          }
          // The daemon now always emits a terminal event before it closes the
          // stream, so reaching here while still "running" means the *transport*
          // dropped — a network blip, a daemon restart, the laptop sleeping —
          // not that the run ended. Settling to "done" (what this used to do)
          // told the user their task had finished while the agent was still
          // working on it. Ask the daemon what actually happened instead.
          const reconcile = reconcileRef.current;
          if (reconcile) {
            void reconcile(sid);
          } else {
            setState((s) => (s === "running" ? "error" : s));
          }
          return;
        }
        if (ev.kind === "complete") {
          // A real terminal event: the run is over, so the reconnect allowance
          // resets rather than carrying into the next turn of this chat.
          reconnectsRef.current = 0;
          setState(cancelling.current ? "cancelled" : "done");
          return;
        }
        // Terminal and unfinished. Must be handled before the generic append
        // below, and must not settle as "done": the `closed` fallback further
        // up resolves a still-"running" stream to "done", so letting `partial`
        // fall through would report the unfinished run as a success.
        if (ev.kind === "partial") {
          reconnectsRef.current = 0;
          setItems((prev) => applyEvent(prev, ev));
          setState(cancelling.current ? "cancelled" : "partial");
          return;
        }
        if (ev.kind === "error") {
          reconnectsRef.current = 0;
          setItems((prev) => applyEvent(prev, ev));
          setState(cancelling.current ? "cancelled" : "error");
          return;
        }
        setItems((prev) => applyEvent(prev, ev));
      });
      unlisteners.current = [...unlisteners.current, off];
    },
    [cleanup]
  );

  /** Append a local system note (e.g. `/help` output) without touching the run
   *  state — these are UI messages, not daemon events, so they are never
   *  persisted and vanish when the chat is reloaded. */
  const appendSystem = useCallback((text: string) => {
    setItems((prev) => [...prev, { kind: "system", text }]);
  }, []);

  /** Seed the stream with a previously-finished conversation. The stream
   *  returns to `idle` so the composer is ready for a follow-up. */
  const loadItems = useCallback(
    (seed: StreamItem[], sid?: string | null) => {
      cleanup();
      setItems(seed);
      setState("idle");
      setStartedAt(null);
      sessionRef.current = sid ?? null;
      setSessionId(sid ?? null);
    },
    [cleanup]
  );

  /**
   * Re-attach to a session whose run is still in flight: replay its durable log
   * from the beginning, then follow it live. Used when a chat that was left
   * running is re-opened.
   *
   * Returns false when the daemon has no live stream for the session — it 404s
   * `/stream/:id` once a run ends, and a task can sit stale-marked "running"
   * (app closed mid-run, daemon restarted). The caller falls back to the static
   * history in that case, so a finished chat shows its transcript rather than
   * an error. The stream is left untouched on failure so nothing is clobbered.
   */
  const attach = useCallback(
    async (daemonUrl: string, sid: string): Promise<boolean> => {
      if (inFlight.current) return false;
      cleanup();
      inFlight.current = true;
      sessionRef.current = sid;
      setSessionId(sid);
      setItems([]);
      setState("running");
      setStartedAt(Date.now());
      daemonUrlRef.current = daemonUrl;
      await subscribe(sid);
      try {
        await invoke("stream_agent", { url: daemonUrl, sessionId: sid, sinceSeq: 0 });
        return true;
      } catch {
        cleanup();
        setState("idle");
        setStartedAt(null);
        return false;
      }
    },
    [cleanup, subscribe]
  );

  /**
   * Decide what a dropped stream actually meant, and recover if the run is
   * still alive.
   *
   * The daemon is authoritative: `/jobs/:id` knows whether the run finished,
   * and its durable event log can be replayed from the start, so a transport
   * failure is recoverable rather than terminal. Only when the job really is
   * terminal — or when reattaching keeps failing — do we settle, and then we
   * settle to what the daemon says rather than assuming success.
   */
  const reconcile = useCallback(
    async (sid: string) => {
      const url = daemonUrlRef.current;
      if (!url) {
        setState((s) => (s === "running" ? "error" : s));
        return;
      }

      let status: string | null = null;
      try {
        const job = await invoke<{ status?: string }>("get_job", { url, sessionId: sid });
        status = job?.status ?? null;
      } catch {
        // Daemon unreachable — fall through to the reattach path, which
        // retries and eventually reports the lost connection honestly.
      }

      // Terminal per the daemon: report *its* outcome, not a guess.
      if (status && status !== "running" && status !== "queued") {
        reconnectsRef.current = 0;
        setState(
          status === "failed"
            ? "error"
            : status === "cancelled"
              ? "cancelled"
              : status === "partial"
                ? "partial"
                : "done"
        );
        return;
      }

      if (reconnectsRef.current >= MAX_RECONNECT_ATTEMPTS) {
        setItems((prev) => [
          ...prev,
          {
            kind: "error",
            text:
              "Lost the connection to the daemon and could not reattach. The run may " +
              "still be going — reopen this chat to catch up once the daemon is back.",
          },
        ]);
        setState("error");
        return;
      }

      reconnectsRef.current += 1;
      // `attach` replays the durable log from seq 0, so the transcript is
      // rebuilt rather than resumed — nothing streamed before the drop is lost.
      const ok = await attach(url, sid);
      if (!ok) {
        setItems((prev) => [
          ...prev,
          {
            kind: "error",
            text: "Connection to the daemon dropped and the run could not be reattached.",
          },
        ]);
        setState("error");
      }
    },
    [attach]
  );

  // `subscribe` reaches reconcile through a ref, so the two can call each other
  // without a circular `useCallback` dependency.
  useEffect(() => {
    reconcileRef.current = reconcile;
  }, [reconcile]);

  const runTask = useCallback(
    async (args: RunArgs): Promise<string | null> => {
      // Re-entrancy guard: bail if a run is already streaming. Claiming the run
      // here — synchronously, before the first `await` — means a second call
      // that races the async `setState("running")` is rejected instead of
      // opening a duplicate /agent run + /stream forwarder.
      if (inFlight.current) return null;
      cleanup();
      inFlight.current = true;
      cancelling.current = false;
      // A fresh run gets a fresh reconnect allowance, and its own daemon URL
      // for the reconcile path.
      reconnectsRef.current = 0;
      daemonUrlRef.current = args.daemonUrl;
      setItems((prev) => [...prev, { kind: "user", text: args.task }]);
      setState("running");
      setStartedAt(Date.now());

      try {
        const sid = await invoke<string>("start_agent_session", {
          url: args.daemonUrl,
          task: args.task,
          provider: args.provider,
          model: args.model,
          approval: args.approval,
          reasoning: args.reasoning,
          mode: args.mode,
          sandbox: args.sandbox,
          resumeSessionId: args.resumeSessionId,
          workspaceRoot: args.workspaceRoot,
        });
        sessionRef.current = sid;
        setSessionId(sid);
        // Subscribe BEFORE the daemon stream opens so no early event is lost.
        await subscribe(sid);
        await invoke("stream_agent", { url: args.daemonUrl, sessionId: sid });
        return sid;
      } catch (e) {
        setItems((prev) => [...prev, { kind: "error", text: String(e) }]);
        setState("error");
        cleanup();
        return null;
      }
    },
    [cleanup, subscribe]
  );

  /**
   * Cancel the in-flight run. The daemon marks the job cancelled and closes its
   * stream; the terminal event then settles the UI as "cancelled". Safe to call
   * when nothing is running (no session → no-op).
   */
  const stop = useCallback(async (daemonUrl: string) => {
    const sid = sessionRef.current;
    if (!sid) return;
    cancelling.current = true;
    try {
      await invoke("cancel_agent_session", { url: daemonUrl, sessionId: sid });
    } catch (e) {
      // A cancel that can't reach the daemon must still free the UI, otherwise
      // the composer stays disabled with no way out.
      setItems((prev) => [...prev, { kind: "error", text: `Stop failed: ${String(e)}` }]);
      setState("error");
      cancelling.current = false;
    }
  }, []);

  return { items, state, sessionId, startedAt, runTask, attach, stop, loadItems, appendSystem };
}
