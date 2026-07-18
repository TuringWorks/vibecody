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

export type RunState = "idle" | "running" | "done" | "error";

interface RunArgs {
  daemonUrl: string;
  task: string;
  provider: string;
  model?: string;
  approval: string;
  reasoning: string;
  /** When set, resume this session (continue the conversation) instead of
   *  starting a fresh one. */
  resumeSessionId?: string;
}

/**
 * Convert a finished chat's persisted events (job_events) into renderable
 * stream items, mirroring the live SSE mapping in `src-tauri/commands.rs`.
 * Consecutive `chunk` events are folded into one agent bubble; `complete` is
 * dropped (it's a duplicate summary). Used to re-render a selected chat (VX
 * bug-3).
 */
export function eventsToStreamItems(events: AgentEventPayload[]): StreamItem[] {
  const items: StreamItem[] = [];
  for (const ev of events) {
    switch (ev.type) {
      case "user":
        items.push({ kind: "user", text: ev.content ?? "" });
        break;
      case "chunk": {
        const last = items[items.length - 1];
        if (last && last.kind === "agent") last.text += ev.content ?? "";
        else items.push({ kind: "agent", text: ev.content ?? "" });
        break;
      }
      case "step":
        items.push({ kind: "tool", tool: ev.tool_name ?? "tool", summary: ev.content ?? "" });
        break;
      case "system":
        items.push({ kind: "system", text: ev.content ?? "" });
        break;
      case "error":
        items.push({ kind: "error", text: ev.content ?? "" });
        break;
      // "complete" carries only a duplicate summary — skip it.
      default:
        break;
    }
  }
  return items;
}

/**
 * Owns the live conversation for the active session (VX-105b). Subscribes to
 * the daemon's SSE events forwarded by the `stream_agent` Tauri command
 * (`agent:chunk` / `agent:complete` / `agent:error`) and appends them to the
 * stream. The daemon is the source of truth; this hook only renders what it
 * emits.
 *
 * `runTask` creates the agent session, opens the stream, and resolves with the
 * `session_id` so the caller can link it to a task (VX-112).
 */
export function useAgentStream() {
  const [items, setItems] = useState<StreamItem[]>([]);
  const [state, setState] = useState<RunState>("idle");
  const unlisteners = useRef<UnlistenFn[]>([]);

  const cleanup = useCallback(() => {
    for (const u of unlisteners.current) u();
    unlisteners.current = [];
  }, []);

  useEffect(() => cleanup, [cleanup]);

  /** Seed the stream with a previously-finished conversation (VX bug-3). The
   *  stream returns to `idle` so the composer is ready for a follow-up. */
  const loadItems = useCallback((seed: StreamItem[]) => {
    cleanup();
    setItems(seed);
    setState("idle");
  }, [cleanup]);

  /** Append to the trailing agent bubble, or start one if the last item isn't agent text. */
  const appendChunk = useCallback((chunk: string) => {
    setItems((prev) => {
      const last = prev[prev.length - 1];
      if (last && last.kind === "agent") {
        const next = prev.slice(0, -1);
        next.push({ kind: "agent", text: last.text + chunk });
        return next;
      }
      return [...prev, { kind: "agent", text: chunk }];
    });
  }, []);

  const runTask = useCallback(
    async (args: RunArgs): Promise<string | null> => {
      cleanup();
      setItems((prev) => [...prev, { kind: "user", text: args.task }]);
      setState("running");

      // Subscribe BEFORE starting the stream so we don't miss early events.
      const offChunk = await listen<string>("agent:chunk", (e) => appendChunk(e.payload));
      const offSystem = await listen<string>("agent:system", (e) => {
        setItems((prev) => [...prev, { kind: "system", text: e.payload }]);
      });
      const offStep = await listen<{ tool: string; summary: string }>("agent:step", (e) => {
        setItems((prev) => [...prev, { kind: "tool", tool: e.payload.tool, summary: e.payload.summary }]);
      });
      const offDone = await listen("agent:complete", () => {
        setState("done");
        cleanup();
      });
      const offErr = await listen<string>("agent:error", (e) => {
        setItems((prev) => [...prev, { kind: "error", text: e.payload }]);
        setState("error");
        cleanup();
      });
      unlisteners.current = [offChunk, offSystem, offStep, offDone, offErr];

      try {
        const sessionId = await invoke<string>("start_agent_session", {
          url: args.daemonUrl,
          task: args.task,
          provider: args.provider,
          model: args.model,
          approval: args.approval,
          reasoning: args.reasoning,
          resumeSessionId: args.resumeSessionId,
        });
        // Begin forwarding the daemon SSE stream into Tauri events.
        await invoke("stream_agent", { url: args.daemonUrl, sessionId });
        return sessionId;
      } catch (e) {
        setItems((prev) => [...prev, { kind: "error", text: String(e) }]);
        setState("error");
        cleanup();
        return null;
      }
    },
    [appendChunk, cleanup]
  );

  return { items, state, runTask, loadItems };
}
