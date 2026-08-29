/**
 * Behavioural tests for the `useModelRegistry` hook — now a binding over the
 * shared `useDaemonModels`.
 *
 * VibeCoder used to build its own provider→model matrix from `STATIC_MODELS`
 * plus a client-side `ollama_list_models` merge, so every model change had to
 * be made twice: once in `vibe-ai/src/catalog.rs` for the daemon and every thin
 * client, and again in TypeScript here. It reads the daemon's `/models` now,
 * and `STATIC_MODELS` is demoted to a first-run fallback.
 *
 * These scenarios are the previous ones carried over — the requirements did not
 * change, only where the list comes from:
 *
 *  1. Providers are available on first mount with no cache and no daemon
 *  2. modelsForProvider returns a list / [] for an unknown provider
 *  3. The hook asks the daemon, not Ollama directly
 *  4. Daemon rows replace the static fallback
 *  5. When the daemon is unreachable, the fallback is kept
 *  6. A successful read is cached
 *  7. A cache is served on mount before the daemon answers
 *  8. The cache is superseded by a live answer, not trusted over it
 *  9. The loading flag is true during the first read and false after
 * 10. An empty daemon answer does not blank the picker
 */

import { renderHook, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { useModelRegistry, STATIC_MODELS, ALL_PROVIDERS } from "../useModelRegistry";

/** Must match `DAEMON_MODELS_CACHE_KEY` in the hook. */
const CACHE = "vibecody:daemon-models:v1";

function rows(...pairs: [string, string][]) {
  return pairs.map(([provider, name]) => ({ id: `${provider}/${name}`, name, provider }));
}

beforeEach(() => {
  mockInvoke.mockReset();
  localStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Given no cache and no daemon", () => {
  it("When the hook mounts, Then the static fallback still lists providers", async () => {
    mockInvoke.mockRejectedValue(new Error("daemon down"));
    const { result } = renderHook(() => useModelRegistry());

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.providers).toEqual(ALL_PROVIDERS);
    expect(result.current.source).toBe("fallback");
  });

  it("When a provider is unknown, Then modelsForProvider returns []", async () => {
    mockInvoke.mockRejectedValue(new Error("daemon down"));
    const { result } = renderHook(() => useModelRegistry());

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.modelsForProvider("nonesuch")).toEqual([]);
    expect(result.current.modelsForProvider("openai")).toEqual(STATIC_MODELS.openai);
  });
});

describe("Given the daemon answers", () => {
  it("When the hook mounts, Then it asks the daemon rather than Ollama directly", async () => {
    mockInvoke.mockResolvedValue(rows(["ollama", "qwen3-coder"]));
    renderHook(() => useModelRegistry());

    // The daemon already merges live Ollama tags, the cloud list and every
    // keyed provider; asking Ollama here would be a second, narrower source.
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("list_daemon_models", expect.anything())
    );
    expect(mockInvoke).not.toHaveBeenCalledWith("ollama_list_models");
  });

  it("When rows arrive, Then they replace the static fallback", async () => {
    mockInvoke.mockResolvedValue(
      rows(["ollama", "qwen3-coder"], ["ollama", "glm-5.3:cloud"], ["claude", "claude-opus-5"])
    );
    const { result } = renderHook(() => useModelRegistry());

    await waitFor(() => expect(result.current.source).toBe("live"));
    expect(result.current.modelsForProvider("ollama")).toEqual([
      "qwen3-coder",
      "glm-5.3:cloud",
    ]);
    expect(result.current.modelsForProvider("claude")).toEqual(["claude-opus-5"]);
  });

  it("Then only providers the daemon actually serves are offered", async () => {
    mockInvoke.mockResolvedValue(rows(["claude", "claude-opus-5"]));
    const { result } = renderHook(() => useModelRegistry());

    // Offering a provider the daemon cannot build produces a selection that
    // fails at request time.
    await waitFor(() => expect(result.current.source).toBe("live"));
    expect(result.current.providers).toEqual(["claude"]);
  });

  it("Then the answer is cached", async () => {
    mockInvoke.mockResolvedValue(rows(["claude", "claude-opus-5"]));
    const { result } = renderHook(() => useModelRegistry());

    await waitFor(() => expect(result.current.source).toBe("live"));
    expect(JSON.parse(localStorage.getItem(CACHE) ?? "[]")).toHaveLength(1);
  });

  /**
   * An empty success is not an answer worth keeping over a good one — caching
   * it would blank the picker and persist the blank.
   */
  it("When the answer is empty, Then the picker is not blanked", async () => {
    localStorage.setItem(CACHE, JSON.stringify(rows(["claude", "claude-opus-5"])));
    mockInvoke.mockResolvedValue([]);
    const { result } = renderHook(() => useModelRegistry());

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.modelsForProvider("claude")).toEqual(["claude-opus-5"]);
    expect(result.current.source).toBe("cache");
  });
});

describe("Given a cached answer from a previous session", () => {
  it("When the hook mounts, Then the cache is shown before the daemon replies", () => {
    localStorage.setItem(CACHE, JSON.stringify(rows(["claude", "cached-model"])));
    mockInvoke.mockReturnValue(new Promise(() => {})); // never settles
    const { result } = renderHook(() => useModelRegistry());

    expect(result.current.source).toBe("cache");
    expect(result.current.modelsForProvider("claude")).toEqual(["cached-model"]);
  });

  it("When the daemon replies, Then the live answer wins over the cache", async () => {
    localStorage.setItem(CACHE, JSON.stringify(rows(["claude", "stale-model"])));
    mockInvoke.mockResolvedValue(rows(["claude", "fresh-model"]));
    const { result } = renderHook(() => useModelRegistry());

    await waitFor(() => expect(result.current.source).toBe("live"));
    expect(result.current.modelsForProvider("claude")).toEqual(["fresh-model"]);
  });

  it("When the daemon is unreachable, Then the cache is kept, not the fallback", async () => {
    localStorage.setItem(CACHE, JSON.stringify(rows(["claude", "cached-model"])));
    mockInvoke.mockRejectedValue(new Error("daemon down"));
    const { result } = renderHook(() => useModelRegistry());

    // The cache is the daemon's own previous answer, so it beats a hardcoded
    // list that may name models this daemon never served.
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.source).toBe("cache");
    expect(result.current.modelsForProvider("claude")).toEqual(["cached-model"]);
  });
});

describe("Given a slow daemon", () => {
  it("When the first read is in flight, Then loading is true; after it, false", async () => {
    let settle: (v: unknown) => void = () => {};
    mockInvoke.mockReturnValue(new Promise((res) => (settle = res)));
    const { result } = renderHook(() => useModelRegistry());

    expect(result.current.loading).toBe(true);
    settle(rows(["claude", "claude-opus-5"]));
    await waitFor(() => expect(result.current.loading).toBe(false));
  });
});
