//! Voyage AI embeddings.
//!
//! `voyage-code-3` is the reason this provider exists: it is trained for code
//! retrieval, which is what most of this workspace's RAG is. Voyage takes the
//! query/document distinction natively via `input_type`, so no client-side
//! prefixing is involved.

use super::{check_batch_len, http_client, json_or_http_error, require_api_key};
use crate::{EmbedKind, Embedder, EmbeddingConfig, EmbeddingError, ModelRef, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct VoyageEmbedder {
    config: EmbeddingConfig,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl VoyageEmbedder {
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let api_key = require_api_key(&config)?;
        Ok(Self {
            base_url: config.resolved_base_url(),
            client: http_client(config.timeout_secs),
            api_key,
            config,
        })
    }
}

#[derive(Serialize)]
struct Req<'a> {
    model: &'a str,
    input: &'a [String],
    input_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dimension: Option<usize>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    index: usize,
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct Resp {
    data: Vec<Item>,
}

#[async_trait]
impl Embedder for VoyageEmbedder {
    fn model(&self) -> &ModelRef {
        &self.config.model
    }

    fn max_batch(&self) -> usize {
        128
    }

    async fn embed_batch(&self, texts: &[String], kind: EmbedKind) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&Req {
                model: &self.config.model.model,
                input: texts,
                input_type: kind.voyage_input_type(),
                output_dimension: self.config.model.dimensions,
            })
            .send()
            .await
            .map_err(|source| EmbeddingError::Transport {
                provider: "voyage",
                source,
            })?;

        let mut items = json_or_http_error::<Resp>("voyage", response).await?.data;
        items.sort_by_key(|i| i.index);
        let vectors = items.into_iter().map(|i| i.embedding).collect();
        check_batch_len("voyage", texts.len(), vectors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderKind;

    #[test]
    fn missing_key_is_a_typed_error() {
        let c = EmbeddingConfig::new(ModelRef::new(ProviderKind::Voyage, "voyage-code-3"));
        assert!(matches!(
            VoyageEmbedder::new(c),
            Err(EmbeddingError::MissingApiKey("voyage"))
        ));
    }

    /// The whole point of Voyage over a symmetric model: queries and documents
    /// must go out with different `input_type` values.
    #[test]
    fn input_type_tracks_embed_kind() {
        let doc = serde_json::to_string(&Req {
            model: "voyage-code-3",
            input: &[],
            input_type: EmbedKind::Document.voyage_input_type(),
            output_dimension: None,
        })
        .expect("serialises");
        let qry = serde_json::to_string(&Req {
            model: "voyage-code-3",
            input: &[],
            input_type: EmbedKind::Query.voyage_input_type(),
            output_dimension: None,
        })
        .expect("serialises");
        assert!(doc.contains("\"input_type\":\"document\""));
        assert!(qry.contains("\"input_type\":\"query\""));
    }

    #[test]
    fn matryoshka_dimension_is_forwarded() {
        let body = serde_json::to_string(&Req {
            model: "voyage-code-3",
            input: &[],
            input_type: "document",
            output_dimension: Some(512),
        })
        .expect("serialises");
        assert!(body.contains("\"output_dimension\":512"));
    }
}
