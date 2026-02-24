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

export type AgentEventType = 'chunk' | 'step' | 'complete' | 'error';

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
export class VibeCLIAgent {
  private baseUrl: string;
  private approval: string;

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
