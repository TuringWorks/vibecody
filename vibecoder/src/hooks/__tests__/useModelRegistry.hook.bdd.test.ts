/**
 * Behavioural tests for the `useModelRegistry` hook — the TTL cache and the
 * dynamic Ollama refresh.
 *
 * Split from useModelRegistry.bdd.test.ts, which became a pure registry-integrity
 * guard when the phantom `gemini-3.5-pro` was found. That rewrite dropped every
 * test that actually *ran* the hook, leaving refresh, caching and the loading
 * flag with no coverage at all. Those are restored here.
 *
 * The division of labour: that file asserts things about the static tables and
 * never mounts anything; this file mounts the hook and asserts what it does.
 *
 * Scenarios:
 *  1. Static providers are available on first mount (no cache, no backend)
 *  2. modelsForProvider returns the static list / [] for an unknown provider
 *  3. refresh() calls invoke("ollama_list_models")
 *  4. Dynamic Ollama models replace the static list when the backend responds
 *  5. When the Ollama backend throws, the static list is kept
 *  6. The cache is written to localStorage after a refresh
 *  7. A fresh cache (< 2h) is used on mount without calling the backend
 *  8. An expired cache (>= 2h) is ignored and triggers a refresh
 *  9. The loading flag is true during refresh and false after
 */

import { renderHook, act, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// ── Mock Tauri invoke ──────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  useModelRegistry,
  STATIC_MODELS,
  ALL_PROVIDERS,
  CACHE_KEY,
} from "../useModelRegistry";

// Imported, not re-declared: a local copy silently went stale when the hook
// bumped the key (it is on `:v3` now), so these tests wrote to a key nothing
// reads and every cache assertion passed vacuously.
const TWO_HOURS_MS = 2 * 60 * 60 * 1000;

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  // Default posture: Ollama not running.
  mockInvoke.mockRejectedValue(new Error("Ollama not running"));
});

afterEach(() => vi.restoreAllMocks());

// ── Scenario 1: static providers without cache or backend ─────────────────────

describe("Given no cache and no backend", () => {
  it("When the hook mounts, Then every known provider is listed", () => {
    const { result } = renderHook(() => useModelRegistry());
    for (const p of ALL_PROVIDERS) {
      expect(result.current.providers).toContain(p);
    }
  });
});

// ── Scenario 2: modelsForProvider ─────────────────────────────────────────────

describe("Given the hook has loaded", () => {
  it('When modelsForProvider("openai") is called, Then it returns the static OpenAI list', () => {
    const { result } = renderHook(() => useModelRegistry());
    expect(result.current.modelsForProvider("openai")).toEqual(STATIC_MODELS.openai);
  });

  it("When modelsForProvider() is given an unknown provider, Then it returns an empty array", () => {
    const { result } = renderHook(() => useModelRegistry());
    expect(result.current.modelsForProvider("unknown-provider")).toEqual([]);
  });
});

// ── Scenarios 3 & 4: dynamic Ollama refresh ───────────────────────────────────

describe("Given Ollama is running and returns models", () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue(["llama3.2", "mistral", "phi3"]);
  });

  it('When refresh() is called, Then invoke("ollama_list_models") is called', async () => {
    const { result } = renderHook(() => useModelRegistry());
    await act(async () => {
      await result.current.refresh();
    });
    expect(mockInvoke).toHaveBeenCalledWith("ollama_list_models");
  });

  it('When refresh() resolves, Then modelsForProvider("ollama") returns the dynamic list', async () => {
    const { result } = renderHook(() => useModelRegistry());
    await act(async () => {
      await result.current.refresh();
    });
    expect(result.current.modelsForProvider("ollama")).toContain("llama3.2");
    expect(result.current.modelsForProvider("ollama")).toContain("mistral");
  });
});

// ── Scenario 5: the backend failing must not empty the picker ─────────────────

describe("Given Ollama is not running (invoke throws)", () => {
  it('When refresh() is called, Then modelsForProvider("ollama") keeps the static list', async () => {
    const { result } = renderHook(() => useModelRegistry());
    const staticOllama = [...STATIC_MODELS.ollama];
    await act(async () => {
      await result.current.refresh();
    });
    expect(result.current.modelsForProvider("ollama")).toEqual(staticOllama);
  });
});

// ── Scenario 6: the cache is written after a refresh ──────────────────────────

describe("Given a successful refresh", () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue(["qwen3", "gemma2"]);
  });

  it("When refresh() completes, Then localStorage holds the cache key", async () => {
    const { result } = renderHook(() => useModelRegistry());
    await act(async () => {
      await result.current.refresh();
    });
    expect(localStorage.getItem(CACHE_KEY)).not.toBeNull();
  });

  it("When refresh() completes, Then the cached ollama models include the dynamic list", async () => {
    const { result } = renderHook(() => useModelRegistry());
    await act(async () => {
      await result.current.refresh();
    });
    const cached = JSON.parse(localStorage.getItem(CACHE_KEY)!);
    expect(cached.models.ollama).toContain("qwen3");
  });
});

// ── Scenario 7: a fresh cache is used without hitting the backend ─────────────

describe("Given a fresh cache (< 2 hours old) in localStorage", () => {
  it("When the hook mounts, Then the cached models are used without calling invoke", async () => {
    const cachedOllamaModels = ["cached-model-1", "cached-model-2"];
    localStorage.setItem(
      CACHE_KEY,
      JSON.stringify({
        providers: ALL_PROVIDERS,
        models: { ...STATIC_MODELS, ollama: cachedOllamaModels },
        updatedAt: Date.now() - 1000, // 1 second old
      }),
    );

    const { result } = renderHook(() => useModelRegistry());
    await waitFor(() => {
      expect(result.current.modelsForProvider("ollama")).toEqual(cachedOllamaModels);
    });
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

// ── Scenario 8: an expired cache triggers a refresh ───────────────────────────

describe("Given an expired cache (>= 2 hours old) in localStorage", () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue(["fresh-model"]);
    localStorage.setItem(
      CACHE_KEY,
      JSON.stringify({
        providers: ALL_PROVIDERS,
        models: { ...STATIC_MODELS },
        updatedAt: Date.now() - TWO_HOURS_MS - 1, // just over the TTL
      }),
    );
  });

  it('When the hook mounts, Then invoke("ollama_list_models") is called', async () => {
    renderHook(() => useModelRegistry());
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("ollama_list_models");
    });
  });
});

// ── Scenario 9: the loading flag ──────────────────────────────────────────────

describe("Given a slow backend response", () => {
  it("When refresh() is in flight, Then loading is true; after completion it is false", async () => {
    let resolve!: () => void;
    mockInvoke.mockReturnValue(
      new Promise<string[]>((r) => {
        resolve = () => r([]);
      }),
    );

    const { result } = renderHook(() => useModelRegistry());
    const refreshPromise = act(async () => {
      result.current.refresh();
    });

    await waitFor(() => expect(result.current.loading).toBe(true));

    act(() => {
      resolve();
    });
    await refreshPromise;

    expect(result.current.loading).toBe(false);
  });
});
