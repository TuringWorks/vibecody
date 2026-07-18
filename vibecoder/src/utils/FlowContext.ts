/**
 * Cascade Flows — unified context store for all AI interactions.
 *
 * Collects events from Chat, Inline Edits, Agent steps, and Terminal commands
 * into a single chronological timeline so every AI interaction can reference
 * the full history of what just happened in the workspace.
 *
 * Designed to be 100% in-memory / frontend-only (no Tauri roundtrip needed).
 */

export type FlowEventKind =
  | "chat"          // user sent a chat message + received reply
  | "diffcomplete"  // ⌘. diff-mode AI edit was applied
  | "agent_step"    // agent executed a tool call
  | "agent_complete"// agent finished a task
  | "agent_partial" // agent stopped with incomplete plan
  | "terminal_cmd"  // user ran a command in the terminal
  | "file_edit";    // user directly edited a file

export interface FlowEvent {
  id: string;
  kind: FlowEventKind;
  /** Short one-line description shown in the timeline. */
  summary: string;
  /** Full detail — injected as context when requested. */
  detail: string;
  /** Milliseconds since epoch. */
  timestamp: number;
  /** Associated file path (if any). */
  filePath?: string;
  /** Token estimate (for context budget). */
  approxTokens: number;
}

const MAX_EVENTS = 200;
const TRUNCATE_DETAIL_AT = 2000; // chars per event detail

let _idCounter = 0;
function nextId(): string {
  return `flow-${Date.now()}-${++_idCounter}`;
}

function approxTokenCount(text: string): number {
  // Rough heuristic: 1 token ≈ 4 chars
  return Math.ceil(text.length / 4);
}

// ── FlowContextManager ────────────────────────────────────────────────────────

class FlowContextManager {
  private events: FlowEvent[] = [];
  private listeners: Array<(events: FlowEvent[]) => void> = [];

  /** Add a new flow event. Thread-safe (single JS thread). */
  add(params: Omit<FlowEvent, "id" | "timestamp" | "approxTokens">): FlowEvent {
    const detail = params.detail.slice(0, TRUNCATE_DETAIL_AT);
    const event: FlowEvent = {
      ...params,
      detail,
      id: nextId(),
      timestamp: Date.now(),
      approxTokens: approxTokenCount(params.summary + detail),
    };
    this.events.push(event);
    if (this.events.length > MAX_EVENTS) {
      this.events.shift();
    }
    this.notify();
    return event;
  }

  /** Return a snapshot of all events. */
  getAll(): FlowEvent[] {
    return [...this.events];
  }

  /** Return events filtered by kind. */
  getByKind(kind: FlowEventKind): FlowEvent[] {
    return this.events.filter((e) => e.kind === kind);
  }

  /** Return the N most recent events. */
  getRecent(n: number): FlowEvent[] {
    return this.events.slice(-n);
  }

  /**
   * Build a compact context summary suitable for injecting into an AI prompt.
   *
   * @param tokenBudget  Maximum tokens to consume (default 2000)
   * @param kinds        Only include these kinds (undefined = all)
   */
  getContextSummary(tokenBudget = 2000, kinds?: FlowEventKind[]): string {
    const pool = (kinds
      ? this.events.filter((e) => kinds.includes(e.kind))
      : this.events
    ).slice().reverse(); // newest first

    const lines: string[] = ["=== Recent Workspace Activity ==="];
    let used = approxTokenCount(lines[0]);

    for (const ev of pool) {
      const timeAgo = formatTimeAgo(ev.timestamp);
      const header = `[${ev.kind.toUpperCase()} ${timeAgo}] ${ev.summary}`;
      const block = ev.detail
        ? `${header}\n${ev.detail}`
        : header;
      const cost = approxTokenCount(block);
      if (used + cost > tokenBudget) break;
      lines.push(block);
      used += cost;
    }

    return lines.join("\n\n");
  }

  /** Clear all stored events. */
  clear(): void {
    this.events = [];
    this.notify();
  }

  /** Subscribe to event list changes. Returns an unsubscribe function. */
  subscribe(fn: (events: FlowEvent[]) => void): () => void {
    this.listeners.push(fn);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== fn);
    };
  }

  private notify(): void {
    const snapshot = [...this.events];
    this.listeners.forEach((fn) => fn(snapshot));
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatTimeAgo(ms: number): string {
  const secs = Math.floor((Date.now() - ms) / 1000);
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  return `${hrs}h ago`;
}

// ── Singleton export ──────────────────────────────────────────────────────────

/** Global singleton — import this everywhere, never construct `new FlowContextManager()`. */
export const flowContext = new FlowContextManager();
