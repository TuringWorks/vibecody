/**
 * useSessionMemory — lightweight per-tab memory with pinning.
 *
 * Facts are extracted heuristically from assistant messages:
 * - Bullet / numbered list items
 * - Lines beginning with "Note:", "Remember:", "Important:", etc.
 * - "I'll remember / I'll note / Worth noting" patterns
 *
 * Pinned facts survive tab switches (stored in localStorage) and are
 * injected into the AI system prompt via getPinnedSystemPromptText().
 */

import { useState, useCallback, useRef } from "react";
import type { Message } from "../components/AIChat";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface MemoryFact {
  id: string;
  text: string;
  source: "extracted" | "manual";
  pinned: boolean;
  tabId: string;
  createdAt: number;
}

// ── Constants ─────────────────────────────────────────────────────────────────

const PINNED_KEY = "vibecody:pinned-memory-facts";
const MAX_PINNED = 50;
const MIN_FACT_LEN = 20;
const MAX_FACTS_PER_TAB = 100;

// Patterns that indicate a memorable fact in an assistant message
const FACT_PATTERNS = [
  /^[-*•]\s+(.{20,})/,                                        // bullet point
  /^\d+[.)]\s+(.{20,})/,                                     // numbered list
  /^(?:Note|Remember|Important|Key point|Warning|Tip):\s+(.+)/i,
  /(?:I'll (?:remember|note|keep in mind) that\s+)(.+)/i,
  /(?:Worth (?:noting|remembering) (?:that )?:?\s*)(.+)/i,
  /(?:Keep in mind[:\s]+)(.+)/i,
];

// Lines to skip (likely code, not facts)
const SKIP_PATTERNS = [
  /^\s*\/\//,      // code comment
  /^\s*#/,         // shell / Python comment
  /^\s*```/,       // code fence
  /^\s*<[a-z]/i,   // XML/HTML tag
];

// ── Pure extraction helper ────────────────────────────────────────────────────

export function extractFacts(messages: Message[]): string[] {
  const seen = new Set<string>();
  const results: string[] = [];

  for (const msg of messages) {
    if (msg.role !== "assistant") continue;

    const lines = msg.content.split("\n");
    for (const raw of lines) {
      const line = raw.trim();
      if (!line || line.length < MIN_FACT_LEN) continue;
      if (SKIP_PATTERNS.some((p) => p.test(line))) continue;

      for (const pattern of FACT_PATTERNS) {
        const m = line.match(pattern);
        if (m) {
          const text = (m[1] ?? line).trim();
          if (text.length < MIN_FACT_LEN) continue;
          const key = text.toLowerCase().replace(/\s+/g, " ");
          if (!seen.has(key)) {
            seen.add(key);
            results.push(text);
          }
          break;
        }
      }
    }
  }

  return results;
}

// ── Hook ──────────────────────────────────────────────────────────────────────

function loadPinned(): MemoryFact[] {
  try {
    const raw = localStorage.getItem(PINNED_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch { return []; }
}

function savePinned(facts: MemoryFact[]) {
  try {
    localStorage.setItem(PINNED_KEY, JSON.stringify(facts.slice(0, MAX_PINNED)));
  } catch { /* quota */ }
}

export function useSessionMemory() {
  const [facts, setFacts] = useState<MemoryFact[]>(() => {
    // Load pinned facts on mount; they are global (tabId = "__pinned__")
    return loadPinned();
  });

  // Track which message indices have already been processed per tab
  const processedRef = useRef<Record<string, number>>({});

  /** Call after each messages update to extract new facts for a tab. */
  const extractFromMessages = useCallback((messages: Message[], tabId: string) => {
    const lastProcessed = processedRef.current[tabId] ?? 0;
    const newMessages = messages.slice(lastProcessed);
    if (newMessages.length === 0) return;

    processedRef.current[tabId] = messages.length;

    const newTexts = extractFacts(newMessages);
    if (newTexts.length === 0) return;

    setFacts((prev) => {
      // Collect existing text keys to avoid duplicates
      const existing = new Set(prev.map((f) => f.text.toLowerCase().replace(/\s+/g, " ")));
      const tabFacts = prev.filter((f) => f.tabId === tabId && !f.pinned);

      // Stay under per-tab cap
      const slots = MAX_FACTS_PER_TAB - tabFacts.length;
      if (slots <= 0) return prev;

      const toAdd: MemoryFact[] = [];
      for (const text of newTexts.slice(0, slots)) {
        const key = text.toLowerCase().replace(/\s+/g, " ");
        if (!existing.has(key)) {
          existing.add(key);
          toAdd.push({
            id: crypto.randomUUID(),
            text,
            source: "extracted",
            pinned: false,
            tabId,
            createdAt: Date.now(),
          });
        }
      }
      return toAdd.length > 0 ? [...prev, ...toAdd] : prev;
    });
  }, []);

  const addManual = useCallback((text: string, tabId: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    setFacts((prev) => [
      ...prev,
      {
        id: crypto.randomUUID(),
        text: trimmed,
        source: "manual",
        pinned: false,
        tabId,
        createdAt: Date.now(),
      },
    ]);
  }, []);

  const pinFact = useCallback((id: string) => {
    setFacts((prev) => {
      const next = prev.map((f) =>
        f.id === id ? { ...f, pinned: true, tabId: "__pinned__" } : f
      );
      savePinned(next.filter((f) => f.pinned));
      return next;
    });
  }, []);

  const unpinFact = useCallback((id: string) => {
    setFacts((prev) => {
      const next = prev.map((f) =>
        f.id === id ? { ...f, pinned: false } : f
      );
      savePinned(next.filter((f) => f.pinned));
      return next;
    });
  }, []);

  const deleteFact = useCallback((id: string) => {
    setFacts((prev) => {
      const next = prev.filter((f) => f.id !== id);
      savePinned(next.filter((f) => f.pinned));
      return next;
    });
  }, []);

  const editFact = useCallback((id: string, newText: string) => {
    const trimmed = newText.trim();
    if (!trimmed) return;
    setFacts((prev) => {
      const next = prev.map((f) => (f.id === id ? { ...f, text: trimmed } : f));
      savePinned(next.filter((f) => f.pinned));
      return next;
    });
  }, []);

  const clearTabFacts = useCallback((tabId: string) => {
    setFacts((prev) => prev.filter((f) => f.pinned || f.tabId !== tabId));
  }, []);

  /** Returns a formatted string injected into the AI system prompt. */
  const getPinnedSystemPromptText = useCallback((): string => {
    const pinned = facts.filter((f) => f.pinned);
    if (pinned.length === 0) return "";
    return "User preferences and remembered facts:\n" + pinned.map((f) => `- ${f.text}`).join("\n");
  }, [facts]);

  const factsForTab = useCallback(
    (tabId: string) => facts.filter((f) => f.pinned || f.tabId === tabId),
    [facts]
  );

  return {
    facts,
    factsForTab,
    extractFromMessages,
    addManual,
    pinFact,
    unpinFact,
    deleteFact,
    editFact,
    clearTabFacts,
    getPinnedSystemPromptText,
  };
}
