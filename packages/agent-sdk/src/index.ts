/**
 * @vibecody/agent-sdk
 *
 * TypeScript SDK for building custom agents with VibeCLI infrastructure.
 *
 * Communicates with a local VibeCLI daemon (`vibecli serve`).
 *
 * @example
 * ```ts
 * import { VibeCLIAgent } from '@vibecody/agent-sdk';
 *
 * const agent = new VibeCLIAgent({
 *   provider: 'claude',
 *   approval: 'full-auto',
 * });
 *
 * for await (const event of agent.run('Add TypeScript strict mode to all files')) {
 *   if (event.type === 'step') console.log(`[${event.tool_name}] ${event.tool_name}`);
 *   if (event.type === 'complete') console.log('Done:', event.content);
 * }
 * ```
 */

// ── Types ─────────────────────────────────────────────────────────────────────

export interface AgentOptions {
  /** AI provider: 'ollama' | 'claude' | 'openai' | 'gemini' | 'grok'. Default: 'ollama' */
  provider?: string;
  /** Tool call approval policy. Default: 'suggest' */
  approval?: 'suggest' | 'auto-edit' | 'full-auto';
  /** VibeCLI daemon port. Default: 7878 */
  port?: number;
  /** VibeCLI daemon host. Default: 'localhost' */
  host?: string;
}

// G6.3 / G7.1 — `system` is a daemon-issued advisory message that's
// not a model token, tool step, completion, or error. Today it carries
// the "Auto-linked to pinned goal …" attribution emitted by
// `auto_link_to_pinned_goal` so SDK / VibeUI / CLI consumers can
// render it as a distinct attribution chip before the model's first
// token.
export type AgentEventType = 'chunk' | 'step' | 'complete' | 'error' | 'system';

export interface AgentEvent {
  type: AgentEventType;
  /** Text content (for 'chunk' and 'complete' events) */
  content?: string;
  /** Step index (0-based) for 'step' events */
  step_num?: number;
  /** Tool name for 'step' events */
  tool_name?: string;
  /** Whether the tool call succeeded for 'step' events */
  success?: boolean;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface HookConfig {
  event: 'PreToolUse' | 'PostToolUse' | 'SessionStart' | 'TaskCompleted' | 'Stop';
  /** Tool names to match (empty = all tools) */
  tools?: string[];
  /** Shell command to run as the hook */
  command: string;
}

export interface JobRecord {
  /** Unique session identifier */
  session_id: string;
  /** Natural-language task description */
  task: string;
  /** Job status */
  status: 'running' | 'complete' | 'failed' | 'cancelled';
  /** AI provider used */
  provider: string;
  /** Unix milliseconds when the job started */
  started_at: number;
  /** Unix milliseconds when the job finished (if done) */
  finished_at?: number;
  /** Short completion summary from the agent */
  summary?: string;
}

// ── VibeCLIAgent ─────────────────────────────────────────────────────────────

/**
 * High-level agent interface. Wraps the VibeCLI daemon API.
 *
 * @example
 * ```ts
 * const agent = new VibeCLIAgent({ provider: 'claude', approval: 'full-auto' });
 * for await (const event of agent.run('Write unit tests for auth.ts')) {
 *   console.log(event);
 * }
 * ```
 */
// ── /goal — durable execution intent (G1.7) ────────────────────────────────

export type GoalStatus = 'active' | 'paused' | 'done' | 'abandoned';
export type GoalLinkKind = 'session' | 'job' | 'recap' | 'note';

export interface Goal {
  id: string;
  title: string;
  statement: string;
  status: GoalStatus;
  workspace?: string | null;
  success_criteria: string[];
  tags: string[];
  created_at: string;
  updated_at: string;
  parent_goal_id?: string | null;
  /** `ExecutionPlan` mirror — left loose so the SDK doesn't bind the
   *  full vibe-ai planner schema. */
  current_plan?: Record<string, unknown> | null;
  schema_version: number;
}

export interface GoalLink {
  id: string;
  goal_id: string;
  kind: GoalLinkKind;
  target_id: string;
  linked_at: string;
  note?: string | null;
}

export interface GoalDetail {
  goal: Goal;
  links: GoalLink[];
}

export interface GoalCreateInput {
  title: string;
  statement?: string;
  workspace?: string | null;
  success_criteria?: string[];
  tags?: string[];
  parent_goal_id?: string | null;
}

export interface GoalPatch {
  title?: string;
  statement?: string;
  status?: GoalStatus;
  success_criteria?: string[];
  tags?: string[];
  /** `null` clears the workspace (sets to global); omit to leave alone. */
  workspace?: string | null;
  /** `null` clears the parent (promotes to root); omit to leave alone. */
  parent_goal_id?: string | null;
}

// G5.3 — recursive subtree response from `/v1/goals/:id/tree`.
export interface GoalTreeNode {
  goal: Goal;
  children: GoalTreeNode[];
  /** Set when the depth budget cut off this node's descendants. */
  truncated?: boolean;
  /** Direct-child count when `truncated` is set. */
  direct_child_count?: number;
  /** Set when this node was re-visited via a cycle in `parent_goal_id`. */
  cycle?: boolean;
}

export interface GoalTreeResponse {
  root: string;
  depth: number;
  tree: GoalTreeNode;
}

// G5.3 — pin lookup response shape.
export interface GoalCurrentResponse {
  workspace: string | null;
  goal_id: string | null;
  pinned_at?: string;
  goal?: Goal;
}

// G5.3 — aggregate recap response (heuristic or LLM-synthesized).
export interface GoalRecapResponse {
  goal_id: string;
  title: string;
  headline: string;
  bullets: string[];
  next_actions: string[];
  artifacts: Array<Record<string, unknown>>;
  sources: Array<{ recap_id: string; kind: string; target_id: string }>;
  generated_at: string;
  recap_synthesizer: 'heuristic' | 'llm';
  recap_provider_override_applied?: boolean;
  recap_provider_requested?: string;
  recap_model_requested?: string;
  recap_llm_error?: string;
}

export class VibeCLIAgent {
  private baseUrl: string;
  private approval: string;
  /** Session ID of the most-recently started run (set by `run()`). */
  private lastSessionId: string | null = null;

  constructor(options: AgentOptions = {}) {
    const host = options.host ?? 'localhost';
    const port = options.port ?? 7878;
    this.baseUrl = `http://${host}:${port}`;
    this.approval = options.approval ?? 'suggest';
  }

  /**
   * Run an agent task. Returns an async generator that yields events.
   *
   * @param task  Natural-language task description.
   * @param approval  Override approval policy for this run.
   */
  async *run(task: string, approval?: string): AsyncGenerator<AgentEvent> {
    const policy = approval ?? this.approval;

    // Start the agent
    const startRes = await fetch(`${this.baseUrl}/agent`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ task, approval: policy }),
    });

    if (!startRes.ok) {
      const body = await startRes.text();
      throw new AgentError(`Failed to start agent: ${startRes.status} ${body}`);
    }

    const { session_id } = await startRes.json() as { session_id: string };
    this.lastSessionId = session_id;

    // Stream events
    const streamRes = await fetch(`${this.baseUrl}/stream/${session_id}`);
    if (!streamRes.ok || !streamRes.body) {
      throw new AgentError(`Failed to open event stream: ${streamRes.status}`);
    }

    yield* this._parseEventStream(streamRes.body);
  }

  /**
   * Single-turn chat (non-streaming).
   */
  async chat(messages: ChatMessage[]): Promise<string> {
    const res = await fetch(`${this.baseUrl}/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ messages }),
    });
    if (!res.ok) {
      throw new AgentError(`Chat failed: ${res.status} ${await res.text()}`);
    }
    const data = await res.json() as { content: string };
    return data.content;
  }

  /**
   * Streaming chat — yields text tokens as they arrive.
   */
  async *chatStream(messages: ChatMessage[]): AsyncGenerator<string> {
    const res = await fetch(`${this.baseUrl}/chat/stream`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ messages }),
    });
    if (!res.ok || !res.body) {
      throw new AgentError(`Chat stream failed: ${res.status}`);
    }
    for await (const data of readSseLines(res.body)) {
      yield data;
    }
  }

  /**
   * List all background jobs (sorted newest-first).
   */
  async listJobs(): Promise<JobRecord[]> {
    const res = await fetch(`${this.baseUrl}/jobs`);
    if (!res.ok) {
      throw new AgentError(`listJobs failed: ${res.status} ${await res.text()}`);
    }
    return res.json() as Promise<JobRecord[]>;
  }

  /**
   * Get a single job by session ID. Returns null if not found.
   */
  async getJob(sessionId: string): Promise<JobRecord | null> {
    const res = await fetch(`${this.baseUrl}/jobs/${encodeURIComponent(sessionId)}`);
    if (res.status === 404) return null;
    if (!res.ok) {
      throw new AgentError(`getJob failed: ${res.status} ${await res.text()}`);
    }
    return res.json() as Promise<JobRecord>;
  }

  /**
   * Stop the most recently started agent run (equivalent to `cancelJob(lastSessionId)`).
   * No-op if no run has been started or the job is already finished.
   */
  async stop(): Promise<void> {
    if (!this.lastSessionId) return;
    await this.cancelJob(this.lastSessionId);
    this.lastSessionId = null;
  }

  /**
   * Cancel a running job. No-op if the job is already finished.
   */
  async cancelJob(sessionId: string): Promise<void> {
    const res = await fetch(`${this.baseUrl}/jobs/${encodeURIComponent(sessionId)}/cancel`, {
      method: 'POST',
    });
    if (!res.ok) {
      throw new AgentError(`cancelJob failed: ${res.status} ${await res.text()}`);
    }
  }

  // ── /goal — durable execution intent (G1.7) ──────────────────────────────
  //
  // Exposed as `agent.goals.*` so SDK consumers can read/write goals without
  // bumping the public surface on `VibeCLIAgent` itself. Each method is a
  // thin proxy to /v1/goals; richer fields (plan, criteria) round-trip
  // verbatim through `Record<string, unknown>`.

  readonly goals = {
    list: async (filter?: { status?: string; workspace?: string; tag?: string; limit?: number }): Promise<Goal[]> => {
      const qs = new URLSearchParams();
      if (filter?.status)    qs.set('status', filter.status);
      if (filter?.workspace) qs.set('workspace', filter.workspace);
      if (filter?.tag)       qs.set('tag', filter.tag);
      if (filter?.limit)     qs.set('limit', String(filter.limit));
      const url = `${this.baseUrl}/v1/goals${qs.size ? `?${qs}` : ''}`;
      const res = await fetch(url);
      if (!res.ok) throw new AgentError(`goals.list failed: ${res.status} ${await res.text()}`);
      const data = (await res.json()) as { goals?: Goal[] };
      return data.goals ?? [];
    },
    get: async (id: string): Promise<GoalDetail> => {
      const res = await fetch(`${this.baseUrl}/v1/goals/${encodeURIComponent(id)}`);
      if (!res.ok) throw new AgentError(`goals.get failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<GoalDetail>;
    },
    create: async (body: GoalCreateInput): Promise<Goal> => {
      const res = await fetch(`${this.baseUrl}/v1/goals`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new AgentError(`goals.create failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Goal>;
    },
    update: async (id: string, patch: GoalPatch): Promise<Goal> => {
      const res = await fetch(`${this.baseUrl}/v1/goals/${encodeURIComponent(id)}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(patch),
      });
      if (!res.ok) throw new AgentError(`goals.update failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Goal>;
    },
    delete: async (id: string): Promise<void> => {
      const res = await fetch(`${this.baseUrl}/v1/goals/${encodeURIComponent(id)}`, { method: 'DELETE' });
      if (!res.ok && res.status !== 404) {
        throw new AgentError(`goals.delete failed: ${res.status} ${await res.text()}`);
      }
    },
    plan: async (id: string, provider?: string, model?: string): Promise<Goal> => {
      const res = await fetch(`${this.baseUrl}/v1/goals/${encodeURIComponent(id)}/plan`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ provider: provider ?? null, model: model ?? null }),
      });
      if (!res.ok) throw new AgentError(`goals.plan failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Goal>;
    },
    start: async (id: string, task?: string): Promise<{ session_id: string; link_id: string; goal_id: string }> => {
      const res = await fetch(`${this.baseUrl}/v1/goals/${encodeURIComponent(id)}/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ task: task ?? null }),
      });
      if (!res.ok) throw new AgentError(`goals.start failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<{ session_id: string; link_id: string; goal_id: string }>;
    },
    link: async (id: string, kind: 'session' | 'job' | 'recap' | 'note', target_id: string, note?: string): Promise<GoalLink> => {
      const res = await fetch(`${this.baseUrl}/v1/goals/${encodeURIComponent(id)}/link`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ kind, target_id, note: note ?? null }),
      });
      if (!res.ok) throw new AgentError(`goals.link failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<GoalLink>;
    },

    // G5.3 — tree + pin + recap-LLM surface, mirroring the new
    // `/v1/goals/:id/tree` and `/v1/goals/current` endpoints.

    /** Recursive subtree walk. `depth` is clamped server-side to [1, 10]
     *  (default 3). Re-visited nodes carry `cycle: true`; nodes whose
     *  descendants weren't expanded carry `truncated: true`. */
    tree: async (id: string, depth?: number): Promise<GoalTreeResponse> => {
      const qs = depth ? `?depth=${depth}` : '';
      const res = await fetch(
        `${this.baseUrl}/v1/goals/${encodeURIComponent(id)}/tree${qs}`,
      );
      if (!res.ok) throw new AgentError(`goals.tree failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<GoalTreeResponse>;
    },

    /** Pin a goal as the "current" execution intent for a workspace
     *  (empty/absent `workspace` → cross-workspace global slot). 404
     *  on unknown goal id; otherwise 200. */
    pin: async (id: string, workspace?: string): Promise<{ workspace: string | null; goal_id: string }> => {
      const res = await fetch(`${this.baseUrl}/v1/goals/current`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ goal_id: id, workspace: workspace ?? null }),
      });
      if (!res.ok) throw new AgentError(`goals.pin failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<{ workspace: string | null; goal_id: string }>;
    },

    /** Clear the pin for a workspace (or the global slot). */
    unpin: async (workspace?: string): Promise<{ workspace: string | null; removed: boolean }> => {
      const qs = workspace ? `?workspace=${encodeURIComponent(workspace)}` : '';
      const res = await fetch(`${this.baseUrl}/v1/goals/current${qs}`, { method: 'DELETE' });
      if (!res.ok) throw new AgentError(`goals.unpin failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<{ workspace: string | null; removed: boolean }>;
    },

    /** Look up the pinned goal. `goal_id: null` when nothing is pinned. */
    current: async (workspace?: string): Promise<GoalCurrentResponse> => {
      const qs = workspace ? `?workspace=${encodeURIComponent(workspace)}` : '';
      const res = await fetch(`${this.baseUrl}/v1/goals/current${qs}`);
      if (!res.ok) throw new AgentError(`goals.current failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<GoalCurrentResponse>;
    },

    /** Cross-store aggregate recap. Pass `provider` + `model` to get
     *  LLM synthesis (response carries `recap_synthesizer: "llm"`);
     *  omit both for the heuristic fold (`"heuristic"`). */
    recap: async (
      id: string,
      opts?: { provider?: string; model?: string },
    ): Promise<GoalRecapResponse> => {
      const body = opts ? { provider: opts.provider ?? null, model: opts.model ?? null } : {};
      const res = await fetch(`${this.baseUrl}/v1/goals/${encodeURIComponent(id)}/recap`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new AgentError(`goals.recap failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<GoalRecapResponse>;
    },
  };

  // kodegraph code-knowledge-graph surface. Exposed as `agent.graph.*` — a thin
  // proxy to /v1/graph/*. No LLM call, so no provider/model. Responses are
  // untyped JSON (kodegraph shapes are daemon-owned; the SDK stays decoupled).

  /** `/v1/graph/status` — `{status:"ready"|"indexing"|"disabled", node_count, edge_count, last_built_at?}`. */
  readonly graph = {
    status: async (): Promise<Record<string, unknown>> => {
      const res = await fetch(`${this.baseUrl}/v1/graph/status`);
      if (!res.ok) throw new AgentError(`graph.status failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Record<string, unknown>>;
    },
    /** `POST /v1/graph/build` — kicks off a background build, returns `{status:"indexing"}`. */
    build: async (): Promise<Record<string, unknown>> => {
      const res = await fetch(`${this.baseUrl}/v1/graph/build`, { method: 'POST' });
      if (!res.ok) throw new AgentError(`graph.build failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Record<string, unknown>>;
    },
    /** `POST /v1/graph/query {query, budget?}` — token-budgeted subgraph. */
    query: async (query: string, budget?: number): Promise<Record<string, unknown>> => {
      const res = await fetch(`${this.baseUrl}/v1/graph/query`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query, budget: budget ?? 2000 }),
      });
      if (!res.ok) throw new AgentError(`graph.query failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Record<string, unknown>>;
    },
    /** `GET /v1/graph/node/:name` — one node's payload. */
    node: async (name: string): Promise<Record<string, unknown>> => {
      const res = await fetch(`${this.baseUrl}/v1/graph/node/${encodeURIComponent(name)}`);
      if (!res.ok) throw new AgentError(`graph.node failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Record<string, unknown>>;
    },
    /** `GET /v1/graph/neighbors/:name` — adjacent nodes. */
    neighbors: async (name: string): Promise<Record<string, unknown>[]> => {
      const res = await fetch(`${this.baseUrl}/v1/graph/neighbors/${encodeURIComponent(name)}`);
      if (!res.ok) throw new AgentError(`graph.neighbors failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Record<string, unknown>[]>;
    },
    /** `GET /v1/graph/path/:from/:to` — `{path:[…], hops}`. */
    path: async (from: string, to: string): Promise<Record<string, unknown>> => {
      const res = await fetch(`${this.baseUrl}/v1/graph/path/${encodeURIComponent(from)}/${encodeURIComponent(to)}`);
      if (!res.ok) throw new AgentError(`graph.path failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Record<string, unknown>>;
    },
    /** `POST /v1/graph/blast {name, max_hops?}` — blast radius. */
    blast: async (name: string, maxHops?: number): Promise<Record<string, unknown>> => {
      const res = await fetch(`${this.baseUrl}/v1/graph/blast`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, max_hops: maxHops ?? 2 }),
      });
      if (!res.ok) throw new AgentError(`graph.blast failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Record<string, unknown>>;
    },
    /** `GET /v1/graph/report` — full `GRAPH_REPORT.md` text (`{report:string}`). */
    report: async (): Promise<Record<string, unknown>> => {
      const res = await fetch(`${this.baseUrl}/v1/graph/report`);
      if (!res.ok) throw new AgentError(`graph.report failed: ${res.status} ${await res.text()}`);
      return res.json() as Promise<Record<string, unknown>>;
    },
  };

  /**
   * Check if the daemon is reachable.
   */
  async isConnected(): Promise<boolean> {
    try {
      const res = await fetch(`${this.baseUrl}/health`);
      return res.ok;
    } catch {
      return false;
    }
  }

  private async *_parseEventStream(body: ReadableStream<Uint8Array>): AsyncGenerator<AgentEvent> {
    for await (const data of readSseLines(body)) {
      try {
        const event: AgentEvent = JSON.parse(data);
        yield event;
        if (event.type === 'complete' || event.type === 'error') break;
      } catch {
        // Skip unparseable lines
      }
    }
  }
}

// ── AgentError ────────────────────────────────────────────────────────────────

export class AgentError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'AgentError';
  }
}

// ── SSE helper ─────────────────────────────────────────────────────────────────

async function *readSseLines(body: ReadableStream<Uint8Array>): AsyncGenerator<string> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buf = '';
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      const lines = buf.split('\n');
      buf = lines.pop() ?? '';
      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6).trim();
          if (data) yield data;
        }
      }
    }
    // Process any remaining buffer
    if (buf.startsWith('data: ')) {
      const data = buf.slice(6).trim();
      if (data) yield data;
    }
  } finally {
    reader.releaseLock();
  }
}

// ── Convenience factory ───────────────────────────────────────────────────────

/**
 * Create a `VibeCLIAgent` instance with sensible defaults.
 *
 * @example
 * ```ts
 * import { createAgent } from '@vibecody/agent-sdk';
 * const agent = createAgent({ provider: 'openai', approval: 'full-auto' });
 * ```
 */
export function createAgent(options?: AgentOptions): VibeCLIAgent {
  return new VibeCLIAgent(options);
}

// ── Re-exports for convenience ─────────────────────────────────────────────────

export type { AgentOptions as VibeCLIAgentOptions };
