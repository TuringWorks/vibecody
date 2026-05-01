//! In-process Mistral.rs backend — TurboQuant-aware local inference via
//! `vibe-infer::MistralGenerator`.
//!
//! ## Build mode
//!
//! Real implementation is gated on `cfg(mistralrs_enabled)`, which the
//! crate's `build.rs` emits when **either** the user opted in with
//! `--features vibe-mistralrs` **or** the build target is macOS (where
//! Metal acceleration is bundled by default). Without that cfg, every
//! method returns [`BackendError::Unavailable`] with a recompile hint —
//! the daemon still builds and the ollama backend still works.
//!
//! ## Lifecycle
//!
//! Generators are loaded lazily on first request and cached by model id in
//! a `RwLock<HashMap<String, Arc<MistralGenerator>>>`. Loading triggers
//! hf-hub download into `~/.cache/huggingface/hub` (mistral.rs's default).
//! KV-cache mode is read from `VIBE_INFER_KV_CACHE` (see
//! [`vibe_infer::mistral::KvCacheMode::from_env`]) — set to `turboquant` to
//! exercise the native CUDA / Metal codec.
//!
//! ## Streaming
//!
//! `vibe_infer::TextGenerator` exposes a unary `generate(req) -> response`
//! API. We synthesize a 2-frame Ollama NDJSON stream from the unary result
//! (one content frame, one `done: true` frame). Real per-token streaming
//! requires going around the `TextGenerator` trait into mistralrs's native
//! streaming surface — deferred to a follow-up.

use async_trait::async_trait;
use futures::stream::BoxStream;
#[cfg(mistralrs_enabled)]
use futures::stream;

#[cfg(mistralrs_enabled)]
use super::backend::ChatMessage;
use super::backend::{
    Backend, BackendError, BackendKind, BackendResult, ChatChunk, ChatRequest,
    GenerateChunk, GenerateRequest, ModelInfo, PullProgress, PullRequest,
};

#[cfg(mistralrs_enabled)]
use std::collections::HashMap;
#[cfg(mistralrs_enabled)]
use std::sync::Arc;
#[cfg(mistralrs_enabled)]
use tokio::sync::RwLock;
#[cfg(mistralrs_enabled)]
use vibe_infer::{
    mistral::{KvCacheMode, MistralGenerator},
    ChatRole as InferChatRole, ChatMessage as InferChatMessage, ChatRequest as InferChatRequest,
    GenerationRequest, InferenceError, TextGenerator,
};

/// Apache-2.0, ungated drop-in for gated `meta-llama/*` repos. When `HF_TOKEN`
/// is missing or the user has not accepted Meta's community license, the first
/// load of a Llama model fails with a 401/403; we substitute this and continue.
/// Same ~7B class, native tool calling, no license acceptance required.
pub const UNGATED_FALLBACK_MODEL: &str = "Qwen/Qwen2.5-Coder-7B-Instruct";

/// True if the requested model id is a gated repo we know to substitute on
/// auth failure. Currently meta-llama/* (Llama 3.x family). Other vendors
/// gate too (some mistralai/* preview repos), but those aren't in our
/// default picker — extend this when they are.
fn is_gated_repo(model_id: &str) -> bool {
    model_id.starts_with("meta-llama/")
}

/// True if the upstream error string looks like an HF gating / auth failure.
/// HF Hub errors aren't typed at this layer (vibe-infer wraps them in
/// `InferenceError::Upstream(String)`), so we string-match on stable
/// fragments. Order: most specific first.
fn looks_like_gated_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("gatedrepoerror")
        || lower.contains("gated repo")
        || lower.contains("401")
        || lower.contains("403")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("access to model")
        || lower.contains("must be authenticated")
}

/// Recommended default mistralrs model for the running daemon, taking
/// `HF_TOKEN` presence into account. Surfaced via `/health` so the frontend
/// can swap its picker default before a user hits the gated 401.
pub fn recommended_default_model() -> &'static str {
    if std::env::var("HF_TOKEN").map(|s| !s.is_empty()).unwrap_or(false) {
        "meta-llama/Llama-3.1-8B-Instruct"
    } else {
        UNGATED_FALLBACK_MODEL
    }
}

/// In-process text-generation backed by `vibe-infer::MistralGenerator`.
pub struct MistralRsBackend {
    #[cfg(mistralrs_enabled)]
    cache: Arc<RwLock<HashMap<String, Arc<MistralGenerator>>>>,
    #[cfg(not(mistralrs_enabled))]
    _private: (),
}

impl Default for MistralRsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MistralRsBackend {
    pub fn new() -> Self {
        Self {
            #[cfg(mistralrs_enabled)]
            cache: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(not(mistralrs_enabled))]
            _private: (),
        }
    }
}

#[cfg(mistralrs_enabled)]
impl MistralRsBackend {
    async fn get_or_load(&self, model_id: &str) -> BackendResult<Arc<MistralGenerator>> {
        if let Some(g) = self.cache.read().await.get(model_id) {
            return Ok(Arc::clone(g));
        }
        let kv_mode = KvCacheMode::from_env();
        tracing::info!(
            "vibecli inference: loading mistralrs model {model_id} (kv_cache={kv_mode:?})"
        );
        let arc = match MistralGenerator::load_with_kv_cache(model_id, kv_mode.clone()).await {
            Ok(gen) => Arc::new(gen),
            Err(e) if is_gated_repo(model_id) && looks_like_gated_error(&e.to_string()) => {
                // Auth/gating failure on a known-gated repo. Fall back to the
                // ungated drop-in so a no-HF_TOKEN user still gets inference.
                // Cache under the *original* key so subsequent requests with the
                // same model id are served from the substitute without retrying.
                tracing::warn!(
                    "vibecli inference: gated load failed for {model_id} ({e}). \
                     Substituting {UNGATED_FALLBACK_MODEL}. \
                     Set HF_TOKEN and accept license at \
                     https://huggingface.co/{model_id} to enable Llama."
                );
                let gen = MistralGenerator::load_with_kv_cache(UNGATED_FALLBACK_MODEL, kv_mode)
                    .await
                    .map_err(map_infer_err)?;
                Arc::new(gen)
            }
            Err(e) => return Err(map_infer_err(e)),
        };
        self.cache
            .write()
            .await
            .insert(model_id.to_string(), Arc::clone(&arc));
        Ok(arc)
    }
}

#[cfg(mistralrs_enabled)]
fn map_infer_err(e: InferenceError) -> BackendError {
    match e {
        InferenceError::ModelNotFound(name, _) => BackendError::ModelNotFound(name),
        InferenceError::BackendNotEnabled(feat) => BackendError::Unavailable(format!(
            "vibe-infer feature `{feat}` not built into this binary"
        )),
        other => BackendError::Upstream(other.to_string()),
    }
}

#[cfg(mistralrs_enabled)]
fn finish_label(reason: vibe_infer::FinishReason) -> &'static str {
    match reason {
        vibe_infer::FinishReason::Stop => "stop",
        vibe_infer::FinishReason::Length => "length",
        vibe_infer::FinishReason::Error => "error",
    }
}

#[cfg(mistralrs_enabled)]
fn map_chat_messages(messages: &[ChatMessage]) -> Vec<InferChatMessage> {
    messages
        .iter()
        .map(|m| InferChatMessage {
            role: parse_role(&m.role),
            content: m.content.clone(),
        })
        .collect()
}

/// Map an Ollama-wire role string to the structured `ChatRole` mistralrs
/// understands. Unknown roles fall back to `User` — preserves user content
/// rather than dropping the turn, but flagged in tracing so future role
/// additions don't silently lose information.
#[cfg(mistralrs_enabled)]
fn parse_role(role: &str) -> InferChatRole {
    match role.to_ascii_lowercase().as_str() {
        "system" => InferChatRole::System,
        "assistant" => InferChatRole::Assistant,
        "tool" => InferChatRole::Tool,
        "user" => InferChatRole::User,
        other => {
            tracing::warn!("vibecli inference: unknown chat role `{other}` — treating as user");
            InferChatRole::User
        }
    }
}

/// Pull `num_predict` (tokens) and `temperature` out of an Ollama-style
/// `options` blob. Ollama uses `num_predict` for the cap; OpenAI uses
/// `max_tokens`. We accept either so curl examples written for the OpenAI
/// world still work.
#[cfg(mistralrs_enabled)]
fn sampler_from_options(opts: Option<&serde_json::Value>) -> (usize, f32) {
    let max_tokens = opts
        .and_then(|v| v.get("num_predict").or_else(|| v.get("max_tokens")))
        .and_then(|v| v.as_u64())
        .unwrap_or(512) as usize;
    let temperature = opts
        .and_then(|v| v.get("temperature"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.7) as f32;
    (max_tokens, temperature)
}

#[async_trait]
impl Backend for MistralRsBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Mistralrs
    }

    #[cfg(mistralrs_enabled)]
    async fn chat(
        &self,
        req: ChatRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<ChatChunk>>> {
        let gen = self.get_or_load(&req.model).await?;
        let messages = map_chat_messages(&req.messages);
        let (max_tokens, temperature) = sampler_from_options(req.options.as_ref());

        // Use the chat-aware path so each turn keeps its role and the
        // model's own template (Qwen ChatML, Llama-3 instruct, etc.) is
        // applied per message — flattening to one user prompt produced
        // doubled `user: user: ...` artifacts and broke multi-turn.
        let resp = gen
            .generate_chat(InferChatRequest {
                messages,
                max_tokens,
                temperature,
                stop: vec![],
            })
            .await
            .map_err(map_infer_err)?;

        let model = req.model.clone();
        let now = chrono::Utc::now().to_rfc3339();
        let content_frame = ChatChunk {
            model: model.clone(),
            created_at: now.clone(),
            message: ChatMessage {
                role: "assistant".into(),
                content: resp.text,
                images: None,
            },
            done: false,
            done_reason: None,
        };
        let done_frame = ChatChunk {
            model,
            created_at: now,
            message: ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                images: None,
            },
            done: true,
            done_reason: Some(finish_label(resp.finish_reason).to_string()),
        };
        Ok(Box::pin(stream::iter(vec![Ok(content_frame), Ok(done_frame)])))
    }

    #[cfg(not(mistralrs_enabled))]
    async fn chat(
        &self,
        _req: ChatRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<ChatChunk>>> {
        Err(BackendError::Unavailable(
            "mistralrs backend not built — recompile vibecli with --features vibe-mistralrs"
                .into(),
        ))
    }

    #[cfg(mistralrs_enabled)]
    async fn generate(
        &self,
        req: GenerateRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<GenerateChunk>>> {
        let gen = self.get_or_load(&req.model).await?;
        let (max_tokens, temperature) = sampler_from_options(req.options.as_ref());

        let resp = gen
            .generate(GenerationRequest {
                prompt: req.prompt,
                max_tokens,
                temperature,
                stop: vec![],
            })
            .await
            .map_err(map_infer_err)?;

        let model = req.model.clone();
        let now = chrono::Utc::now().to_rfc3339();
        let content_frame = GenerateChunk {
            model: model.clone(),
            created_at: now.clone(),
            response: resp.text,
            done: false,
            done_reason: None,
        };
        let done_frame = GenerateChunk {
            model,
            created_at: now,
            response: String::new(),
            done: true,
            done_reason: Some(finish_label(resp.finish_reason).to_string()),
        };
        Ok(Box::pin(stream::iter(vec![
            Ok(content_frame),
            Ok(done_frame),
        ])))
    }

    #[cfg(not(mistralrs_enabled))]
    async fn generate(
        &self,
        _req: GenerateRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<GenerateChunk>>> {
        Err(BackendError::Unavailable(
            "mistralrs backend not built — recompile vibecli with --features vibe-mistralrs"
                .into(),
        ))
    }

    #[cfg(mistralrs_enabled)]
    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>> {
        let cache = self.cache.read().await;
        let now = chrono::Utc::now().to_rfc3339();
        Ok(cache
            .keys()
            .map(|name| ModelInfo {
                name: name.clone(),
                modified_at: now.clone(),
                size: 0,
                backend: BackendKind::Mistralrs,
                digest: None,
            })
            .collect())
    }

    #[cfg(not(mistralrs_enabled))]
    async fn list_models(&self) -> BackendResult<Vec<ModelInfo>> {
        Ok(Vec::new())
    }

    #[cfg(mistralrs_enabled)]
    async fn pull(
        &self,
        req: PullRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<PullProgress>>> {
        // mistralrs lazy-loads on first use, so "pull" = "load and cache."
        // We surface this as a 2-frame stream (downloading → success) so
        // ollama-compat clients can poll the same shape they expect.
        let _ = self.get_or_load(&req.name).await?;
        let progress = vec![
            Ok(PullProgress {
                status: format!("loaded {} into mistralrs cache", req.name),
                digest: None,
                total: None,
                completed: None,
            }),
            Ok(PullProgress {
                status: "success".into(),
                digest: None,
                total: None,
                completed: None,
            }),
        ];
        Ok(Box::pin(stream::iter(progress)))
    }

    #[cfg(not(mistralrs_enabled))]
    async fn pull(
        &self,
        _req: PullRequest,
    ) -> BackendResult<BoxStream<'static, BackendResult<PullProgress>>> {
        Err(BackendError::Unavailable(
            "mistralrs backend not built — recompile vibecli with --features vibe-mistralrs"
                .into(),
        ))
    }

    #[cfg(mistralrs_enabled)]
    async fn show(&self, name: &str) -> BackendResult<ModelInfo> {
        let cache = self.cache.read().await;
        if cache.contains_key(name) {
            Ok(ModelInfo {
                name: name.to_string(),
                modified_at: chrono::Utc::now().to_rfc3339(),
                size: 0,
                backend: BackendKind::Mistralrs,
                digest: None,
            })
        } else {
            Err(BackendError::ModelNotFound(name.into()))
        }
    }

    #[cfg(not(mistralrs_enabled))]
    async fn show(&self, name: &str) -> BackendResult<ModelInfo> {
        Err(BackendError::ModelNotFound(name.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_gated_repo_recognizes_meta_llama() {
        assert!(is_gated_repo("meta-llama/Llama-3.1-8B-Instruct"));
        assert!(is_gated_repo("meta-llama/Llama-3.2-3B-Instruct"));
        assert!(!is_gated_repo("Qwen/Qwen2.5-Coder-7B-Instruct"));
        assert!(!is_gated_repo("microsoft/Phi-3.5-mini-instruct"));
        assert!(!is_gated_repo(""));
    }

    #[test]
    fn looks_like_gated_error_matches_hf_auth_signatures() {
        // Real HF Hub error fragments observed in the wild.
        assert!(looks_like_gated_error("GatedRepoError: Cannot access gated repo"));
        assert!(looks_like_gated_error("401 Client Error: Unauthorized"));
        assert!(looks_like_gated_error("HTTP 403 Forbidden"));
        assert!(looks_like_gated_error(
            "Access to model meta-llama/Llama-3.1-8B-Instruct is restricted"
        ));
        assert!(looks_like_gated_error(
            "you must be authenticated to access this resource"
        ));
        // Non-auth failures should not match.
        assert!(!looks_like_gated_error("connection refused"));
        assert!(!looks_like_gated_error("model file corrupt"));
        assert!(!looks_like_gated_error("CUDA out of memory"));
    }

    #[test]
    fn recommended_default_respects_hf_token() {
        // Save and restore the env var so the test doesn't pollute siblings.
        let prev = std::env::var("HF_TOKEN").ok();
        // SAFETY: tests run single-threaded by default within this module;
        // env mutation is acceptable for the duration of the test.
        unsafe {
            std::env::remove_var("HF_TOKEN");
        }
        assert_eq!(recommended_default_model(), UNGATED_FALLBACK_MODEL);

        unsafe {
            std::env::set_var("HF_TOKEN", "hf_test_dummy");
        }
        assert_eq!(recommended_default_model(), "meta-llama/Llama-3.1-8B-Instruct");

        // Restore.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("HF_TOKEN", v),
                None => std::env::remove_var("HF_TOKEN"),
            }
        }
    }
}
