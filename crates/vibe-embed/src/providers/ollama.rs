//! Ollama embeddings.
//!
//! Two wire formats exist in the wild:
//!   - `POST /api/embed`      — batch, `{model, input: [..]}` → `{embeddings: [[..]]}`
//!   - `POST /api/embeddings` — single, `{model, prompt}`     → `{embedding: [..]}`
//!
//! The batch endpoint is strongly preferred: indexing a workspace is thousands
//! of chunks, and one request per chunk is the difference between a minute and
//! twenty. Older daemons only have the singular route, so a 404 on `/api/embed`
//! demotes this instance to the legacy path for the rest of its life rather
//! than paying the failed round-trip on every batch.

use super::{check_batch_len, http_client, json_or_http_error};
use crate::{EmbedKind, Embedder, EmbeddingConfig, EmbeddingError, ModelRef, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct OllamaEmbedder {
    config: EmbeddingConfig,
    base_url: String,
    client: reqwest::Client,
    /// Set once we learn this daemon has no `/api/embed`.
    legacy_only: AtomicBool,
}

impl OllamaEmbedder {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            base_url: config.resolved_base_url(),
            client: http_client(config.timeout_secs),
            legacy_only: AtomicBool::new(false),
            config,
        }
    }

    /// Ollama has no native query/document field, so asymmetric models get
    /// their prefix applied client-side from the catalog.
    fn prepare(&self, texts: &[String], kind: EmbedKind) -> Vec<String> {
        match self.config.model.catalog_entry() {
            Some(m) => texts
                .iter()
                .map(|t| m.apply_prefix(t, kind).into_owned())
                .collect(),
            None => texts.to_vec(),
        }
    }

    async fn embed_batch_endpoint(&self, inputs: &[String]) -> Result<Option<Vec<Vec<f32>>>> {
        #[derive(Serialize)]
        struct Req<'a> {
            model: &'a str,
            input: &'a [String],
        }
        #[derive(Deserialize)]
        struct Resp {
            embeddings: Vec<Vec<f32>>,
        }

        let response = self
            .client
            .post(format!("{}/api/embed", self.base_url))
            .json(&Req {
                model: &self.config.model.model,
                input: inputs,
            })
            .send()
            .await
            .map_err(|source| EmbeddingError::Transport {
                provider: "ollama",
                source,
            })?;

        // 404 means this daemon predates /api/embed. Anything else (model not
        // found, OOM) is a real error the caller needs to see.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            tracing::debug!("Ollama has no /api/embed; falling back to /api/embeddings");
            self.legacy_only.store(true, Ordering::Relaxed);
            return Ok(None);
        }

        let resp: Resp = json_or_http_error("ollama", response).await?;
        Ok(Some(resp.embeddings))
    }

    async fn embed_legacy_endpoint(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        #[derive(Serialize)]
        struct Req<'a> {
            model: &'a str,
            prompt: &'a str,
        }
        #[derive(Deserialize)]
        struct Resp {
            embedding: Vec<f32>,
        }

        let mut out = Vec::with_capacity(inputs.len());
        for text in inputs {
            let response = self
                .client
                .post(format!("{}/api/embeddings", self.base_url))
                .json(&Req {
                    model: &self.config.model.model,
                    prompt: text,
                })
                .send()
                .await
                .map_err(|source| EmbeddingError::Transport {
                    provider: "ollama",
                    source,
                })?;
            let resp: Resp = json_or_http_error("ollama", response).await?;
            out.push(resp.embedding);
        }
        Ok(out)
    }
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    fn model(&self) -> &ModelRef {
        &self.config.model
    }

    fn max_batch(&self) -> usize {
        // Ollama embeds a batch in one forward pass on the local machine;
        // oversized batches mostly cost memory, not throughput.
        32
    }

    async fn embed_batch(&self, texts: &[String], kind: EmbedKind) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let inputs = self.prepare(texts, kind);

        let vectors = if self.legacy_only.load(Ordering::Relaxed) {
            self.embed_legacy_endpoint(&inputs).await?
        } else {
            match self.embed_batch_endpoint(&inputs).await? {
                Some(v) => v,
                None => self.embed_legacy_endpoint(&inputs).await?,
            }
        };

        check_batch_len("ollama", texts.len(), vectors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderKind;

    fn embedder(model: &str) -> OllamaEmbedder {
        OllamaEmbedder::new(EmbeddingConfig::new(ModelRef::new(
            ProviderKind::Ollama,
            model,
        )))
    }

    #[test]
    fn known_asymmetric_model_gets_its_prefix() {
        let e = embedder("nomic-embed-text");
        let docs = e.prepare(&["fn main() {}".to_string()], EmbedKind::Document);
        let queries = e.prepare(&["fn main() {}".to_string()], EmbedKind::Query);
        assert!(docs[0].starts_with("search_document: "));
        assert!(queries[0].starts_with("search_query: "));
    }

    /// A model we have no metadata for must be sent verbatim. Inventing a
    /// prefix for an unknown model would corrupt its embedding space.
    #[test]
    fn unknown_model_is_sent_verbatim() {
        let e = embedder("someones-custom-embed");
        let out = e.prepare(&["hello".to_string()], EmbedKind::Query);
        assert_eq!(out[0], "hello");
    }

    #[tokio::test]
    async fn empty_batch_makes_no_request() {
        // Points at a port nothing listens on: if this tried to connect it
        // would error rather than return Ok.
        let e = OllamaEmbedder::new(
            EmbeddingConfig::new(ModelRef::new(ProviderKind::Ollama, "nomic-embed-text"))
                .with_base_url(Some("http://127.0.0.1:1".into())),
        );
        assert!(e
            .embed_batch(&[], EmbedKind::Query)
            .await
            .expect("no request is made for an empty batch")
            .is_empty());
    }

    #[test]
    fn base_url_override_is_honoured() {
        let e = OllamaEmbedder::new(
            EmbeddingConfig::new(ModelRef::new(ProviderKind::Ollama, "bge-m3"))
                .with_base_url(Some("http://gpu-box:11434".into())),
        );
        assert_eq!(e.base_url, "http://gpu-box:11434");
    }

    #[test]
    fn dimension_comes_from_the_catalog_when_known() {
        assert_eq!(embedder("nomic-embed-text").dim(), Some(768));
        assert_eq!(embedder("unknown-model").dim(), None);
    }
}
