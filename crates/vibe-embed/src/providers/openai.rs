//! OpenAI `/v1/embeddings`, and anything that speaks the same shape.
//!
//! The base URL is overridable, which is what makes this backend also cover
//! Azure OpenAI, LiteLLM, vLLM's OpenAI-compatible server, and Hugging Face
//! text-embeddings-inference — all without a new provider variant.

use super::{check_batch_len, http_client, json_or_http_error, require_api_key};
use crate::{EmbedKind, Embedder, EmbeddingConfig, EmbeddingError, ModelRef, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct OpenAIEmbedder {
    config: EmbeddingConfig,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIEmbedder {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Deserialize)]
struct Item {
    /// Position in the request. The API documents that results may come back
    /// in any order, so we sort rather than trusting arrival order — a
    /// mis-ordered batch would attach every vector to the wrong chunk, and
    /// nothing downstream could detect it.
    #[serde(default)]
    index: usize,
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct Resp {
    data: Vec<Item>,
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    fn model(&self) -> &ModelRef {
        &self.config.model
    }

    fn max_batch(&self) -> usize {
        128
    }

    async fn embed_batch(&self, texts: &[String], _kind: EmbedKind) -> Result<Vec<Vec<f32>>> {
        // OpenAI's embedders are symmetric — no query/document distinction.
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
                dimensions: self.config.model.dimensions,
            })
            .send()
            .await
            .map_err(|source| EmbeddingError::Transport {
                provider: "openai",
                source,
            })?;

        let mut items: Vec<Item> = json_or_http_error::<Resp>("openai", response).await?.data;
        items.sort_by_key(|i| i.index);
        let vectors = items.into_iter().map(|i| i.embedding).collect();
        check_batch_len("openai", texts.len(), vectors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderKind;

    fn cfg(model: &str) -> EmbeddingConfig {
        EmbeddingConfig::new(ModelRef::new(ProviderKind::OpenAI, model))
            .with_api_key(Some("sk-test".into()))
    }

    #[test]
    fn missing_key_is_a_typed_error() {
        let c = EmbeddingConfig::new(ModelRef::new(ProviderKind::OpenAI, "text-embedding-3-small"));
        assert!(matches!(
            OpenAIEmbedder::new(c),
            Err(EmbeddingError::MissingApiKey("openai"))
        ));
    }

    #[test]
    fn whitespace_key_counts_as_missing() {
        let c = cfg("text-embedding-3-small").with_api_key(Some("   ".into()));
        assert!(matches!(
            OpenAIEmbedder::new(c),
            Err(EmbeddingError::MissingApiKey("openai"))
        ));
    }

    #[test]
    fn base_url_override_enables_azure_and_proxies() {
        let e = OpenAIEmbedder::new(
            cfg("text-embedding-3-small")
                .with_base_url(Some("https://my.openai.azure.com/openai/v1/".into())),
        )
        .expect("builds with a key");
        assert_eq!(e.base_url, "https://my.openai.azure.com/openai/v1");
    }

    /// Out-of-order responses must be re-sorted, or every vector lands on the
    /// wrong chunk with no visible symptom.
    #[test]
    fn results_are_sorted_by_index() {
        let json = r#"{"data":[
            {"index":2,"embedding":[3.0]},
            {"index":0,"embedding":[1.0]},
            {"index":1,"embedding":[2.0]}
        ]}"#;
        let mut items = serde_json::from_str::<Resp>(json).expect("parses").data;
        items.sort_by_key(|i| i.index);
        let flat: Vec<f32> = items.iter().map(|i| i.embedding[0]).collect();
        assert_eq!(flat, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn dimensions_are_only_sent_when_requested() {
        let none = serde_json::to_string(&Req {
            model: "text-embedding-3-large",
            input: &[],
            dimensions: None,
        })
        .expect("serialises");
        assert!(!none.contains("dimensions"));

        let some = serde_json::to_string(&Req {
            model: "text-embedding-3-large",
            input: &[],
            dimensions: Some(256),
        })
        .expect("serialises");
        assert!(some.contains("\"dimensions\":256"));
    }

    #[tokio::test]
    async fn empty_batch_makes_no_request() {
        let e = OpenAIEmbedder::new(cfg("text-embedding-3-small").with_base_url(Some(
            "http://127.0.0.1:1".into(),
        )))
        .expect("builds");
        assert!(e
            .embed_batch(&[], EmbedKind::Document)
            .await
            .expect("no request")
            .is_empty());
    }
}
