//! Local Edit provider — uses Ollama with GGUF models optimized for
//! fill-in-middle (FIM) code completion and next-edit prediction.

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

/// System prompt optimized for code completion and editing tasks.
const CODE_EDIT_SYSTEM_PROMPT: &str = "\
You are an expert code completion engine. Your job is to predict the next edit \
or fill in missing code. Return ONLY the code that should be inserted — no \
explanations, no markdown fences, no commentary. Match the surrounding style, \
indentation, and conventions exactly.";

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<GenerateOptions>,
}

#[derive(Debug, Serialize)]
struct GenerateOptions {
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
    stop: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Debug, Serialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<GenerateOptions>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponseMsg {
    message: OllamaChatMessageOut,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaChatMessageOut {
    #[allow(dead_code)]
    role: String,
    content: String,
}

/// AI provider that talks to a local Ollama instance running a GGUF model
/// specifically tuned for code edits and fill-in-middle completions.
pub struct LocalEditProvider {
    model: String,
    api_url: String,
    client: reqwest::Client,
}

impl LocalEditProvider {
    /// Create a new local-edit provider.
    ///
    /// * `model` — Ollama model name (e.g. `"deepseek-coder:6.7b"`, `"codellama:7b-code"`)
    /// * `api_url` — Base URL of the Ollama server; defaults to `http://localhost:11434`
    pub fn new(model: String, api_url: Option<String>) -> Self {
        let api_url = api_url.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            model,
            api_url,
            client,
        }
    }

    /// Build a fill-in-middle prompt from code context.
    fn build_fim_prompt(context: &CodeContext) -> String {
        let lang = &context.language;
        let file_hint = context
            .file_path
            .as_deref()
            .map(|p| format!("// File: {}\n", p))
            .unwrap_or_default();

        format!(
            "{file_hint}\
             // Language: {lang}\n\
             <PRE>{prefix}<SUF>{suffix}<MID>",
            file_hint = file_hint,
            lang = lang,
            prefix = context.prefix,
            suffix = context.suffix,
        )
    }

    fn default_options() -> GenerateOptions {
        GenerateOptions {
            temperature: 0.2,
            num_predict: Some(256),
            stop: vec![
                "<EOT>".to_string(),
                "</s>".to_string(),
                "<|endoftext|>".to_string(),
            ],
        }
    }
}

#[async_trait]
impl AIProvider for LocalEditProvider {
    fn name(&self) -> &str {
        "local-edit"
    }

    async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.api_url))
            .send()
            .await
            .is_ok()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = Self::build_fim_prompt(context);

        let request = OllamaGenerateRequest {
            model: self.model.clone(),
            prompt,
            system: CODE_EDIT_SYSTEM_PROMPT.to_string(),
            stream: false,
            options: Some(Self::default_options()),
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.api_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to local Ollama for edit completion")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read local-edit response body")?;

        if !status.is_success() {
            anyhow::bail!("Local-edit Ollama API error ({}): {}", status, body);
        }

        let parsed: OllamaGenerateResponse = serde_json::from_str(&body)
            .context(format!("Failed to parse local-edit response: {}", body))?;

        Ok(CompletionResponse {
            text: parsed.response,
            model: self.model.clone(),
            usage: None,
        })
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = Self::build_fim_prompt(context);

        let request = OllamaGenerateRequest {
            model: self.model.clone(),
            prompt,
            system: CODE_EDIT_SYSTEM_PROMPT.to_string(),
            stream: true,
            options: Some(Self::default_options()),
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.api_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to local Ollama")?;

        let stream = response.bytes_stream();

        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let parsed: OllamaGenerateResponse = serde_json::from_slice(&chunk)?;
                Ok(parsed.response)
            })
            .boxed();

        Ok(completion_stream)
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let mut ollama_messages: Vec<OllamaChatMessage> = vec![OllamaChatMessage {
            role: "system".to_string(),
            content: CODE_EDIT_SYSTEM_PROMPT.to_string(),
        }];

        for m in messages {
            ollama_messages.push(OllamaChatMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone(),
            });
        }

        // Inject context into last user message if present.
        if let Some(ctx) = context {
            if let Some(last) = ollama_messages.last_mut() {
                if last.role == "user" {
                    last.content = format!("Context:\n{}\n\n{}", ctx, last.content);
                }
            }
        }

        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages: ollama_messages,
            stream: false,
            options: Some(Self::default_options()),
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.api_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send chat request to local Ollama")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read local-edit chat response")?;

        if !status.is_success() {
            anyhow::bail!("Local-edit chat error ({}): {}", status, body);
        }

        let parsed: OllamaChatResponseMsg = serde_json::from_str(&body)
            .context(format!("Failed to parse local-edit chat response: {}", body))?;

        Ok(parsed.message.content)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let mut ollama_messages: Vec<OllamaChatMessage> = vec![OllamaChatMessage {
            role: "system".to_string(),
            content: CODE_EDIT_SYSTEM_PROMPT.to_string(),
        }];

        for m in messages {
            ollama_messages.push(OllamaChatMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone(),
            });
        }

        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages: ollama_messages,
            stream: true,
            options: Some(Self::default_options()),
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.api_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming chat to local Ollama")?;

        let stream = response.bytes_stream();

        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let parsed: OllamaChatResponseMsg = serde_json::from_slice(&chunk)?;
                Ok(parsed.message.content)
            })
            .boxed();

        Ok(completion_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::CodeContext;

    #[test]
    fn test_local_edit_provider_name() {
        let provider = LocalEditProvider::new("deepseek-coder:6.7b".into(), None);
        assert_eq!(provider.name(), "local-edit");
    }

    #[test]
    fn test_local_edit_default_url() {
        let provider = LocalEditProvider::new("codellama:7b-code".into(), None);
        assert_eq!(provider.api_url, "http://localhost:11434");
    }

    #[test]
    fn test_local_edit_custom_url() {
        let provider = LocalEditProvider::new(
            "starcoder2:3b".into(),
            Some("http://192.168.1.100:11434".into()),
        );
        assert_eq!(provider.api_url, "http://192.168.1.100:11434");
        assert_eq!(provider.model, "starcoder2:3b");
    }

    #[test]
    fn test_build_fim_prompt_basic() {
        let ctx = CodeContext {
            language: "rust".to_string(),
            file_path: Some("src/main.rs".to_string()),
            prefix: "fn main() {\n    ".to_string(),
            suffix: "\n}".to_string(),
            additional_context: vec![],
        };
        let prompt = LocalEditProvider::build_fim_prompt(&ctx);
        assert!(prompt.contains("// File: src/main.rs"));
        assert!(prompt.contains("// Language: rust"));
        assert!(prompt.contains("<PRE>fn main()"));
        assert!(prompt.contains("<SUF>\n}"));
        assert!(prompt.contains("<MID>"));
    }

    #[test]
    fn test_build_fim_prompt_no_file_path() {
        let ctx = CodeContext {
            language: "python".to_string(),
            file_path: None,
            prefix: "def hello():".to_string(),
            suffix: "".to_string(),
            additional_context: vec![],
        };
        let prompt = LocalEditProvider::build_fim_prompt(&ctx);
        assert!(!prompt.contains("// File:"));
        assert!(prompt.contains("// Language: python"));
        assert!(prompt.contains("<PRE>def hello():"));
    }

    #[test]
    fn test_default_options() {
        let opts = LocalEditProvider::default_options();
        assert!((opts.temperature - 0.2).abs() < f32::EPSILON);
        assert_eq!(opts.num_predict, Some(256));
        assert_eq!(opts.stop.len(), 3);
    }

    #[test]
    fn default_options_stop_tokens() {
        let opts = LocalEditProvider::default_options();
        assert!(opts.stop.contains(&"<EOT>".to_string()));
        assert!(opts.stop.contains(&"</s>".to_string()));
        assert!(opts.stop.contains(&"<|endoftext|>".to_string()));
    }

    #[test]
    fn generate_options_serializes_correctly() {
        let opts = GenerateOptions {
            temperature: 0.3,
            num_predict: Some(128),
            stop: vec!["<EOT>".into()],
        };
        let json = serde_json::to_value(&opts).unwrap();
        // f32 → JSON f64 loses precision, so use approximate comparison
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.3).abs() < 1e-6, "temperature {temp} not close to 0.3");
        assert_eq!(json["num_predict"], 128);
    }

    #[test]
    fn generate_options_skips_none_num_predict() {
        let opts = GenerateOptions {
            temperature: 0.5,
            num_predict: None,
            stop: vec![],
        };
        let json = serde_json::to_value(&opts).unwrap();
        assert!(json.get("num_predict").is_none());
    }

    #[test]
    fn build_fim_prompt_with_additional_context() {
        let ctx = CodeContext {
            language: "typescript".to_string(),
            file_path: Some("src/app.ts".to_string()),
            prefix: "const x = ".to_string(),
            suffix: ";".to_string(),
            additional_context: vec!["import React from 'react';".to_string()],
        };
        let prompt = LocalEditProvider::build_fim_prompt(&ctx);
        // additional_context is not used in FIM prompt, but prefix/suffix should be
        assert!(prompt.contains("<PRE>const x = "));
        assert!(prompt.contains("<SUF>;"));
        assert!(prompt.contains("// Language: typescript"));
    }

    #[test]
    fn ollama_generate_request_serializes() {
        let req = OllamaGenerateRequest {
            model: "codellama:7b".into(),
            prompt: "test".into(),
            system: "sys".into(),
            stream: false,
            options: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "codellama:7b");
        assert_eq!(json["stream"], false);
        // options: None should be serialized as null or skipped
        assert!(json.get("options").is_none() || json["options"].is_null());
    }

    // ── generate request with options ─────────────────────────────────

    #[test]
    fn ollama_generate_request_with_options() {
        let req = OllamaGenerateRequest {
            model: "deepseek-coder:6.7b".into(),
            prompt: "complete this".into(),
            system: CODE_EDIT_SYSTEM_PROMPT.to_string(),
            stream: true,
            options: Some(LocalEditProvider::default_options()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "deepseek-coder:6.7b");
        assert_eq!(json["stream"], true);
        let opts = &json["options"];
        assert!(opts["temperature"].as_f64().unwrap() > 0.0);
        assert_eq!(opts["num_predict"], 256);
        assert_eq!(opts["stop"].as_array().unwrap().len(), 3);
    }

    // ── chat request serialization ────────────────────────────────────

    #[test]
    fn ollama_chat_request_serializes() {
        let req = OllamaChatRequest {
            model: "starcoder2:3b".into(),
            messages: vec![
                OllamaChatMessage { role: "system".into(), content: "sys".into() },
                OllamaChatMessage { role: "user".into(), content: "q".into() },
            ],
            stream: false,
            options: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "starcoder2:3b");
        assert_eq!(json["messages"].as_array().unwrap().len(), 2);
        assert_eq!(json["stream"], false);
    }

    // ── response deserialization ──────────────────────────────────────

    #[test]
    fn ollama_generate_response_deser() {
        let json = r#"{"response":"fn hello()","done":true}"#;
        let resp: OllamaGenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response, "fn hello()");
        assert!(resp.done);
    }

    #[test]
    fn ollama_generate_response_deser_not_done() {
        let json = r#"{"response":"partial","done":false}"#;
        let resp: OllamaGenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response, "partial");
        assert!(!resp.done);
    }

    #[test]
    fn ollama_chat_response_deser() {
        let json = r#"{"message":{"role":"assistant","content":"completed code"},"done":true}"#;
        let resp: OllamaChatResponseMsg = serde_json::from_str(json).unwrap();
        assert_eq!(resp.message.content, "completed code");
        assert_eq!(resp.message.role, "assistant");
    }

    // ── FIM prompt edge cases ─────────────────────────────────────────

    #[test]
    fn build_fim_prompt_empty_prefix_and_suffix() {
        let ctx = CodeContext {
            language: "go".to_string(),
            file_path: None,
            prefix: "".to_string(),
            suffix: "".to_string(),
            additional_context: vec![],
        };
        let prompt = LocalEditProvider::build_fim_prompt(&ctx);
        assert!(prompt.contains("// Language: go"));
        assert!(prompt.contains("<PRE>"));
        assert!(prompt.contains("<SUF>"));
        assert!(prompt.contains("<MID>"));
    }

    #[test]
    fn build_fim_prompt_multiline_code() {
        let ctx = CodeContext {
            language: "python".to_string(),
            file_path: Some("app.py".to_string()),
            prefix: "def greet(name):\n    ".to_string(),
            suffix: "\n\ndef main():\n    pass".to_string(),
            additional_context: vec![],
        };
        let prompt = LocalEditProvider::build_fim_prompt(&ctx);
        assert!(prompt.contains("// File: app.py"));
        assert!(prompt.contains("<PRE>def greet(name):\n    "));
        assert!(prompt.contains("<SUF>\n\ndef main():\n    pass"));
    }

    // ── system prompt ─────────────────────────────────────────────────

    #[test]
    fn code_edit_system_prompt_is_nonempty() {
        assert!(!CODE_EDIT_SYSTEM_PROMPT.is_empty());
        assert!(CODE_EDIT_SYSTEM_PROMPT.contains("code"));
    }

    // ── provider name and URL ─────────────────────────────────────────

    #[test]
    fn provider_stores_model_name() {
        let p = LocalEditProvider::new("qwen2:7b".into(), None);
        assert_eq!(p.model, "qwen2:7b");
    }
}
