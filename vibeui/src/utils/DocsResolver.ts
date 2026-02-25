/**
 * DocsResolver — fetch library documentation for `@docs:<name>` context references.
 *
 * Detects the registry from the package name and file context:
 * - Rust crates → docs.rs
 * - npm packages → npmjs.com
 * - Python packages → pypi.org
 *
 * Results are cached in sessionStorage for 24 hours to avoid re-fetching.
 */

import { invoke } from "@tauri-apps/api/core";

export type DocRegistry = "rs" | "npm" | "py";

export interface DocResult {
  name: string;
  registry: DocRegistry;
  summary: string;
  /** Resolved version string if available. */
  version?: string;
}

const CACHE_KEY_PREFIX = "vibeui:docs:";
const CACHE_TTL_MS = 24 * 60 * 60 * 1000; // 24 hours

interface CacheEntry {
  result: DocResult;
  cachedAt: number;
}

function cacheGet(key: string): DocResult | null {
  try {
    const raw = sessionStorage.getItem(CACHE_KEY_PREFIX + key);
    if (!raw) return null;
    const entry: CacheEntry = JSON.parse(raw);
    if (Date.now() - entry.cachedAt > CACHE_TTL_MS) {
      sessionStorage.removeItem(CACHE_KEY_PREFIX + key);
      return null;
    }
    return entry.result;
  } catch {
    return null;
  }
}

function cacheSet(key: string, result: DocResult): void {
  try {
    const entry: CacheEntry = { result, cachedAt: Date.now() };
    sessionStorage.setItem(CACHE_KEY_PREFIX + key, JSON.stringify(entry));
  } catch {
    // Ignore quota errors
  }
}

/**
 * Detect which registry a package name belongs to.
 *
 * Heuristics:
 * - Name starts with `rs:` or ends in `.rs` → Rust
 * - Name starts with `py:` → Python
 * - Name starts with `npm:` → npm
 * - Name containing `-` and all lowercase with no `.` → likely Rust (Cargo convention)
 * - Name containing `/` or `@` → npm (scoped package)
 * - Default → npm
 */
export function detectRegistry(name: string): DocRegistry {
  if (name.startsWith("rs:") || name.endsWith(".rs")) return "rs";
  if (name.startsWith("py:") || name.startsWith("pypi:")) return "py";
  if (name.startsWith("npm:")) return "npm";
  if (name.includes("/") || name.startsWith("@")) return "npm";
  // Rust crate names are lowercase with hyphens/underscores, no dots
  if (/^[a-z][a-z0-9_-]*$/.test(name) && !name.includes(".")) {
    // Ambiguous — default to Rust for single-word names that look like crate names
    return "rs";
  }
  return "npm";
}

/** Strip the `rs:`, `py:`, `npm:`, `pypi:` prefix if present. */
function stripPrefix(name: string): string {
  return name.replace(/^(rs|py|npm|pypi):/, "");
}

/**
 * Resolve a `@docs:<name>` reference to a formatted documentation string.
 * Returns the doc text suitable for injection into an AI chat message.
 */
export async function resolveDoc(rawName: string): Promise<DocResult> {
  const registry = detectRegistry(rawName);
  const name = stripPrefix(rawName.split(":").pop() ?? rawName);
  const cacheKey = `${registry}:${name}`;

  const cached = cacheGet(cacheKey);
  if (cached) return cached;

  try {
    const raw = await invoke<string>("fetch_doc_content", { name, registry });
    const result: DocResult = { name, registry, summary: raw };
    cacheSet(cacheKey, result);
    return result;
  } catch (err) {
    return {
      name,
      registry,
      summary: `(Could not fetch docs for "${name}" from ${registry}: ${err})`,
    };
  }
}

/**
 * Format a DocResult for injection into a chat message.
 */
export function formatDocForContext(doc: DocResult): string {
  const registryLabel: Record<DocRegistry, string> = {
    rs: "docs.rs",
    npm: "npmjs.com",
    py: "PyPI",
  };
  return `=== Documentation: ${doc.name} (${registryLabel[doc.registry]}) ===\n${doc.summary}`;
}
