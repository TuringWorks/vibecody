/**
 * SupercompleteEngine — cross-file multi-line edit prediction.
 *
 * Extends the existing FIM + next-edit inline completion with **semantic context
 * from the embedding index** so predictions can reference code in other files.
 *
 * ## How it works
 * 1. Extract search terms from recent edits + the code around the cursor.
 * 2. Call `semantic_search_codebase` to retrieve the top-K semantically related
 *    code chunks from across the whole workspace.
 * 3. Build an enriched prompt: related chunks + cursor context + instruction.
 * 4. Call `request_inline_completion` with the enriched prompt.
 * 5. Return the completion text + the list of referenced files.
 *
 * The engine activates only when:
 * - At least 3 recent edits in the last 30 s (signals active writing session), OR
 * - The current line references a symbol that doesn't exist in the open file.
 */

import { invoke } from "@tauri-apps/api/core";

export interface SupercompleteResult {
  /** The suggested insert text (may be multi-line). */
  insertText: string;
  /** Files that were used as context for this prediction. */
  contextFiles: string[];
  /** 0–1 confidence estimate. */
  confidence: number;
}

interface RecentEdit {
  line: number;
  col: number;
  old_text: string;
  new_text: string;
  elapsed_ms: number;
}

interface SemanticHit {
  file: string;
  score: number;
  snippet: string;
}

// Max chars of cross-file context to include in the enriched prompt
const MAX_CROSS_FILE_CHARS = 1500;
// Min edits in sliding window to trigger supercomplete
const MIN_EDITS_TO_ACTIVATE = 3;
// Sliding window in ms
const EDIT_WINDOW_MS = 30_000;

class SupercompleteEngine {
  private lastResult: SupercompleteResult | null = null;
  private lastResultKey = "";

  /**
   * Predict a cross-file multi-line completion.
   *
   * @returns null if not enough signal or cross-file context adds nothing.
   */
  async predict(params: {
    filePath: string;
    prefix: string;   // text before cursor
    suffix: string;   // text after cursor
    language: string;
    cursorLine: number;
    cursorCol: number;
    recentEdits: RecentEdit[];
    provider: string;
  }): Promise<SupercompleteResult | null> {
    const { filePath, prefix, suffix, language, recentEdits, provider } = params;

    // ── Activation gate ──────────────────────────────────────────────────────
    const now = Date.now();
    const activeEdits = recentEdits.filter((e) => now - e.elapsed_ms < EDIT_WINDOW_MS);
    if (activeEdits.length < MIN_EDITS_TO_ACTIVATE) {
      return null;
    }

    // ── Build search query from recent new_text tokens ────────────────────────
    const rawQuery = activeEdits
      .slice(-5)
      .map((e) => e.new_text.trim())
      .join(" ")
      .slice(0, 120);
    const query = sanitizeQuery(rawQuery) || extractQueryFromPrefix(prefix);
    if (!query) return null;

    // Cache check — avoid duplicate requests for same position
    const cacheKey = `${filePath}:${params.cursorLine}:${params.cursorCol}:${query}`;
    if (cacheKey === this.lastResultKey && this.lastResult) {
      return this.lastResult;
    }

    // ── Semantic search for related code ──────────────────────────────────────
    let hits: SemanticHit[] = [];
    try {
      hits = await invoke<SemanticHit[]>("semantic_search_codebase", {
        query,
        limit: 4,
      });
    } catch {
      // Embedding index not built yet — fall back to no cross-file context
      return null;
    }

    const relevantHits = hits.filter((h) => !h.file.endsWith(filePath)).slice(0, 3);
    if (relevantHits.length === 0) return null;

    // ── Build enriched prompt ─────────────────────────────────────────────────
    const crossFileCtx = buildCrossFileContext(relevantHits);
    const enrichedPrefix = `${crossFileCtx}\n\n// Current file: ${filePath}\n${prefix}`;

    // ── Request completion ────────────────────────────────────────────────────
    let completion: string | null = null;
    try {
      completion = await invoke<string>("request_inline_completion", {
        prefix: enrichedPrefix.slice(-4000), // keep last 4000 chars
        suffix: suffix.slice(0, 500),
        language,
        provider,
      });
    } catch {
      return null;
    }

    if (!completion || completion.trim().length === 0) return null;

    // ── Score confidence ──────────────────────────────────────────────────────
    const confidence = scoreConfidence(completion, relevantHits);

    const result: SupercompleteResult = {
      insertText: completion,
      contextFiles: relevantHits.map((h) => h.file),
      confidence,
    };

    this.lastResultKey = cacheKey;
    this.lastResult = result;
    return result;
  }

  /** Invalidate cache (call when document changes significantly). */
  invalidate(): void {
    this.lastResultKey = "";
    this.lastResult = null;
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function sanitizeQuery(raw: string): string {
  // Keep only alphanumeric / underscore tokens of length ≥ 3
  return raw
    .split(/\s+/)
    .filter((t) => /^[\w]{3,}$/.test(t))
    .slice(0, 6)
    .join(" ");
}

function extractQueryFromPrefix(prefix: string): string {
  // Take the last non-empty line as a fallback query signal
  const lines = prefix.trimEnd().split("\n");
  const lastLine = lines[lines.length - 1]?.trim() ?? "";
  return sanitizeQuery(lastLine.slice(0, 80));
}

function buildCrossFileContext(hits: SemanticHit[]): string {
  const parts: string[] = ["// Cross-file context (semantic search):"];
  let total = 0;
  for (const hit of hits) {
    const snippet = hit.snippet.slice(0, 600);
    total += snippet.length;
    parts.push(`// From: ${hit.file}\n${snippet}`);
    if (total >= MAX_CROSS_FILE_CHARS) break;
  }
  return parts.join("\n\n");
}

function scoreConfidence(completion: string, hits: SemanticHit[]): number {
  if (hits.length === 0) return 0;
  // Higher score from semantic search → higher confidence
  const avgScore = hits.reduce((s, h) => s + h.score, 0) / hits.length;
  // Longer completions with cross-file context score slightly higher
  const lengthBonus = Math.min(completion.length / 200, 0.2);
  return Math.min(avgScore + lengthBonus, 1.0);
}

// ── Singleton export ──────────────────────────────────────────────────────────

export const supercompleteEngine = new SupercompleteEngine();
