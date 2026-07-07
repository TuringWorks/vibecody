/**
 * Tests for @vibecody/agent-sdk
 *
 * All HTTP is mocked via vi.stubGlobal('fetch', ...) so no real daemon is needed.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  VibeCLIAgent,
  AgentError,
  createAgent,
} from './index';
import type { AgentEvent, JobRecord, ChatMessage, SkilloptTrainEvent } from './index';

// ── SSE helpers ───────────────────────────────────────────────────────────────

/** Build a ReadableStream that emits the given lines then closes. */
function makeStream(...lines: string[]): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  const chunks = lines.map(l => encoder.encode(l));
  let i = 0;
  return new ReadableStream({
    pull(controller) {
      if (i < chunks.length) {
        controller.enqueue(chunks[i++]);
      } else {
        controller.close();
      }
    },
  });
}

/** Wrap SSE events into a stream body. */
function sseStream(...events: AgentEvent[]): ReadableStream<Uint8Array> {
  const lines = events.map(e => `data: ${JSON.stringify(e)}\n`);
  return makeStream(...lines);
}

// ── Test data ─────────────────────────────────────────────────────────────────

const SESSION_ID = 'sess-abc-123';

const JOB_RUNNING: JobRecord = {
  session_id: SESSION_ID,
  task: 'Write unit tests',
  status: 'running',
  provider: 'ollama',
  started_at: 1_700_000_000_000,
};

const JOB_COMPLETE: JobRecord = {
  ...JOB_RUNNING,
  status: 'complete',
  finished_at: 1_700_000_005_000,
  summary: 'Done',
};

// ── Setup / teardown ──────────────────────────────────────────────────────────

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi.fn();
  vi.stubGlobal('fetch', fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

// ── VibeCLIAgent constructor ───────────────────────────────────────────────────

describe('VibeCLIAgent constructor', () => {
  it('uses default host localhost and port 7878', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream({ type: 'complete', content: 'ok' }) });

    const events: AgentEvent[] = [];
    for await (const e of agent.run('task')) events.push(e);

    expect(fetchMock.mock.calls[0][0]).toContain('http://localhost:7878');
  });

  it('uses custom host and port when provided', async () => {
    const agent = new VibeCLIAgent({ host: '192.168.1.1', port: 9000 });
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream({ type: 'complete', content: 'ok' }) });

    const events: AgentEvent[] = [];
    for await (const e of agent.run('task')) events.push(e);

    expect(fetchMock.mock.calls[0][0]).toContain('http://192.168.1.1:9000');
  });

  it('defaults approval to "suggest"', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream({ type: 'complete', content: 'done' }) });

    const events: AgentEvent[] = [];
    for await (const e of agent.run('task')) events.push(e);

    const body = JSON.parse(fetchMock.mock.calls[0][1].body as string);
    expect(body.approval).toBe('suggest');
  });
});

// ── createAgent factory ───────────────────────────────────────────────────────

describe('createAgent', () => {
  it('returns a VibeCLIAgent instance', () => {
    const agent = createAgent();
    expect(agent).toBeInstanceOf(VibeCLIAgent);
  });

  it('passes options through', async () => {
    const agent = createAgent({ approval: 'full-auto', port: 8000 });
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream({ type: 'complete', content: 'done' }) });

    const events: AgentEvent[] = [];
    for await (const e of agent.run('task')) events.push(e);

    const url = fetchMock.mock.calls[0][0] as string;
    expect(url).toContain(':8000');
    const body = JSON.parse(fetchMock.mock.calls[0][1].body as string);
    expect(body.approval).toBe('full-auto');
  });
});

// ── VibeCLIAgent.run ──────────────────────────────────────────────────────────

describe('VibeCLIAgent.run', () => {
  it('POSTs to /agent with task and approval', async () => {
    const agent = new VibeCLIAgent({ approval: 'auto-edit' });
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream({ type: 'complete', content: 'done' }) });

    const events: AgentEvent[] = [];
    for await (const e of agent.run('Do something')) events.push(e);

    expect(fetchMock.mock.calls[0][0]).toMatch(/\/agent$/);
    const body = JSON.parse(fetchMock.mock.calls[0][1].body as string);
    expect(body).toEqual({ task: 'Do something', approval: 'auto-edit' });
  });

  it('per-run approval overrides constructor default', async () => {
    const agent = new VibeCLIAgent({ approval: 'suggest' });
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream({ type: 'complete', content: 'done' }) });

    const events: AgentEvent[] = [];
    for await (const e of agent.run('task', 'full-auto')) events.push(e);

    const body = JSON.parse(fetchMock.mock.calls[0][1].body as string);
    expect(body.approval).toBe('full-auto');
  });

  it('yields chunk, step, and complete events in order', async () => {
    const agent = new VibeCLIAgent();
    const eventSeq: AgentEvent[] = [
      { type: 'chunk', content: 'Hello ' },
      { type: 'step', step_num: 0, tool_name: 'read_file', success: true },
      { type: 'chunk', content: 'world' },
      { type: 'complete', content: 'Hello world' },
    ];
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream(...eventSeq) });

    const received: AgentEvent[] = [];
    for await (const e of agent.run('task')) received.push(e);

    expect(received).toEqual(eventSeq);
  });

  it('stops iteration after a complete event', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({
      ok: true,
      body: sseStream(
        { type: 'complete', content: 'done' },
        { type: 'chunk', content: 'should not appear' },
      ),
    });

    const received: AgentEvent[] = [];
    for await (const e of agent.run('task')) received.push(e);

    expect(received).toHaveLength(1);
    expect(received[0].type).toBe('complete');
  });

  it('stops iteration after an error event', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({
      ok: true,
      body: sseStream(
        { type: 'error', content: 'Something went wrong' },
        { type: 'chunk', content: 'never' },
      ),
    });

    const received: AgentEvent[] = [];
    for await (const e of agent.run('task')) received.push(e);

    expect(received).toHaveLength(1);
    expect(received[0].type).toBe('error');
  });

  it('throws AgentError when POST /agent returns non-2xx', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, text: async () => 'internal error' });

    await expect(async () => {
      for await (const _ of agent.run('task')) { /* nothing */ }
    }).rejects.toThrow(AgentError);
  });

  it('throws AgentError when stream endpoint returns non-2xx', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: false, status: 404, body: null });

    await expect(async () => {
      for await (const _ of agent.run('task')) { /* nothing */ }
    }).rejects.toThrow(AgentError);
  });

  it('skips non-data SSE lines without throwing', async () => {
    const agent = new VibeCLIAgent();
    const encoder = new TextEncoder();
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode('event: ping\n'));
        controller.enqueue(encoder.encode(': keep-alive comment\n'));
        controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'complete', content: 'ok' })}\n`));
        controller.close();
      },
    });

    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body });

    const received: AgentEvent[] = [];
    for await (const e of agent.run('task')) received.push(e);

    expect(received).toHaveLength(1);
    expect(received[0].type).toBe('complete');
  });

  it('skips malformed JSON data lines without throwing', async () => {
    const agent = new VibeCLIAgent();
    const encoder = new TextEncoder();
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode('data: {not valid json}\n'));
        controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'complete', content: 'ok' })}\n`));
        controller.close();
      },
    });

    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body });

    const received: AgentEvent[] = [];
    for await (const e of agent.run('task')) received.push(e);

    expect(received).toHaveLength(1);
  });
});

// ── VibeCLIAgent.chat ─────────────────────────────────────────────────────────

describe('VibeCLIAgent.chat', () => {
  it('POSTs messages to /chat and returns content', async () => {
    const agent = new VibeCLIAgent();
    const messages: ChatMessage[] = [{ role: 'user', content: 'Hello' }];
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ content: 'Hi there!' }) });

    const result = await agent.chat(messages);

    expect(fetchMock.mock.calls[0][0]).toMatch(/\/chat$/);
    const body = JSON.parse(fetchMock.mock.calls[0][1].body as string);
    expect(body.messages).toEqual(messages);
    expect(result).toBe('Hi there!');
  });

  it('throws AgentError on non-2xx response', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 503, text: async () => 'unavailable' });

    await expect(agent.chat([{ role: 'user', content: 'ping' }])).rejects.toThrow(AgentError);
  });

  it('sends Content-Type: application/json header', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ content: 'ok' }) });

    await agent.chat([{ role: 'user', content: 'hi' }]);

    const headers = fetchMock.mock.calls[0][1].headers as Record<string, string>;
    expect(headers['Content-Type']).toBe('application/json');
  });
});

// ── VibeCLIAgent.chatStream ───────────────────────────────────────────────────

describe('VibeCLIAgent.chatStream', () => {
  it('POSTs to /chat/stream and yields tokens', async () => {
    const agent = new VibeCLIAgent();
    const encoder = new TextEncoder();
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode('data: Hello\ndata:  world\n'));
        controller.close();
      },
    });
    fetchMock.mockResolvedValueOnce({ ok: true, body });

    const tokens: string[] = [];
    for await (const t of agent.chatStream([{ role: 'user', content: 'hi' }])) {
      tokens.push(t);
    }

    expect(fetchMock.mock.calls[0][0]).toMatch(/\/chat\/stream$/);
    expect(tokens).toEqual(['Hello', 'world']);
  });

  it('throws AgentError on non-2xx or missing body', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 400 });

    await expect(async () => {
      for await (const _ of agent.chatStream([{ role: 'user', content: 'test' }])) { /* nothing */ }
    }).rejects.toThrow(AgentError);
  });
});

// ── VibeCLIAgent.listJobs ─────────────────────────────────────────────────────

describe('VibeCLIAgent.listJobs', () => {
  it('GETs /jobs and returns job array', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => [JOB_RUNNING, JOB_COMPLETE] });

    const jobs = await agent.listJobs();

    expect(fetchMock.mock.calls[0][0]).toMatch(/\/jobs$/);
    expect(jobs).toHaveLength(2);
    expect(jobs[0].session_id).toBe(SESSION_ID);
  });

  it('throws AgentError on non-2xx', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, text: async () => 'err' });

    await expect(agent.listJobs()).rejects.toThrow(AgentError);
  });
});

// ── VibeCLIAgent.getJob ───────────────────────────────────────────────────────

describe('VibeCLIAgent.getJob', () => {
  it('GETs /jobs/:id and returns the job', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => JOB_COMPLETE });

    const job = await agent.getJob(SESSION_ID);

    const url = fetchMock.mock.calls[0][0] as string;
    expect(url).toContain(`/jobs/${SESSION_ID}`);
    expect(job).toEqual(JOB_COMPLETE);
  });

  it('returns null on 404', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ status: 404, ok: false });

    const job = await agent.getJob('nonexistent');
    expect(job).toBeNull();
  });

  it('throws AgentError on other non-2xx status', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, text: async () => 'server error' });

    await expect(agent.getJob(SESSION_ID)).rejects.toThrow(AgentError);
  });

  it('URL-encodes session IDs with special characters', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ status: 404, ok: false });

    await agent.getJob('sess/with spaces&more');

    const url = fetchMock.mock.calls[0][0] as string;
    expect(url).not.toContain(' ');
    expect(url).not.toContain('/jobs/sess/with');
  });
});

// ── VibeCLIAgent.cancelJob ────────────────────────────────────────────────────

describe('VibeCLIAgent.cancelJob', () => {
  it('POSTs to /jobs/:id/cancel', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({}) });

    await agent.cancelJob(SESSION_ID);

    const url = fetchMock.mock.calls[0][0] as string;
    expect(url).toContain(`/jobs/${SESSION_ID}/cancel`);
    expect(fetchMock.mock.calls[0][1].method).toBe('POST');
  });

  it('throws AgentError on non-2xx', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 404, text: async () => 'not found' });

    await expect(agent.cancelJob(SESSION_ID)).rejects.toThrow(AgentError);
  });
});

// ── VibeCLIAgent.stop ─────────────────────────────────────────────────────────

describe('VibeCLIAgent.stop', () => {
  it('is a no-op when no run has been started', async () => {
    const agent = new VibeCLIAgent();
    await agent.stop(); // Should not throw and should not call fetch
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('cancels the last session after a run', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream({ type: 'complete', content: 'done' }) });

    for await (const _ of agent.run('task')) { /* drain */ }

    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({}) });
    await agent.stop();

    const cancelCall = fetchMock.mock.calls[2];
    expect(cancelCall[0]).toContain(`/jobs/${SESSION_ID}/cancel`);
  });

  it('clears lastSessionId so second stop() is a no-op', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body: sseStream({ type: 'complete', content: 'done' }) });

    for await (const _ of agent.run('task')) { /* drain */ }

    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({}) });
    await agent.stop();
    const callsAfterFirstStop = fetchMock.mock.calls.length;

    await agent.stop(); // second stop — should be a no-op
    expect(fetchMock.mock.calls.length).toBe(callsAfterFirstStop);
  });
});

// ── VibeCLIAgent.isConnected ──────────────────────────────────────────────────

describe('VibeCLIAgent.isConnected', () => {
  it('returns true when /health returns 200', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true });

    expect(await agent.isConnected()).toBe(true);
    expect(fetchMock.mock.calls[0][0]).toMatch(/\/health$/);
  });

  it('returns false when /health returns non-2xx', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 503 });

    expect(await agent.isConnected()).toBe(false);
  });

  it('returns false when fetch throws (network unreachable)', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockRejectedValueOnce(new Error('ECONNREFUSED'));

    expect(await agent.isConnected()).toBe(false);
  });
});

// ── AgentError ────────────────────────────────────────────────────────────────

describe('AgentError', () => {
  it('has name "AgentError"', () => {
    const e = new AgentError('test message');
    expect(e.name).toBe('AgentError');
  });

  it('is an instance of Error', () => {
    expect(new AgentError('x')).toBeInstanceOf(Error);
  });

  it('carries the message', () => {
    const e = new AgentError('something failed');
    expect(e.message).toBe('something failed');
  });
});

// ── SSE multi-chunk buffering ─────────────────────────────────────────────────

describe('SSE stream — multi-chunk buffering', () => {
  it('handles a data line split across two chunks', async () => {
    const agent = new VibeCLIAgent();
    const encoder = new TextEncoder();
    const event: AgentEvent = { type: 'complete', content: 'hello' };
    const line = `data: ${JSON.stringify(event)}\n`;

    // Split the SSE line in the middle of the JSON
    const half = Math.floor(line.length / 2);
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode(line.slice(0, half)));
        controller.enqueue(encoder.encode(line.slice(half)));
        controller.close();
      },
    });

    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body });

    const received: AgentEvent[] = [];
    for await (const e of agent.run('task')) received.push(e);

    expect(received).toHaveLength(1);
    expect(received[0].content).toBe('hello');
  });

  it('handles data line in the trailing buffer (no trailing newline)', async () => {
    const agent = new VibeCLIAgent();
    const encoder = new TextEncoder();
    const event: AgentEvent = { type: 'complete', content: 'trailing' };
    // No trailing \n — ends up in buf
    const line = `data: ${JSON.stringify(event)}`;
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(encoder.encode(line));
        controller.close();
      },
    });

    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => ({ session_id: SESSION_ID }) });
    fetchMock.mockResolvedValueOnce({ ok: true, body });

    const received: AgentEvent[] = [];
    for await (const e of agent.run('task')) received.push(e);

    expect(received).toHaveLength(1);
    expect(received[0].content).toBe('trailing');
  });
});

// ── VibeCLIAgent.skillopt.streamTrain (typed SSE consumer) ────────────────────

describe('VibeCLIAgent.skillopt.streamTrain', () => {
  it('yields job → epoch* → done in order from the typed SSE stream', async () => {
    const agent = new VibeCLIAgent();
    const job = JSON.stringify({ job_id: 'math-anthropic-claude-1', status: 'running', llm: { provider: 'anthropic', model: 'claude' } });
    const epoch0 = JSON.stringify({ epoch: 0, best_val: 0.5, accepted: 1, rejected: 0, spent_tokens: 10, early_stopped: false });
    const epoch1 = JSON.stringify({ epoch: 1, best_val: 1.0, accepted: 1, rejected: 1, spent_tokens: 20, early_stopped: true });
    const done = JSON.stringify({ id: 'math-anthropic-claude-1', state: 'done' });
    const body = makeStream(
      `event: job\n`, `data: ${job}\n`, `\n`,
      `event: epoch\n`, `data: ${epoch0}\n`, `\n`,
      `event: epoch\n`, `data: ${epoch1}\n`, `\n`,
      `event: done\n`, `data: ${done}\n`, `\n`,
    );
    fetchMock.mockResolvedValueOnce({ ok: true, body });

    const received: SkilloptTrainEvent[] = [];
    for await (const e of agent.skillopt.streamTrain('math', 'repo', undefined, {}, 'anthropic', 'claude')) {
      received.push(e);
    }

    expect(received).toHaveLength(4);
    expect(received[0]).toEqual({ type: 'job', job: JSON.parse(job) });
    expect(received[1]).toEqual({ type: 'epoch', epoch: JSON.parse(epoch0) });
    expect(received[2]).toEqual({ type: 'epoch', epoch: JSON.parse(epoch1) });
    expect(received[3]).toEqual({ type: 'done', final: JSON.parse(done) });
  });

  it('stops after an error event and surfaces the error text', async () => {
    const agent = new VibeCLIAgent();
    const body = makeStream(
      `event: error\n`, `data: skill 'nope' not in catalog\n`, `\n`,
    );
    fetchMock.mockResolvedValueOnce({ ok: true, body });

    const received: SkilloptTrainEvent[] = [];
    for await (const e of agent.skillopt.streamTrain('nope', 'repo', undefined, {}, 'anthropic', 'claude')) {
      received.push(e);
    }

    expect(received).toEqual([{ type: 'error', error: "skill 'nope' not in catalog" }]);
  });

  it('throws AgentError on non-2xx', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, text: async () => 'boom' });

    await expect(async () => {
      for await (const _ of agent.skillopt.streamTrain('s', 'repo', undefined, {}, 'p', 'm')) {
        void _;
      }
    }).rejects.toThrow(AgentError);
  });

  it('POSTs to /v1/skillopt/train/stream with the train body shape', async () => {
    const agent = new VibeCLIAgent();
    fetchMock.mockResolvedValueOnce({ ok: true, body: makeStream(`event: done\n`, `data: null\n`, `\n`) });

    for await (const _ of agent.skillopt.streamTrain('rust-tests', 'static', 't1', { epochs: 2 }, 'anthropic', 'claude')) {
      void _;
    }

    const call = fetchMock.mock.calls[0];
    expect(call[0]).toBe('http://localhost:7878/v1/skillopt/train/stream');
    const init = call[1] as RequestInit;
    expect(init.method).toBe('POST');
    const sent = JSON.parse(init.body as string);
    expect(sent).toEqual({
      skill: 'rust-tests',
      env: { kind: 'static', tasks: 't1' },
      config: { epochs: 2 },
      provider: 'anthropic',
      model: 'claude',
    });
  });
});
