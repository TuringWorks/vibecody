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
  type: 'chunk' | 'step' | 'complete' | 'error';
  content?: string;
  step_num?: number;
  tool_name?: string;
  success?: boolean;
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
