//! Ollama AI provider implementation

use crate::provider::{
    record_stop_reason, AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, StopReason, StopReasonSink, ProviderConfig,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

/// Ollama Cloud / Turbo models — datacenter-hosted, addressed by the `*-cloud`
/// suffix. Unlike local models these are never reported by `/api/tags`, so the
/// daemon advertises them statically (see `serve::list_models`). A request for
/// any model whose name contains "cloud" keeps its Bearer even on a loopback
/// endpoint (see [`OllamaProvider::new`]) so the local→cloud proxy can reach
/// ollama.com. Requires an Ollama Cloud / Turbo token (`OLLAMA_API_KEY` env or
/// the encrypted ProfileStore key).
///
/// Keep in sync with `vibecoder/src/constants/ollamaModels.ts`. Every tag is
/// verified against a live `POST /api/show`: Ollama Cloud answers `410 Gone`
/// for a retired model — with its retirement date — and `404` for a tag that
/// never existed, so this list is checked rather than transcribed. The listing
/// page shows base names without their `:cloud` / `:<size>-cloud` suffix, so
/// the suffix has to be probed, not assumed.
///
/// Retired and removed 2026-08-05: `glm-4.6`, `kimi-k2:1t`, `minimax-m2` (all
/// 2026-06-16) and `deepseek-v3.1:671b` (2026-07-15).
///
/// Re-verified 2026-08-29: all seventeen existing tags still answer `200`, so
/// nothing was retired this round. Added `glm-5.3` and `glm-5.3-flash`, the two
/// entries on the listing page that were missing here. Both are cloud-only —
/// they answer `404` without the `:cloud` suffix.
///
/// Source: <https://ollama.com/search?c=cloud>
/// One model Ollama has pulled, and what it costs to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledModel {
    /// Exactly as Ollama reports it, tag included — that is what must be sent back.
    pub name: String,
    /// Stored weights in bytes. 0 when Ollama reports none (a cloud entry).
    pub size_bytes: u64,
}

pub const OLLAMA_CLOUD_MODELS: &[&str] = &[
    // Z.ai · 753B MoE, 1M context, thinking + tools (measured via /api/show).
    "glm-5.3:cloud",
    // Z.ai · 321B, 1M context, and the only cloud entry reporting `vision`.
    "glm-5.3-flash:cloud",
    "glm-5.2:cloud",
    "glm-5.1:cloud",
    "kimi-k3:cloud",
    "kimi-k2.7-code:cloud",
    "kimi-k2.6:cloud",
    "deepseek-v4-pro:cloud",
    "deepseek-v4-flash:cloud",
    "qwen3.5:cloud",
    "minimax-m3:cloud",
    "minimax-m2.7:cloud",
    "nemotron-3-ultra:cloud",
    "nemotron-3-super:cloud",
    "nemotron-3-nano:30b-cloud",
    "mistral-large-3:675b-cloud",
    "gemma4:cloud",
    "gpt-oss:120b-cloud",
    "gpt-oss:20b-cloud",
];

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
    /// Context window to serve this request with.
    ///
    /// Left unset by default — Ollama 0.32 sizes the window from the model
    /// (measured: a 14,028-token prompt was evaluated whole with no `num_ctx`
    /// at all), and pinning a large value on a small machine costs KV-cache
    /// memory the model may not have. Older servers default to 4096 and
    /// silently drop the front of a longer prompt — the system prompt, tools
    /// and all — which is what `VIBECLI_OLLAMA_NUM_CTX` is for.
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ctx: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    /// Native tool definitions. Omitted for plain chat; sent whenever the
    /// conversation came from the agent loop.
    ///
    /// Without this a model trained for native tool calling has nothing to
    /// call: it narrates intent and returns an empty turn, which the agent
    /// reads as a reasoning-only response. The response side has always
    /// transcribed native `tool_calls` back to `<tool_call>` markup — this is
    /// the missing outbound half.
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
    /// Reasoning tokens. Thinking models (gpt-oss, glm, qwen3, deepseek-r1)
    /// put their whole turn here and leave `content` empty until they answer —
    /// dropping this field makes such a turn look like an empty response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thinking: Option<String>,
    /// Native (structured) tool calls. Models emit these instead of the
    /// `<tool_call>` markup the agent's parser expects, so we transcribe them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
    /// Base64-encoded images for vision models (Qwen2-VL, GLM-4V, LLaVA, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

impl OllamaChatMessage {
    /// A request-side message: no reasoning, no tool calls, no images.
    fn outgoing(role: String, content: String) -> Self {
        Self {
            role,
            content,
            thinking: None,
            tool_calls: None,
            images: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaToolCall {
    #[serde(default)]
    function: Option<OllamaToolFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaToolFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<serde_json::Value>,
}

/// Transcribe a native Ollama tool call into the `<tool_call name="…">` markup
/// [`crate::tools::parse_tool_calls`] understands.
///
/// Two shapes occur in the wild:
///  - the plain one — `function.name` is the tool, `function.arguments` its args;
///  - the gpt-oss wrapper — `function.name` is `"tool_use"` and the real tool
///    sits in `arguments.name` / `arguments.arguments`.
fn tool_call_to_xml(call: &OllamaToolCall) -> Option<String> {
    let function = call.function.as_ref()?;
    let (name, args) = match (function.name.as_deref(), function.arguments.as_ref()) {
        (Some("tool_use"), Some(wrapped)) => (
            wrapped.get("name").and_then(|v| v.as_str())?.to_string(),
            wrapped.get("arguments").cloned(),
        ),
        (Some(name), args) => (name.to_string(), args.cloned()),
        _ => return None,
    };

    Some(crate::tools::render_tool_call(&name, args.as_ref()))
}

/// Tool definitions for this conversation, or `None` for a plain chat.
///
/// Both vocabularies are advertised here — the agent loop's and the chat
/// panel's — because a model trained for native tool calling cannot use tools
/// that exist only in prose. Which set applies is read off the system prompt's
/// marker; see [`crate::tools::tool_definitions_for`].
fn tools_for(messages: &[Message]) -> Option<Vec<serde_json::Value>> {
    crate::tools::tool_definitions_for(messages.iter().map(|m| m.content.as_str()))
}

/// Models known *not* to accept a `tools` field, keyed by `base_url\0model`.
///
/// Ollama answers `400 … does not support tools` for those, which would turn a
/// working plain chat into a hard failure now that the chat path advertises
/// tools too. One rejection is remembered so the retry happens once per model,
/// not once per turn.
static TOOLS_UNSUPPORTED: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Context window override, from `VIBECLI_OLLAMA_NUM_CTX`.
///
/// Absent by default; see [`OllamaOptions::num_ctx`] for why this is opt-in
/// rather than a pinned number.
fn configured_num_ctx() -> Option<usize> {
    std::env::var("VIBECLI_OLLAMA_NUM_CTX")
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
}

/// Did the server evaluate far fewer prompt tokens than were sent?
///
/// Pure, so the threshold is testable without a tracing subscriber. `~4 chars
/// per token` is rough and varies by tokenizer, so only a gross shortfall —
/// under half the estimate, on a prompt big enough for the estimate to mean
/// anything — is treated as evidence.
fn prompt_looks_truncated(sent_chars: usize, evaluated_tokens: usize) -> bool {
    let estimated = sent_chars / 4;
    estimated > 512 && evaluated_tokens > 0 && evaluated_tokens * 2 < estimated
}

/// Warn when the server evaluated far fewer prompt tokens than were sent —
/// the signature of a context window smaller than the prompt.
///
/// Ollama truncates from the *front*, which is where the system prompt and the
/// tool contract live, so a truncated turn looks like a model that forgot how
/// to call tools rather than like a configuration problem. There is no flag in
/// the response saying so; the token count is the only evidence.
fn warn_if_prompt_truncated(sent_chars: usize, prompt_eval_count: Option<usize>) {
    let Some(evaluated) = prompt_eval_count.filter(|n| *n > 0) else {
        return;
    };
    let estimated = sent_chars / 4;
    if prompt_looks_truncated(sent_chars, evaluated) {
        tracing::warn!(
            estimated_prompt_tokens = estimated,
            evaluated_prompt_tokens = evaluated,
            "Ollama evaluated far fewer prompt tokens than were sent — the context window is \
             likely smaller than the prompt, and Ollama drops the *front* of it (system prompt \
             and tool contract first). Set VIBECLI_OLLAMA_NUM_CTX to raise it.",
        );
    }
}

/// True when this error is Ollama refusing the `tools` field itself, rather
/// than anything about the conversation.
fn is_tools_unsupported(body: &str) -> bool {
    let body = body.to_ascii_lowercase();
    body.contains("does not support tools") || body.contains("does not support tool")
}

/// Flatten a response message into the text the rest of the stack consumes:
/// reasoning wrapped in `<thinking>` tags, then content, then any native tool
/// calls transcribed to markup.
fn message_to_text(msg: &OllamaChatMessage) -> String {
    let mut out = String::new();
    if let Some(thinking) = msg.thinking.as_deref().filter(|t| !t.is_empty()) {
        out.push_str("<thinking>");
        out.push_str(thinking);
        out.push_str("</thinking>\n");
    }
    out.push_str(&msg.content);
    for call in msg.tool_calls.iter().flatten() {
        if let Some(xml) = tool_call_to_xml(call) {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&xml);
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaChatMessage>,
    done: bool,
    /// Prompt tokens the server actually evaluated. Compared against what was
    /// sent to spot a silently truncated prompt — see
    /// [`warn_if_prompt_truncated`].
    #[serde(default)]
    prompt_eval_count: Option<usize>,
    /// Why generation stopped, on the final line only: `"stop"` for a natural
    /// end, `"length"` when the reply was cut at `num_predict`.
    ///
    /// This field was missing, so serde dropped it and a reply truncated at the
    /// 16K default cap was indistinguishable from a finished one — the chat
    /// rendered it as complete and the user typed "continue" to get the rest.
    #[serde(default)]
    done_reason: Option<String>,
}

/// Map Ollama's `done_reason` onto [`StopReason`].
///
/// An absent reason yields `None`, not `Natural`: older servers omit the field
/// entirely, and reading that silence as "finished cleanly" is the assumption
/// this whole change exists to remove.
fn ollama_stop_reason(done_reason: Option<&str>) -> Option<StopReason> {
    match done_reason? {
        "stop" => Some(StopReason::Natural),
        "length" => Some(StopReason::Length),
        other => Some(StopReason::Other(other.to_string())),
    }
}

/// Ollama AI provider
pub struct OllamaProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    base_url: String,
    display_name: String,
    /// Resolved API key: explicit config/env key, or `None` (no auth sent).
    api_key: Option<String>,
}

impl OllamaProvider {
    /// Create a new Ollama provider.
    ///
    /// API key resolution: `config.api_key` first, then `OLLAMA_API_KEY` env var.
    /// If neither is set, no auth header is sent (standard Ollama needs no auth).
    pub fn new(config: ProviderConfig) -> Self {
        let raw_url = config
            .api_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
        // Normalize: OLLAMA_HOST env var is often set without a scheme
        let base_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
            raw_url
        } else {
            format!("http://{}", raw_url)
        };

        let display_name = format!("Ollama ({})", config.model);

        // Resolve API key: explicit config → env var → None (no auth).
        //
        // BUT: the Ollama desktop app (0.24+) signed in to Ollama Cloud will
        // *proxy a request to ollama.com* whenever a valid cloud Bearer is
        // attached — even for the loopback server and a locally-installed model.
        // The cloud then 500s on the agent's multi-turn re-prompt ("Internal
        // Server Error (ref: …)"), so a purely local model appears broken. A
        // cloud Bearer is also meaningless for a loopback server (needs no auth).
        //
        // So we drop the Bearer for a *local* model on a loopback endpoint,
        // forcing local execution. Cloud models (name contains "cloud") keep the
        // key so the local→cloud proxy can still reach them.
        let base_lower = base_url.to_ascii_lowercase();
        let is_loopback = base_lower.contains("127.0.0.1")
            || base_lower.contains("//localhost")
            || base_lower.contains("//[::1]")
            || base_lower.contains("0.0.0.0");
        let model_is_cloud = config.model.contains("cloud");
        let api_key = if is_loopback && !model_is_cloud {
            None
        } else {
            config
                .api_key
                .clone()
                .or_else(|| std::env::var("OLLAMA_API_KEY").ok())
        };

        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .connect_timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url,
            display_name,
            api_key,
        }
    }

    fn build_prompt(&self, context: &CodeContext) -> String {
        format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        )
    }

    /// Build a POST request, adding Bearer auth only when an API key is configured.
    fn auth_post(&self, url: String) -> reqwest::RequestBuilder {
        let req = self.client.post(url);
        match &self.api_key {
            Some(key) => req.header("Authorization", format!("Bearer {}", key)),
            None => req,
        }
    }

    /// Build a GET request, adding Bearer auth only when an API key is configured.
    fn auth_get(&self, url: String) -> reqwest::RequestBuilder {
        let req = self.client.get(url);
        match &self.api_key {
            Some(key) => req.header("Authorization", format!("Bearer {}", key)),
            None => req,
        }
    }

    /// Turn a non-2xx Ollama response into an error a user can act on.
    ///
    /// Ollama Cloud retires hosted models on a published schedule and answers
    /// `410 Gone` for one afterwards, with the retirement date in the body.
    /// That is permanent: no retry, no token, and no amount of waiting brings
    /// it back — the only fix is to pick a different model. Passing the raw
    /// body through (`Ollama API error (410 Gone): {"error":"…"}`) buries that
    /// under JSON and reads like a transient outage, so people retry it.
    ///
    /// A retirement cannot be prevented by a static model list: it happens
    /// server-side, after the list was written, and `/api/tags` may still
    /// advertise the model locally. The error path is the only place that can
    /// know.
    fn api_error(&self, status: reqwest::StatusCode, body: &str, what: &str) -> anyhow::Error {
        let detail = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v.get("error")?.as_str().map(str::to_owned));

        if status == reqwest::StatusCode::GONE {
            let model = &self.config.model;
            return anyhow::anyhow!(
                "The Ollama model `{model}` has been retired and can no longer be used.\n\
                 Pick a different model — this is permanent, so retrying will not help.\n\
                 Ollama's message: {}",
                detail.as_deref().unwrap_or(body)
            );
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            let model = &self.config.model;
            return anyhow::anyhow!(
                "Ollama does not have a model named `{model}`.\n\
                 Pull it with `ollama pull {model}`, or pick a model that is installed.\n\
                 Ollama's message: {}",
                detail.as_deref().unwrap_or(body)
            );
        }

        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::PAYMENT_REQUIRED
        {
            return anyhow::anyhow!(
                "Ollama rejected the request for `{}` ({status}). Cloud-hosted models \
                 need an Ollama Cloud/Turbo token — add one in Settings → Providers.\n\
                 Ollama's message: {}",
                self.config.model,
                detail.as_deref().unwrap_or(body)
            );
        }

        anyhow::anyhow!(
            "Ollama {what} failed ({status}): {}",
            detail.as_deref().unwrap_or(body)
        )
    }

    /// Cache key for this endpoint+model pair.
    fn tools_cache_key(&self) -> String {
        format!("{}\0{}", self.base_url, self.config.model)
    }

    /// Tool schemas to advertise, unless this model has already told us it
    /// takes none.
    fn tools_for_request(&self, messages: &[Message]) -> Option<Vec<serde_json::Value>> {
        let defs = tools_for(messages)?;
        let known_unsupported = TOOLS_UNSUPPORTED
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&self.tools_cache_key());
        (!known_unsupported).then_some(defs)
    }

    /// Remember that this model rejects the `tools` field, so the next turn
    /// goes out without it instead of failing again.
    fn remember_tools_unsupported(&self) {
        tracing::warn!(
            model = %self.config.model,
            "Model does not accept native tool definitions — falling back to prompt-only tools",
        );
        TOOLS_UNSUPPORTED
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(self.tools_cache_key());
    }

    fn build_options(&self) -> Option<OllamaOptions> {
        // Always send options to ensure a reasonable num_predict default (16K tokens).
        // Ollama's built-in default is 2048 which truncates large code generation.
        Some(OllamaOptions {
            temperature: self.config.temperature,
            num_predict: self.config.max_tokens.or(Some(16_384)),
            num_ctx: configured_num_ctx(),
        })
    }

    /// List available Ollama models that support chat.
    ///
    /// Names only. [`list_models_detailed`](Self::list_models_detailed) is the
    /// same listing with the size each model needs in order to load.
    pub async fn list_models(base_url: Option<String>) -> Result<Vec<String>> {
        Ok(Self::list_models_detailed(base_url)
            .await?
            .into_iter()
            .map(|m| m.name)
            .collect())
    }

    /// List available Ollama models that support chat, with their sizes.
    ///
    /// Fetches all models from `/api/tags` and drops embedding models (e.g.
    /// nomic-embed-text), which are not chat models.
    ///
    /// The size is what Ollama reports for the stored weights, and it is the
    /// number that decides whether a model can run at all: asked for a 19.8 GB
    /// model on a 24 GB machine, Ollama answers `model requires 19.7 GiB but
    /// only 17.3 GiB are available` with HTTP 500 — measured. A picker that
    /// cannot see the size cannot avoid offering that model as its default.
    ///
    /// Auth is sent only when `OLLAMA_API_KEY` is set.
    pub async fn list_models_detailed(base_url: Option<String>) -> Result<Vec<InstalledModel>> {
        let base_url = base_url.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
        let api_key = std::env::var("OLLAMA_API_KEY").ok();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let mut req = client.get(format!("{}/api/tags", base_url));
        if let Some(ref key) = api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let response = req.send().await.context("Failed to connect to Ollama")?;

        #[derive(Deserialize)]
        struct ModelListResponse {
            models: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
            /// Bytes of stored weights. A cloud-hosted entry stores nothing
            /// locally and reports none, which stays 0 rather than a guess.
            #[serde(default)]
            size: u64,
            #[serde(default)]
            details: ModelDetails,
        }

        #[derive(Deserialize, Default)]
        struct ModelDetails {
            #[serde(default)]
            family: String,
        }

        let list: ModelListResponse = response
            .json()
            .await
            .context("Failed to parse model list")?;

        // Embedding models are not chat models. The classifier lives in
        // `vibe_embed::catalog` so this filter and the embedding-model picker
        // can never disagree about which is which — they read the same rule.
        let chat_models: Vec<InstalledModel> = list
            .models
            .into_iter()
            .filter(|m| {
                !vibe_embed::catalog::looks_like_embedding_model(&m.name, &m.details.family)
            })
            .map(|m| InstalledModel {
                name: m.name,
                size_bytes: m.size,
            })
            .collect();

        Ok(chat_models)
    }

    /// List the locally-pulled models that look like **embedding** models.
    ///
    /// The complement of the filter in [`list_models`](Self::list_models),
    /// and the reason it exists: a user who has run `ollama pull bge-m3`
    /// should see `bge-m3` in the embedding-model picker. Before this, every
    /// embedding model was stripped from the only listing path, so the
    /// registry could not offer one even though it was installed.
    ///
    /// Returned names are exactly as Ollama reports them, tag included
    /// (`nomic-embed-text:latest`) — that is what must be sent back on
    /// `/api/embed`.
    pub async fn list_embedding_models(base_url: Option<String>) -> Result<Vec<String>> {
        let base_url = base_url.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
        let api_key = std::env::var("OLLAMA_API_KEY").ok();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let mut req = client.get(format!("{}/api/tags", base_url));
        if let Some(ref key) = api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        let response = req.send().await.context("Failed to connect to Ollama")?;

        #[derive(Deserialize)]
        struct ModelListResponse {
            models: Vec<ModelInfo>,
        }
        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
            #[serde(default)]
            details: ModelDetails,
        }
        #[derive(Deserialize, Default)]
        struct ModelDetails {
            #[serde(default)]
            family: String,
        }

        let list: ModelListResponse = response
            .json()
            .await
            .context("Failed to parse model list")?;

        Ok(list
            .models
            .into_iter()
            .filter(|m| vibe_embed::catalog::looks_like_embedding_model(&m.name, &m.details.family))
            .map(|m| m.name)
            .collect())
    }
}


impl OllamaProvider {
    /// The streaming body shared by [`AIProvider::stream_chat`] and
    /// [`AIProvider::stream_chat_reporting`].
    ///
    /// `stop` is `None` for the plain path, which keeps that caller's
    /// behaviour exactly as it was; the reporting path passes a sink and gets
    /// `done_reason` back out of the final line.
    async fn stream_chat_inner(
        &self,
        messages: &[Message],
        stop: Option<StopReasonSink>,
    ) -> Result<CompletionStream> {
        let ollama_messages: Vec<OllamaChatMessage> = messages
            .iter()
            .map(|m| OllamaChatMessage::outgoing(m.role.as_str().to_string(), m.content.clone()))
            .collect();

        let request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: ollama_messages,
            stream: true,
            options: self.build_options(),
            tools: self.tools_for_request(messages),
        };

        let response = self
            .auth_post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming chat request to Ollama")?;

        let response = if !response.status().is_success() && request.tools.is_some() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            if !is_tools_unsupported(&error_text) {
                return Err(self.api_error(status, &error_text, "streaming chat"));
            }
            // Same 400-means-no-tools fallback as the non-streaming path.
            self.remember_tools_unsupported();
            let retry = OllamaChatRequest {
                tools: None,
                ..request
            };
            let response = self
                .auth_post(format!("{}/api/chat", self.base_url))
                .json(&retry)
                .send()
                .await
                .context("Failed to send streaming chat request to Ollama")?;
            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(self.api_error(status, &error_text, "streaming chat"));
            }
            response
        } else if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(self.api_error(status, &error_text, "streaming chat"));
        } else {
            response
        };

        let stream = response.bytes_stream();

        // Buffer for bytes that don't yet form complete UTF-8 or complete
        // JSON lines.  Cloud/remote Ollama models can split chunks at
        // arbitrary byte boundaries (mid-character or mid-JSON-object).
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        // Reasoning streams token-by-token, so the `<thinking>` wrapper spans
        // many chunks: opened on the first reasoning token, closed when prose,
        // a tool call, or the end of the stream arrives.
        let thinking_open = std::sync::Arc::new(std::sync::Mutex::new(false));

        // Captured before the closure: the closure is `move` and outlives the
        // borrow of `messages`.
        let sent_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        let stop = stop.clone();
        let completion_stream = stream
            .map(move |chunk| -> Result<String, anyhow::Error> {
                let chunk = chunk?;
                let mut guard = buf.lock().unwrap_or_else(|e| e.into_inner());
                guard.extend_from_slice(&chunk);

                // Try to decode as much valid UTF-8 as possible from the
                // front of the buffer.  If the buffer ends with a partial
                // multi-byte sequence, leave those bytes for the next chunk.
                let valid_up_to = match std::str::from_utf8(&guard) {
                    Ok(_) => guard.len(),
                    Err(e) => e.valid_up_to(),
                };
                if valid_up_to == 0 {
                    return Ok(String::new());
                }

                let text = String::from_utf8_lossy(&guard[..valid_up_to]).into_owned();
                let remainder = guard[valid_up_to..].to_vec();
                *guard = remainder;

                let mut result = String::new();
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<OllamaChatResponse>(line) {
                        Ok(response) => {
                            let done = response.done;
                            // The final line carries `prompt_eval_count`, and
                            // it is the only evidence that the server dropped
                            // the front of the prompt. This check existed for
                            // the non-streaming `chat()` only — which nothing
                            // in the product calls: the chat panel and the
                            // agent both stream. So the one diagnostic for
                            // "your context window is smaller than your
                            // prompt" never fired where it was needed.
                            if done {
                                warn_if_prompt_truncated(sent_chars, response.prompt_eval_count);
                                // The final line is the only one carrying
                                // `done_reason`, and it is the only evidence
                                // that the reply was cut at `num_predict`
                                // rather than finished.
                                if let (Some(sink), Some(reason)) = (
                                    stop.as_ref(),
                                    ollama_stop_reason(response.done_reason.as_deref()),
                                ) {
                                    record_stop_reason(sink, reason);
                                }
                            }
                            if let Some(msg) = response.message {
                                let mut open =
                                    thinking_open.lock().unwrap_or_else(|e| e.into_inner());
                                if let Some(t) = msg.thinking.as_deref().filter(|t| !t.is_empty()) {
                                    if !*open {
                                        result.push_str("<thinking>");
                                        *open = true;
                                    }
                                    result.push_str(t);
                                }
                                // Anything that isn't reasoning ends the block.
                                let ends_thinking =
                                    !msg.content.is_empty() || msg.tool_calls.is_some() || done;
                                if *open && ends_thinking {
                                    result.push_str("</thinking>\n");
                                    *open = false;
                                }
                                drop(open);

                                result.push_str(&msg.content);
                                for call in msg.tool_calls.iter().flatten() {
                                    if let Some(xml) = tool_call_to_xml(call) {
                                        if !result.is_empty() && !result.ends_with('\n') {
                                            result.push('\n');
                                        }
                                        result.push_str(&xml);
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Partial JSON line — push it back into the buffer
                            // so the next chunk can complete it.
                            let mut guard2 = buf.lock().unwrap_or_else(|e| e.into_inner());
                            let mut leftover = line.as_bytes().to_vec();
                            leftover.push(b'\n');
                            leftover.extend_from_slice(&guard2);
                            *guard2 = leftover;
                        }
                    }
                }
                Ok(result)
            })
            .boxed();

        Ok(completion_stream)
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &str {
        &self.display_name
    }

    async fn is_available(&self) -> bool {
        // Try to ping the Ollama API
        self.auth_get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = self.build_prompt(context);

        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt,
            stream: false,
            options: self.build_options(),
        };

        let response = self
            .auth_post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(CompletionResponse {
            text: ollama_response.response,
            model: self.config.model.clone(),
            usage: None,
        })
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = self.build_prompt(context);

        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt,
            stream: true,
            options: self.build_options(),
        };

        let response = self
            .auth_post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        let stream = response.bytes_stream();

        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let response: OllamaResponse = serde_json::from_slice(&chunk)?;
                Ok(response.response)
            })
            .boxed();

        Ok(completion_stream)
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let mut ollama_messages: Vec<OllamaChatMessage> = messages
            .iter()
            .map(|m| OllamaChatMessage::outgoing(m.role.as_str().to_string(), m.content.clone()))
            .collect();

        // Inject context into the last user message if available
        if let Some(ctx) = context {
            if let Some(last_msg) = ollama_messages.last_mut() {
                if last_msg.role == "user" {
                    last_msg.content = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.content);
                }
            }
        }

        let request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: ollama_messages,
            stream: false,
            options: self.build_options(),
            tools: self.tools_for_request(messages),
        };

        let response = self
            .auth_post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send chat request to Ollama")?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        // A model that takes no `tools` field says so with a 400. Drop the
        // field, remember the model, and send the same turn again — the
        // alternative is a chat panel that fails outright for such a model
        // now that the chat path advertises tools too.
        let body_text = if !status.is_success()
            && request.tools.is_some()
            && is_tools_unsupported(&body_text)
        {
            self.remember_tools_unsupported();
            let retry = OllamaChatRequest {
                tools: None,
                ..request
            };
            let response = self
                .auth_post(format!("{}/api/chat", self.base_url))
                .json(&retry)
                .send()
                .await
                .context("Failed to send chat request to Ollama")?;
            let status = response.status();
            let body_text = response
                .text()
                .await
                .context("Failed to read response body")?;
            if !status.is_success() {
                return Err(self.api_error(status, &body_text, "chat"));
            }
            body_text
        } else if !status.is_success() {
            return Err(self.api_error(status, &body_text, "chat"));
        } else {
            body_text
        };

        let ollama_response: OllamaChatResponse = serde_json::from_str(&body_text).context(
            format!("Failed to parse Ollama chat response: {}", body_text),
        )?;
        warn_if_prompt_truncated(
            messages.iter().map(|m| m.content.len()).sum(),
            ollama_response.prompt_eval_count,
        );

        Ok(ollama_response
            .message
            .as_ref()
            .map(message_to_text)
            .unwrap_or_default())
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        self.stream_chat_inner(messages, None).await
    }

    async fn stream_chat_reporting(
        &self,
        messages: &[Message],
        stop: StopReasonSink,
    ) -> Result<CompletionStream> {
        self.stream_chat_inner(messages, Some(stop)).await
    }

    /// Ask the server what this model's window actually is.
    ///
    /// `/api/show` reports it under `model_info` with an architecture prefix
    /// (`qwen3.context_length`, `gemma3.context_length`), so it is the exact
    /// number for the exact blob that is loaded — no table can be as accurate,
    /// because the same tag can be re-pulled with a different quantisation.
    ///
    /// This is the provider that most needs it. Local windows are small, and
    /// Ollama's response to an oversized prompt is to drop the *front* of it —
    /// the system prompt and the tool contract — without saying so.
    ///
    /// An explicit `VIBECLI_OLLAMA_NUM_CTX` wins: it is what the server will
    /// actually be told to use, so it is the real window for the request even
    /// when the blob could hold more.
    async fn context_window(&self) -> Option<usize> {
        if let Some(configured) = configured_num_ctx() {
            return Some(configured);
        }
        crate::context_window::cached("Ollama", &self.config.model, || async {
            let body = self
                .auth_post(format!("{}/api/show", self.base_url))
                .json(&serde_json::json!({ "model": self.config.model }))
                .send()
                .await
                .ok()?
                .json::<serde_json::Value>()
                .await
                .ok()?;
            crate::context_window::from_ollama_show(&body)
        })
        .await
    }

    fn advertises_native_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        // Ollama vision support depends on the model. Common vision models:
        // qwen2-vl, qwen2.5-vl, glm-4v, llava, bakllava, moondream, deepseek-vl
        // We return true and let the model handle it — Ollama will error if the
        // model doesn't support images, which is better than silently dropping them.
        true
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[crate::provider::ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        let mut ollama_messages: Vec<OllamaChatMessage> = messages
            .iter()
            .map(|m| OllamaChatMessage::outgoing(m.role.as_str().to_string(), m.content.clone()))
            .collect();

        // Inject context into the last user message if available
        if let Some(ctx) = context {
            if let Some(last_msg) = ollama_messages.last_mut() {
                if last_msg.role == "user" {
                    last_msg.content = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.content);
                }
            }
        }

        // Attach images to the last user message (Ollama expects base64 strings)
        if !images.is_empty() {
            if let Some(last_user) = ollama_messages.iter_mut().rev().find(|m| m.role == "user") {
                last_user.images = Some(images.iter().map(|img| img.base64.clone()).collect());
            }
        }

        let request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: ollama_messages,
            stream: false,
            options: self.build_options(),
            tools: self.tools_for_request(messages),
        };

        let response = self
            .auth_post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send vision request to Ollama")?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            return Err(self.api_error(status, &body_text, "vision"));
        }

        let ollama_response: OllamaChatResponse = serde_json::from_str(&body_text).context(
            format!("Failed to parse Ollama vision response: {}", body_text),
        )?;

        Ok(ollama_response
            .message
            .as_ref()
            .map(message_to_text)
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(model: &str) -> OllamaProvider {
        OllamaProvider::new(ProviderConfig::new("ollama".to_string(), model.to_string()))
    }

    /// A retired cloud model is permanent. The message must say so, name the
    /// model, and keep Ollama's own text (which carries the retirement date) —
    /// the raw 410 body read like a transient outage, so people retried it.
    #[test]
    fn retired_model_error_is_actionable() {
        let p = provider("qwen3-coder:480b");
        let body = r#"{"error":"qwen3-coder:480b was retired at 2026-07-15 00:00:00 -0700 PDT (ref: fc5e3ca3)"}"#;
        let msg = p
            .api_error(reqwest::StatusCode::GONE, body, "streaming chat")
            .to_string();

        assert!(msg.contains("qwen3-coder:480b"), "names the model: {msg}");
        assert!(msg.contains("retired"), "says retired: {msg}");
        assert!(
            msg.contains("retrying will not help"),
            "tells the user not to retry: {msg}"
        );
        assert!(msg.contains("2026-07-15"), "keeps Ollama's detail: {msg}");
        // The raw JSON envelope must not survive into the user-facing text.
        assert!(!msg.contains("{\"error\""), "no raw JSON: {msg}");
    }

    /// 404 is the opposite advice — the model is fine, it just isn't pulled.
    #[test]
    fn missing_model_error_suggests_pulling_it() {
        let p = provider("llama4");
        let msg = p
            .api_error(
                reqwest::StatusCode::NOT_FOUND,
                r#"{"error":"model 'llama4' not found"}"#,
                "chat",
            )
            .to_string();
        assert!(msg.contains("ollama pull llama4"), "{msg}");
        assert!(!msg.contains("retired"), "must not blame retirement: {msg}");
    }

    /// A cloud model without a token is otherwise indistinguishable from a
    /// broken endpoint.
    #[test]
    fn unauthorized_error_points_at_the_cloud_token() {
        let p = provider("kimi-k3:cloud");
        for status in [
            reqwest::StatusCode::UNAUTHORIZED,
            reqwest::StatusCode::PAYMENT_REQUIRED,
        ] {
            let msg = p
                .api_error(status, r#"{"error":"unauthorized"}"#, "chat")
                .to_string();
            assert!(msg.contains("Settings → Providers"), "{status}: {msg}");
        }
    }

    /// Anything unrecognised still surfaces the status and Ollama's message
    /// rather than being swallowed into a generic string.
    #[test]
    fn unknown_status_keeps_the_detail() {
        let p = provider("m");
        let msg = p
            .api_error(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"error":"out of memory"}"#,
                "chat",
            )
            .to_string();
        assert!(msg.contains("out of memory"), "{msg}");
        assert!(msg.contains("500"), "{msg}");
    }

    /// A non-JSON body (an HTML error page from a proxy) must pass through, not
    /// vanish because `error` could not be parsed out of it.
    #[test]
    fn non_json_body_is_preserved() {
        let p = provider("m");
        let msg = p
            .api_error(
                reqwest::StatusCode::BAD_GATEWAY,
                "<html>502 nope</html>",
                "chat",
            )
            .to_string();
        assert!(msg.contains("502 nope"), "{msg}");
    }

    #[test]
    fn test_build_prompt() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string());
        let provider = OllamaProvider::new(config);

        let context = CodeContext {
            language: "rust".to_string(),
            file_path: None,
            prefix: "fn main() {\n    ".to_string(),
            suffix: "\n}".to_string(),
            additional_context: vec![],
        };

        let prompt = provider.build_prompt(&context);
        assert!(prompt.contains("rust"));
        assert!(prompt.contains("fn main()"));
    }

    // ── build_options ────────────────────────────────────────────────────

    #[test]
    fn build_options_defaults_when_no_config() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string());
        let provider = OllamaProvider::new(config);
        let opts = provider.build_options().unwrap();
        assert!(opts.temperature.is_none());
        assert_eq!(opts.num_predict, Some(16_384));
    }

    #[test]
    fn build_options_some_when_temperature_set() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_temperature(0.5);
        let provider = OllamaProvider::new(config);
        let opts = provider.build_options();
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert!((opts.temperature.unwrap() - 0.5).abs() < 0.001);
        assert_eq!(opts.num_predict, Some(16_384));
    }

    #[test]
    fn build_options_some_when_max_tokens_set() {
        let config =
            ProviderConfig::new("ollama".to_string(), "codellama".to_string()).with_max_tokens(256);
        let provider = OllamaProvider::new(config);
        let opts = provider.build_options();
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert!(opts.temperature.is_none());
        assert_eq!(opts.num_predict, Some(256));
    }

    #[test]
    fn build_options_both_set() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_temperature(0.9)
            .with_max_tokens(1024);
        let provider = OllamaProvider::new(config);
        let opts = provider.build_options().unwrap();
        assert!((opts.temperature.unwrap() - 0.9).abs() < 0.001);
        assert_eq!(opts.num_predict, Some(1024));
    }

    // ── URL normalization in new() ───────────────────────────────────────

    #[test]
    fn url_default_when_none() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "http://127.0.0.1:11434");
    }

    #[test]
    fn url_preserves_http_prefix() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_api_url("http://myhost:11434".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "http://myhost:11434");
    }

    #[test]
    fn url_preserves_https_prefix() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_api_url("https://ollama.example.com".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "https://ollama.example.com");
    }

    #[test]
    fn url_prepends_http_when_no_scheme() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_api_url("192.168.1.100:11434".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "http://192.168.1.100:11434");
    }

    #[test]
    fn url_prepends_http_for_hostname_only() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string())
            .with_api_url("ollama-server".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.base_url, "http://ollama-server");
    }

    // ── display name ────────────────────────────────────────────────────

    #[test]
    fn display_name_includes_model() {
        let config = ProviderConfig::new("ollama".to_string(), "llama3.2:8b".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.name(), "Ollama (llama3.2:8b)");
    }

    // ── request serde with skip_serializing_if ──────────────────────────

    #[test]
    fn ollama_request_omits_none_options() {
        let req = OllamaRequest {
            model: "codellama".to_string(),
            prompt: "test".to_string(),
            stream: false,
            options: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            !json.contains("options"),
            "options should be omitted when None"
        );
        assert!(json.contains("\"model\""));
        assert!(json.contains("\"prompt\""));
        assert!(json.contains("\"stream\""));
    }

    #[test]
    fn ollama_request_includes_options_when_some() {
        let req = OllamaRequest {
            model: "codellama".to_string(),
            prompt: "test".to_string(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(0.7),
                num_predict: Some(100),
                num_ctx: None,
            }),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"options\""));
        assert!(json.contains("\"temperature\""));
        assert!(json.contains("\"num_predict\""));
    }

    #[test]
    fn ollama_options_omits_none_fields() {
        let opts = OllamaOptions {
            temperature: None,
            num_predict: Some(512),
            num_ctx: None,
        };
        let json = serde_json::to_string(&opts).unwrap();
        assert!(
            !json.contains("temperature"),
            "temperature should be omitted when None"
        );
        assert!(json.contains("\"num_predict\":512"));
    }

    #[test]
    fn ollama_options_omits_both_none_fields() {
        let opts = OllamaOptions {
            temperature: None,
            num_predict: None,
            num_ctx: None,
        };
        let json = serde_json::to_string(&opts).unwrap();
        // Should be an empty object
        assert_eq!(json, "{}");
    }

    /// The agent path must advertise tools — this is the bug that made
    /// native-tool-calling models (minimax-m3, and friends) narrate "let me
    /// check the workspace" and then return an empty turn.
    #[test]
    fn agent_conversation_advertises_tools() {
        let msgs = vec![Message {
            role: crate::MessageRole::System,
            content: crate::tools::TOOL_SYSTEM_PROMPT.to_string(),
        }];
        let tools = tools_for(&msgs).expect("agent conversations must carry tools");
        assert_eq!(tools.len(), crate::tools::AVAILABLE_TOOL_NAMES.len());
        assert!(tools
            .iter()
            .any(|t| t["function"]["name"] == "list_directory"));
    }

    /// A plain chat panel never asked for tools; handing them over would get
    /// `<tool_call>` markup rendered as literal text.
    #[test]
    fn plain_chat_sends_no_tools() {
        let msgs = vec![Message {
            role: crate::MessageRole::User,
            content: "what is the capital of France?".to_string(),
        }];
        assert!(tools_for(&msgs).is_none());
    }

    #[test]
    fn tools_are_omitted_from_the_wire_when_absent() {
        let req = OllamaChatRequest {
            model: "llama3".to_string(),
            messages: vec![OllamaChatMessage::outgoing(
                "user".to_string(),
                "hi".to_string(),
            )],
            stream: false,
            options: None,
            tools: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("tools"), "None tools must not hit the wire");
    }

    #[test]
    fn ollama_chat_request_omits_none_options() {
        let req = OllamaChatRequest {
            model: "llama3".to_string(),
            messages: vec![OllamaChatMessage::outgoing(
                "user".to_string(),
                "hello".to_string(),
            )],
            stream: false,
            options: None,
            tools: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("options"));
    }

    #[test]
    fn ollama_response_deser() {
        let json = r#"{"response":"Hello world","done":true}"#;
        let resp: OllamaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response, "Hello world");
        assert!(resp.done);
    }

    #[test]
    fn ollama_chat_response_deser() {
        let json = r#"{"message":{"role":"assistant","content":"reply"},"done":true}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        let msg = resp.message.unwrap();
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "reply");
        assert!(resp.done);
    }

    #[test]
    fn ollama_chat_response_done_without_message() {
        let json = r#"{"done":true,"total_duration":123456}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.message.is_none());
        assert!(resp.done);
    }

    // ── Reasoning + native tool calls ──────────────────────────────────
    //
    // Thinking models (gpt-oss, glm, qwen3, deepseek-r1) answer with an empty
    // `content`, putting the turn in `thinking` and/or `tool_calls`. Reading
    // only `content` made every such turn look like an empty response, which
    // stalled the agent loop before its first step.

    #[test]
    fn thinking_field_is_captured() {
        let json =
            r#"{"message":{"role":"assistant","content":"","thinking":"pondering"},"done":false}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        let msg = resp.message.unwrap();
        assert_eq!(msg.thinking.as_deref(), Some("pondering"));
        assert_eq!(message_to_text(&msg), "<thinking>pondering</thinking>\n");
    }

    #[test]
    fn native_tool_call_becomes_tool_call_markup() {
        let json = r#"{"message":{"role":"assistant","content":"","tool_calls":[
            {"function":{"name":"list_directory","arguments":{"path":"src"}}}]},"done":true}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        let text = message_to_text(&resp.message.unwrap());
        assert_eq!(
            text,
            "<tool_call name=\"list_directory\"><path>src</path></tool_call>"
        );
        // …and the agent's parser accepts what we produced.
        let calls = crate::tools::parse_tool_calls(&text);
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn gpt_oss_tool_use_wrapper_is_unwrapped() {
        // gpt-oss nests the real call: function.name == "tool_use".
        let json = r#"{"message":{"role":"assistant","content":"","tool_calls":[
            {"id":"call_1","function":{"index":0,"name":"tool_use",
             "arguments":{"name":"list_directory","arguments":{"path":"."}}}}]},"done":true}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        let text = message_to_text(&resp.message.unwrap());
        assert_eq!(
            text,
            "<tool_call name=\"list_directory\"><path>.</path></tool_call>"
        );
        assert_eq!(crate::tools::parse_tool_calls(&text).len(), 1);
    }

    #[test]
    fn non_string_tool_arguments_are_rendered_as_json() {
        let json = r#"{"message":{"role":"assistant","content":"","tool_calls":[
            {"function":{"name":"spawn_agent","arguments":{"task":"go","max_steps":3}}}]},"done":true}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        let text = message_to_text(&resp.message.unwrap());
        assert!(text.contains("<max_steps>3</max_steps>"), "got {text}");
        assert!(text.contains("<task>go</task>"), "got {text}");
    }

    #[test]
    fn thinking_content_and_tool_call_are_all_kept() {
        let json = r#"{"message":{"role":"assistant","content":"On it.","thinking":"hmm",
            "tool_calls":[{"function":{"name":"list_directory","arguments":{"path":"src"}}}]},"done":true}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        let text = message_to_text(&resp.message.unwrap());
        assert_eq!(
            text,
            "<thinking>hmm</thinking>\nOn it.\n<tool_call name=\"list_directory\"><path>src</path></tool_call>"
        );
    }

    #[test]
    fn outgoing_messages_carry_no_reasoning_fields() {
        let msg = OllamaChatMessage::outgoing("user".into(), "hi".into());
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"role":"user","content":"hi"}"#);
    }

    #[test]
    fn tool_call_without_function_is_skipped() {
        let call: OllamaToolCall = serde_json::from_str("{}").unwrap();
        assert!(tool_call_to_xml(&call).is_none());
    }

    // ── API key resolution ─────────────────────────────────────────────

    // The loopback override (don't ship a Bearer to a loopback Ollama for a
    // local model — desktop Ollama 0.24+ would proxy the call to the cloud)
    // kicks in for `127.0.0.1` / `localhost` / `[::1]` / `0.0.0.0`. The two
    // precedence tests below use a remote `api_url` so they exercise the
    // explicit-key → env-var fallback chain without that override interfering.

    #[test]
    fn api_key_uses_config_when_set() {
        let config = ProviderConfig::new("ollama".to_string(), "llama3".to_string())
            .with_api_url("https://ollama.example.com".to_string())
            .with_api_key("my-secret-key".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.api_key, Some("my-secret-key".to_string()));
    }

    #[test]
    fn no_api_key_when_unconfigured() {
        // Without config key or OLLAMA_API_KEY env, api_key should be None
        // (no auth sent to vanilla Ollama).
        // Note: this test may see Some if OLLAMA_API_KEY is set in the environment.
        let config = ProviderConfig::new("ollama".to_string(), "llama3".to_string());
        let provider = OllamaProvider::new(config);
        if std::env::var("OLLAMA_API_KEY").is_err() {
            assert_eq!(provider.api_key, None);
        }
    }

    #[test]
    fn config_api_key_takes_precedence() {
        let config = ProviderConfig::new("ollama".to_string(), "llama3".to_string())
            .with_api_url("https://ollama.example.com".to_string())
            .with_api_key("config-key".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.api_key, Some("config-key".to_string()));
    }

    #[test]
    fn loopback_local_model_drops_explicit_api_key() {
        // Locks the d6af8fc5 fix: an explicit config key is *intentionally*
        // dropped when targeting a loopback Ollama with a non-cloud model,
        // so a stray cloud Bearer can't trigger a desktop-Ollama→cloud proxy.
        let config = ProviderConfig::new("ollama".to_string(), "llama3".to_string())
            .with_api_key("would-be-cloud-bearer".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.api_key, None);
    }

    #[test]
    fn loopback_cloud_model_keeps_api_key() {
        // Cloud models (name contains "cloud") still need the Bearer so the
        // local→cloud proxy can authenticate.
        let config = ProviderConfig::new("ollama".to_string(), "llama3-cloud".to_string())
            .with_api_key("cloud-bearer".to_string());
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.api_key, Some("cloud-bearer".to_string()));
    }

    // ── num_ctx and truncation detection ─────────────────────────────────

    /// Unset by default: Ollama 0.32 sizes the window itself, and pinning a
    /// large value costs KV-cache memory a small machine may not have.
    #[test]
    fn num_ctx_is_absent_from_the_wire_unless_configured() {
        let provider = OllamaProvider::new(ProviderConfig::new(
            "ollama".to_string(),
            "lfm2.5:latest".to_string(),
        ));
        let opts = provider.build_options().expect("options are always sent");
        assert_eq!(opts.num_ctx, None);
        let json = serde_json::to_string(&opts).expect("serialize");
        assert!(!json.contains("num_ctx"), "{json}");
    }

    #[test]
    fn a_configured_num_ctx_is_serialized() {
        let opts = OllamaOptions {
            temperature: None,
            num_predict: None,
            num_ctx: Some(32_768),
        };
        let json = serde_json::to_string(&opts).expect("serialize");
        assert!(json.contains("\"num_ctx\":32768"), "{json}");
    }

    /// The prompt was truncated by the server, which drops the *front* — the
    /// system prompt and the tool contract. The model then looks like it forgot
    /// how to call tools, and emits malformed markup instead.
    #[test]
    fn a_grossly_short_prompt_eval_is_flagged_as_truncation() {
        // ~10k tokens sent, 2k evaluated.
        assert!(prompt_looks_truncated(40_000, 2_000));
    }

    #[test]
    fn a_normal_turn_is_not_flagged() {
        assert!(!prompt_looks_truncated(40_000, 9_800));
    }

    /// The char-per-token ratio is a guess, so a small prompt can differ from
    /// the estimate by a lot for innocent reasons. Only large prompts count.
    #[test]
    fn a_small_prompt_is_never_flagged() {
        assert!(!prompt_looks_truncated(1_000, 1));
    }

    /// A server that reports no count at all is unknown, not truncated.
    #[test]
    fn a_missing_count_is_not_evidence_of_truncation() {
        assert!(!prompt_looks_truncated(40_000, 0));
    }

    /// The response carries the only evidence that a prompt was truncated —
    /// there is no flag for it — so the field must survive deserialization.
    #[test]
    #[test]
    fn a_reply_cut_at_the_cap_is_distinguishable_from_a_finished_one() {
        // The bug: `done_reason` was not a field, so serde dropped it and both
        // of these parsed identically. The chat rendered the truncated one as
        // complete and the user had to type "continue" to get the rest.
        let finished = r#"{"message":{"role":"assistant","content":"hi"},"done":true,"done_reason":"stop"}"#;
        let truncated = r#"{"message":{"role":"assistant","content":"hi"},"done":true,"done_reason":"length"}"#;

        let finished: OllamaChatResponse = serde_json::from_str(finished).unwrap();
        let truncated: OllamaChatResponse = serde_json::from_str(truncated).unwrap();

        assert_eq!(
            ollama_stop_reason(finished.done_reason.as_deref()),
            Some(StopReason::Natural)
        );
        assert_eq!(
            ollama_stop_reason(truncated.done_reason.as_deref()),
            Some(StopReason::Length)
        );
        assert!(ollama_stop_reason(truncated.done_reason.as_deref())
            .expect("a reason")
            .is_truncated());
    }

    #[test]
    fn a_server_that_reports_no_reason_stays_unknown() {
        // Older servers omit `done_reason`. Absent must stay absent: reading
        // the silence as `Natural` would assert the reply is complete on no
        // evidence, which is the assumption this whole field exists to remove.
        let json = r#"{"message":{"role":"assistant","content":"hi"},"done":true}"#;
        let parsed: OllamaChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.done_reason, None);
        assert_eq!(ollama_stop_reason(parsed.done_reason.as_deref()), None);
    }

    #[test]
    fn an_unmodelled_reason_is_kept_verbatim_not_called_natural() {
        // `Other` must not be treated as a clean finish, and must not trigger
        // an auto-continue either -- it is not evidence of truncation.
        let r = ollama_stop_reason(Some("load")).expect("a reason");
        assert_eq!(r, StopReason::Other("load".to_string()));
        assert!(!r.is_truncated());
    }

    fn prompt_eval_count_is_captured() {
        let json = r#"{"message":{"role":"assistant","content":"hi"},"done":true,"prompt_eval_count":4517}"#;
        let parsed: OllamaChatResponse = serde_json::from_str(json).expect("parse");
        assert_eq!(parsed.prompt_eval_count, Some(4517));
    }

    #[test]
    fn a_response_without_the_count_still_parses() {
        let json = r#"{"message":{"role":"assistant","content":"hi"},"done":true}"#;
        let parsed: OllamaChatResponse = serde_json::from_str(json).expect("parse");
        assert_eq!(parsed.prompt_eval_count, None);
    }
}
