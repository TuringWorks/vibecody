/**
 * The wording of a fix request is load-bearing.
 *
 * Two failures are already on record and both are wording, not logic: a request
 * that does not name the path gets a second copy of the file back instead of a
 * fix, and a request that does not say how sure the finding is gets a "fix"
 * applied to a false positive. These pin both, plus the cap, which must never
 * let a partial hand-off read as the whole set.
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  buildFixRequest,
  fixLabel,
  locationOf,
  sendFixToChat,
  FIX_BATCH_LIMIT,
  type FixItem,
} from '../fixWithAI';

const item = (over: Partial<FixItem> = {}): FixItem => ({
  file: 'src/a.rs',
  line: 42,
  severity: 'critical',
  title: 'SQL injection (CWE-89)',
  message: 'Query built by concatenation.',
  suggestion: 'Use a parameterised query.',
  ...over,
});

describe('buildFixRequest', () => {
  it('names the file and line of a single finding in the opening line', () => {
    const request = buildFixRequest([item()], { source: 'security scanner' });
    expect(request).toContain('Fix this security scanner finding in src/a.rs:42.');
    expect(request).toMatch(/do not create a new file/i);
  });

  it('carries severity, line, title, message and suggestion for each item', () => {
    const request = buildFixRequest([item()], { source: 'code review' });
    expect(request).toContain('- [critical] line 42 — SQL injection (CWE-89): Query built by concatenation.');
    expect(request).toContain('Suggested fix: Use a parameterised query.');
  });

  it('groups items under the file they belong to', () => {
    const request = buildFixRequest(
      [item({ line: 1 }), item({ file: 'src/b.rs', line: 2 }), item({ line: 3 })],
      { source: 'code review' },
    );
    // Two files, each named once, with their own items beneath.
    expect(request.match(/^src\/a\.rs:$/gm)).toHaveLength(1);
    expect(request.match(/^src\/b\.rs:$/gm)).toHaveLength(1);
    expect(request).toContain('Fix these 3 code review findings.');
  });

  it('says what a capped batch left out', () => {
    const request = buildFixRequest([item(), item({ line: 43 })], {
      source: 'security scanner',
      total: 40,
    });
    expect(request).toContain('the first 2 of 40');
    expect(request).toContain('the remaining 38 are not listed here');
  });

  it('renders notes verbatim under their item, before the suggestion', () => {
    const request = buildFixRequest(
      [item({ notes: ['NOT verified: no model has checked this candidate.'] })],
      { source: 'security scanner' },
    );
    const lines = request.split('\n');
    const noteAt = lines.findIndex((l) => l.includes('NOT verified'));
    const fixAt = lines.findIndex((l) => l.includes('Suggested fix'));
    expect(noteAt).toBeGreaterThan(-1);
    expect(fixAt).toBeGreaterThan(noteAt);
  });

  it('puts caller instructions between the shared two', () => {
    const request = buildFixRequest([item()], {
      source: 'security scanner',
      instructions: ['Check every finding marked NOT verified first.'],
    });
    const lines = request.split('\n');
    const inPlace = lines.findIndex((l) => l.startsWith('Edit each file in place'));
    const caller = lines.findIndex((l) => l.startsWith('Check every finding'));
    const minimal = lines.findIndex((l) => l.startsWith('Keep each change minimal'));
    expect(inPlace).toBeLessThan(caller);
    expect(caller).toBeLessThan(minimal);
  });

  it('does not invent a location for a finding that names no file', () => {
    const request = buildFixRequest([item({ file: null, line: null })], { source: 'code review' });
    expect(request).toContain('(no file given)');
    expect(request).not.toContain('line ');
  });
});

describe('locationOf', () => {
  it('drops a line number nobody gave', () => {
    expect(locationOf(item({ line: 0 }))).toBe('src/a.rs');
    expect(locationOf(item({ line: null }))).toBe('src/a.rs');
    expect(locationOf(item())).toBe('src/a.rs:42');
  });
});

describe('fixLabel', () => {
  it('names the cap and the true count whenever the cap bites', () => {
    expect(fixLabel(1)).toBe('Fix with AI');
    expect(fixLabel(2)).toBe('Fix all 2 with AI');
    expect(fixLabel(FIX_BATCH_LIMIT)).toBe(`Fix all ${FIX_BATCH_LIMIT} with AI`);
    expect(fixLabel(40)).toBe(`Fix first ${FIX_BATCH_LIMIT} of 40 with AI`);
  });
});

describe('sendFixToChat', () => {
  const injected: string[] = [];
  const listener = (e: Event) => injected.push((e as CustomEvent<string>).detail);

  beforeEach(() => {
    injected.length = 0;
    window.addEventListener('vibecoder:inject-context', listener);
  });
  afterEach(() => window.removeEventListener('vibecoder:inject-context', listener));

  it('writes the request into the composer and reports that it did', () => {
    expect(sendFixToChat([item()], { source: 'code review' })).toBe(true);
    expect(injected).toHaveLength(1);
    expect(injected[0]).toContain('src/a.rs:42');
  });

  it('sends nothing for an empty batch, so a stray click cannot clear the composer', () => {
    expect(sendFixToChat([], { source: 'code review' })).toBe(false);
    expect(injected).toHaveLength(0);
  });

  it('edits nothing — the hand-off is a message, not an action', () => {
    const dispatched: string[] = [];
    const spy = vi.spyOn(window, 'dispatchEvent').mockImplementation((e: Event) => {
      dispatched.push(e.type);
      return true;
    });
    sendFixToChat([item()], { source: 'code review' });
    spy.mockRestore();
    expect(dispatched).toEqual(['vibecoder:inject-context']);
  });
});
