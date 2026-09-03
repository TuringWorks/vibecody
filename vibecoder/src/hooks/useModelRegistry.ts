/**
 * useModelRegistry — Shared cached provider→model matrix.
 *
 * Caches the provider/model list in localStorage with a 2-hour TTL.
 * All panels that need model selection import this hook to get
 * consistent, fast model dropdowns without redundant API calls.
 */
import { useCallback, useMemo } from "react";
import {
  useDaemonModels,
  providersOf,
  type DaemonModel,
} from "@vibe/shared/hooks/useDaemonModels";
import { OLLAMA_CHAT_MODELS } from "../constants/ollamaModels";

/**
 * Versioned: bump when the static catalog / merge logic changes so a stale
 * cache (e.g. one saved before Ollama Cloud models were added) is discarded on
 * load instead of lingering until the TTL expires and hiding the newer models.
 *
 * Exported so tests seed and assert against the *same* constant. When the key
 * was duplicated as a literal in the test file, the `:v2` bump silently left
 * the tests writing to a key nothing reads — they failed without anything
 * being wrong with the hook.
 */
// v4 (2026-09-02): retired-model sweep — a v3 cache still holds
// gpt-5.6-sol-pro (never a real id), deepseek-chat (retired 2026-07-24),
// grok-4.20, and gemini-3-pro / gemini-3.1-pro / gemini-2.5-pro, all of which
// fail at request time. v3 (2026-08-05) swept deepseek-v3.1:671b-cloud and
// friends, which 410.
export const CACHE_KEY = "vibecody:model-registry:v4";

/**
 * Where the list actually comes from now.
 *
 * VibeCoder used to build its own provider→model matrix from `STATIC_MODELS`
 * plus a client-side merge of `ollama_list_models`, which meant every model
 * change had to be made twice — once in `vibe-ai/src/catalog.rs` for the
 * daemon and every thin client, and again here. The daemon's `/models` already
 * merges live Ollama tags, the cloud list and every keyed provider, so this
 * reads that instead and `STATIC_MODELS` is demoted to a first-run fallback.
 */
const DAEMON_URL = "http://localhost:7878";
const DAEMON_MODELS_CACHE_KEY = "vibecody:daemon-models:v1";
const CACHE_TTL_MS = 2 * 60 * 60 * 1000; // 2 hours

/**
 * Single source of truth for providers and models.
 *
 * To add a provider: add an entry to STATIC_MODELS and PROVIDER_DEFAULT_MODEL.
 * To add/update models for a provider: edit the array in STATIC_MODELS.
 * All panels consume this via useModelRegistry() — no other file needs changing.
 */

/** Known models per provider (static fallback when API unavailable) */
export const STATIC_MODELS: Record<string, string[]> = {
  // claude-code uses the local Claude Code CLI — works with Free, Pro, Max, Team, and Enterprise plans
  // without consuming Anthropic API credits.
  //
  // Both Claude rows are swept against Anthropic's own model-status table
  // (platform.claude.com/docs/en/about-claude/model-deprecations, read
  // 2026-09-02) and hold only rows marked `Active`.
  //
  // claude-fable-5-1 added 2026-09-02 (Active, retirement not sooner than
  // 2027-09-01). claude-sonnet-4-5 dropped the same day: still Active, but its
  // tentative retirement is "not sooner than September 29, 2026" — inside a
  // release cycle from now, while everything else here has months of runway.
  // claude-haiku-4-5 is the next shortest (not sooner than 2026-10-15) and is
  // kept only because it is Anthropic's sole cheap tier; re-check before then.
  //
  // Mythos 5 stays omitted: it was restored only to approved *US
  // organisations*, permanently. A flat string[] cannot say "available to some
  // callers", so listing it would 403 for most users. It waits on per-model
  // availability metadata.
  "claude-code": ["claude-fable-5-1", "claude-opus-5", "claude-fable-5", "claude-sonnet-5", "claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6", "claude-sonnet-4-6", "claude-haiku-4-5"],
  claude: ["claude-fable-5-1", "claude-opus-5", "claude-fable-5", "claude-sonnet-5", "claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6", "claude-sonnet-4-6", "claude-haiku-4-5"],
  // Swept 2026-09-02 against developers.openai.com/api/docs/models + /deprecations.
  //
  // The three `*-pro` ids that used to lead this row were never model ids.
  // "Pro" on the 5.6 family is a request parameter — `reasoning.mode: "pro"` on
  // the Responses API — and OpenAI's deprecation table spells the replacement
  // for gpt-5-pro-2025-10-06 as "gpt-5.6-sol (reasoning.mode: pro)". So
  // gpt-5.6-sol-pro and its siblings were three picker entries that could only
  // 404. gpt-5.5-pro is real: the separate -pro id died *with* the 5.6
  // generation, not before it. See the RULE on the gemini row — this is the
  // same failure, an id written the way the marketing name reads.
  //
  // gpt-5 removed: its only snapshot (gpt-5-2025-08-07) was deprecated
  // 2026-06-11, API shutdown 2026-12-11.
  //
  // gpt-4o / gpt-4o-mini / gpt-4.1 / gpt-4.1-mini are kept, at the end of the
  // row. They are two generations behind everything above and OpenAI's model
  // guidance routes all four to the 5.6 family, but the deprecation table still
  // lists them **Active on the API** — leaving ChatGPT on 2026-02-13 retired
  // them from the consumer product, not from here. The RULE below is about ids
  // that are not callable; "superseded" is not the same thing, and an id that
  // still answers stays until it is dated. Briefly cut in the 2026-09-02 sweep
  // and restored the same day.
  //
  // Their neighbours in that generation stay out: gpt-4.1-nano, gpt-4-turbo and
  // gpt-3.5-turbo are deprecated with an API shutdown on 2026-10-23, and gpt-5's
  // only snapshot (gpt-5-2025-08-07) shuts down 2026-12-11.
  openai: ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5-pro", "gpt-5.5", "gpt-5.4", "gpt-5.4-mini", "gpt-5.3-codex", "gpt-5.3-chat", "gpt-4.1", "gpt-4.1-mini", "gpt-4o", "gpt-4o-mini"],
  // gemini-3.6-flash is the current workhorse (shipped 2026-07-21) and stays the
  // default; 3.8-flash (2026-09-02) and 3.7-flash (2026-08-13) are offered but
  // not defaulted to until they have a track record.
  //
  // gemini-3.5-pro was removed 2026-08-10 for never having GA'd. The 2026-09-02
  // sweep found the *same* mistake twice more: `gemini-3.1-pro` and
  // `gemini-3-pro` were both listed and neither is a callable model code.
  // Google ships the current Pro as `gemini-3.1-pro-preview`, and
  // `gemini-3-pro-preview` is already in the deprecated/shut-down table. The
  // `-preview` suffix is load-bearing — an id written the way the marketing
  // name reads is not an id.
  //
  // gemini-2.5-pro removed: a newly created GCP project gets 404 "no longer
  // available to new users", so for most callers it is already retired ahead of
  // its published date. gemini-2.5-flash removed with it — the whole 2.5 line
  // goes no earlier than 2026-10-16.
  //
  // RULE: only ship model ids that are shipped and callable today. A forecast
  // belongs in a planning note, never in the registry — the narrative can
  // absorb a wrong projection, the code cannot. Enforced by
  // `defaults are present in their provider list` in useModelRegistry.test.ts.
  gemini: ["gemini-3.8-flash", "gemini-3.7-flash", "gemini-3.6-flash", "gemini-3.5-flash", "gemini-3.5-flash-lite", "gemini-3.1-flash-lite", "gemini-3.1-pro-preview"],
  // llama-3.1-8b-instant / llama-3.3-70b-versatile were deprecated 2026-06-17 (Groq
  // points at gpt-oss-20b / gpt-oss-120b); mixtral-8x7b-32768 and gemma2-9b-it are gone.
  groq: ["openai/gpt-oss-120b", "openai/gpt-oss-20b", "qwen/qwen3.6-27b", "minimaxai/minimax-m2.7", "groq/compound", "groq/compound-mini"],
  // grok-4.6 (2026-08-12) is xAI's current flagship. Bare "grok-4.20" dropped
  // 2026-09-02 — docs.x.ai lists no such id; that generation is addressed as
  // grok-4.20-0309-reasoning / -non-reasoning, and 4.6/4.5/4.3 supersede it.
  grok: ["grok-4.6", "grok-4.5", "grok-4.3"],
  mistral: ["mistral-large-latest", "mistral-medium-latest", "mistral-small-latest", "codestral-latest"],
  // "deepseek-v4" was never an API id — the shipped pair is v4-pro / v4-flash.
  // deepseek-chat / deepseek-reasoner removed 2026-09-02: DeepSeek retired both
  // legacy names on 2026-07-24 and its model-list endpoint now returns only the
  // v4 pair. deepseek-chat was also this provider's default, so every
  // unconfigured DeepSeek call was aimed at a retired id.
  deepseek: ["deepseek-v4-pro", "deepseek-v4-flash"],
  cerebras: ["gpt-oss-120b", "gemma-4-31b", "zai-glm-4.7"],
  perplexity: ["sonar-pro", "sonar", "sonar-reasoning-pro", "sonar-deep-research"],
  together: ["moonshotai/Kimi-K2.7-Code", "Qwen/Qwen3.8-Max", "Qwen/Qwen3.5-397B-A17B", "deepseek-ai/DeepSeek-V4-Pro"],
  // Both previous entries were off serverless: Fireworks pulled its Llama models
  // after 2026-05-14 (llama-v3p3-70b-instruct migrated to gpt-oss-120b) and
  // mixtral-8x7b-instruct is two years stale. `GET /v1/models` on the account
  // is the authority; these two are a starting point and the picker takes a
  // typed id.
  fireworks: ["accounts/fireworks/models/gpt-oss-120b", "accounts/fireworks/models/minimax-m3"],
  // OpenRouter doubles as the home for frontier open-weight models that have no dedicated
  // VibeCody provider key yet — notably Moonshot's Kimi K2.7 Code (2026-06-13), which cuts
  // thinking tokens ~30% vs K2.6. Add a first-class Moonshot provider (6-file dance) if usage warrants.
  // Verified against the live https://openrouter.ai/api/v1/models catalog on 2026-08-05.
  openrouter: ["moonshotai/kimi-k3", "moonshotai/kimi-k2.7-code", "moonshotai/kimi-k2.6", "z-ai/glm-5.2", "qwen/qwen3.8-max", "deepseek/deepseek-v4-pro", "minimax/minimax-m3", "x-ai/grok-4.5", "anthropic/claude-opus-5", "anthropic/claude-sonnet-5", "openai/gpt-5.6-sol", "google/gemini-3.6-flash"],
  // Azure deployment names are the operator's to choose, so this is a hint list
  // of what Foundry currently offers. gpt-4-turbo retired long ago; gpt-4o
  // retires on Foundry 2026-10-01 and gpt-4o-mini went 2026-03-31.
  azure_openai: ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5", "gpt-5.4"],
  // Bedrock ids take an `anthropic.` prefix. The previous entries were both dead:
  // claude-3-5-sonnet retired 2025-10-28, claude-3-haiku retires 2026-04-19.
  bedrock: ["anthropic.claude-opus-5", "anthropic.claude-sonnet-5", "anthropic.claude-opus-4-8", "anthropic.claude-haiku-4-5"],
  // Copilot brokers several vendors, but only the OpenAI ids are listed: its
  // slugs for the Anthropic and Google models are not the vendors' own (it
  // spells them `claude-opus-4.1`-style) and this registry ships no id it has
  // not verified. `GET /models` on api.githubcopilot.com is the authority for
  // an account, and the picker takes a typed id.
  copilot: ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5", "gpt-5.4", "gpt-5.4-mini", "gpt-5.3-codex"],
  ollama: OLLAMA_CHAT_MODELS,
  // vibecli-mistralrs talks to the local vibecli daemon (default :7878) and
  // pins the in-process mistralrs backend via X-VibeCLI-Backend. Models here
  // are repo ids that mistralrs can lazy-load from Hugging Face on first use.
  // NOTE: meta-llama/* repos are gated — first-time download requires the
  // user to accept Meta's community license on the model page and supply
  // an HF_TOKEN. Qwen and Phi repos are fully open.
  // vLLM and LM Studio serve whatever the operator loaded, so these are
  // starting points rather than claims about any machine — both pickers accept
  // a typed-in id, and each server's own /v1/models is the authority.
  vllm: [
    "meta-llama/Llama-3.1-8B-Instruct",
    "meta-llama/Llama-3.3-70B-Instruct",
    "Qwen/Qwen2.5-Coder-32B-Instruct",
    "Qwen/Qwen2.5-Coder-7B-Instruct",
    "mistralai/Mistral-7B-Instruct-v0.3",
    "microsoft/Phi-3.5-mini-instruct",
  ],
  lmstudio: [
    "qwen2.5-coder-7b-instruct",
    "qwen2.5-coder-14b-instruct",
    "meta-llama-3.1-8b-instruct",
    "mistral-7b-instruct-v0.3",
    "phi-3.5-mini-instruct",
  ],
  "vibecli-mistralrs": [
    "meta-llama/Llama-3.1-8B-Instruct",
    "meta-llama/Llama-3.2-3B-Instruct",
    "meta-llama/Llama-3.2-1B-Instruct",
    "Qwen/Qwen3.6-Coder-7B-Instruct",
    "Qwen/Qwen3.6-7B-Instruct",
    "Qwen/Qwen2.5-Coder-7B-Instruct",
    "Qwen/Qwen2.5-7B-Instruct",
    "Qwen/Qwen2.5-Coder-1.5B-Instruct",
    "Qwen/Qwen2.5-3B-Instruct",
    "Qwen/Qwen2.5-1.5B-Instruct",
    "Qwen/Qwen2.5-0.5B-Instruct",
    "microsoft/Phi-3.5-mini-instruct",
  ],
  // glm-5.2 (Z.ai, 744B, 2026-06-13) leads the Artificial Analysis open-weight intelligence index.
  // glm-4-plus / glm-4-flash dropped 2026-08-05 — the whole GLM-4.x line below 4.7 is retired
  // (Ollama Cloud reports glm-4.6 retired 2026-06-16).
  zhipu: ["glm-5.2", "glm-5.1", "glm-5", "glm-4.7", "glm-4.7-flash"],
  vercel_ai: [],
  // MiniMax-M3 (2026-06-01): 1M-token context + native multimodality in one open-weight model.
  // abab6.5s-chat dropped 2026-08-05 — superseded by the M-series.
  minimax: ["MiniMax-M3", "MiniMax-M2.7"],
  // Meta-Llama-3.3-70B-Instruct is not deprecated — SambaNova still calls it its
  // most battle-tested model — but it was the only entry, which made the picker
  // look like a one-model provider. The rest are what SambaNova Cloud serves now.
  sambanova: ["DeepSeek-V3.2", "MiniMax-M3", "MiniMax-M2.7", "gpt-oss-120b", "gemma-4-31B-it", "Meta-Llama-3.3-70B-Instruct"],
  // Poolside AI — purpose-built coding models (Poolside AI).
  poolside: ["poolside/laguna-s-2.1", "poolside/laguna-xs-2.1", "poolside/laguna-m-1"],
};

export const ALL_PROVIDERS = Object.keys(STATIC_MODELS);

/**
 * `STATIC_MODELS` in the daemon's own row shape, for a first run that has never
 * reached a daemon.
 *
 * Tier 3 and nothing else: once `/models` has answered even once, its response
 * is cached and this is never consulted again. It exists so a fresh install
 * shows a picker before the daemon finishes autostarting — not as a second
 * catalog to maintain.
 */
const STATIC_FALLBACK_ROWS: DaemonModel[] = Object.entries(STATIC_MODELS).flatMap(
  ([provider, names]) =>
    names.map((name) => ({ id: `${provider}/${name}`, name, provider }))
);

/**
 * Preferred default provider for panels that need an initial selection.
 * Points at the embedded mistralrs backend served by the local VibeCLI daemon —
 * privacy-preserving, no API key required, and the strategic direction for
 * VibeCody inference. Panels that need vision or cloud-only capabilities
 * should pick a different default explicitly.
 */
export const DEFAULT_PROVIDER = "vibecli-mistralrs";

const DEFAULT_PROVIDER_CACHE_KEY = "vibecody:default-provider";
const DAEMON_HEALTH_URL = "http://localhost:7878/health";
/** Fallback when the embedded daemon is not reachable. */
const EMBEDDED_UNREACHABLE_FALLBACK = "ollama";

/**
 * Synchronous default-provider read. Returns the value cached by the last
 * `probeAndCacheDefaultProvider()` call, or `DEFAULT_PROVIDER` if nothing is
 * cached yet. Safe to call in `useState(...)` initializers and function
 * parameter defaults.
 */
export function getDefaultProvider(): string {
  try {
    return localStorage.getItem(DEFAULT_PROVIDER_CACHE_KEY) || DEFAULT_PROVIDER;
  } catch {
    return DEFAULT_PROVIDER;
  }
}

/**
 * Pings the VibeCLI daemon's `/health` endpoint and caches the result. The
 * cached value is only read on the NEXT app launch (by `getDefaultProvider`),
 * so this never races with user selections in the current session.
 *
 * Also reads `mistralrs_recommended_default` from the response and mutates
 * `PROVIDER_DEFAULT_MODEL["vibecli-mistralrs"]` in place — the daemon swaps
 * the recommendation to an ungated Apache-2.0 model when `HF_TOKEN` is
 * absent, so the picker doesn't pre-select a model that would 401 on load.
 *
 * Call once on app mount.
 */
export async function probeAndCacheDefaultProvider(): Promise<void> {
  let resolved = EMBEDDED_UNREACHABLE_FALLBACK;
  try {
    const res = await fetch(DAEMON_HEALTH_URL, { signal: AbortSignal.timeout(800) });
    if (res.ok) {
      resolved = DEFAULT_PROVIDER;
      try {
        const body = await res.json() as { mistralrs_recommended_default?: string };
        const rec = body?.mistralrs_recommended_default;
        if (typeof rec === "string" && rec.length > 0 && rec !== PROVIDER_DEFAULT_MODEL["vibecli-mistralrs"]) {
          PROVIDER_DEFAULT_MODEL["vibecli-mistralrs"] = rec;
        }
      } catch {
        // body wasn't JSON or didn't include the field — keep the static default
      }
    }
  } catch {
    // daemon unreachable — keep fallback
  }
  try {
    localStorage.setItem(DEFAULT_PROVIDER_CACHE_KEY, resolved);
  } catch {
    // localStorage unavailable — silently skip
  }
}

/** Default model to pre-select when a provider is chosen in a dropdown. */
export const PROVIDER_DEFAULT_MODEL: Record<string, string> = {
  "claude-code": "claude-opus-5",
  claude:       "claude-opus-5",
  openai:       "gpt-5.6-sol",
  gemini:       "gemini-3.6-flash",
  groq:         "openai/gpt-oss-120b",
  grok:         "grok-4.6",
  mistral:      "mistral-large-latest",
  deepseek:     "deepseek-v4-pro",
  cerebras:     "gpt-oss-120b",
  perplexity:   "sonar-pro",
  together:     "moonshotai/Kimi-K2.7-Code",
  fireworks:    "accounts/fireworks/models/gpt-oss-120b",
  openrouter:   "anthropic/claude-opus-5",
  azure_openai: "gpt-5.6-sol",
  bedrock:      "anthropic.claude-opus-5",
  copilot:      "gpt-5.6-sol",
  ollama:       "devstral-2",
  vllm: "meta-llama/Llama-3.1-8B-Instruct",
  lmstudio: "qwen2.5-coder-7b-instruct",
  "vibecli-mistralrs": "meta-llama/Llama-3.1-8B-Instruct",
  zhipu:        "glm-5.2",
  vercel_ai:    "",
  minimax:      "MiniMax-M3",
  sambanova:    "DeepSeek-V3.2",
  poolside:     "poolside/laguna-s-2.1",
};

/**
 * Chat-engine display label → the provider id this registry (and the backend's
 * `build_temp_provider`) is keyed by.
 *
 * The toolbar dropdown is filled from `get_available_ai_providers`, which
 * returns `ChatEngine::get_provider_names()` — every provider builds its name
 * as `"<Label> (<model>)"`, so the toolbar's value is a *display name*, never a
 * provider id. Looking one up in `PROVIDER_DEFAULT_MODEL` misses, and the miss
 * is silent: it reads as "no model selected".
 *
 * Keep in sync with `DISPLAY_LABELS` in `vibe-ai/src/providers.rs`, which does
 * the same job for callers that reach the backend with a display name.
 */
const DISPLAY_LABEL_TO_PROVIDER: Record<string, string> = {
  "azureopenai": "azure_openai",
  "bedrock": "bedrock",
  "cerebras": "cerebras",
  "claude": "claude",
  "copilot": "copilot",
  "deepseek": "deepseek",
  "fireworks ai": "fireworks",
  "gemini": "gemini",
  "grok": "grok",
  "groq": "groq",
  "minimax": "minimax",
  "mistral": "mistral",
  "ollama": "ollama",
  "openai": "openai",
  "openrouter": "openrouter",
  "perplexity": "perplexity",
  "poolside": "poolside",
  "sambanova": "sambanova",
  "together ai": "together",
  "vercelai": "vercel_ai",
  "vibecli mistralrs": "vibecli-mistralrs",
  "zhipu": "zhipu",
};

/** A toolbar selection resolved to what an LLM request actually needs. */
export interface ProviderSelection {
  /** Provider id — the key `build_temp_provider` matches on. Empty if unset. */
  provider: string;
  /** Concrete model id. Empty when no model can be determined. */
  model: string;
}

/**
 * Resolve a toolbar selection into `{ provider, model }`.
 *
 * Accepts either shape a caller may hold: the chat engine's display name
 * (`"Ollama (gpt-oss:120b-cloud)"` → the model the *user picked*) or a bare
 * provider id (`"ollama"` → this registry's default model). Anything it cannot
 * resolve comes back as-is with an empty model, so a caller's own empty-state
 * check still fires rather than a wrong provider being guessed.
 */
export function parseProviderSelection(selection: string): ProviderSelection {
  const trimmed = (selection ?? "").trim();
  if (!trimmed) return { provider: "", model: "" };

  const open = trimmed.lastIndexOf(" (");
  if (open > 0 && trimmed.endsWith(")")) {
    const label = trimmed.slice(0, open);
    const model = trimmed.slice(open + 2, -1).trim();
    const provider = DISPLAY_LABEL_TO_PROVIDER[label.toLowerCase()];
    if (provider) {
      return { provider, model: model || PROVIDER_DEFAULT_MODEL[provider] || "" };
    }
  }

  return { provider: trimmed, model: PROVIDER_DEFAULT_MODEL[trimmed] ?? "" };
}

export interface ModelInfo {
  id: string;
  name: string;
  provider: string;
}

export interface ModelRegistryData {
  providers: string[];
  models: Record<string, string[]>;
  updatedAt: number;
}

// The registry's own cache read/write are gone: `useDaemonModels` owns the
// cache now, and it stores the daemon's rows rather than a matrix derived from
// them. `CACHE_KEY` stays exported because other modules still clear it.

/**
 * Hook that provides the cached provider→model matrix.
 *
 * Returns:
 * - `providers`: List of all provider names
 * - `modelsForProvider(provider)`: Models available for a given provider
 * - `loading`: Whether a refresh is in progress
 * - `refresh()`: Force refresh from backend
 * - `lastUpdated`: Timestamp of last cache update
 */
export function useModelRegistry() {
  const { models: rows, source, loading, refresh } = useDaemonModels({
    daemonUrl: DAEMON_URL,
    cacheKey: DAEMON_MODELS_CACHE_KEY,
    fallback: STATIC_FALLBACK_ROWS,
    pollMs: CACHE_TTL_MS,
  });

  // One provider→models matrix, derived from whichever tier answered. Derived
  // with `useMemo` rather than mirrored into state: state that only ever
  // restates a prop goes stale the moment the prop changes.
  const models = useMemo(() => {
    const matrix: Record<string, string[]> = {};
    for (const row of rows) {
      if (!row.name) continue;
      (matrix[row.provider] ??= []).push(row.name);
    }
    return matrix;
  }, [rows]);

  // Providers the daemon actually serves, in catalog order. `ALL_PROVIDERS` is
  // the static superset and is only right when nothing has answered — offering
  // a provider the daemon cannot build is how a picker produces a selection
  // that fails at request time.
  const providers = useMemo(() => {
    // On the fallback tier the rows *are* `STATIC_MODELS`, whose provider set
    // is `ALL_PROVIDERS` by definition — deriving it from rows instead would
    // quietly drop any provider whose static list is empty (`vercel_ai`), which
    // is a behaviour change nobody asked for.
    if (source === "fallback") return ALL_PROVIDERS;
    const seen = providersOf(rows);
    return seen.length > 0 ? seen : ALL_PROVIDERS;
  }, [rows, source]);

  const modelsForProvider = useCallback(
    (provider: string): string[] => models[provider] ?? [],
    [models]
  );

  return {
    providers,
    modelsForProvider,
    loading,
    refresh,
    /** Which tier the list came from: `live`, `cache`, or `fallback`. */
    source,
    /** Kept for callers that render a freshness stamp. */
    lastUpdated: source === "live" ? Date.now() : 0,
  };
}
