//! Google Gemini embeddings via `models/{model}:batchEmbedContents`.
//!
//! Two Gemini-specific details this backend handles:
//!
//! 1. The API key goes in the `x-goog-api-key` header, not the `?key=` query
//!    parameter. Both work, but a key in a URL ends up in proxy logs, shell
//!    history and error messages.
//! 2. When `outputDimensionality` truncates a `gemini-embedding-001` vector,
//!    the result comes back **un-normalised**. Cosine similarity elsewhere in
//!    this workspace assumes comparable magnitudes, so truncated vectors are
//!    re-normalised here rather than leaving every downstream consumer to
//!    remember.

use super::{check_batch_len, http_client, json_or_http_error, require_api_key};
use crate::{EmbedKind, Embedder, EmbeddingConfig, EmbeddingError, ModelRef, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct GeminiEmbedder {
    config: EmbeddingConfig,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl GeminiEmbedder {
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let api_key = require_api_key(&config)?;
        Ok(Self {
            base_url: config.resolved_base_url(),
            client: http_client(config.timeout_secs),
            api_key,
            config,
        })
    }

    /// Gemini wants the fully-qualified `models/…` form in the request body.
    fn qualified_model(&self) -> String {
        let m = &self.config.model.model;
        if m.starts_with("models/") {
            m.clone()
        } else {
            format!("models/{m}")
        }
    }
}

#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct Content<'a> {
    parts: [Part<'a>; 1],
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    content: Content<'a>,
    #[serde(rename = "taskType")]
    task_type: &'a str,
    #[serde(rename = "outputDimensionality", skip_serializing_if = "Option::is_none")]
    output_dimensionality: Option<usize>,
}

#[derive(Serialize)]
struct BatchRequest<'a> {
    requests: Vec<EmbedRequest<'a>>,
}

#[derive(Deserialize)]
struct Values {
    #[serde(default)]
    values: Vec<f32>,
}

#[derive(Deserialize)]
struct Resp {
    #[serde(default)]
    embeddings: Vec<Values>,
}

/// Scale a vector to unit length. A zero vector is returned untouched — there
/// is no meaningful direction to preserve, and dividing would produce NaNs
/// that poison every later comparison.
fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 && norm.is_finite() {
        v.iter_mut().for_each(|x| *x /= norm);
    }
    v
}

#[async_trait]
impl Embedder for GeminiEmbedder {
    fn model(&self) -> &ModelRef {
        &self.config.model
    }

    fn max_batch(&self) -> usize {
        100
    }

    async fn embed_batch(&self, texts: &[String], kind: EmbedKind) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let qualified = self.qualified_model();
        let requests = texts
            .iter()
            .map(|t| EmbedRequest {
                model: &qualified,
                content: Content {
                    parts: [Part { text: t }],
                },
                task_type: kind.gemini_task_type(),
                output_dimensionality: self.config.model.dimensions,
            })
            .collect();

        let url = format!("{}/{}:batchEmbedContents", self.base_url, qualified);
        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&BatchRequest { requests })
            .send()
            .await
            .map_err(|source| EmbeddingError::Transport {
                provider: "gemini",
                source,
            })?;

        let resp: Resp = json_or_http_error("gemini", response).await?;
        // Truncated vectors arrive un-normalised; native-dimension ones are
        // already unit length, so normalising them again is a no-op.
        let needs_renorm = self.config.model.dimensions.is_some();
        let vectors = resp
            .embeddings
            .into_iter()
            .map(|e| if needs_renorm { l2_normalize(e.values) } else { e.values })
            .collect();
        check_batch_len("gemini", texts.len(), vectors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderKind;

    fn embedder(model: &str) -> GeminiEmbedder {
        GeminiEmbedder::new(
            EmbeddingConfig::new(ModelRef::new(ProviderKind::Gemini, model))
                .with_api_key(Some("key".into())),
        )
        .expect("builds with a key")
    }

    #[test]
    fn missing_key_is_a_typed_error() {
        let c = EmbeddingConfig::new(ModelRef::new(ProviderKind::Gemini, "gemini-embedding-001"));
        assert!(matches!(
            GeminiEmbedder::new(c),
            Err(EmbeddingError::MissingApiKey("gemini"))
        ));
    }

    #[test]
    fn model_is_qualified_once_only() {
        assert_eq!(
            embedder("gemini-embedding-001").qualified_model(),
            "models/gemini-embedding-001"
        );
        assert_eq!(
            embedder("models/gemini-embedding-001").qualified_model(),
            "models/gemini-embedding-001"
        );
    }

    #[test]
    fn normalize_produces_unit_length() {
        let v = l2_normalize(vec![3.0, 4.0]);
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    /// A zero vector must survive normalisation intact. Dividing by zero here
    /// would write NaNs into the index, and NaN comparisons fail silently.
    #[test]
    fn normalize_leaves_zero_vector_alone() {
        let v = l2_normalize(vec![0.0, 0.0, 0.0]);
        assert_eq!(v, vec![0.0, 0.0, 0.0]);
        assert!(v.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn task_type_tracks_embed_kind() {
        let body = serde_json::to_string(&EmbedRequest {
            model: "models/gemini-embedding-001",
            content: Content {
                parts: [Part { text: "x" }],
            },
            task_type: EmbedKind::Query.gemini_task_type(),
            output_dimensionality: Some(768),
        })
        .expect("serialises");
        assert!(body.contains("\"taskType\":\"RETRIEVAL_QUERY\""));
        assert!(body.contains("\"outputDimensionality\":768"));
    }

    #[test]
    fn parses_batch_response() {
        let r: Resp = serde_json::from_str(r#"{"embeddings":[{"values":[1.0,2.0]}]}"#)
            .expect("parses");
        assert_eq!(r.embeddings.len(), 1);
        assert_eq!(r.embeddings[0].values, vec![1.0, 2.0]);
    }
}
