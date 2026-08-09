//! Cohere `/v2/embed`.
//!
//! Cohere takes the query/document distinction natively via `input_type`, and
//! returns vectors nested under the requested encoding
//! (`{"embeddings": {"float": [[...]]}}`) rather than as a flat list.

use super::{check_batch_len, http_client, json_or_http_error, require_api_key};
use crate::{EmbedKind, Embedder, EmbeddingConfig, EmbeddingError, ModelRef, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct CohereEmbedder {
    config: EmbeddingConfig,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl CohereEmbedder {
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
    texts: &'a [String],
    input_type: &'a str,
    embedding_types: [&'a str; 1],
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dimension: Option<usize>,
}

#[derive(Deserialize)]
struct Embeddings {
    #[serde(default)]
    float: Vec<Vec<f32>>,
}

#[derive(Deserialize)]
struct Resp {
    embeddings: Embeddings,
}

#[async_trait]
impl Embedder for CohereEmbedder {
    fn model(&self) -> &ModelRef {
        &self.config.model
    }

    fn max_batch(&self) -> usize {
        96
    }

    async fn embed_batch(&self, texts: &[String], kind: EmbedKind) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let response = self
            .client
            .post(format!("{}/embed", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&Req {
                model: &self.config.model.model,
                texts,
                input_type: kind.cohere_input_type(),
                embedding_types: ["float"],
                output_dimension: self.config.model.dimensions,
            })
            .send()
            .await
            .map_err(|source| EmbeddingError::Transport {
                provider: "cohere",
                source,
            })?;

        let resp: Resp = json_or_http_error("cohere", response).await?;
        // Cohere preserves input order and returns no per-item index, so
        // there is nothing to sort by — order is the only correlation we get.
        check_batch_len("cohere", texts.len(), resp.embeddings.float)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderKind;

    #[test]
    fn missing_key_is_a_typed_error() {
        let c = EmbeddingConfig::new(ModelRef::new(ProviderKind::Cohere, "embed-v4.0"));
        assert!(matches!(
            CohereEmbedder::new(c),
            Err(EmbeddingError::MissingApiKey("cohere"))
        ));
    }

    #[test]
    fn input_type_tracks_embed_kind() {
        let body = serde_json::to_string(&Req {
            model: "embed-v4.0",
            texts: &[],
            input_type: EmbedKind::Query.cohere_input_type(),
            embedding_types: ["float"],
            output_dimension: None,
        })
        .expect("serialises");
        assert!(body.contains("\"input_type\":\"search_query\""));
        assert!(body.contains("\"embedding_types\":[\"float\"]"));
    }

    /// The nested `{embeddings:{float:[...]}}` shape is easy to get wrong;
    /// pin it.
    #[test]
    fn parses_nested_float_embeddings() {
        let json = r#"{"embeddings":{"float":[[1.0,2.0],[3.0,4.0]]},"id":"x"}"#;
        let r: Resp = serde_json::from_str(json).expect("parses");
        assert_eq!(r.embeddings.float.len(), 2);
        assert_eq!(r.embeddings.float[1], vec![3.0, 4.0]);
    }

    /// If a future API version returns only `int8`, we must fail the length
    /// check rather than hand back zero vectors as if they were embeddings.
    #[test]
    fn absent_float_encoding_fails_the_length_check() {
        let r: Resp = serde_json::from_str(r#"{"embeddings":{"int8":[[1]]}}"#).expect("parses");
        assert!(check_batch_len("cohere", 1, r.embeddings.float).is_err());
    }
}
