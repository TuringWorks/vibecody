/**
 * EmbeddingModelPicker — choose the model that powers semantic search / RAG.
 *
 * Three things this UI is careful about, because each one silently ruins
 * retrieval quality if the user cannot see it:
 *
 *  1. **Locality.** A cloud embedding model ships every indexed source file to
 *     a third party. That is a decision, so it is labelled before the choice,
 *     not buried in docs.
 *  2. **What is already built.** Indexes are per-model and kept side by side,
 *     so switching to a model that already has an index is instant and
 *     switching back never re-embeds. The picker says which models are
 *     already indexed rather than making every switch look equally expensive.
 *  3. **Unknown dimensions stay unknown.** A model pulled into Ollama that we
 *     ship no metadata for shows "measured on first use", not a guessed
 *     number — the real value is whatever the model returns, and that is what
 *     gets written into the index header.
 */
import { useMemo, useState } from "react";
import {
  modelsForProvider,
  useEmbeddingIndexStatus,
  useEmbeddingModels,
  type EmbeddingModel,
  type ProviderCatalog,
} from "../hooks/useEmbeddingModels";

interface Props {
  workspacePath: string | null;
}

const hint: React.CSSProperties = {
  fontSize: "var(--font-size-sm)",
  color: "var(--text-secondary)",
  margin: "4px 0 0",
  lineHeight: 1.4,
};

const row: React.CSSProperties = {
  display: "flex",
  gap: 8,
  alignItems: "center",
  flexWrap: "wrap",
};

function availabilityNote(p: ProviderCatalog): string | null {
  switch (p.availability.state) {
    case "ready":
      return null;
    case "needs_api_key":
      return `Add a ${p.display_name} API key in Settings → Providers to use these models.`;
    case "not_compiled_in":
      return "This build has no in-process embedding backend (rebuild with --features candle).";
    default: {
      // Exhaustiveness: a new availability state must be handled, not ignored.
      const never: never = p.availability;
      return never;
    }
  }
}

function describeDimension(m: EmbeddingModel, chosen?: number): string {
  if (chosen) return `${chosen} dimensions`;
  if (m.dimension === null) return "dimension measured on first use";
  return `${m.dimension} dimensions`;
}

export function EmbeddingModelPicker({ workspacePath }: Props) {
  const { state, saving, select } = useEmbeddingModels();
  const { status, building, error: indexError, build } = useEmbeddingIndexStatus(workspacePath);
  const [pendingError, setPendingError] = useState<string | null>(null);

  const selected = state.status === "ready" ? state.data.selected : null;

  const providers = state.status === "ready" ? state.data.providers : [];
  const activeProvider = useMemo(
    () => providers.find((p) => p.id === selected?.provider) ?? null,
    [providers, selected?.provider],
  );

  const models = useMemo(() => {
    if (!activeProvider || state.status !== "ready") return [];
    return modelsForProvider(activeProvider, state.data.ollamaInstalled);
  }, [activeProvider, state]);

  const activeModel = models.find((m) => m.id === selected?.model) ?? null;

  /** Models with an index already on disk — switching to these costs nothing. */
  const indexedSlugs = useMemo(
    () =>
      new Set(
        (status?.available ?? []).map((h) => `${h.model.provider}/${h.model.model}`),
      ),
    [status],
  );

  const choose = async (next: { provider?: string; model?: string; dimensions?: number }) => {
    if (!selected) return;
    setPendingError(null);
    const provider = next.provider ?? selected.provider;
    // Switching provider resets the model to that provider's default, since
    // the current model id means nothing to a different provider.
    const model =
      next.model ??
      (next.provider
        ? providers.find((p) => p.id === next.provider)?.default_model ?? ""
        : selected.model);
    try {
      await select({
        provider,
        model,
        // A dimension chosen for one model is meaningless for another.
        dimensions: next.dimensions ?? (next.model || next.provider ? undefined : selected.dimensions),
        base_url: selected.base_url,
      });
    } catch (e) {
      setPendingError(String(e));
    }
  };

  if (state.status === "loading") {
    return <p style={hint}>Loading embedding models…</p>;
  }
  if (state.status === "error") {
    return (
      <p style={{ ...hint, color: "var(--error-color)" }}>
        Could not load embedding models: {state.message}
      </p>
    );
  }

  const ollama = state.data.ollamaInstalled;

  return (
    <section>
      <h3 style={{ margin: "0 0 4px", fontSize: 16, fontWeight: 600, color: "var(--text-primary)" }}>
        Embedding model
      </h3>
      <p style={hint}>
        Powers semantic code search, <code>@codebase:</code>, and memory recall. Each model keeps its
        own index, so switching is reversible and switching back never re-embeds.
      </p>

      <div style={{ ...row, marginTop: 12 }}>
        <label className="panel-label" htmlFor="embed-provider">
          Provider
        </label>
        <select
          id="embed-provider"
          className="panel-input"
          value={selected?.provider ?? ""}
          disabled={saving}
          onChange={(e) => void choose({ provider: e.target.value })}
        >
          {providers.map((p) => (
            <option key={p.id} value={p.id} disabled={p.availability.state !== "ready"}>
              {p.display_name}
              {p.is_local ? " — local" : " — cloud"}
              {p.availability.state === "needs_api_key" ? " (needs API key)" : ""}
              {p.availability.state === "not_compiled_in" ? " (unavailable)" : ""}
            </option>
          ))}
        </select>

        <label className="panel-label" htmlFor="embed-model">
          Model
        </label>
        <select
          id="embed-model"
          className="panel-input"
          value={selected?.model ?? ""}
          disabled={saving || models.length === 0}
          onChange={(e) => void choose({ model: e.target.value })}
        >
          {models.map((m) => (
            <option key={m.id} value={m.id}>
              {m.display_name}
              {m.recommended_for_code ? " ★ code" : ""}
              {indexedSlugs.has(`${m.provider}/${m.id}`) ? " • indexed" : ""}
            </option>
          ))}
        </select>
      </div>

      {activeProvider && !activeProvider.is_local && (
        <p style={{ ...hint, color: "var(--warning-color, var(--text-secondary))" }}>
          Indexing sends the contents of your source files to {activeProvider.display_name}.
        </p>
      )}

      {activeProvider && availabilityNote(activeProvider) && (
        <p style={{ ...hint, color: "var(--error-color)" }}>{availabilityNote(activeProvider)}</p>
      )}

      {activeModel && (
        <p style={hint}>
          {describeDimension(activeModel, selected?.dimensions)}
          {activeModel.max_input_tokens ? ` · ${activeModel.max_input_tokens} token limit` : ""}
          {activeModel.notes ? ` · ${activeModel.notes}` : ""}
        </p>
      )}

      {activeModel && activeModel.supported_dimensions.length > 1 && (
        <div style={{ ...row, marginTop: 8 }}>
          <label className="panel-label" htmlFor="embed-dim">
            Output dimensions
          </label>
          <select
            id="embed-dim"
            className="panel-input"
            value={selected?.dimensions ?? activeModel.dimension ?? ""}
            disabled={saving}
            onChange={(e) => void choose({ dimensions: Number(e.target.value) })}
          >
            {activeModel.supported_dimensions.map((d) => (
              <option key={d} value={d}>
                {d}
                {d === activeModel.dimension ? " (native)" : ""}
              </option>
            ))}
          </select>
          <span style={hint}>Smaller vectors index faster and use less disk, at some recall.</span>
        </div>
      )}

      {selected?.provider === "ollama" && ollama.status === "unreachable" && (
        <p style={{ ...hint, color: "var(--error-color)" }}>
          Ollama is not reachable ({ollama.error}). Start it, then <code>ollama pull {selected.model}</code>.
        </p>
      )}

      {pendingError && <p style={{ ...hint, color: "var(--error-color)" }}>{pendingError}</p>}

      {/* ── Index state for this workspace ─────────────────────────────── */}
      {workspacePath && status && (
        <div style={{ marginTop: 16 }}>
          <div style={row}>
            <span style={{ fontSize: "var(--font-size-md)", color: "var(--text-primary)" }}>
              {status.built && status.current
                ? `Indexed: ${status.current.chunk_count} chunks from ${status.current.file_count} files`
                : "No index for this model yet"}
            </span>
            <button
              className="panel-btn panel-btn-primary"
              disabled={building}
              onClick={() => void build()}
            >
              {building ? "Indexing…" : status.built ? "Rebuild index" : "Build index"}
            </button>
          </div>

          {status.available.length > 0 && (
            <p style={hint}>
              Also on disk:{" "}
              {status.available
                .filter((h) => `${h.model.provider}/${h.model.model}` !== `${selected?.provider}/${selected?.model}`)
                .map((h) => `${h.model.provider}/${h.model.model} (${h.chunk_count} chunks)`)
                .join(", ") || "none"}
            </p>
          )}

          {indexError && <p style={{ ...hint, color: "var(--error-color)" }}>{indexError}</p>}
        </div>
      )}
    </section>
  );
}
