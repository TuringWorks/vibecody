---
layout: page
title: Embedding Models
permalink: /embeddings/
---

# Embedding Models

Semantic code search, `@codebase:`, and memory recall all run on an **embedding
model**. VibeCody supports many of them, across several providers, and keeps a
separate index per model so switching is cheap and reversible.

Works out of the box with no configuration: the default is a local Ollama model
that needs no account and no API key.

---

## Quick start

```bash
# The zero-config path — local, free, no key.
ollama pull nomic-embed-text
vibecli            # then, in the REPL:
/index             # build the semantic index
/index-status      # what is indexed, with which model
/qa how does authentication work?
```

In VibeCoder: **Settings → Embeddings**.

---

## Supported providers

| Provider | Runs | Key | Notable models |
|---|---|---|---|
| `ollama` | Locally | — | `nomic-embed-text`, `mxbai-embed-large`, `bge-m3`, `all-minilm`, `snowflake-arctic-embed2`, `embeddinggemma`, `granite-embedding`, **plus anything you `ollama pull`** |
| `openai` | Cloud | `openai` | `text-embedding-3-small`, `text-embedding-3-large`, `text-embedding-ada-002` |
| `voyage` | Cloud | `voyage` | **`voyage-code-3`** (best for code), `voyage-3-large`, `voyage-3.5`, `voyage-3.5-lite` |
| `cohere` | Cloud | `cohere` | `embed-v4.0`, `embed-english-v3.0`, `embed-multilingual-v3.0`, `embed-english-light-v3.0` |
| `gemini` | Cloud | `gemini` | `gemini-embedding-001`, `text-embedding-004` |
| `local` | In-process | — | `all-MiniLM-L6-v2` (requires a build with `--features candle`) |

API keys live in the encrypted ProfileStore under the provider name — the same
entries the chat providers use. If you have already added an OpenAI key for
chat, OpenAI embeddings work with no extra setup. **Keys are never written to
`config.toml`, to an index file, or to any plaintext file.**

### Models we don't ship metadata for still work

The catalog is a hint list, not an allow-list. `ollama pull some-custom-embed`
and select it — VibeCody will use it. What you lose is only the metadata we
cannot discover from an API: the model's dimension shows as *"measured on first
use"* until it embeds something, and no task prefix is applied.

That is deliberate. A guessed dimension written into an index header is worse
than an absent one, because absent triggers a measurement and wrong triggers
silence.

---

## Configuration

Desktop selection is stored in the ProfileStore (Settings → Embeddings). The
CLI and daemon read `config.toml`:

```toml
[index]
enabled = true
embedding_provider = "voyage"
embedding_model = "voyage-code-3"

# Optional: Matryoshka truncation, for models that support it.
embedding_dimensions = 512

# Optional: remote Ollama, Azure OpenAI, a LiteLLM proxy, a TEI server.
embedding_base_url = "http://gpu-box:11434"
```

An unrecognised provider is an **error**, not a silent fallback. Indexing a
whole workspace with a model you did not ask for is slow to notice and, on a
paid provider, not free.

### Pointing at a different endpoint

`embedding_base_url` is why there is no separate "Azure" provider. Anything
speaking the OpenAI `/v1/embeddings` shape works through `openai`:

```toml
[index]
embedding_provider = "openai"
embedding_model = "text-embedding-3-large"
embedding_base_url = "https://my-resource.openai.azure.com/openai/v1"
```

Same for a remote Ollama host, a LiteLLM proxy, or Hugging Face
text-embeddings-inference.

---

## One index per model

Vectors from two models are not comparable. Rather than invalidating your index
every time you try a different model, VibeCody keeps them side by side:

```text
<workspace>/.vibecli/index/
  index__ollama__nomic-embed-text.json        ← vectors
  index__ollama__nomic-embed-text.meta.json   ← header: model, dimension, counts
  index__voyage__voyage-code-3.json
  index__voyage__voyage-code-3.meta.json
```

Consequences worth knowing:

- **Switching is instant** if the target model already has an index.
- **Switching back never re-embeds.** The old index is still there and still valid.
- **Trying a model is cheap.** Nothing is destroyed by evaluating an alternative.
- Disk is the cost. Each index is roughly `chunks × dimensions × 4` bytes.

`/index-status` (CLI), `GET /index/status` (daemon), and the Embeddings settings
page all list what is built.

### The header, and why it matters

Every index carries a header: format version, the model that built it, and the
**observed** dimension — the length of the vectors actually stored, not a value
from a lookup table. Opening an index with a mismatched embedder fails with a
message naming both models.

Without that check, a model change does not error. `cosine_similarity` returns
`0.0` for a length mismatch and a meaningless number for a same-length
mismatch, so search would keep "working" and quietly return the wrong code.

---

## Documents and queries embed differently

Asymmetric models place stored passages and search queries in deliberately
different regions of the space. VibeCody always tells the model which side it
is embedding:

- **Natively** where the provider supports it — Voyage `input_type`, Cohere
  `input_type`, Gemini `taskType`.
- **By prefix** where it does not — `search_document: ` / `search_query: ` for
  `nomic-embed-text`, an instruction prefix for `mxbai-embed-large`, `query: `
  for `snowflake-arctic-embed2`.

Getting this wrong does not error; it just costs recall. That is why it is a
required argument throughout the codebase rather than an option with a default.

---

## Memory stores

`vibe-memory` and OpenMemory default to a built-in hash engine — free, offline,
and **lexical rather than semantic**. It reports itself as
`local/vibe-memory-hash` (and `local/openmemory-tfidf`), never as a real model,
so a hash-built index can never be mistaken for a trained-model one.

Pass any real embedder for actual semantic recall:

```rust
use vibe_embed::{EmbeddingConfig, ModelRef, ProviderKind};

let embedder = EmbeddingConfig::new(
    ModelRef::new(ProviderKind::Ollama, "nomic-embed-text"),
).build()?;
let store = GlobalMemStore::open()?.with_embedder(embedder)?;
```

Every memory row records the model that embedded it. Search compares only rows
it can compare and **logs** the ones it skipped:

```text
WARN memory search skipped entries embedded with a different model
  compared=14 skipped_other_model=132 model=ollama/nomic-embed-text
```

Previously, changing the dimension made every existing memory score `0.0` and
vanish from every search — rows still in the database, no signal at all.

---

## HTTP API

All routes require the bearer token (see [security]({{ site.baseurl }}/security/)).

### `GET /embeddings/models`

Catalog, availability, the selected model, and the embedding models actually
pulled into the local Ollama.

```json
{
  "providers": [
    {
      "id": "voyage",
      "display_name": "Voyage AI",
      "requires_api_key": true,
      "is_local": false,
      "availability": { "state": "needs_api_key" },
      "default_model": "voyage-code-3",
      "models": [ /* … */ ]
    }
  ],
  "selected": { "provider": "ollama", "model": "nomic-embed-text" },
  "ollama_installed": { "status": "ok", "models": ["nomic-embed-text:latest", "bge-m3:latest"] }
}
```

`ollama_installed.status` is `"unreachable"` when Ollama is not running — an
empty list would read as "no models installed", which needs different advice.

### `POST /embeddings/embed`

```json
{ "texts": ["fn main() {}"], "kind": "document", "provider": "voyage", "model": "voyage-code-3" }
```

`kind` is `"document"` (default) or `"query"`. `provider`/`model` are optional
and override the configured selection. The response's `dimension` is the length
actually returned, not a catalog value.

### `GET /index/status`

Which models this workspace has an index for. `built` refers to the *selected*
model; `available` lists every index on disk.

### `POST /index/build`

Builds and saves the index. Responds when the index is **written**, not when
the job starts — embedding a workspace takes real time and, on a paid provider,
real money.

### `GET /health`

Includes an `embedding` block: the model, whether it is local, whether this
workspace is indexed with it, and which other models are indexed. Never the key.

---

## Client support

| Client | Support |
|---|---|
| VibeCLI (REPL + daemon) | `/index`, `/index-status`, `/qa`, all four routes |
| VibeCoder | Settings → Embeddings, `@codebase:`, Tauri commands |
| Agent SDK | `listEmbeddingModels`, `embed`, `indexStatus`, `buildIndex` |
| VS Code extension | `listEmbeddingModels`, `indexStatus`, `buildIndex` |
| vibe-indexer | `--provider` / `--model` / `--dimensions` / `--base-url`, `GET /embeddings/models` |

Mobile and watch clients do **not** build or query code indexes — they have no
workspace — so the routes are deliberately not fanned out to them.

---

## Choosing a model

- **Indexing code, want the best results, willing to pay** — `voyage/voyage-code-3`. It is trained for code retrieval.
- **Private repo, nothing leaves the machine** — `ollama/nomic-embed-text` (default) or `ollama/bge-m3` for longer chunks.
- **Already paying OpenAI** — `openai/text-embedding-3-small` is cheap and adequate; `-3-large` is better and truncatable.
- **Low disk / very large repo** — pick a Matryoshka model and truncate: `voyage-code-3` at 256, or `text-embedding-3-large` at 256.
- **Fastest possible local** — `ollama/all-minilm` (384d, but a 256-token limit — chunk aggressively).

Cloud providers upload the contents of every indexed source file. The picker
labels each provider *local* or *cloud* before you choose, and the daemon's
startup banner repeats the warning when a cloud provider is selected.

---

## Migrating from older versions

A pre-existing `.vibecli/index.json` is migrated automatically the first time
an index is opened. It lands under the model that built it and the legacy file
is removed.

The old format serialised the whole provider — **API key included** — into
plaintext JSON. Migration reads the model name and discards the credential; the
new format stores a model reference only.
