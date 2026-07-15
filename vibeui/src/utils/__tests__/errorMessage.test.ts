import { describe, it, expect } from 'vitest';
import { errorMessage } from '../errorMessage';

describe('errorMessage', () => {
  it('returns a thrown string as-is', () => {
    expect(errorMessage('boom')).toBe('boom');
  });

  it('extracts .message from an Error', () => {
    expect(errorMessage(new Error('kaput'))).toBe('kaput');
  });

  it('extracts .message from an Error-like object', () => {
    expect(errorMessage({ message: 'nope' })).toBe('nope');
  });

  it('stringifies a non-string .message', () => {
    expect(errorMessage({ message: 404 })).toBe('404');
  });

  it('returns undefined for objects without a message (caller keeps its fallback)', () => {
    expect(errorMessage({ code: 'X' })).toBeUndefined();
    expect(errorMessage(errorMessage({}) || 'fallback')).toBe('fallback');
  });

  it('returns undefined for null / undefined', () => {
    expect(errorMessage(null)).toBeUndefined();
    expect(errorMessage(undefined)).toBeUndefined();
  });

  it('composes with a fallback the way call sites use it', () => {
    expect(errorMessage(null) || 'Failed to load board').toBe('Failed to load board');
    expect(errorMessage('specific') || 'Failed to load board').toBe('specific');
  });
});
