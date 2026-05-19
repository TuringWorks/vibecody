/**
 * VibeCLI Daemon API client.
 *
 * Wraps the HTTP API exposed by `vibecli serve` at http://localhost:<port>.
 *
 * @example
 * ```ts
 * const client = new VibeCLIClient({ port: 7878 });
 * const { sessionId } = await client.startAgent('Fix the failing test');
 * for await (const event of client.streamAgent(sessionId)) {
 *   if (event.type === 'chunk') process.stdout.write(event.content ?? '');
 *   if (event.type === 'complete') break;
 * }
 * ```
 */

export interface ClientOptions {
  port?: number;
  host?: string;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface AgentEvent {
  // G6.3 / G7.1 — `system` is a daemon-issued advisory message that
  // isn't a model token, tool step, completion, or error. Today it
  // carries the "Auto-linked to pinned goal …" attribution emitted
  // by `auto_link_to_pinned_goal`.
  type: 'chunk' | 'step' | 'complete' | 'error' | 'system';
  content?: string;
  step_num?: number;
  tool_name?: string;
  success?: boolean;
}

export interface JobRecord {
  session_id: string;
  task: string;
  status: 'running' | 'complete' | 'failed' | 'cancelled';
  provider: string;
  started_at: number;
  finished_at?: number;
  summary?: string;
}

/** Durable execution intent (G1.7). Subset of the daemon's full `Goal` —
 *  rich fields (plan, criteria, tags) are accessible via the raw JSON path
 *  for clients that want them. */
export interface ExecGoalSummary {
  id: string;
  title: string;
  status: 'active' | 'paused' | 'done' | 'abandoned';
  workspace?: string | null;
  statement: string;
  created_at: string;
  updated_at: string;
}

export class VibeCLIClient {
  private baseUrl: string;

  constructor(options: ClientOptions = {}) {
    const host = options.host ?? 'localhost';
    const port = options.port ?? 7878;
    this.baseUrl = `http://${host}:${port}`;
  }

  /** Check daemon liveness. Resolves `true` if reachable. */
  async isAlive(): Promise<boolean> {
    try {
      const res = await fetch(`${this.baseUrl}/health`);
      return res.ok;
    } catch {
      return false;
    }
  }

  /** Single-turn chat (non-streaming). */
  async chat(messages: ChatMessage[]): Promise<string> {
    const res = await fetch(`${this.baseUrl}/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ messages }),
    });
    if (!res.ok) {
      throw new Error(`Chat failed: ${res.status} ${await res.text()}`);
    }
    const data = await res.json() as { content: string };
    return data.content;
  }

  /** Streaming chat: yields text chunks from the daemon. */
  async *chatStream(messages: ChatMessage[]): AsyncGenerator<string> {
    const res = await fetch(`${this.baseUrl}/chat/stream`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ messages }),
    });
    if (!res.ok || !res.body) {
      throw new Error(`Chat stream failed: ${res.status}`);
    }
    yield* readSseText(res.body);
  }

  /** Start an agent task. Returns the session_id for streaming. */
  async startAgent(task: string, approval?: string): Promise<{ sessionId: string }> {
    const res = await fetch(`${this.baseUrl}/agent`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ task, approval }),
    });
    if (!res.ok) {
      throw new Error(`Agent start failed: ${res.status} ${await res.text()}`);
    }
    const data = await res.json() as { session_id: string };
    return { sessionId: data.session_id };
  }

  /** List recent background jobs. */
  async listJobs(): Promise<JobRecord[]> {
    try {
      const res = await fetch(`${this.baseUrl}/jobs`);
      if (!res.ok) return [];
      return (await res.json()) as JobRecord[];
    } catch {
      return [];
    }
  }

  /** Cancel a running job. */
  async cancelJob(sessionId: string): Promise<void> {
    await fetch(`${this.baseUrl}/jobs/${sessionId}/cancel`, { method: 'POST' });
  }

  // ── /goal — durable execution intent (G1.7) ──────────────────

  /** List goals, optionally filtered by status. Returns `[]` on any failure
   *  so VS Code can render an empty list without a notification storm. */
  async listGoals(status?: ExecGoalSummary['status']): Promise<ExecGoalSummary[]> {
    try {
      const qs = status ? `?status=${encodeURIComponent(status)}` : '';
      const res = await fetch(`${this.baseUrl}/v1/goals${qs}`);
      if (!res.ok) return [];
      const data = (await res.json()) as { goals?: ExecGoalSummary[] };
      return data.goals ?? [];
    } catch {
      return [];
    }
  }

  /** Create a goal. Title is required; statement and workspace are optional. */
  async createGoal(title: string, statement?: string, workspace?: string): Promise<ExecGoalSummary> {
    const res = await fetch(`${this.baseUrl}/v1/goals`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title, statement: statement ?? '', workspace: workspace ?? null }),
    });
    if (!res.ok) {
      throw new Error(`Goal create failed: ${res.status} ${await res.text()}`);
    }
    return (await res.json()) as ExecGoalSummary;
  }

  /** G13.1 — pinned-goal ids visible from VS Code. Returns the union
   *  of the global pin and the optional workspace pin so the goals
   *  tree can ★-mark either case without two round-trips at every
   *  refresh. Empty list on any failure (keeps the tree quiet). */
  async getPinnedGoalIds(workspace?: string): Promise<string[]> {
    const ids = new Set<string>();
    const fetchOne = async (qs: string) => {
      try {
        const res = await fetch(`${this.baseUrl}/v1/goals/current${qs}`);
        if (!res.ok) return;
        const data = (await res.json()) as { goal_id?: string | null };
        if (data.goal_id) ids.add(data.goal_id);
      } catch {
        /* silent — empty pin set is the right fallback for the tree */
      }
    };
    // Global pin (workspace=""): the most common case mobile/watch hit.
    await fetchOne('');
    if (workspace) {
      await fetchOne(`?workspace=${encodeURIComponent(workspace)}`);
    }
    return [...ids];
  }

  /** Start a new session bound to a goal. Returns the new session id. */
  async startGoal(goalId: string, task?: string): Promise<{ sessionId: string }> {
    const res = await fetch(`${this.baseUrl}/v1/goals/${encodeURIComponent(goalId)}/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ task: task ?? null }),
    });
    if (!res.ok) {
      throw new Error(`Goal start failed: ${res.status} ${await res.text()}`);
    }
    const data = await res.json() as { session_id: string };
    return { sessionId: data.session_id };
  }

  /** Stream agent events for a running session. */
  async *streamAgent(sessionId: string): AsyncGenerator<AgentEvent> {
    const res = await fetch(`${this.baseUrl}/stream/${sessionId}`);
    if (!res.ok || !res.body) {
      throw new Error(`Stream not found: ${sessionId}`);
    }
    for await (const data of readSseData(res.body)) {
      try {
        const event: AgentEvent = JSON.parse(data);
        yield event;
        if (event.type === 'complete' || event.type === 'error') break;
      } catch {
        // Ignore unparseable events
      }
    }
  }
}

// ── SSE helpers ────────────────────────────────────────────────────────────────

async function *readSseText(body: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() ?? '';
      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const text = line.slice(6);
          if (text) yield text;
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}

async function *readSseData(body: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  yield* readSseText(body);
}
