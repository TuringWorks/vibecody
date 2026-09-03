//! One OpenAI-compatible provider, described by data instead of copied by hand.
//!
//! Twelve providers in this directory speak the same protocol and already share
//! the wire types in [`super::openai_compat`]. What they did not share was the
//! provider itself: each carried its own ~157-line `AIProvider` impl that
//! differed from its neighbours in **12 to 18 lines** — the display name, the
//! base URL, and the environment variable named in one error message. Measured
//! across groq/cerebras/together/fireworks/sambanova, normalising those away
//! left the files essentially identical.
//!
//! So the variation is data, and it lives in [`CompatSpec`]. Adding an
//! OpenAI-compatible provider is now a const, not a file.
//!
//! ## Why auth is part of the spec
//!
//! The existing twelve are all hosted services where a missing key is a
//! configuration error worth reporting. vLLM and LM Studio are the opposite:
//! they run on the developer's own machine and usually accept no credential at
//! all. Modelling that as "key is empty" would make every local provider report
//! itself unavailable and refuse to send. [`Auth`] makes the difference
//! explicit, so a local server is not judged by a cloud service's rules.

use super::openai_compat::{self, ChatRequest};
use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message,
    ProviderConfig, StopReasonSink,
};
use anyhow::{Context, Result};
use async_trait::async_trait;

/// How a provider expects to be authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    /// A hosted service. No key means misconfiguration: say so, and name the
    /// variable the user is expected to set.
    ApiKey { env_var: &'static str },
    /// A server on the developer's own machine. A key is accepted if given —
    /// vLLM can be started with `--api-key` — but its absence is normal and
    /// must not be reported as an error.
    LocalOptional,
}

/// Everything that distinguishes one OpenAI-compatible provider from another.
#[derive(Debug, Clone, Copy)]
pub struct CompatSpec {
    /// The provider id the rest of the stack routes on — `"cerebras"`,
    /// `"lmstudio"`. Distinct from `label`, which is prose for humans: the id
    /// has to match the daemon's `create_provider` arms and the harness
    /// profile table, and lowercasing a label to guess it would silently
    /// produce `"lm studio"`.
    pub id: &'static str,
    /// Shown to the user, e.g. `"Cerebras"`, and used in error messages.
    pub label: &'static str,
    /// Used when `ProviderConfig::api_url` is not set. Includes the version
    /// path (`/v1`) where the vendor's API has one.
    pub default_base_url: &'static str,
    pub auth: Auth,
}

impl CompatSpec {
    pub const fn cloud(
        id: &'static str,
        label: &'static str,
        base: &'static str,
        env_var: &'static str,
    ) -> Self {
        Self {
            id,
            label,
            default_base_url: base,
            auth: Auth::ApiKey { env_var },
        }
    }

    pub const fn local(id: &'static str, label: &'static str, base: &'static str) -> Self {
        Self {
            id,
            label,
            default_base_url: base,
            auth: Auth::LocalOptional,
        }
    }

    pub const fn is_local(&self) -> bool {
        matches!(self.auth, Auth::LocalOptional)
    }
}

/// vLLM's OpenAI-compatible server, `vllm serve` / `python -m vllm.entrypoints.openai.api_server`.
pub const VLLM: CompatSpec = CompatSpec::local("vllm", "vLLM", "http://localhost:8000/v1");

/// LM Studio's local server (Developer tab → Start Server).
pub const LM_STUDIO: CompatSpec =
    CompatSpec::local("lmstudio", "LM Studio", "http://localhost:1234/v1");

/// An OpenAI-compatible provider, behaviour identical to the hand-written
/// twelve, with the differences supplied by `spec`.
pub struct CompatProvider {
    spec: CompatSpec,
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl CompatProvider {
    pub fn new(spec: CompatSpec, config: ProviderConfig) -> Self {
        let display_name = format!("{} ({})", spec.label, config.model);
        Self {
            spec,
            config,
            client: openai_compat::default_http_client(),
            display_name,
        }
    }

    /// The configuration this provider was built with.
    ///
    /// Public because callers legitimately ask what model and limits a
    /// constructed provider carries — the per-provider test suites did exactly
    /// that against the field when it lived in their own module.
    pub fn config(&self) -> &ProviderConfig {
        &self.config
    }

    pub fn base_url(&self) -> String {
        self.config
            .api_url
            .clone()
            .unwrap_or_else(|| self.spec.default_base_url.to_string())
    }

    pub fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url())
    }

    /// The key to send, or an error naming what to set.
    ///
    /// A local server gets an empty string rather than an error: the header is
    /// still sent, which vLLM and LM Studio ignore when started without
    /// `--api-key`, and a user who *did* start one with a key can still supply
    /// it through the ordinary config path.
    pub fn api_key(&self) -> Result<&str> {
        match self.spec.auth {
            Auth::ApiKey { env_var } => self
                .config
                .api_key
                .as_deref()
                .with_context(|| format!("{} API key not set ({})", self.spec.label, env_var)),
            Auth::LocalOptional => Ok(self.config.api_key.as_deref().unwrap_or("")),
        }
    }

    fn make_request(
        &self,
        messages: &[Message],
        context: Option<String>,
        stream: bool,
    ) -> ChatRequest {
        let profile = self.harness_profile();
        let tools = ChatRequest::tools_for(messages, &profile);
        ChatRequest {
            model: self.config.model.clone(),
            messages: openai_compat::build_messages(messages, context),
            // An explicit setting on the provider is the caller's decision and
            // wins; the profile only fills in what the caller left open.
            temperature: self.config.temperature.or(profile.temperature),
            max_tokens: self
                .config
                .max_tokens
                .or(profile.max_output_tokens.map(|t| t as usize)),
            stream,
            parallel_tool_calls: ChatRequest::parallel_for(tools.as_ref(), &profile),
            tools,
        }
    }

    fn code_prompt(context: &CodeContext) -> Vec<Message> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        vec![
            Message {
                role: crate::provider::MessageRole::System,
                content: "You are a helpful coding assistant.".to_string(),
            },
            Message {
                role: crate::provider::MessageRole::User,
                content: prompt,
            },
        ]
    }
}

impl CompatProvider {
    /// The streaming body shared by [`AIProvider::stream_chat`] and
    /// [`AIProvider::stream_chat_reporting`].
    ///
    /// `stop` is `None` on the plain path, leaving that caller exactly as
    /// it was; the reporting path passes a sink and gets the endpoint's
    /// finish reason back out of the final SSE chunk.
    async fn stream_chat_inner(
        &self,
        messages: &[Message],
        stop: Option<StopReasonSink>,
    ) -> Result<CompletionStream> {
        let api_key = self.api_key()?;
        let request = self.make_request(messages, None, true);
        openai_compat::send_stream_request_reporting(
            &self.client,
            &self.chat_url(),
            api_key,
            &request,
            self.spec.label,
            stop,
        )
        .await
    }
}

#[async_trait]
impl AIProvider for CompatProvider {
    fn name(&self) -> &str {
        &self.display_name
    }

    /// A hosted provider is available when it has a key. A local one is
    /// available when its server answers — the same question ollama asks, and
    /// the only one that means anything for a process the user starts.
    async fn is_available(&self) -> bool {
        match self.spec.auth {
            Auth::ApiKey { .. } => self.config.api_key.is_some(),
            Auth::LocalOptional => {
                let url = format!("{}/models", self.base_url());
                matches!(self.client.get(&url).send().await, Ok(r) if r.status().is_success())
            }
        }
    }

    /// Read the window from this vendor's `/models` listing.
    ///
    /// One implementation covers every provider built from this macro, which
    /// is most of them. Vendors disagree only on the field name, and
    /// [`crate::context_window::from_models_list`] knows the spellings.
    ///
    /// A failed probe answers `None` — unknown — never a guess. The listing is
    /// unauthenticated on some vendors and keyed on others, so the key is
    /// attached when there is one and its absence is not treated as an error.
    async fn context_window(&self) -> Option<usize> {
        crate::context_window::cached(self.spec.label, &self.config.model, || async {
            let url = format!("{}/models", self.base_url());
            let mut req = self.client.get(&url);
            if let Ok(key) = self.api_key() {
                if !key.is_empty() {
                    req = req.bearer_auth(key);
                }
            }
            let body = req
                .send()
                .await
                .ok()?
                .json::<serde_json::Value>()
                .await
                .ok()?;
            crate::context_window::from_models_list(&body, &self.config.model)
        })
        .await
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        self.chat_response(&Self::code_prompt(context), None).await
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        self.stream_chat(&Self::code_prompt(context)).await
    }

    async fn chat_response(
        &self,
        messages: &[Message],
        context: Option<String>,
    ) -> Result<CompletionResponse> {
        let api_key = self.api_key()?;
        let request = self.make_request(messages, context, false);
        openai_compat::send_chat_request(
            &self.client,
            &self.chat_url(),
            api_key,
            &request,
            self.spec.label,
        )
        .await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    /// Answered by the pair's profile, like every other provider — the
    /// hardcoded `true` here predated profiles and could not be turned off for
    /// a model whose native tool calling is worse than its prose.
    fn harness_profile(&self) -> crate::harness::ModelProfile {
        crate::harness::profile_for(self.spec.id, &self.config.model)
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

    async fn chat_with_images(
        &self,
        messages: &[Message],
        _images: &[ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        self.chat(messages, context).await
    }
}

/// Declare an OpenAI-compatible provider as a named type over [`CompatProvider`].
///
/// The twelve hand-written providers each exposed a concrete type —
/// `CerebrasProvider`, `GroqProvider` — that callers and their own test suites
/// name directly. Replacing them with `CompatProvider` everywhere would be a
/// wide, mechanical edit across the workspace *and* would throw away those
/// suites, which are the only evidence the migration preserved behaviour.
///
/// So the name stays and the body goes. Each provider keeps its type and its
/// tests; what it loses is ~150 lines that differed from its neighbour's by a
/// dozen.
#[macro_export]
macro_rules! openai_compat_provider {
    ($ty:ident, $id:literal, $label:literal, $base:expr, $env:literal) => {
        /// The provider's identity, as data. Public so the daemon can describe
        /// it without constructing one.
        pub const SPEC: $crate::providers::compat::CompatSpec =
            $crate::providers::compat::CompatSpec::cloud($id, $label, $base, $env);

        #[doc = concat!("`", $label, "`, an OpenAI-compatible provider.")]
        pub struct $ty($crate::providers::compat::CompatProvider);

        impl $ty {
            pub fn new(config: $crate::provider::ProviderConfig) -> Self {
                Self($crate::providers::compat::CompatProvider::new(SPEC, config))
            }
        }

        // Gives the inherent helpers — `base_url`, `chat_url`, `api_key` — which
        // the existing test suites call directly.
        impl std::ops::Deref for $ty {
            type Target = $crate::providers::compat::CompatProvider;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        #[async_trait::async_trait]
        impl $crate::provider::AIProvider for $ty {
            fn name(&self) -> &str {
                $crate::provider::AIProvider::name(&self.0)
            }
            async fn is_available(&self) -> bool {
                self.0.is_available().await
            }
            async fn complete(
                &self,
                context: &$crate::provider::CodeContext,
            ) -> anyhow::Result<$crate::provider::CompletionResponse> {
                self.0.complete(context).await
            }
            async fn stream_complete(
                &self,
                context: &$crate::provider::CodeContext,
            ) -> anyhow::Result<$crate::provider::CompletionStream> {
                self.0.stream_complete(context).await
            }
            async fn chat_response(
                &self,
                messages: &[$crate::provider::Message],
                context: Option<String>,
            ) -> anyhow::Result<$crate::provider::CompletionResponse> {
                self.0.chat_response(messages, context).await
            }
            async fn chat(
                &self,
                messages: &[$crate::provider::Message],
                context: Option<String>,
            ) -> anyhow::Result<String> {
                self.0.chat(messages, context).await
            }
            async fn stream_chat(
                &self,
                messages: &[$crate::provider::Message],
            ) -> anyhow::Result<$crate::provider::CompletionStream> {
                self.0.stream_chat(messages).await
            }
            async fn chat_with_images(
                &self,
                messages: &[$crate::provider::Message],
                images: &[$crate::provider::ImageAttachment],
                context: Option<String>,
            ) -> anyhow::Result<String> {
                self.0.chat_with_images(messages, images, context).await
            }
            /// Delegated like every other capability: the inner
            /// `CompatProvider` is the one that builds the request, so it is
            /// the one that knows whether schemas ride along.
            fn harness_profile(&self) -> $crate::harness::ModelProfile {
                $crate::provider::AIProvider::harness_profile(&self.0)
            }
            fn advertises_native_tools(&self) -> bool {
                $crate::provider::AIProvider::advertises_native_tools(&self.0)
            }
            /// Delegated for the same reason as the rest: the inner provider
            /// holds the base URL, the key and the model id.
            async fn context_window(&self) -> Option<usize> {
                self.0.context_window().await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOUD: CompatSpec = CompatSpec::cloud(
        "testly",
        "Testly",
        "https://api.testly.ai/v1",
        "TESTLY_API_KEY",
    );

    fn config(model: &str) -> ProviderConfig {
        ProviderConfig {
            model: model.to_string(),
            ..Default::default()
        }
    }

    fn with_key(model: &str, key: &str) -> ProviderConfig {
        ProviderConfig {
            model: model.to_string(),
            api_key: Some(key.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn the_display_name_names_the_provider_and_the_model() {
        let p = CompatProvider::new(CLOUD, config("m-1"));
        assert_eq!(p.name(), "Testly (m-1)");
    }

    #[test]
    fn the_base_url_falls_back_to_the_spec() {
        let p = CompatProvider::new(CLOUD, config("m-1"));
        assert_eq!(p.base_url(), "https://api.testly.ai/v1");
        assert_eq!(p.chat_url(), "https://api.testly.ai/v1/chat/completions");
    }

    #[test]
    fn an_explicit_api_url_overrides_the_spec() {
        let cfg = ProviderConfig {
            model: "m-1".to_string(),
            api_url: Some("http://127.0.0.1:9999/v1".to_string()),
            ..Default::default()
        };
        let p = CompatProvider::new(CLOUD, cfg);
        assert_eq!(p.chat_url(), "http://127.0.0.1:9999/v1/chat/completions");
    }

    /// A hosted provider with no key is misconfigured, and the message has to
    /// name the variable — that error is the only place the user learns it.
    #[test]
    fn a_cloud_provider_without_a_key_errors_naming_the_env_var() {
        let p = CompatProvider::new(CLOUD, config("m-1"));
        let err = p.api_key().expect_err("a hosted provider needs a key");
        assert!(err.to_string().contains("TESTLY_API_KEY"), "{err}");
        assert!(err.to_string().contains("Testly"), "{err}");
    }

    /// The whole reason `Auth` exists. Judging a local server by the cloud
    /// rule would make vLLM and LM Studio refuse to send on a machine where
    /// they are running perfectly well.
    #[test]
    fn a_local_provider_without_a_key_is_not_an_error() {
        for spec in [VLLM, LM_STUDIO] {
            let p = CompatProvider::new(spec, config("m-1"));
            assert_eq!(
                p.api_key().expect("a local server needs no key"),
                "",
                "{} should accept an absent key",
                spec.label
            );
        }
    }

    #[test]
    fn a_local_provider_still_uses_a_key_when_one_is_given() {
        // `vllm serve --api-key sk-…` is a supported way to run it.
        let p = CompatProvider::new(VLLM, with_key("m-1", "sk-local"));
        assert_eq!(p.api_key().unwrap(), "sk-local");
    }

    #[test]
    fn the_local_specs_point_at_each_servers_documented_default_port() {
        assert_eq!(VLLM.default_base_url, "http://localhost:8000/v1");
        assert_eq!(LM_STUDIO.default_base_url, "http://localhost:1234/v1");
        assert!(VLLM.is_local() && LM_STUDIO.is_local());
        assert!(!CLOUD.is_local());
    }

    /// Availability for a hosted provider is answerable offline; for a local
    /// one it is not, and must not be faked. This pins only the offline half.
    #[tokio::test]
    async fn a_cloud_provider_is_available_exactly_when_it_has_a_key() {
        assert!(
            !CompatProvider::new(CLOUD, config("m-1"))
                .is_available()
                .await
        );
        assert!(
            CompatProvider::new(CLOUD, with_key("m-1", "sk-test"))
                .is_available()
                .await
        );
    }

    /// A local provider must not report itself available merely because a key
    /// is present — nothing is listening at a made-up port, and saying
    /// otherwise is how a broken setup looks configured.
    #[tokio::test]
    async fn a_local_provider_with_nothing_listening_is_unavailable() {
        let cfg = ProviderConfig {
            model: "m-1".to_string(),
            // Port 1 is reserved and never a real inference server.
            api_url: Some("http://127.0.0.1:1/v1".to_string()),
            api_key: Some("sk-irrelevant".to_string()),
            ..Default::default()
        };
        let p = CompatProvider::new(VLLM, cfg);
        assert!(!p.is_available().await);
    }
}
