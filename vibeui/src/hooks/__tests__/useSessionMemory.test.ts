import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { extractFacts, useSessionMemory } from '../useSessionMemory';
import type { Message } from '../../components/AIChat';

// ── Helpers ───────────────────────────────────────────────────────────────────

function msg(role: 'user' | 'assistant', content: string): Message {
  return { role, content } as Message;
}

// ── extractFacts (pure) ───────────────────────────────────────────────────────

describe('extractFacts', () => {
  it('returns empty array for no messages', () => {
    expect(extractFacts([])).toEqual([]);
  });

  it('ignores user messages', () => {
    expect(extractFacts([msg('user', '- This is a bullet point of user text')])).toEqual([]);
  });

  it('extracts bullet point facts', () => {
    const facts = extractFacts([msg('assistant', '- Use TypeScript for all new files')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('Use TypeScript for all new files');
  });

  it('extracts asterisk bullet facts', () => {
    const facts = extractFacts([msg('assistant', '* Always use async/await for async operations')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('Always use async/await for async operations');
  });

  it('extracts numbered list facts', () => {
    const facts = extractFacts([msg('assistant', '1. Prefer functional components over class components')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('Prefer functional components over class components');
  });

  it('extracts numbered list with parenthesis format', () => {
    const facts = extractFacts([msg('assistant', '2) Always write tests for new functionality')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('Always write tests for new functionality');
  });

  it('extracts Note: prefix facts', () => {
    const facts = extractFacts([msg('assistant', 'Note: The user prefers snake_case for Rust code')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('The user prefers snake_case for Rust code');
  });

  it('extracts Remember: prefix facts (case-insensitive)', () => {
    const facts = extractFacts([msg('assistant', 'remember: This project uses Tauri 2 not Electron')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('This project uses Tauri 2 not Electron');
  });

  it('extracts Important: prefix facts', () => {
    const facts = extractFacts([msg('assistant', 'Important: Never skip git hooks during commit')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('Never skip git hooks during commit');
  });

  it('extracts Key point: prefix facts', () => {
    const facts = extractFacts([msg('assistant', 'Key point: The workspace path must be absolute')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('The workspace path must be absolute');
  });

  it('extracts Warning: prefix facts', () => {
    const facts = extractFacts([msg('assistant', "Warning: Don't modify generated files directly")]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe("Don't modify generated files directly");
  });

  it('extracts Tip: prefix facts', () => {
    const facts = extractFacts([msg('assistant', 'Tip: Use cargo check before cargo build to save time')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toBe('Use cargo check before cargo build to save time');
  });

  it("extracts \"I'll remember that\" pattern", () => {
    const facts = extractFacts([msg('assistant', "I'll remember that you prefer dark mode themes")]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toContain('you prefer dark mode themes');
  });

  it("extracts \"I'll note\" pattern", () => {
    const facts = extractFacts([msg('assistant', "I'll note that the API key is stored in ~/.vibecli/config.toml")]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toContain('the API key is stored in ~/.vibecli/config.toml');
  });

  it("extracts \"Worth noting\" pattern", () => {
    const facts = extractFacts([msg('assistant', 'Worth noting that this codebase uses Rust workspaces')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toContain('this codebase uses Rust workspaces');
  });

  it("extracts \"Keep in mind\" pattern", () => {
    const facts = extractFacts([msg('assistant', 'Keep in mind: the tests run in watch mode by default')]);
    expect(facts).toHaveLength(1);
    expect(facts[0]).toContain('the tests run in watch mode by default');
  });

  it('skips lines shorter than MIN_FACT_LEN (20 chars)', () => {
    const facts = extractFacts([msg('assistant', '- Short text')]);
    expect(facts).toHaveLength(0);
  });

  it('skips code comment lines', () => {
    const facts = extractFacts([msg('assistant', '  // This is a code comment with enough characters')]);
    expect(facts).toHaveLength(0);
  });

  it('skips shell comment lines', () => {
    const facts = extractFacts([msg('assistant', '# This is a bash comment with enough characters')]);
    expect(facts).toHaveLength(0);
  });

  it('skips code fence lines', () => {
    const facts = extractFacts([msg('assistant', '```typescript\nconst x = 1;\n```')]);
    expect(facts).toHaveLength(0);
  });

  it('skips XML/HTML tag lines', () => {
    const facts = extractFacts([msg('assistant', '<div className="something">content here for length</div>')]);
    expect(facts).toHaveLength(0);
  });

  it('deduplicates identical facts (case-insensitive)', () => {
    const facts = extractFacts([
      msg('assistant', '- Use TypeScript for all new files in this project'),
      msg('assistant', '- use typescript for all new files in this project'),
    ]);
    expect(facts).toHaveLength(1);
  });

  it('extracts multiple facts from a single message', () => {
    const facts = extractFacts([msg('assistant', `Here are some important points:
- Use TypeScript for all new frontend files
- Follow the existing naming conventions always
- Write tests for every new component you create`)]);
    expect(facts).toHaveLength(3);
  });

  it('extracts facts from multiple messages', () => {
    const facts = extractFacts([
      msg('assistant', '- Always prefer named exports over default exports'),
      msg('user', '- This user bullet should be ignored completely'),
      msg('assistant', 'Note: This project targets Rust edition 2021'),
    ]);
    expect(facts).toHaveLength(2);
  });

  it('trims whitespace from extracted facts', () => {
    const facts = extractFacts([msg('assistant', '-   Use Tailwind utility classes consistently  ')]);
    expect(facts[0]).toBe('Use Tailwind utility classes consistently');
  });
});

// ── useSessionMemory hook ─────────────────────────────────────────────────────

describe('useSessionMemory', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
    // Stub crypto.randomUUID
    let counter = 0;
    vi.spyOn(globalThis.crypto, 'randomUUID').mockImplementation(() => `test-id-${++counter}` as `${string}-${string}-${string}-${string}-${string}`);
  });

  it('initializes with empty facts when localStorage is empty', () => {
    const { result } = renderHook(() => useSessionMemory());
    expect(result.current.facts).toEqual([]);
  });

  it('loads pinned facts from localStorage on mount', () => {
    const pinned = [{ id: 'abc', text: 'Pinned fact loaded from storage', source: 'manual', pinned: true, tabId: '__pinned__', createdAt: 1000 }];
    localStorage.setItem('vibecody:pinned-memory-facts', JSON.stringify(pinned));
    const { result } = renderHook(() => useSessionMemory());
    expect(result.current.facts).toHaveLength(1);
    expect(result.current.facts[0].text).toBe('Pinned fact loaded from storage');
  });

  it('handles corrupt localStorage without throwing', () => {
    localStorage.setItem('vibecody:pinned-memory-facts', 'not-valid-json{{{');
    expect(() => renderHook(() => useSessionMemory())).not.toThrow();
  });

  // ── extractFromMessages ──────────────────────────────────────────────────

  it('extractFromMessages adds facts from new assistant messages', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => {
      result.current.extractFromMessages(
        [msg('assistant', '- Always use absolute imports in this project')],
        'tab-1',
      );
    });
    expect(result.current.facts).toHaveLength(1);
    expect(result.current.facts[0].text).toBe('Always use absolute imports in this project');
    expect(result.current.facts[0].tabId).toBe('tab-1');
    expect(result.current.facts[0].source).toBe('extracted');
    expect(result.current.facts[0].pinned).toBe(false);
  });

  it('extractFromMessages does not re-process already-seen messages', () => {
    const { result } = renderHook(() => useSessionMemory());
    const messages = [msg('assistant', '- Always use absolute imports in this project')];
    act(() => { result.current.extractFromMessages(messages, 'tab-1'); });
    act(() => { result.current.extractFromMessages(messages, 'tab-1'); });
    expect(result.current.facts).toHaveLength(1);
  });

  it('extractFromMessages processes only new messages on each call', () => {
    const { result } = renderHook(() => useSessionMemory());
    const batch1 = [msg('assistant', '- First fact that is long enough to be extracted')];
    const batch2 = [
      msg('assistant', '- First fact that is long enough to be extracted'),
      msg('assistant', '- Second fact which is also long enough to extract'),
    ];
    act(() => { result.current.extractFromMessages(batch1, 'tab-1'); });
    act(() => { result.current.extractFromMessages(batch2, 'tab-1'); });
    // Only 2 unique facts total (not 3)
    expect(result.current.facts).toHaveLength(2);
  });

  it('extractFromMessages is per-tab (separate tracking per tabId)', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => {
      result.current.extractFromMessages(
        [msg('assistant', '- Tab one fact that is long enough here')],
        'tab-1',
      );
    });
    act(() => {
      result.current.extractFromMessages(
        [msg('assistant', '- Tab two fact that is also long enough')],
        'tab-2',
      );
    });
    expect(result.current.facts).toHaveLength(2);
    expect(result.current.facts.find(f => f.tabId === 'tab-1')).toBeDefined();
    expect(result.current.facts.find(f => f.tabId === 'tab-2')).toBeDefined();
  });

  // ── addManual ────────────────────────────────────────────────────────────

  it('addManual adds a manual fact', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('Remember to check staging before production', 'tab-1'); });
    expect(result.current.facts).toHaveLength(1);
    expect(result.current.facts[0].source).toBe('manual');
    expect(result.current.facts[0].pinned).toBe(false);
    expect(result.current.facts[0].tabId).toBe('tab-1');
  });

  it('addManual trims whitespace', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('  Trimmed manual fact  ', 'tab-1'); });
    expect(result.current.facts[0].text).toBe('Trimmed manual fact');
  });

  it('addManual ignores empty/whitespace-only input', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('   ', 'tab-1'); });
    expect(result.current.facts).toHaveLength(0);
  });

  // ── pinFact / unpinFact ──────────────────────────────────────────────────

  it('pinFact marks a fact as pinned and persists to localStorage', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('Deploy to staging before production', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.pinFact(id); });
    expect(result.current.facts[0].pinned).toBe(true);
    expect(result.current.facts[0].tabId).toBe('__pinned__');
    const stored = JSON.parse(localStorage.getItem('vibecody:pinned-memory-facts') ?? '[]');
    expect(stored).toHaveLength(1);
    expect(stored[0].pinned).toBe(true);
  });

  it('unpinFact marks a fact as not pinned and updates localStorage', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('Deploy to staging before production', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.pinFact(id); });
    act(() => { result.current.unpinFact(id); });
    expect(result.current.facts[0].pinned).toBe(false);
    const stored = JSON.parse(localStorage.getItem('vibecody:pinned-memory-facts') ?? '[]');
    expect(stored).toHaveLength(0);
  });

  // ── deleteFact ───────────────────────────────────────────────────────────

  it('deleteFact removes the fact from state', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('A fact to be deleted soon', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.deleteFact(id); });
    expect(result.current.facts).toHaveLength(0);
  });

  it('deleteFact removes pinned fact from localStorage', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('A pinned fact to delete', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.pinFact(id); });
    act(() => { result.current.deleteFact(id); });
    const stored = JSON.parse(localStorage.getItem('vibecody:pinned-memory-facts') ?? '[]');
    expect(stored).toHaveLength(0);
  });

  // ── editFact ─────────────────────────────────────────────────────────────

  it('editFact updates the text of an existing fact', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('Original text for this test fact', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.editFact(id, 'Updated text for this test fact'); });
    expect(result.current.facts[0].text).toBe('Updated text for this test fact');
  });

  it('editFact trims whitespace', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('Original fact text content here', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.editFact(id, '  Trimmed edit text content  '); });
    expect(result.current.facts[0].text).toBe('Trimmed edit text content');
  });

  it('editFact ignores empty input', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('Original text that should remain here', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.editFact(id, ''); });
    expect(result.current.facts[0].text).toBe('Original text that should remain here');
  });

  it('editFact persists pinned fact updates to localStorage', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('Original pinned fact for edit test', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.pinFact(id); });
    act(() => { result.current.editFact(id, 'Updated pinned fact for edit test'); });
    const stored = JSON.parse(localStorage.getItem('vibecody:pinned-memory-facts') ?? '[]');
    expect(stored[0].text).toBe('Updated pinned fact for edit test');
  });

  // ── clearTabFacts ────────────────────────────────────────────────────────

  it('clearTabFacts removes unpinned facts for the given tab', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => {
      result.current.addManual('Tab 1 first fact content here', 'tab-1');
      result.current.addManual('Tab 2 second fact content here', 'tab-2');
    });
    act(() => { result.current.clearTabFacts('tab-1'); });
    expect(result.current.facts.find(f => f.tabId === 'tab-1')).toBeUndefined();
    expect(result.current.facts.find(f => f.tabId === 'tab-2')).toBeDefined();
  });

  it('clearTabFacts preserves pinned facts even from the cleared tab', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => { result.current.addManual('Pinned fact from tab-1 to keep', 'tab-1'); });
    const id = result.current.facts[0].id;
    act(() => { result.current.pinFact(id); });
    act(() => { result.current.clearTabFacts('tab-1'); });
    expect(result.current.facts).toHaveLength(1);
    expect(result.current.facts[0].pinned).toBe(true);
  });

  // ── factsForTab ──────────────────────────────────────────────────────────

  it('factsForTab returns facts for the given tab plus pinned facts', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => {
      result.current.addManual('Tab 1 specific fact for this tab', 'tab-1');
      result.current.addManual('Tab 2 specific fact for other tab', 'tab-2');
    });
    // Pin tab-2 fact
    const tab2Id = result.current.facts.find(f => f.tabId === 'tab-2')!.id;
    act(() => { result.current.pinFact(tab2Id); });

    const forTab1 = result.current.factsForTab('tab-1');
    // Should include: tab-1 fact + pinned fact
    expect(forTab1).toHaveLength(2);
    expect(forTab1.some(f => f.tabId === 'tab-1')).toBe(true);
    expect(forTab1.some(f => f.pinned)).toBe(true);
  });

  // ── getPinnedSystemPromptText ─────────────────────────────────────────────

  it('getPinnedSystemPromptText returns empty string when nothing is pinned', () => {
    const { result } = renderHook(() => useSessionMemory());
    expect(result.current.getPinnedSystemPromptText()).toBe('');
  });

  it('getPinnedSystemPromptText returns formatted prompt with pinned facts', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => {
      result.current.addManual('User prefers Rust over TypeScript always', 'tab-1');
      result.current.addManual('All commits must pass CI pipeline checks', 'tab-1');
    });
    const ids = result.current.facts.map(f => f.id);
    act(() => { result.current.pinFact(ids[0]); result.current.pinFact(ids[1]); });
    const prompt = result.current.getPinnedSystemPromptText();
    expect(prompt).toContain('User preferences and remembered facts:');
    expect(prompt).toContain('- User prefers Rust over TypeScript always');
    expect(prompt).toContain('- All commits must pass CI pipeline checks');
  });

  it('getPinnedSystemPromptText lists only pinned facts', () => {
    const { result } = renderHook(() => useSessionMemory());
    act(() => {
      result.current.addManual('Unpinned fact that should be excluded from prompt', 'tab-1');
      result.current.addManual('Pinned fact that should be included in prompt', 'tab-1');
    });
    const pinnedId = result.current.facts[1].id;
    act(() => { result.current.pinFact(pinnedId); });
    const prompt = result.current.getPinnedSystemPromptText();
    expect(prompt).not.toContain('Unpinned fact');
    expect(prompt).toContain('Pinned fact that should be included in prompt');
  });

  // ── MAX_PINNED localStorage cap ───────────────────────────────────────────

  it('localStorage is capped at 50 pinned facts', () => {
    const { result } = renderHook(() => useSessionMemory());
    // Add 55 facts and pin them all
    for (let i = 0; i < 55; i++) {
      act(() => { result.current.addManual(`Fact number ${i} which is long enough to store`, 'tab-1'); });
    }
    const ids = result.current.facts.map(f => f.id);
    for (const id of ids) {
      act(() => { result.current.pinFact(id); });
    }
    const stored = JSON.parse(localStorage.getItem('vibecody:pinned-memory-facts') ?? '[]');
    expect(stored.length).toBeLessThanOrEqual(50);
  });
});
