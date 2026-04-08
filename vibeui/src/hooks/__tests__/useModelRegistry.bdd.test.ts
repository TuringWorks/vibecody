/**
 * BDD tests for useModelRegistry — provider/model matrix with TTL cache.
 *
 * Scenarios:
 *  1. Returns static providers on first mount (no cache, no backend)
 *  2. Static models are present for all known providers
 *  3. PROVIDER_DEFAULT_MODEL covers every provider in STATIC_MODELS
 *  4. modelsForProvider returns the model list for a known provider
 *  5. modelsForProvider returns [] for an unknown provider
 *  6. Dynamic refresh calls invoke("ollama_list_models")
 *  7. Dynamic Ollama models replace the static list when backend responds
 *  8. When Ollama backend throws, static list is kept
 *  9. Cache is written to localStorage after a refresh
 * 10. Cache is loaded from localStorage when fresh (< 2 hours old)
 * 11. Expired cache (>= 2 hours) is ignored and triggers a refresh
 * 12. loading flag is true during refresh and false after
 */

import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import {
  useModelRegistry,
  STATIC_MODELS,
  ALL_PROVIDERS,
  PROVIDER_DEFAULT_MODEL,
} from '../useModelRegistry';

const CACHE_KEY = 'vibecody:model-registry';
const TWO_HOURS_MS = 2 * 60 * 60 * 1000;

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  // Default: Ollama not running
  mockInvoke.mockRejectedValue(new Error('Ollama not running'));
});

afterEach(() => vi.restoreAllMocks());

// ── Scenario 1: Static providers returned without cache ───────────────────────

describe('Given no cache and no backend', () => {
  it('When the hook mounts, Then providers includes known providers like "claude" and "openai"', async () => {
    const { result } = renderHook(() => useModelRegistry());
    expect(result.current.providers).toContain('claude');
    expect(result.current.providers).toContain('openai');
    expect(result.current.providers).toContain('ollama');
  });

  it('When the hook mounts, Then all ALL_PROVIDERS entries are in the providers list', () => {
    const { result } = renderHook(() => useModelRegistry());
    for (const p of ALL_PROVIDERS) {
      expect(result.current.providers).toContain(p);
    }
  });
});

// ── Scenario 2: Static models present for all providers ──────────────────────

describe('Given STATIC_MODELS', () => {
  it('Then every provider has at least one model entry (or an empty array for vercel_ai)', () => {
    for (const [provider, models] of Object.entries(STATIC_MODELS)) {
      if (provider === 'vercel_ai') continue; // intentionally empty
      expect(models.length, `${provider} has no models`).toBeGreaterThan(0);
    }
  });

  it('Then claude models include claude-sonnet-4-6', () => {
    expect(STATIC_MODELS.claude).toContain('claude-sonnet-4-6');
  });

  it('Then openai models include gpt-4o', () => {
    expect(STATIC_MODELS.openai).toContain('gpt-4o');
  });
});

// ── Scenario 3: PROVIDER_DEFAULT_MODEL covers all providers ──────────────────

describe('Given PROVIDER_DEFAULT_MODEL', () => {
  it('Then every provider in STATIC_MODELS has a default model entry', () => {
    for (const provider of Object.keys(STATIC_MODELS)) {
      expect(
        Object.prototype.hasOwnProperty.call(PROVIDER_DEFAULT_MODEL, provider),
        `${provider} missing from PROVIDER_DEFAULT_MODEL`
      ).toBe(true);
    }
  });

  it('Then claude default is claude-sonnet-4-6', () => {
    expect(PROVIDER_DEFAULT_MODEL.claude).toBe('claude-sonnet-4-6');
  });

  it('Then openai default is gpt-4o', () => {
    expect(PROVIDER_DEFAULT_MODEL.openai).toBe('gpt-4o');
  });
});

// ── Scenario 4 & 5: modelsForProvider ────────────────────────────────────────

describe('Given the hook has loaded', () => {
  it('When modelsForProvider("openai") is called, Then it returns the static OpenAI model list', () => {
    const { result } = renderHook(() => useModelRegistry());
    expect(result.current.modelsForProvider('openai')).toEqual(STATIC_MODELS.openai);
  });

  it('When modelsForProvider("unknown-provider") is called, Then it returns an empty array', () => {
    const { result } = renderHook(() => useModelRegistry());
    expect(result.current.modelsForProvider('unknown-provider')).toEqual([]);
  });
});

// ── Scenario 6 & 7: Dynamic Ollama refresh ───────────────────────────────────

describe('Given Ollama is running and returns models', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue(['llama3.2', 'mistral', 'phi3']);
  });

  it('When refresh() is called, Then invoke("ollama_list_models") is called', async () => {
    const { result } = renderHook(() => useModelRegistry());
    await act(async () => { await result.current.refresh(); });
    expect(mockInvoke).toHaveBeenCalledWith('ollama_list_models');
  });

  it('When refresh() resolves, Then modelsForProvider("ollama") returns the dynamic list', async () => {
    const { result } = renderHook(() => useModelRegistry());
    await act(async () => { await result.current.refresh(); });
    expect(result.current.modelsForProvider('ollama')).toContain('llama3.2');
    expect(result.current.modelsForProvider('ollama')).toContain('mistral');
  });
});

// ── Scenario 8: Graceful Ollama failure ──────────────────────────────────────

describe('Given Ollama is not running (invoke throws)', () => {
  it('When refresh() is called, Then modelsForProvider("ollama") keeps the static list', async () => {
    const { result } = renderHook(() => useModelRegistry());
    const staticOllama = [...STATIC_MODELS.ollama];
    await act(async () => { await result.current.refresh(); });
    expect(result.current.modelsForProvider('ollama')).toEqual(staticOllama);
  });
});

// ── Scenario 9: Cache is written after refresh ───────────────────────────────

describe('Given a successful refresh', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue(['qwen3', 'gemma2']);
  });

  it('When refresh() completes, Then localStorage contains the cache key', async () => {
    const { result } = renderHook(() => useModelRegistry());
    await act(async () => { await result.current.refresh(); });
    expect(localStorage.getItem(CACHE_KEY)).not.toBeNull();
  });

  it('When refresh() completes, Then the cached ollama models include the dynamic list', async () => {
    const { result } = renderHook(() => useModelRegistry());
    await act(async () => { await result.current.refresh(); });
    const cached = JSON.parse(localStorage.getItem(CACHE_KEY)!);
    expect(cached.models.ollama).toContain('qwen3');
  });
});

// ── Scenario 10: Fresh cache is loaded on mount ───────────────────────────────

describe('Given a fresh cache (< 2 hours old) in localStorage', () => {
  it('When the hook mounts, Then the cached models are used without calling invoke', async () => {
    const cachedOllamaModels = ['cached-model-1', 'cached-model-2'];
    const cached = {
      providers: ALL_PROVIDERS,
      models: { ...STATIC_MODELS, ollama: cachedOllamaModels },
      updatedAt: Date.now() - 1000, // 1 second old
    };
    localStorage.setItem(CACHE_KEY, JSON.stringify(cached));

    const { result } = renderHook(() => useModelRegistry());
    // Cache is fresh — should not trigger a refresh
    await waitFor(() => {
      expect(result.current.modelsForProvider('ollama')).toEqual(cachedOllamaModels);
    });
    // invoke should not be called since cache is fresh
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

// ── Scenario 11: Expired cache triggers refresh ───────────────────────────────

describe('Given an expired cache (>= 2 hours old) in localStorage', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue(['fresh-model']);
    const expired = {
      providers: ALL_PROVIDERS,
      models: { ...STATIC_MODELS },
      updatedAt: Date.now() - TWO_HOURS_MS - 1, // just over 2 hours
    };
    localStorage.setItem(CACHE_KEY, JSON.stringify(expired));
  });

  it('When the hook mounts, Then invoke("ollama_list_models") is called', async () => {
    renderHook(() => useModelRegistry());
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('ollama_list_models');
    });
  });
});

// ── Scenario 12: loading flag ─────────────────────────────────────────────────

describe('Given a slow backend response', () => {
  it('When refresh() is in flight, Then loading is true; after completion it is false', async () => {
    let resolve!: () => void;
    mockInvoke.mockReturnValue(new Promise<string[]>(r => { resolve = () => r([]); }));

    const { result } = renderHook(() => useModelRegistry());
    const refreshPromise = act(async () => { result.current.refresh(); });

    // loading becomes true once refresh starts
    await waitFor(() => expect(result.current.loading).toBe(true));

    // Resolve the backend call
    act(() => { resolve(); });
    await refreshPromise;

    expect(result.current.loading).toBe(false);
  });
});
