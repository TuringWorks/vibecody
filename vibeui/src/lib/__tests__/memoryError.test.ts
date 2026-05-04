import { describe, it, expect } from 'vitest';
import { classifyMemoryError } from '../memoryError';

describe('classifyMemoryError', () => {
  it('detects permission errors and points at the store directory', () => {
    expect(classifyMemoryError('Permission denied').hint).toMatch(/openmemory/);
    expect(classifyMemoryError('EACCES: read-only filesystem').hint).toMatch(/permissions/i);
  });

  it('detects disk-full and suggests decay', () => {
    expect(classifyMemoryError('No space left on device').hint).toMatch(/decay/i);
    expect(classifyMemoryError('ENOSPC: disk full').hint).toMatch(/decay/i);
  });

  it('detects corrupt-JSON and suggests restore-or-reset', () => {
    expect(classifyMemoryError('invalid JSON at line 3').hint).toMatch(/corrupt/i);
    expect(classifyMemoryError('expected value at byte 0').hint).toMatch(/corrupt/i);
    expect(classifyMemoryError('EOF while parsing').hint).toMatch(/corrupt/i);
  });

  it('detects not-found and suggests refresh', () => {
    expect(classifyMemoryError("memory 'abc' not found").hint).toMatch(/Refresh/i);
    expect(classifyMemoryError('404 not found').hint).toMatch(/Refresh/i);
  });

  it('detects daemon connection problems', () => {
    expect(classifyMemoryError('connection refused').hint).toMatch(/vibecli serve/);
    expect(classifyMemoryError('network unreachable').hint).toMatch(/daemon/i);
    expect(classifyMemoryError('request timeout').hint).toMatch(/daemon/i);
  });

  it('detects encryption / passphrase errors', () => {
    expect(classifyMemoryError('decryption failed').hint).toMatch(/passphrase/i);
    expect(classifyMemoryError('invalid passphrase').hint).toMatch(/set-key/);
  });

  it('detects import-format errors', () => {
    expect(classifyMemoryError('import failed: unknown format').hint).toMatch(/openmemory|mem0|Zep/i);
  });

  it('returns no hint for unclassified errors', () => {
    const r = classifyMemoryError('Some unique never-seen failure');
    expect(r.message).toBe('Some unique never-seen failure');
    expect(r.hint).toBeUndefined();
  });

  it('always echoes the original message verbatim', () => {
    const raw = 'Permission denied (os error 13)';
    expect(classifyMemoryError(raw).message).toBe(raw);
  });
});
