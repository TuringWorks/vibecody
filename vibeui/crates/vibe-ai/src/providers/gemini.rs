//! Google Gemini provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
}

#[derive(Debug, Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Debug, Deserialize)]
struct GeminiPartResponse {
    text: String,
}

/// Gemini provider
pub struct GeminiProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    fn build_contents(&self, messages: &[Message], context: Option<String>) -> Vec<GeminiContent> {
        let mut gemini_contents = Vec::new();

        for m in messages {
            let role = match m.role {
                crate::provider::MessageRole::User => "user",
                crate::provider::MessageRole::Assistant => "model",
                crate::provider::MessageRole::System => "user", // Gemini doesn't have system role in v1beta, usually mapped to user
            };
            
            gemini_contents.push(GeminiContent {
                role: role.to_string(),
                parts: vec![GeminiPart { text: m.content.clone() }],
            });
        }

        if let Some(ctx) = context {
            if let Some(last_msg) = gemini_contents.last_mut() {
                if last_msg.role == "user" {
                    last_msg.parts[0].text = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.parts[0].text);
                }
            }
        }

        gemini_contents
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    fn name(&self) -> &str {
        "Gemini"
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        
        let messages = vec![
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];

        let response_text = self.chat(&messages, None).await?;

        Ok(CompletionResponse {
            text: response_text,
            model: self.config.model.clone(),
            usage: None,
        })
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        
        let messages = vec![
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];

        self.stream_chat(&messages).await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let api_key = self.config.api_key.as_ref().context("Gemini API key not found")?;
        let request = GeminiRequest {
            contents: self.build_contents(messages, context),
            generation_config: Some(GeminiConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.config.model
        );

        let response = self.client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }

        let gemini_response: GeminiResponse = response.json().await.context("Failed to parse Gemini response")?;
        
        if let Some(candidates) = gemini_response.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(part) = candidate.content.parts.first() {
                    return Ok(part.text.clone());
                }
            }
        }
        
        anyhow::bail!("No content in Gemini response")
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Gemini API key not found")?;
        let request = GeminiRequest {
            contents: self.build_contents(messages, None),
            generation_config: Some(GeminiConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent",
            self.config.model
        );

        let response = self.client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                // Gemini streamGenerateContent returns a JSON array of response objects.
                // Chunks may contain full or partial JSON. Try multiple parse strategies.

                // Strategy 1: parse as a single GeminiResponse object.
                if let Ok(response) = serde_json::from_slice::<GeminiResponse>(&chunk) {
                    if let Some(candidates) = response.candidates {
                        if let Some(candidate) = candidates.first() {
                            if let Some(part) = candidate.content.parts.first() {
                                return Ok(part.text.clone());
                            }
                        }
                    }
                }

                // Strategy 2: parse as a JSON array of GeminiResponse objects.
                if let Ok(responses) = serde_json::from_slice::<Vec<GeminiResponse>>(&chunk) {
                    let mut content = String::new();
                    for response in responses {
                        if let Some(candidates) = response.candidates {
                            if let Some(candidate) = candidates.first() {
                                if let Some(part) = candidate.content.parts.first() {
                                    content.push_str(&part.text);
                                }
                            }
                        }
                    }
                    if !content.is_empty() {
                        return Ok(content);
                    }
                }

                // Strategy 3: strip leading/trailing array punctuation and try
                // to extract individual JSON objects separated by commas.
                let text = String::from_utf8_lossy(&chunk);
                let trimmed = text.trim().trim_start_matches('[').trim_start_matches(',')
                    .trim_end_matches(']').trim_end_matches(',').trim();
                if !trimmed.is_empty() {
                    if let Ok(response) = serde_json::from_str::<GeminiResponse>(trimmed) {
                        if let Some(candidates) = response.candidates {
                            if let Some(candidate) = candidates.first() {
                                if let Some(part) = candidate.content.parts.first() {
                                    return Ok(part.text.clone());
                                }
                            }
                        }
                    }
                }

                // Chunk could not be parsed — return empty to keep the stream alive.
                Ok(String::new())
            })
            .boxed();

        Ok(completion_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "gemini".into(),
            api_key: Some("AIza-test".into()),
            api_url: None,
            model: "gemini-2.0-flash".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_gemini() {
        let p = GeminiProvider::new(test_config());
        assert_eq!(p.name(), "Gemini");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = GeminiProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = GeminiProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn gemini_request_serde() {
        let req = GeminiRequest {
            contents: vec![GeminiContent {
                role: "user".into(),
                parts: vec![GeminiPart { text: "hello".into() }],
            }],
            generation_config: Some(GeminiConfig { temperature: Some(0.5), max_output_tokens: Some(100) }),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["contents"][0]["role"], "user");
        assert_eq!(json["generation_config"]["temperature"], 0.5);
    }

    #[test]
    fn gemini_response_deser() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"world"}]}}]}"#;
        let resp: GeminiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.candidates.unwrap()[0].content.parts[0].text, "world");
    }

    #[test]
    fn build_contents_maps_roles_correctly() {
        use crate::provider::MessageRole;
        let p = GeminiProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "hi".into() },
            Message { role: MessageRole::Assistant, content: "hello".into() },
            Message { role: MessageRole::System, content: "sys prompt".into() },
        ];
        let contents = p.build_contents(&messages, None);
        assert_eq!(contents[0].role, "user");
        assert_eq!(contents[1].role, "model"); // Gemini uses "model" for assistant
        assert_eq!(contents[2].role, "user");  // System mapped to user
    }

    #[test]
    fn build_contents_appends_context() {
        use crate::provider::MessageRole;
        let p = GeminiProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "question".into() },
        ];
        let contents = p.build_contents(&messages, Some("context data".into()));
        assert!(contents[0].parts[0].text.contains("Context:"));
        assert!(contents[0].parts[0].text.contains("context data"));
        assert!(contents[0].parts[0].text.contains("question"));
    }

    #[test]
    fn build_contents_no_context_unchanged() {
        use crate::provider::MessageRole;
        let p = GeminiProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "raw".into() },
        ];
        let contents = p.build_contents(&messages, None);
        assert_eq!(contents[0].parts[0].text, "raw");
    }

    #[test]
    fn gemini_config_skips_none_fields() {
        let cfg = GeminiConfig { temperature: None, max_output_tokens: None };
        let json = serde_json::to_value(&cfg).unwrap();
        assert!(json.get("temperature").is_none());
        assert!(json.get("max_output_tokens").is_none());
    }

    #[test]
    fn gemini_response_empty_candidates() {
        let json = r#"{"candidates":null}"#;
        let resp: GeminiResponse = serde_json::from_str(json).unwrap();
        assert!(resp.candidates.is_none());
    }

    // ── additional serde & edge case tests ──────────────────────────────

    #[test]
    fn gemini_response_missing_candidates_field() {
        let json = r#"{}"#;
        let resp: GeminiResponse = serde_json::from_str(json).unwrap();
        assert!(resp.candidates.is_none());
    }

    #[test]
    fn gemini_response_multiple_candidates() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"a"}]}},{"content":{"parts":[{"text":"b"}]}}]}"#;
        let resp: GeminiResponse = serde_json::from_str(json).unwrap();
        let candidates = resp.candidates.unwrap();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[1].content.parts[0].text, "b");
    }

    #[test]
    fn gemini_response_multiple_parts() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"part1"},{"text":"part2"}]}}]}"#;
        let resp: GeminiResponse = serde_json::from_str(json).unwrap();
        let parts = &resp.candidates.unwrap()[0].content.parts;
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].text, "part1");
        assert_eq!(parts[1].text, "part2");
    }

    #[test]
    fn gemini_request_omits_none_generation_config() {
        let req = GeminiRequest {
            contents: vec![],
            generation_config: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("generation_config").is_none());
    }

    #[test]
    fn gemini_content_part_roundtrip() {
        let part = GeminiPart { text: "hello world".into() };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("hello world"));
    }

    #[test]
    fn build_contents_empty_messages() {
        let p = GeminiProvider::new(test_config());
        let result = p.build_contents(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_contents_empty_messages_with_context() {
        let p = GeminiProvider::new(test_config());
        let result = p.build_contents(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_contents_context_skipped_when_last_is_model() {
        use crate::provider::MessageRole;
        let p = GeminiProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "q".into() },
            Message { role: MessageRole::Assistant, content: "a".into() },
        ];
        let result = p.build_contents(&messages, Some("ignored ctx".into()));
        // Last role is "model" (mapped from Assistant), so context is NOT injected
        assert_eq!(result[1].parts[0].text, "a");
        assert_eq!(result[0].parts[0].text, "q");
    }

    #[test]
    fn build_contents_multi_turn_context_on_last_user() {
        use crate::provider::MessageRole;
        let p = GeminiProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "q1".into() },
            Message { role: MessageRole::Assistant, content: "a1".into() },
            Message { role: MessageRole::User, content: "q2".into() },
        ];
        let result = p.build_contents(&messages, Some("extra info".into()));
        // First user message unchanged
        assert_eq!(result[0].parts[0].text, "q1");
        // Last user message has context
        assert!(result[2].parts[0].text.contains("extra info"));
        assert!(result[2].parts[0].text.contains("q2"));
    }

    #[test]
    fn gemini_config_includes_set_fields() {
        let cfg = GeminiConfig { temperature: Some(0.5), max_output_tokens: Some(4096) };
        let json = serde_json::to_value(&cfg).unwrap();
        assert_eq!(json["temperature"], 0.5);
        assert_eq!(json["max_output_tokens"], 4096);
    }

    #[test]
    fn provider_preserves_model_config() {
        let mut cfg = test_config();
        cfg.model = "gemini-1.5-pro".into();
        cfg.temperature = Some(0.2);
        cfg.max_tokens = Some(1024);
        let p = GeminiProvider::new(cfg);
        assert_eq!(p.config.model, "gemini-1.5-pro");
        assert_eq!(p.config.temperature, Some(0.2));
        assert_eq!(p.config.max_tokens, Some(1024));
    }
}
