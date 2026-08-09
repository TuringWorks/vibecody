/**
 * useEmbeddingModels — the embedding-model picker's data source.
 *
 * Deliberately *not* part of `useModelRegistry`: that hook lists chat models,
 * and the two must not be interchangeable. Selecting `nomic-embed-text` as a
 * chat model or `gpt-5.5` as an embedding model both fail, and mixing them in
 * one list is how that happens.
 *
 * No model list lives here. The catalog, availability rules and defaults come
 * from `vibe-embed` via the `embedding_list_models` command, so a model added
 * to the Rust catalog appears in this picker, in the CLI, and on the daemon
 * route at the same time — with no TypeScript change.
 *
 * There is no cache. Availability depends on which API keys are present and
 * whether Ollama is running, both of which change while the app is open; a
 * two-hour cache would keep telling a user their new key had not registered.
 */
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/** Why a provider can or cannot be used right now. */
export type Availability =
  | { state: "ready" }
  | { state: "needs_api_key" }
  | { state: "not_compiled_in" };

export interface EmbeddingModel {
  provider: string;
  id: string;
  display_name: string;
  /** Native output dimension, when documented. */
  dimension: number | null;
  /** Dimensions the model can be truncated to (Matryoshka). Empty if fixed. */
  supported_dimensions: number[];
  max_input_tokens: number | null;
  document_prefix: string;
  query_prefix: string;
  /** Trained or benchmarked specifically on code retrieval. */
  recommended_for_code: boolean;
  notes: string;
}

export interface ProviderCatalog {
  provider: string;
  id: string;
  display_name: string;
  requires_api_key: boolean;
  /** True when embedding never leaves this machine. */
  is_local: boolean;
  availability: Availability;
  models: EmbeddingModel[];
  default_model: string | null;
}

export interface EmbeddingSettings {
  provider: string;
  model: string;
  dimensions?: number;
  base_url?: string;
}

/**
 * Models pulled into the local Ollama daemon.
 *
 * `unreachable` is its own state on purpose: an empty list would read as "you
 * have no embedding models installed" when the truth is "Ollama is not
 * running", and those need different advice.
 */
export type OllamaInstalled =
  | { status: "ok"; models: string[] }
  | { status: "unreachable"; error: string };

interface ListResponse {
  providers: ProviderCatalog[];
  selected: EmbeddingSettings;
  ollamaInstalled: OllamaInstalled;
}

export interface IndexHeader {
  format_version: number;
  model: { provider: string; model: string; dimensions?: number };
  dimension: number | null;
  chunk_count: number;
  file_count: number;
  built_at: number | null;
}

export interface IndexStatus {
  selected: EmbeddingSettings;
  description: string;
  /** Whether the *selected* model has an index for this workspace. */
  built: boolean;
  current: IndexHeader | null;
  /** Every index on disk, including other models — all free to switch to. */
  available: IndexHeader[];
}

type LoadState =
  | { status: "loading" }
  | { status: "ready"; data: ListResponse }
  | { status: "error"; message: string };

export function useEmbeddingModels() {
  const [state, setState] = useState<LoadState>({ status: "loading" });
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(async () => {
    setState({ status: "loading" });
    try {
      const data = await invoke<ListResponse>("embedding_list_models");
      setState({ status: "ready", data });
    } catch (e) {
      setState({ status: "error", message: String(e) });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  /**
   * Select a model. Resolves to the stored settings, or rejects with the
   * backend's message — an unsupported Matryoshka dimension, say, which is
   * far better caught here than partway through indexing a workspace.
   */
  const select = useCallback(
    async (next: EmbeddingSettings): Promise<EmbeddingSettings> => {
      setSaving(true);
      try {
        const saved = await invoke<EmbeddingSettings>("embedding_set_settings", {
          provider: next.provider,
          model: next.model,
          dimensions: next.dimensions ?? null,
          baseUrl: next.base_url ?? null,
        });
        await refresh();
        return saved;
      } finally {
        setSaving(false);
      }
    },
    [refresh],
  );

  return { state, saving, refresh, select } as const;
}

/** Index state for one workspace under the currently selected model. */
export function useEmbeddingIndexStatus(workspace: string | null) {
  const [status, setStatus] = useState<IndexStatus | null>(null);
  const [building, setBuilding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!workspace) {
      setStatus(null);
      return;
    }
    try {
      setStatus(await invoke<IndexStatus>("embedding_index_status", { workspace }));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [workspace]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const build = useCallback(async () => {
    if (!workspace) return;
    setBuilding(true);
    setError(null);
    try {
      await invoke("embedding_index_build", { workspace });
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBuilding(false);
    }
  }, [workspace, refresh]);

  return { status, building, error, refresh, build } as const;
}

/**
 * Models to offer for a provider: the shipped catalog, plus anything the user
 * has pulled into Ollama that the catalog does not already cover.
 *
 * A pulled model with no catalog entry gets `dimension: null` rather than a
 * guess — the dimension is discovered when it first embeds, and a wrong number
 * shown here would end up in an index header.
 */
export function modelsForProvider(
  catalog: ProviderCatalog,
  installed: OllamaInstalled,
): EmbeddingModel[] {
  if (catalog.id !== "ollama" || installed.status !== "ok") return catalog.models;

  const known = new Set(catalog.models.map((m) => m.id));
  const stripTag = (name: string) => name.split(":")[0];
  const extras = installed.models
    .filter((name) => !known.has(name) && !known.has(stripTag(name)))
    .map(
      (name): EmbeddingModel => ({
        provider: "ollama",
        id: name,
        display_name: name,
        dimension: null,
        supported_dimensions: [],
        max_input_tokens: null,
        document_prefix: "",
        query_prefix: "",
        recommended_for_code: false,
        notes: "Pulled locally. Dimension is measured on first use.",
      }),
    );
  return [...catalog.models, ...extras];
}
